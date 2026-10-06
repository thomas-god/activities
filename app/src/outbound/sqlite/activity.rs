use std::{collections::HashMap, str::FromStr};

use anyhow::anyhow;
use chrono::{DateTime, FixedOffset};
use sqlx::{
    ConnectOptions, Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

use crate::{
    domain::{
        models::{
            UserId,
            activity::{
                Activity, ActivityDuration, ActivityDurationCurve, ActivityDurationCurves,
                ActivityFeedback, ActivityId, ActivityMetric, ActivityMetrics, ActivityName,
                ActivityNaturalKey, ActivityNutrition, ActivityRpe, ActivityStartTime,
                ActivityWithParsedData, DurationCurveType, Sport, WorkoutType,
            },
            search::{SearchDocument, SearchDocumentEvent, SearchDocumentType},
        },
        ports::{
            DateTimeRange, IClock,
            activity::{
                ActivityRepository, DurationCurveEvent, GetActivityError, GetRawActivityError,
                ListActivitiesError, ListActivitiesFilters, RawActivity, RawDataRepository,
                SaveActivityError, SimilarActivityError, UpdateActivityMetricError,
            },
            search::RemainingDocuments,
        },
    },
    inbound::parser::ParseFile,
};

type ActivityRow = (
    ActivityId,
    UserId,
    Option<ActivityName>,
    ActivityStartTime,
    Option<ActivityDuration>,
    Sport,
    Option<ActivityRpe>,
    Option<WorkoutType>,
    Option<ActivityNutrition>,
    Option<ActivityFeedback>,
);

type DurationCurveRow = (
    Option<DurationCurveType>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

type SearchDocumentRow = (
    ActivityId,
    UserId,
    SearchDocumentEvent,
    String,
    chrono::DateTime<chrono::Utc>,
);

#[derive(Debug, Clone)]
pub struct SqliteActivityRepository<R, FP, C> {
    writer: SqlitePool,
    readers: SqlitePool,
    raw_data_repository: R,
    file_parser: FP,
    clock: C,
}

impl<R, FP, C> SqliteActivityRepository<R, FP, C> {
    pub async fn new(
        url: &str,
        raw_data_repository: R,
        file_parser: FP,
        clock: C,
    ) -> Result<Self, sqlx::Error> {
        let writer_options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .log_slow_statements(
                log::LevelFilter::Warn,
                std::time::Duration::from_millis(100),
            )
            .journal_mode(SqliteJournalMode::Wal);

        let writer = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(writer_options)
            .await?;

        // Run migrations using writer pool
        sqlx::migrate!("migrations/activities").run(&writer).await?;

        let readers_options = SqliteConnectOptions::from_str(url)?
            .journal_mode(SqliteJournalMode::Wal)
            .read_only(true);
        let readers = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(readers_options)
            .await?;

        Ok(Self {
            writer,
            readers,
            raw_data_repository,
            file_parser,
            clock,
        })
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn metric_rowid(&self, metric: &ActivityMetric) -> Result<i64, anyhow::Error> {
        if let Some(rowid) = sqlx::query_scalar::<_, i64>(
            "
            SELECT rowid FROM t_activities_metrics WHERE metric = ?1 LIMIT 1;
        ",
        )
        .bind(metric)
        .fetch_optional(&self.readers)
        .await
        .map_err(|err| anyhow!(err))?
        {
            return Ok(rowid);
        }

        if let Some(rowid) = sqlx::query_scalar::<_, i64>(
            "
            INSERT INTO t_activities_metrics (metric)
            VALUES (?1)
            ON CONFLICT (metric) DO NOTHING
            RETURNING rowid;
        ",
        )
        .bind(metric)
        .fetch_optional(&self.writer)
        .await
        .map_err(|err| anyhow!(err))?
        {
            return Ok(rowid);
        };

        Err(anyhow!(
            "Unable to insert {:} into t_activities_metrics",
            metric
        ))
    }

    #[tracing::instrument(skip_all, err)]
    async fn list_user_activities(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> Result<Vec<Activity>, ListActivitiesError> {
        let mut builder = sqlx::QueryBuilder::<'_, Sqlite>::new(
               "SELECT id, user_id, name, start_time, duration, sport, rpe, workout_type, nutrition, feedback
               FROM t_activities_v2",
           );
        builder.push(" WHERE user_id = ").push_bind(user);

        if let Some(date_range) = filters.date_range() {
            builder
                .push(" AND start_time >= ")
                .push_bind(date_range.start());
            builder
                .push(" AND start_time < ")
                .push_bind(date_range.end());
        }

        builder.push("ORDER BY start_time DESC ");

        if let Some(limit) = *filters.limit() {
            builder.push("LIMIT ").push_bind(limit as i64);
        }

        let query = builder.build_query_as::<'_, ActivityRow>();

        query
            .fetch_all(&self.readers)
            .await
            .map_err(|err| ListActivitiesError::Unknown(anyhow!(err)))
            .map(|rows| {
                rows.into_iter()
                    .map(
                        |(
                            id,
                            user_id,
                            name,
                            start_time,
                            duration,
                            sport,
                            rpe,
                            workout_type,
                            nutrition,
                            feedback,
                        )| {
                            Activity::new(
                                id,
                                user_id,
                                name,
                                start_time,
                                duration.unwrap_or_default(),
                                sport,
                                rpe,
                                workout_type,
                                nutrition,
                                feedback,
                            )
                        },
                    )
                    .collect()
            })
    }
}

impl<R, FP, C> SqliteActivityRepository<R, FP, C>
where
    R: RawDataRepository,
    FP: ParseFile,
    C: IClock,
{
    #[tracing::instrument(skip_all, err)]
    async fn load_timeseries(
        &self,
        id: &ActivityId,
        activity: Activity,
    ) -> Result<ActivityWithParsedData, anyhow::Error> {
        let raw_data = match self.raw_data_repository.get_raw_data(id).await {
            Ok(raw_data) => raw_data,
            Err(err) => return Err(anyhow!(err)),
        };

        let extension = raw_data
            .extension()
            .try_into()
            .map_err(|_| anyhow!("Unsupported file format: {}", raw_data.extension()))?;

        let parsed_content = match self
            .file_parser
            .try_bytes_into_domain(&extension, raw_data.raw_content())
        {
            Ok(parsed_content) => parsed_content,
            Err(err) => return Err(anyhow!(err)),
        };

        let duration_curves = self.load_duration_curves(id, activity.user()).await?;

        Ok(ActivityWithParsedData::new(
            activity,
            parsed_content.timeseries().clone(),
            parsed_content.statistics().clone(),
            duration_curves,
        ))
    }

    async fn save_search_document(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        document: SearchDocument,
    ) -> Result<(), anyhow::Error> {
        sqlx::query(
            "
            INSERT INTO t_outbox_activity_search (activity_id, user, event, content, occurred_at)
            VALUES (?1, ?2, ?3, ?4, ?5);",
        )
        .bind(document.document_id())
        .bind(document.user())
        .bind(document.event().to_string())
        .bind(document.content())
        .bind(document.occurred_at())
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|err| {
            anyhow!(
                "Unable to save activity search document {}. {err}",
                document.document_id()
            )
        })
    }

    async fn save_duration_curve(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        activity: &ActivityId,
        user: &UserId,
        date: &ActivityStartTime,
        curve: &ActivityDurationCurve,
    ) -> Result<(), anyhow::Error> {
        let rows_affected = sqlx::query(
            "
            INSERT INTO t_duration_curves
                (
                    activity_id, user_id, type, date,
                    secs_5, secs_10, secs_30,
                    mins_1, mins_2, mins_5, mins_10, mins_20, mins_30,
                    hours_1, hours_2, hours_5
                )
            VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16
            )
            ON CONFLICT (activity_id, user_id, type) DO NOTHING;", // reflect duration curve immutability
        )
        .bind(activity)
        .bind(user)
        .bind(curve.curve_type())
        .bind(date.datetime())
        .bind(curve.values()[0])
        .bind(curve.values()[1])
        .bind(curve.values()[2])
        .bind(curve.values()[3])
        .bind(curve.values()[4])
        .bind(curve.values()[5])
        .bind(curve.values()[6])
        .bind(curve.values()[7])
        .bind(curve.values()[8])
        .bind(curve.values()[9])
        .bind(curve.values()[10])
        .bind(curve.values()[11])
        .execute(&mut **tx)
        .await
        .map(|result| result.rows_affected())
        .map_err(|err| {
            anyhow!(
                "Unable to save duration curve {} for activity {}. {err}",
                curve.curve_type(),
                activity
            )
        })?;

        // Only notify the training service when a new curve was actually inserted: curves
        // are immutable, conflicting saves are ignored.
        if rows_affected > 0 {
            self.duration_curve_to_outbox(tx, activity, user, DurationCurveEvent::Created)
                .await
        } else {
            Ok(())
        }
    }

    async fn save_null_duration_curve(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        activity: &ActivityId,
        user: &UserId,
        date: &ActivityStartTime,
    ) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO t_duration_curves (activity_id, user_id, type, date)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT (activity_id, user_id) WHERE type IS NULL DO NOTHING;", // reflect duration curve immutability
        )
        .bind(activity)
        .bind(user)
        .bind(None::<DurationCurveType>)
        .bind(date.datetime())
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|err| {
            anyhow!(
                "Unable to save null duration curve for activity {}. {err}",
                activity
            )
        })
        // For null-curves no need to write them to the duration curve outbox.
    }

    async fn load_duration_curves(
        &self,
        activity: &ActivityId,
        user: &UserId,
    ) -> Result<ActivityDurationCurves, anyhow::Error> {
        let rows = sqlx::query_as::<_, DurationCurveRow>(
            "SELECT type, secs_5, secs_10, secs_30,
                    mins_1, mins_2, mins_5, mins_10, mins_20, mins_30,
                    hours_1, hours_2, hours_5
            FROM t_duration_curves
            WHERE activity_id = ?1 AND user_id = ?2;",
        )
        .bind(activity)
        .bind(user)
        .fetch_all(&self.readers)
        .await
        .map_err(|err| {
            anyhow!(
                "Unable to load duration curves for activity {}. {err}",
                activity
            )
        })?;

        // Rows with a NULL type mark activities that were processed but have no valid
        // duration curve; they yield no curve.
        let curves = rows
            .into_iter()
            .filter_map(
                |(
                    curve_type,
                    secs_5,
                    secs_10,
                    secs_30,
                    mins_1,
                    mins_2,
                    mins_5,
                    mins_10,
                    mins_20,
                    mins_30,
                    hours_1,
                    hours_2,
                    hours_5,
                )| {
                    curve_type.map(|curve_type| {
                        ActivityDurationCurve::new(
                            curve_type,
                            [
                                secs_5.map(|value| value as f32),
                                secs_10.map(|value| value as f32),
                                secs_30.map(|value| value as f32),
                                mins_1.map(|value| value as f32),
                                mins_2.map(|value| value as f32),
                                mins_5.map(|value| value as f32),
                                mins_10.map(|value| value as f32),
                                mins_20.map(|value| value as f32),
                                mins_30.map(|value| value as f32),
                                hours_1.map(|value| value as f32),
                                hours_2.map(|value| value as f32),
                                hours_5.map(|value| value as f32),
                            ],
                        )
                    })
                },
            )
            .collect();

        Ok(ActivityDurationCurves::new(curves))
    }

    async fn delete_duration_curves(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        activity: &ActivityId,
        user: &UserId,
    ) -> Result<(), anyhow::Error> {
        sqlx::query(
            "DELETE FROM t_duration_curves
             WHERE activity_id = ?1 AND user_id = ?2;",
        )
        .bind(activity)
        .bind(user)
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|err| {
            anyhow!(
                "Unable to delete duration curves for activity {}. {err}",
                activity
            )
        })?;

        self.duration_curve_to_outbox(tx, activity, user, DurationCurveEvent::Deleted)
            .await
    }

    async fn duration_curve_to_outbox(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        activity: &ActivityId,
        user: &UserId,
        event: DurationCurveEvent,
    ) -> Result<(), anyhow::Error> {
        sqlx::query(
            "INSERT INTO t_outbox_duration_curve (activity_id, user_id, event, occurred_at)
            VALUES (?1, ?2, ?3, ?4);",
        )
        .bind(activity)
        .bind(user)
        .bind(event)
        .bind(self.clock.now())
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|err| {
            anyhow!(
                "Unable to save null duration curve for activity {}. {err}",
                activity
            )
        })
    }
}

impl<R, FP, C> ActivityRepository for SqliteActivityRepository<R, FP, C>
where
    R: RawDataRepository,
    FP: ParseFile,
    C: IClock,
{
    #[tracing::instrument(skip_all, err)]
    async fn save_activity(
        &self,
        activity: &ActivityWithParsedData,
    ) -> Result<(), SaveActivityError> {
        let mut tx = self
            .writer
            .begin()
            .await
            .map_err(|err| SaveActivityError::Unknown(err.into()))?;

        sqlx::query(
            "INSERT INTO t_activities_v2 (
                id, user_id, name, start_time, duration, sport, natural_key, rpe, workout_type, nutrition, feedback
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
            )
            ON CONFLICT (id)
            DO UPDATE SET
                name=excluded.name,
                rpe=excluded.rpe,
                workout_type=excluded.workout_type,
                nutrition=excluded.nutrition,
                feedback=excluded.feedback;",
        )
        .bind(activity.id())
        .bind(activity.user())
        .bind(activity.name())
        .bind(activity.start_time().datetime())
        .bind(activity.duration())
        .bind(activity.sport())
        .bind(activity.natural_key())
        .bind(activity.rpe())
        .bind(activity.workout_type())
        .bind(activity.nutrition())
        .bind(activity.feedback())
        .execute(&mut *tx)
        .await
        .map(|_| ())
        .map_err(|err| {
            SaveActivityError::Unknown(anyhow!("Unable to save activity {}. {err}", activity.id()))
        })?;

        let search_document = activity
            .activity()
            .to_search_document(SearchDocumentEvent::Updated, self.clock.now());

        self.save_search_document(&mut tx, search_document).await?;

        for curve in activity.duration_curves().iter() {
            self.save_duration_curve(
                &mut tx,
                activity.id(),
                activity.user(),
                activity.start_time(),
                curve,
            )
            .await?;
        }

        if activity.duration_curves().is_empty() {
            self.save_null_duration_curve(
                &mut tx,
                activity.id(),
                activity.user(),
                activity.start_time(),
            )
            .await?;
        }

        tx.commit()
            .await
            .map_err(|err| SaveActivityError::Unknown(err.into()))
    }

    #[tracing::instrument(skip_all, err)]
    async fn update_activity(&self, activity: &Activity) -> Result<(), SaveActivityError> {
        sqlx::query(
            "UPDATE t_activities_v2
                SET
                    name = ?3,
                    rpe = ?4,
                    workout_type = ?5,
                    nutrition = ?6,
                    feedback = ?7
                WHERE id = ?1 AND user_id = ?2;",
        )
        .bind(activity.id())
        .bind(activity.user())
        .bind(activity.name())
        .bind(activity.rpe())
        .bind(activity.workout_type())
        .bind(activity.nutrition())
        .bind(activity.feedback())
        .execute(&self.writer)
        .await
        .map(|_| ())
        .map_err(|err| {
            SaveActivityError::Unknown(anyhow!(
                "Unable to update activity {}. {err}",
                activity.id()
            ))
        })
    }

    #[tracing::instrument(skip_all, err)]
    async fn delete_activity(
        &self,
        user: &UserId,
        activity: &ActivityId,
    ) -> Result<(), anyhow::Error> {
        let mut tx = self.writer.begin().await.map_err(|err| anyhow!(err))?;

        sqlx::query("DELETE FROM t_activities_v2 WHERE id = ?1")
            .bind(activity)
            .execute(&mut *tx)
            .await
            .map(|_| ())
            .map_err(|err| anyhow!("Unable to delete activity {}. {err}", activity))?;

        let search_document = SearchDocument::new(
            SearchDocumentType::Activity,
            activity.to_string(),
            user.clone(),
            SearchDocumentEvent::Deleted,
            String::default(),
            self.clock.now(),
        );

        self.save_search_document(&mut tx, search_document).await?;

        self.delete_duration_curves(&mut tx, activity, user).await?;

        tx.commit().await.map_err(|err| anyhow!(err))
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_activity(
        &self,
        user: &UserId,
        id: &ActivityId,
    ) -> Result<Option<Activity>, GetActivityError> {
        match sqlx::query_as::<_, ActivityRow>(
            "SELECT id, user_id, name, start_time, duration, sport, rpe, workout_type, nutrition, feedback
            FROM t_activities_v2
            WHERE id = ?1 AND user_id = ?2
            LIMIT 1;",
        )
        .bind(id)
        .bind(user)
        .fetch_one(&self.readers)
        .await
        {
            Ok((id, user_id, name, start_time, duration , sport, rpe, workout_type, nutrition, feedback)) => {

                Ok(Some(Activity::new(
                    id,
                    user_id,
                    name,
                    start_time,
                    duration.unwrap_or_default(),
                    sport,
                    rpe,
                    workout_type,
                    nutrition,
                    feedback,
                )))
            }
            Err(sqlx::Error::RowNotFound) => {
                Err(GetActivityError::ActivityDoesNotExist(id.clone()))
            }
            Err(err) => Err(GetActivityError::Unknown(anyhow!(err))),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_activity_with_metrics(
        &self,
        user: &UserId,
        id: &ActivityId,
        metrics: &[ActivityMetric],
    ) -> Result<Option<(Activity, ActivityMetrics)>, GetActivityError> {
        let mut builder = sqlx::QueryBuilder::<'_, Sqlite>::new(
            "
        SELECT
            t_activities_v2.id,
            t_activities_metrics.metric,
            t_activities_metrics_values.value
        FROM t_activities_v2
        JOIN t_activities_metrics_values
            ON t_activities_metrics_values.activity_rowid = t_activities_v2.rowid
        JOIN t_activities_metrics
            ON t_activities_metrics_values.metric_rowid = t_activities_metrics.rowid",
        );

        builder.push(" WHERE t_activities_v2.id = ").push_bind(id);
        builder
            .push(" AND t_activities_v2.user_id = ")
            .push_bind(user);

        builder.push(" AND t_activities_metrics.metric IN (");
        for (idx, metric) in metrics.iter().enumerate() {
            builder.push(" ").push_bind(metric);
            if idx < metrics.len() - 1 {
                builder.push(",");
            }
        }
        builder.push(") ");

        let query = builder.build_query_as::<'_, (ActivityId, ActivityMetric, Option<f64>)>();
        let mut metrics_values: Vec<(ActivityMetric, Option<f64>)> = Vec::new();
        for (_activity, metric, value) in query
            .fetch_all(&self.readers)
            .await
            .map_err(|err| GetActivityError::Unknown(anyhow!(err)))?
        {
            metrics_values.push((metric, value));
        }

        let Some(activity) = self.get_activity(user, id).await? else {
            return Err(GetActivityError::ActivityDoesNotExist(id.clone()));
        };

        Ok(Some((
            activity,
            ActivityMetrics::new(HashMap::from_iter(metrics_values)),
        )))
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_activity_with_parsed_data(
        &self,
        user: &UserId,
        id: &ActivityId,
    ) -> Result<Option<ActivityWithParsedData>, GetActivityError> {
        let activity = self
            .get_activity(user, id)
            .await?
            .ok_or_else(|| GetActivityError::ActivityDoesNotExist(id.clone()))?;

        let activity_with_parsed_data = match self.load_timeseries(id, activity).await {
            Ok(value) => value,
            Err(err) => return Err(GetActivityError::Unknown(anyhow!(err))),
        };

        Ok(Some(activity_with_parsed_data))
    }

    #[tracing::instrument(skip_all, err)]
    async fn list_activity_documents(
        &self,
        batch_size: i64,
        page: i64,
    ) -> Result<(Vec<SearchDocument>, RemainingDocuments), anyhow::Error> {
        let limit = batch_size + 1; // Extra sentinel row to detect if there is another page after
        let offset = page * batch_size;
        let rows = sqlx::query_as::<_, ActivityRow>("
            SELECT id, user_id, name, start_time, duration, sport, rpe, workout_type, nutrition, feedback
            FROM t_activities_v2
            ORDER BY rowid
            LIMIT ?1 OFFSET ?2;"
        )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.readers)
            .await?;

        let documents_remaining = RemainingDocuments::from(rows.len() as i64 > batch_size);

        // The extra row was only fetched to detect whether more documents remain: it is not part
        // of this page.
        let documents = rows
            .into_iter()
            .take(batch_size as usize)
            .map(
                |(
                    id,
                    user_id,
                    name,
                    start_time,
                    duration,
                    sport,
                    rpe,
                    workout_type,
                    nutrition,
                    feedback,
                )| {
                    Activity::new(
                        id,
                        user_id,
                        name,
                        start_time,
                        duration.unwrap_or_default(),
                        sport,
                        rpe,
                        workout_type,
                        nutrition,
                        feedback,
                    )
                    .to_search_document(SearchDocumentEvent::Updated, self.clock.now())
                },
            )
            .collect();

        Ok((documents, documents_remaining))
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_raw_activity(
        &self,
        user: &UserId,
        activity: &ActivityId,
    ) -> Result<RawActivity, GetRawActivityError> {
        let Some(_) = sqlx::query_as::<_, (ActivityId,)>(
            "SELECT id
            FROM t_activities_v2
            WHERE user_id = ?1 and id = ?2
            LIMIT 1;",
        )
        .bind(user)
        .bind(activity)
        .fetch_optional(&self.readers)
        .await
        .map_err(|err| GetRawActivityError::Unknown(anyhow!(err)))?
        else {
            return Err(GetRawActivityError::ActivityDoesNotExist(activity.clone()));
        };

        let content = self
            .raw_data_repository
            .get_raw_data(activity)
            .await
            .map_err(|err| GetRawActivityError::Unknown(anyhow!(err)))?;

        Ok(RawActivity::new(
            format!("{}.{}", activity, content.extension()),
            content.raw_content(),
        ))
    }

    #[tracing::instrument(skip_all, err)]
    async fn list_all_raw_activities(
        &self,
        user: &UserId,
    ) -> Result<Vec<RawActivity>, ListActivitiesError> {
        let activities: Vec<ActivityId> = sqlx::query_as::<_, (ActivityId,)>(
            "SELECT id
            FROM t_activities_v2
            WHERE user_id = ?1;",
        )
        .bind(user)
        .fetch_all(&self.readers)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(id,)| id)
        .collect();

        let mut files = Vec::new();
        for id in activities {
            if let Ok(res) = self.raw_data_repository.get_raw_data(&id).await {
                files.push(RawActivity::new(
                    format!("{id}.{}", res.extension()),
                    res.raw_content(),
                ));
            }
        }

        Ok(files)
    }

    #[tracing::instrument(skip_all, err)]
    async fn list_activities_with_parsed_data(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> Result<Vec<ActivityWithParsedData>, ListActivitiesError> {
        let activities = self.list_user_activities(user, filters).await?;

        let mut res = vec![];
        for activity in activities.into_iter() {
            let Ok(activity_with_parsed_data) =
                self.load_timeseries(&activity.id().clone(), activity).await
            else {
                continue;
            };
            res.push(activity_with_parsed_data);
        }
        Ok(res)
    }

    #[tracing::instrument(skip_all, err)]
    async fn update_activity_metric(
        &self,
        activity: &ActivityId,
        metric: &ActivityMetric,
        value: &Option<f64>,
    ) -> Result<(), UpdateActivityMetricError> {
        let activity_rowid = sqlx::query_scalar::<_, i64>(
            "
            SELECT rowid FROM t_activities_v2 WHERE id = ?1 LIMIT 1;
        ",
        )
        .bind(activity)
        .fetch_one(&self.readers)
        .await
        .map_err(|_err| UpdateActivityMetricError::ActivityDoesNotExist(activity.clone()))?;

        let metric_rowid = self.metric_rowid(metric).await?;

        sqlx::query(
            "INSERT INTO t_activities_metrics_values
            (activity_rowid, metric_rowid, value)
            VALUES (?1, ?2, ?3)
            ON CONFLICT (activity_rowid, metric_rowid)
            DO UPDATE SET value=excluded.value;",
        )
        .bind(activity_rowid)
        .bind(metric_rowid)
        .bind(value)
        .execute(&self.writer)
        .await
        .map(|_| ())
        .map_err(|err| match err {
            sqlx::Error::Database(db_error) if db_error.is_foreign_key_violation() => {
                UpdateActivityMetricError::ActivityDoesNotExist(activity.clone())
            }
            _ => UpdateActivityMetricError::Unknown(anyhow!(err)),
        })
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_activities_with_metrics(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
        metrics: &[ActivityMetric],
    ) -> Result<Vec<(Activity, ActivityMetrics)>, ListActivitiesError> {
        let mut builder = sqlx::QueryBuilder::<'_, Sqlite>::new("
        SELECT
            t_activities_v2.id,
            t_activities_metrics.metric,
            t_activities_metrics_values.value
        FROM t_activities_v2
        JOIN t_activities_metrics_values ON t_activities_metrics_values.activity_rowid = t_activities_v2.rowid
        JOIN t_activities_metrics ON t_activities_metrics_values.metric_rowid = t_activities_metrics.rowid");

        builder
            .push(" WHERE t_activities_v2.user_id = ")
            .push_bind(user);

        if let Some(date_range) = filters.date_range() {
            builder
                .push(" AND t_activities_v2.start_time >= ")
                .push_bind(date_range.start());
            builder
                .push(" AND t_activities_v2.start_time < ")
                .push_bind(date_range.end());
        }

        builder.push(" AND t_activities_metrics.metric IN (");
        for (idx, metric) in metrics.iter().enumerate() {
            builder.push(" ").push_bind(metric);
            if idx < metrics.len() - 1 {
                builder.push(",");
            }
        }
        builder.push(") ");
        let query = builder.build_query_as::<'_, (ActivityId, ActivityMetric, Option<f64>)>();
        let mut metrics_values: HashMap<ActivityId, Vec<(ActivityMetric, Option<f64>)>> =
            HashMap::new();
        for (activity, metric, value) in query
            .fetch_all(&self.readers)
            .await
            .map_err(|err| ListActivitiesError::Unknown(anyhow!(err)))?
        {
            match metrics_values.get_mut(&activity) {
                Some(vals) => vals.push((metric, value)),
                None => {
                    metrics_values.insert(activity, vec![(metric, value)]);
                }
            }
        }

        let mut res = vec![];
        for activity in self.list_user_activities(user, filters).await? {
            let metrics = metrics_values.remove(activity.id()).unwrap_or_default();
            res.push((activity, ActivityMetrics::new(HashMap::from_iter(metrics))));
        }

        Ok(res)
    }

    #[tracing::instrument(skip_all, err)]
    async fn similar_activity_exists(
        &self,
        natural_key: &ActivityNaturalKey,
    ) -> Result<bool, SimilarActivityError> {
        match sqlx::query("SELECT natural_key FROM t_activities_v2 WHERE natural_key = ?1;")
            .bind(natural_key)
            .fetch_optional(&self.readers)
            .await
        {
            Ok(row) => Ok(row.is_some()),
            Err(sqlx::Error::RowNotFound) => Ok(false),
            Err(err) => Err(SimilarActivityError::Unknown(anyhow!(err))),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_user_history_date_range(
        &self,
        user: &UserId,
    ) -> Result<Option<crate::domain::ports::DateTimeRange>, anyhow::Error> {
        // Option<DateTime<FixedOffset>> because MIN/MAX(...) return NULL if the set is empty
        match sqlx::query_as::<_, (Option<DateTime<FixedOffset>>, Option<DateTime<FixedOffset>>)>(
            "
        SELECT MIN(start_time), MAX(start_time)
        FROM t_activities_v2
        WHERE user_id = ?1;",
        )
        .bind(user)
        .fetch_optional(&self.readers)
        .await
        {
            Ok(Some((Some(start), Some(end)))) => Ok(Some(DateTimeRange::new(start, Some(end)))),
            Ok(Some(_)) => Ok(None),
            Ok(None) => Ok(None),
            Err(err) => Err(anyhow!(
                "Unable to get history date range for user {}. {err}",
                user
            )),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_outbox_documents_to_process(&self) -> Result<Vec<SearchDocument>, anyhow::Error> {
        sqlx::query_as::<_, SearchDocumentRow>(
            "SELECT activity_id, user, event, content, occurred_at
            FROM t_outbox_activity_search
            WHERE processed_at IS NULL;",
        )
        .fetch_all(&self.readers)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(activity, user, event, content, occurred_at)| {
                    SearchDocument::new(
                        SearchDocumentType::Activity,
                        activity.to_string(),
                        user,
                        event,
                        content,
                        occurred_at,
                    )
                })
                .collect::<Vec<SearchDocument>>()
        })
        .map_err(|err| anyhow!(err))
    }

    #[tracing::instrument(skip_all, err)]
    async fn mark_outbox_document_as_processed(
        &self,
        document: &SearchDocument,
        processed_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), anyhow::Error> {
        sqlx::query(
            "UPDATE t_outbox_activity_search SET processed_at = ?1
            WHERE activity_id = ?2 AND event = ?3 AND content = ?4 AND occurred_at = ?5;",
        )
        .bind(processed_at)
        .bind(document.document_id())
        .bind(document.event())
        .bind(document.content())
        .bind(document.occurred_at())
        .execute(&self.writer)
        .await
        .map(|_| ())
        .map_err(|err| anyhow!(err))
    }
}

#[cfg(test)]
mod test_sqlite_activity_repository {

    use std::collections::HashMap;

    use chrono::NaiveDate;
    use rand::random_range;
    use tempfile::NamedTempFile;

    use crate::{
        clock::{Clock, clock_test_utils::FakeClock},
        domain::{
            models::{
                UserId,
                activity::{
                    ActiveTime, ActivityDuration, ActivityDurationCurve, ActivityDurationCurves,
                    ActivityPatch, ActivityStartTime, ActivityStatistics, ActivityTimeseries,
                    BonkStatus, DurationCurveType, Sport, Timeseries, TimeseriesActiveTime,
                    TimeseriesMetric, TimeseriesTime, TimeseriesValue,
                },
            },
            ports::{
                DateRange,
                activity::{GetRawDataError, RawContent, test_utils::MockRawDataRepository},
            },
        },
        inbound::parser::{ParseBytesError, ParsedFileContent, test_utils::MockFileParser},
    };

    use super::*;

    #[tokio::test]
    async fn test_init_table() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");

        sqlx::query("select count(*) from t_activities_v2;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
    }

    fn build_activity() -> ActivityWithParsedData {
        ActivityWithParsedData::new(
            Activity::new_empty(
                ActivityId::new(),
                UserId::test_default(),
                ActivityStartTime::from_timestamp(random_range(100..1200)).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
            ),
            ActivityTimeseries::default(),
            ActivityStatistics::default(),
            ActivityDurationCurves::default(),
        )
    }

    fn build_activity_starting_at(start: &DateTime<FixedOffset>) -> ActivityWithParsedData {
        ActivityWithParsedData::new(
            Activity::new_empty(
                ActivityId::new(),
                UserId::test_default(),
                ActivityStartTime::new(*start),
                ActivityDuration::default(),
                Sport::Cycling,
            ),
            ActivityTimeseries::default(),
            ActivityStatistics::default(),
            ActivityDurationCurves::default(),
        )
    }

    #[tokio::test]
    async fn test_save_activity() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();

        repository
            .save_activity(&activity)
            .await
            .expect("Should have succeed");

        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_activities_v2;")
                .fetch_one(&repository.readers)
                .await
                .unwrap(),
            1
        );
    }

    fn build_duration_curve(curve_type: DurationCurveType) -> ActivityDurationCurve {
        ActivityDurationCurve::new(
            curve_type,
            [
                Some(1.),
                Some(2.),
                Some(3.),
                Some(4.),
                Some(5.),
                Some(6.),
                Some(7.),
                Some(8.),
                Some(9.),
                Some(10.),
                Some(11.),
                None,
            ],
        )
    }

    async fn test_repository() -> (
        SqliteActivityRepository<
            crate::domain::ports::activity::test_utils::MockRawDataRepository,
            crate::inbound::parser::test_utils::MockFileParser,
            Clock,
        >,
        NamedTempFile,
    ) {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");

        // The db file must be kept alive for the readers pool to be able to connect.
        (repository, db_file)
    }

    #[tokio::test]
    async fn test_save_duration_curve() {
        let (repository, _db_file) = test_repository().await;

        let activity_id = ActivityId::new();
        let user = UserId::test_default();
        let date = ActivityStartTime::from_timestamp(1000).unwrap();
        let curve = build_duration_curve(DurationCurveType::Power);

        let mut tx = repository.writer.begin().await.unwrap();
        repository
            .save_duration_curve(&mut tx, &activity_id, &user, &date, &curve)
            .await
            .expect("should save the curve");
        tx.commit().await.unwrap();

        let row: (ActivityId, UserId, DurationCurveType, ActivityStartTime) =
            sqlx::query_as("select activity_id, user_id, type, date from t_duration_curves;")
                .fetch_one(&repository.readers)
                .await
                .unwrap();
        assert_eq!(row.0, activity_id);
        assert_eq!(row.1, user);
        assert_eq!(row.2, DurationCurveType::Power);
        assert_eq!(row.3, date);

        let values: (
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
        ) = sqlx::query_as(
            "select secs_5, secs_10, secs_30, mins_1, mins_2, mins_5, mins_10, mins_20, mins_30, hours_1, hours_2, hours_5 from t_duration_curves;",
        )
        .fetch_one(&repository.readers)
        .await
        .unwrap();

        let actual = [
            values.0, values.1, values.2, values.3, values.4, values.5, values.6, values.7,
            values.8, values.9, values.10, values.11,
        ];
        for (actual, expected) in actual.iter().zip(curve.values().iter()) {
            assert_eq!(*actual, expected.map(|value| value as f64));
        }
    }

    #[tokio::test]
    async fn test_save_duration_curve_pace() {
        let (repository, _db_file) = test_repository().await;

        let activity_id = ActivityId::new();
        let user = UserId::test_default();
        let date = ActivityStartTime::from_timestamp(1000).unwrap();
        // A curve without any value: all columns should be saved as NULL.
        let curve = ActivityDurationCurve::new(DurationCurveType::Pace, [None; 12]);

        let mut tx = repository.writer.begin().await.unwrap();
        repository
            .save_duration_curve(&mut tx, &activity_id, &user, &date, &curve)
            .await
            .expect("should save the curve");
        tx.commit().await.unwrap();

        let row: (DurationCurveType, i64) =
            sqlx::query_as("select type, coalesce(secs_5, -1) from t_duration_curves;")
                .fetch_one(&repository.readers)
                .await
                .unwrap();
        assert_eq!(row.0, DurationCurveType::Pace);
        assert_eq!(row.1, -1); // NULL marker
    }

    #[tokio::test]
    async fn test_save_duration_curves_same_activity() {
        let (repository, _db_file) = test_repository().await;

        let activity_id = ActivityId::new();
        let user = UserId::test_default();
        let date = ActivityStartTime::from_timestamp(1000).unwrap();

        let mut tx = repository.writer.begin().await.unwrap();
        for curve_type in [DurationCurveType::Power, DurationCurveType::Pace] {
            repository
                .save_duration_curve(
                    &mut tx,
                    &activity_id,
                    &user,
                    &date,
                    &build_duration_curve(curve_type),
                )
                .await
                .expect("should save the curve");
        }
        tx.commit().await.unwrap();

        // Both curves are saved for the same activity, without conflicting on the
        // (activity_id, user_id, type) unique constraint.
        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_duration_curves;")
                .fetch_one(&repository.readers)
                .await
                .unwrap(),
            2
        );
    }

    #[tokio::test]
    async fn test_save_duration_curve_is_immutable() {
        let (repository, _db_file) = test_repository().await;

        let activity_id = ActivityId::new();
        let user = UserId::test_default();
        let date = ActivityStartTime::from_timestamp(1000).unwrap();
        let curve = build_duration_curve(DurationCurveType::Power);

        let mut tx = repository.writer.begin().await.unwrap();
        repository
            .save_duration_curve(&mut tx, &activity_id, &user, &date, &curve)
            .await
            .expect("should save the curve");

        // Saving a different curve for the same (activity, user, type) should be ignored.
        let other_curve = ActivityDurationCurve::new(DurationCurveType::Power, [Some(100.); 12]);
        repository
            .save_duration_curve(&mut tx, &activity_id, &user, &date, &other_curve)
            .await
            .expect("should ignore the conflicting curve");
        tx.commit().await.unwrap();

        // Only one row exists and it still holds the original curve values.
        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_duration_curves;")
                .fetch_one(&repository.readers)
                .await
                .unwrap(),
            1
        );

        let secs_5: f64 = sqlx::query_scalar("select secs_5 from t_duration_curves;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
        assert_eq!(secs_5, 1.);
    }

    fn build_activity_with_curves(curves: Vec<ActivityDurationCurve>) -> ActivityWithParsedData {
        ActivityWithParsedData::new(
            Activity::new_empty(
                ActivityId::new(),
                UserId::test_default(),
                ActivityStartTime::from_timestamp(random_range(100..1200)).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
            ),
            ActivityTimeseries::default(),
            ActivityStatistics::default(),
            ActivityDurationCurves::new(curves),
        )
    }

    #[tokio::test]
    async fn test_save_activity_saves_duration_curves() {
        let (repository, _db_file) = test_repository().await;

        let activity = build_activity_with_curves(vec![
            build_duration_curve(DurationCurveType::Power),
            build_duration_curve(DurationCurveType::Pace),
        ]);

        repository
            .save_activity(&activity)
            .await
            .expect("should save the activity");

        // Both curves are saved, and no null marker is stored.
        assert_eq!(
            sqlx::query_scalar::<_, u64>(
                "select count(*) from t_duration_curves where type is not null;"
            )
            .fetch_one(&repository.readers)
            .await
            .unwrap(),
            2
        );
        assert_eq!(
            sqlx::query_scalar::<_, u64>(
                "select count(*) from t_duration_curves where type is null;"
            )
            .fetch_one(&repository.readers)
            .await
            .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn test_save_activity_without_duration_curves_saves_null_marker() {
        let (repository, _db_file) = test_repository().await;

        // An activity without any curve still gets a row, marking it as processed.
        let activity = build_activity();
        assert!(activity.duration_curves().is_empty());

        repository
            .save_activity(&activity)
            .await
            .expect("should save the activity");

        assert_eq!(
            sqlx::query_scalar::<_, u64>(
                "select count(*) from t_duration_curves where type is null;"
            )
            .fetch_one(&repository.readers)
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, u64>(
                "select count(*) from t_duration_curves where type is not null;"
            )
            .fetch_one(&repository.readers)
            .await
            .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn test_delete_activity_deletes_duration_curves() {
        let (repository, _db_file) = test_repository().await;

        let activity = build_activity_with_curves(vec![
            build_duration_curve(DurationCurveType::Power),
            build_duration_curve(DurationCurveType::Pace),
        ]);

        repository
            .save_activity(&activity)
            .await
            .expect("should save the activity");
        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_duration_curves;")
                .fetch_one(&repository.readers)
                .await
                .unwrap(),
            2
        );

        repository
            .delete_activity(activity.user(), activity.id())
            .await
            .expect("deletion should have succeeded");

        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_duration_curves;")
                .fetch_one(&repository.readers)
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn test_delete_activity_deletes_null_duration_curve_marker() {
        let (repository, _db_file) = test_repository().await;

        let activity = build_activity();
        assert!(activity.duration_curves().is_empty());

        repository
            .save_activity(&activity)
            .await
            .expect("should save the activity");
        assert_eq!(
            sqlx::query_scalar::<_, u64>(
                "select count(*) from t_duration_curves where type is null;"
            )
            .fetch_one(&repository.readers)
            .await
            .unwrap(),
            1
        );

        repository
            .delete_activity(activity.user(), activity.id())
            .await
            .expect("deletion should have succeeded");

        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_duration_curves;")
                .fetch_one(&repository.readers)
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn test_load_duration_curves() {
        let (repository, _db_file) = test_repository().await;

        let activity = build_activity_with_curves(vec![
            build_duration_curve(DurationCurveType::Power),
            build_duration_curve(DurationCurveType::Pace),
        ]);

        repository
            .save_activity(&activity)
            .await
            .expect("should save the activity");

        let curves = repository
            .load_duration_curves(activity.id(), activity.user())
            .await
            .expect("should load the duration curves");

        assert_eq!(curves.iter().count(), 2);
        for curve_type in [DurationCurveType::Power, DurationCurveType::Pace] {
            let curve = curves
                .iter()
                .find(|curve| curve.curve_type() == curve_type)
                .unwrap_or_else(|| panic!("missing {:?} curve", curve_type));
            assert_eq!(curve.values(), build_duration_curve(curve_type).values());
        }
    }

    #[tokio::test]
    async fn test_load_duration_curves_without_curves() {
        let (repository, _db_file) = test_repository().await;

        // An activity saved without curves only has a NULL-type marker row.
        let activity = build_activity();
        assert!(activity.duration_curves().is_empty());

        repository
            .save_activity(&activity)
            .await
            .expect("should save the activity");

        let curves = repository
            .load_duration_curves(activity.id(), activity.user())
            .await
            .expect("should load the duration curves");
        assert!(curves.is_empty());

        // An activity that was never saved has no row at all, and also loads as empty.
        let curves = repository
            .load_duration_curves(&ActivityId::new(), activity.user())
            .await
            .expect("should load the duration curves");
        assert!(curves.is_empty());
    }

    #[tokio::test]
    async fn test_load_duration_curves_ignores_null_marker() {
        let (repository, _db_file) = test_repository().await;

        let activity_id = ActivityId::new();
        let user = UserId::test_default();
        let date = ActivityStartTime::from_timestamp(1000).unwrap();

        let mut tx = repository.writer.begin().await.unwrap();
        repository
            .save_null_duration_curve(&mut tx, &activity_id, &user, &date)
            .await
            .expect("should save the null curve");
        repository
            .save_duration_curve(
                &mut tx,
                &activity_id,
                &user,
                &date,
                &build_duration_curve(DurationCurveType::Power),
            )
            .await
            .expect("should save the curve");
        tx.commit().await.unwrap();

        // The NULL-type marker row yields no curve; only the real curve is loaded.
        let curves = repository
            .load_duration_curves(&activity_id, &user)
            .await
            .expect("should load the duration curves");

        assert_eq!(curves.iter().count(), 1);
        let curve = curves.iter().next().unwrap();
        assert_eq!(curve.curve_type(), DurationCurveType::Power);
        assert_eq!(
            curve.values(),
            build_duration_curve(DurationCurveType::Power).values()
        );
    }

    #[tokio::test]
    async fn test_save_null_duration_curve() {
        let (repository, _db_file) = test_repository().await;

        let activity_id = ActivityId::new();
        let user = UserId::test_default();
        let date = ActivityStartTime::from_timestamp(1000).unwrap();

        let mut tx = repository.writer.begin().await.unwrap();
        repository
            .save_null_duration_curve(&mut tx, &activity_id, &user, &date)
            .await
            .expect("should save the null curve");
        tx.commit().await.unwrap();

        // The activity has a row, marking it as processed even though it has no curve.
        let row: (ActivityId, UserId, ActivityStartTime) = sqlx::query_as(
            "select activity_id, user_id, date from t_duration_curves where type is null;",
        )
        .fetch_one(&repository.readers)
        .await
        .unwrap();
        assert_eq!(row.0, activity_id);
        assert_eq!(row.1, user);
        assert_eq!(row.2, date);

        // All value columns are NULL.
        let values: (
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
        ) = sqlx::query_as(
            "select
                secs_5, secs_10, secs_30,
                mins_1, mins_2, mins_5, mins_10, mins_20, mins_30,
                hours_1, hours_2, hours_5
            from t_duration_curves
            where type is null;",
        )
        .fetch_one(&repository.readers)
        .await
        .unwrap();
        assert!(
            [
                values.0, values.1, values.2, values.3, values.4, values.5, values.6, values.7,
                values.8, values.9, values.10, values.11
            ]
            .iter()
            .all(|value| value.is_none())
        );
    }

    #[tokio::test]
    async fn test_save_null_duration_curve_is_idempotent() {
        let (repository, _db_file) = test_repository().await;

        let activity_id = ActivityId::new();
        let user = UserId::test_default();
        let date = ActivityStartTime::from_timestamp(1000).unwrap();

        let mut tx = repository.writer.begin().await.unwrap();
        for _ in 0..2 {
            repository
                .save_null_duration_curve(&mut tx, &activity_id, &user, &date)
                .await
                .expect("should save the null curve");
        }

        // A real curve can still be saved alongside the null marker.
        repository
            .save_duration_curve(
                &mut tx,
                &activity_id,
                &user,
                &date,
                &build_duration_curve(DurationCurveType::Power),
            )
            .await
            .expect("should save the curve");
        tx.commit().await.unwrap();

        // The null marker is stored only once, but does not conflict with real curves.
        assert_eq!(
            sqlx::query_scalar::<_, u64>(
                "select count(*) from t_duration_curves where type is null;"
            )
            .fetch_one(&repository.readers)
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, u64>(
                "select count(*) from t_duration_curves where type is not null;"
            )
            .fetch_one(&repository.readers)
            .await
            .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_update_existing_activity_id_updates_optional_fields() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();

        repository
            .save_activity(&activity)
            .await
            .expect("Should have succeed");

        let patched_activity = activity.patch(ActivityPatch::new(
            Some(Some(ActivityName::from("Another name"))),
            Some(Some(ActivityRpe::Eight)),
            Some(Some(WorkoutType::CrossTraining)),
            Some(Some(ActivityNutrition::new(BonkStatus::None, None))),
            Some(Some(ActivityFeedback::from("Another feedback"))),
        ));

        repository
            .update_activity(&patched_activity.activity())
            .await
            .expect("Should have succeeded");

        let activity = repository
            .get_activity(&UserId::test_default(), patched_activity.id())
            .await
            .expect("Should have returned the activity")
            .expect("Activity should be some");

        assert_eq!(activity.feedback(), patched_activity.feedback());
        assert_eq!(activity.name(), patched_activity.name());
        assert_eq!(activity.rpe(), patched_activity.rpe());
        assert_eq!(activity.workout_type(), patched_activity.workout_type());
        assert_eq!(activity.nutrition(), patched_activity.nutrition());
    }

    #[tokio::test]
    async fn test_delete_activity() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();

        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_activities_v2;")
                .fetch_one(&repository.readers)
                .await
                .unwrap(),
            1
        );

        repository
            .delete_activity(activity.user(), activity.id())
            .await
            .expect("Deletion should have succeeded");

        assert_eq!(
            sqlx::query_scalar::<_, u64>("select count(*) from t_activities_v2;")
                .fetch_one(&repository.readers)
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn test_delete_activity_does_not_exist_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();

        repository
            .delete_activity(activity.user(), activity.id())
            .await
            .expect("Should have returned ok");
    }

    #[tokio::test]
    async fn test_get_activity() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();

        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .get_activity(&UserId::test_default(), activity.id())
            .await
            .expect("Get should have succeeded")
            .expect("Should not be None");

        assert_eq!(res.id(), activity.id());
        assert_eq!(res.user(), activity.user());
        assert_eq!(res.name(), activity.name());
        assert_eq!(res.start_time(), activity.start_time());
        assert_eq!(res.sport(), activity.sport());
    }

    #[tokio::test]
    async fn test_get_activity_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();

        let res = repository
            .get_activity(&UserId::test_default(), activity.id())
            .await
            .expect_err("Get should have failed");

        let GetActivityError::ActivityDoesNotExist(id) = res else {
            unreachable!("Should have returned GetActivityError::ActivityDoesNotExist(id)")
        };

        assert_eq!(id, *activity.id());
    }

    #[tokio::test]
    async fn test_get_activity_wrong_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();

        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .get_activity(&UserId::from("another_user"), activity.id())
            .await
            .expect_err("Get should have failed");

        let GetActivityError::ActivityDoesNotExist(id) = res else {
            unreachable!("Should have returned GetActivityError::ActivityDoesNotExist(id)")
        };

        assert_eq!(id, *activity.id());
    }

    #[tokio::test]
    async fn test_get_activity_with_metrics_wrong_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();

        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .get_activity_with_metrics(
                &UserId::from("another_user"),
                activity.id(),
                &[ActivityMetric::AvgPower],
            )
            .await
            .expect_err("Get should have failed");

        let GetActivityError::ActivityDoesNotExist(id) = res else {
            unreachable!("Should have returned GetActivityError::ActivityDoesNotExist(id)")
        };

        assert_eq!(id, *activity.id());
    }

    #[tokio::test]
    async fn test_list_activities() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .list_user_activities(&UserId::test_default(), &ListActivitiesFilters::empty())
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 2);
    }

    #[tokio::test]
    async fn test_list_activities_with_limit() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .list_user_activities(
                &UserId::test_default(),
                &ListActivitiesFilters::empty().set_limit(Some(1)),
            )
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_list_activities_with_date_range() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity_starting_at(
            &"2025-09-29T12:34:00+02:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
        );
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let activity = build_activity_starting_at(
            &"2025-10-03T12:34:00+02:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
        );
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .list_user_activities(
                &UserId::test_default(),
                &ListActivitiesFilters::empty().set_date_range(Some(DateRange::new(
                    "2025-09-10".parse::<NaiveDate>().unwrap(),
                    "2025-10-01".parse::<NaiveDate>().unwrap(),
                ))),
            )
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_list_activities_with_date_range_timezone() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity_starting_at(
            &"2025-09-10T08:34:00-10:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
        );
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .list_user_activities(
                &UserId::test_default(),
                &ListActivitiesFilters::empty().set_date_range(Some(DateRange::new(
                    "2025-09-10".parse::<NaiveDate>().unwrap(),
                    "2025-09-11".parse::<NaiveDate>().unwrap(),
                ))),
            )
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_list_activities_ignore_other_users() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .list_user_activities(
                &UserId::from("another_user"),
                &ListActivitiesFilters::empty(),
            )
            .await
            .expect("Get should have succeeded");

        assert_eq!(res.len(), 0);
    }

    #[tokio::test]
    async fn test_natural_key_exists() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");
        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        assert!(
            repository
                .similar_activity_exists(&activity.natural_key())
                .await
                .expect("Should not have err")
        );
    }

    #[tokio::test]
    async fn test_natural_key_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");

        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        assert!(
            !repository
                .similar_activity_exists(&ActivityNaturalKey::from("another_key"))
                .await
                .expect("Should not have err")
        );
    }

    fn build_parsed_file_content() -> ParsedFileContent {
        ParsedFileContent::new(
            Sport::Cycling,
            ActivityStartTime::from_timestamp(120).unwrap(),
            ActivityDuration::from(0.0),
            ActivityStatistics::new(HashMap::new()),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![0, 1, 2, 3]),
                TimeseriesActiveTime::new(vec![
                    ActiveTime::Running(0),
                    ActiveTime::Running(1),
                    ActiveTime::Running(2),
                    ActiveTime::Running(3),
                ]),
                vec![],
                vec![Timeseries::new(
                    TimeseriesMetric::Altitude,
                    vec![
                        Some(TimeseriesValue::Float(12.3)),
                        Some(TimeseriesValue::Float(12.3)),
                        Some(TimeseriesValue::Float(12.3)),
                        Some(TimeseriesValue::Float(12.3)),
                    ],
                )],
            )
            .unwrap(),
            "fit".to_string(),
            vec![],
        )
    }

    #[tokio::test]
    async fn test_get_activity_with_parsed_data_ok() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .times(1)
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![])));
        let mut file_parser = MockFileParser::new();
        file_parser
            .expect_try_bytes_into_domain()
            .times(1)
            .returning(|_, __| Ok(build_parsed_file_content()));
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
            Clock::new(),
        )
        .await
        .expect("repo should init");

        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        let res = repository
            .get_activity_with_parsed_data(&UserId::test_default(), activity.id())
            .await
            .expect("Should have succeeded")
            .expect("Should not be none");

        assert_eq!(
            res.timeseries().metrics().first().unwrap(),
            &Timeseries::new(
                TimeseriesMetric::Altitude,
                vec![
                    Some(TimeseriesValue::Float(12.3)),
                    Some(TimeseriesValue::Float(12.3)),
                    Some(TimeseriesValue::Float(12.3)),
                    Some(TimeseriesValue::Float(12.3)),
                ],
            )
        );
    }

    #[tokio::test]
    async fn test_get_activity_with_parsed_data_loads_duration_curves() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .times(1)
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![])));
        let mut file_parser = MockFileParser::new();
        file_parser
            .expect_try_bytes_into_domain()
            .times(1)
            .returning(|_, __| Ok(build_parsed_file_content()));
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
            Clock::new(),
        )
        .await
        .expect("repo should init");

        let activity = build_activity_with_curves(vec![
            build_duration_curve(DurationCurveType::Power),
            build_duration_curve(DurationCurveType::Pace),
        ]);
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        let res = repository
            .get_activity_with_parsed_data(activity.user(), activity.id())
            .await
            .expect("Should have succeeded")
            .expect("Should not be none");

        // The saved curves are loaded back, not the default empty curves.
        let curves = res.duration_curves();
        assert_eq!(curves.iter().count(), 2);
        for curve_type in [DurationCurveType::Power, DurationCurveType::Pace] {
            let curve = curves
                .iter()
                .find(|curve| curve.curve_type() == curve_type)
                .unwrap_or_else(|| panic!("missing {:?} curve", curve_type));
            assert_eq!(curve.values(), build_duration_curve(curve_type).values());
        }
    }

    #[tokio::test]
    async fn test_get_activity_with_parsed_data_wrong_user() {
        let raw_data_repo = MockRawDataRepository::new();
        let file_parser = MockFileParser::new();
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
            Clock::new(),
        )
        .await
        .expect("repo should init");

        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        let res = repository
            .get_activity_with_parsed_data(&UserId::from("another_user"), activity.id())
            .await
            .expect_err("Should have failed");

        let GetActivityError::ActivityDoesNotExist(id) = res else {
            unreachable!("Should have returned GetActivityError::ActivityDoesNotExist(id)")
        };

        assert_eq!(id, *activity.id());
    }

    #[tokio::test]
    async fn test_get_activity_with_parsed_data_get_raw_data_fails() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .times(1)
            .returning(|_| Err(GetRawDataError::Unknown));
        let mut file_parser = MockFileParser::new();
        file_parser.expect_try_bytes_into_domain().times(0);
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
            Clock::new(),
        )
        .await
        .expect("repo should init");

        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        repository
            .get_activity_with_parsed_data(&UserId::test_default(), activity.id())
            .await
            .expect_err("Should have failed");
    }

    #[tokio::test]
    async fn test_get_activity_with_parsed_data_raw_data_parsing_fails() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .times(1)
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![])));
        let mut file_parser = MockFileParser::new();
        file_parser
            .expect_try_bytes_into_domain()
            .times(1)
            .returning(|_, __| Err(ParseBytesError::InvalidContent));

        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
            Clock::new(),
        )
        .await
        .expect("repo should init");

        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        repository
            .get_activity_with_parsed_data(&UserId::test_default(), activity.id())
            .await
            .expect_err("Should have failed");
    }

    #[tokio::test]
    async fn test_list_activities_with_parsed_data_ok() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .times(2)
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![])));
        let mut file_parser = MockFileParser::new();
        file_parser
            .expect_try_bytes_into_domain()
            .times(2)
            .returning(|_, __| Ok(build_parsed_file_content()));
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
            Clock::new(),
        )
        .await
        .expect("repo should init");

        // Insert 2 activities
        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");
        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        let res = repository
            .list_activities_with_parsed_data(activity.user(), &ListActivitiesFilters::empty())
            .await
            .expect("Should have succeeded");

        assert_eq!(res.len(), 2);
    }

    #[tokio::test]
    async fn test_list_activities_with_parsed_data_with_limit() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![])));
        let mut file_parser = MockFileParser::new();
        file_parser
            .expect_try_bytes_into_domain()
            .returning(|_, __| Ok(build_parsed_file_content()));
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
            Clock::new(),
        )
        .await
        .expect("repo should init");

        // Insert 2 activities
        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");
        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        let res = repository
            .list_activities_with_parsed_data(
                activity.user(),
                &ListActivitiesFilters::empty().set_limit(Some(1)),
            )
            .await
            .expect("Should have succeeded");

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_list_activities_with_parsed_data_ok_ignore_failed_activities() {
        let mut raw_data_repo = MockRawDataRepository::new();
        raw_data_repo
            .expect_get_raw_data()
            .times(1)
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![])));
        raw_data_repo
            .expect_get_raw_data()
            .times(1)
            .return_once(|_| Err(GetRawDataError::Unknown));
        let mut file_parser = MockFileParser::new();
        file_parser
            .expect_try_bytes_into_domain()
            .times(1)
            .returning(|_, __| Ok(build_parsed_file_content()));
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repo,
            file_parser,
            Clock::new(),
        )
        .await
        .expect("repo should init");

        // Insert 2 activities
        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");
        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Save should have succeeded");

        let res = repository
            .list_activities_with_parsed_data(activity.user(), &ListActivitiesFilters::empty())
            .await
            .expect("Should have succeeded");

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_user_history_date_range_when_no_activities() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");

        assert!(
            repository
                .get_user_history_date_range(&UserId::test_default())
                .await
                .expect("Should be Ok")
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_user_history_date_range() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");

        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let another_activity = build_activity();
        repository
            .save_activity(&another_activity)
            .await
            .expect("Insertion should have succeed");

        let date_range = repository
            .get_user_history_date_range(&UserId::test_default())
            .await
            .expect("Should be Ok")
            .expect("Should be Some");
        let expected_start = activity
            .start_time()
            .datetime()
            .min(another_activity.start_time().datetime());
        let expected_end = activity
            .start_time()
            .datetime()
            .max(another_activity.start_time().datetime());

        assert_eq!(date_range.start(), expected_start);
        assert_eq!(date_range.end().expect("End should be some"), *expected_end);
    }

    #[tokio::test]
    async fn test_list_all_raw_activities_no_activities() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");

        let res = repository
            .list_all_raw_activities(&UserId::test_default())
            .await
            .expect("Should not err");

        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_list_all_raw_activities_no_activities_for_this_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            MockRawDataRepository::new(),
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");

        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");

        let res = repository
            .list_all_raw_activities(&UserId::from("another_user"))
            .await
            .expect("Should not err");

        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_list_all_raw_activities_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let mut raw_data_repository = MockRawDataRepository::new();
        raw_data_repository
            .expect_get_raw_data()
            .times(1)
            .returning(|_| Ok(RawContent::new("fit".to_string(), vec![0, 1, 2])));
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repository,
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");

        let activity = build_activity();
        repository
            .save_activity(&activity)
            .await
            .expect("Insertion should have succeed");
        let res = repository
            .list_all_raw_activities(&UserId::test_default())
            .await
            .expect("Should not err");

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_list_all_raw_activities_skip_missing_raw_files() {
        let db_file = NamedTempFile::new().unwrap();
        let activity_1 = build_activity();
        let activity_1_id = activity_1.id().clone();
        let activity_2 = build_activity();

        let mut raw_data_repository = MockRawDataRepository::new();
        raw_data_repository
            .expect_get_raw_data()
            .times(2)
            .returning(move |id| {
                if id == &activity_1_id {
                    Ok(RawContent::new("fit".to_string(), vec![0, 1, 2]))
                } else {
                    Err(GetRawDataError::Unknown)
                }
            });
        let repository = SqliteActivityRepository::new(
            &db_file.path().to_string_lossy(),
            raw_data_repository,
            MockFileParser::new(),
            Clock::new(),
        )
        .await
        .expect("repo should init");

        repository
            .save_activity(&activity_1)
            .await
            .expect("Insertion should have succeed");
        repository
            .save_activity(&activity_2)
            .await
            .expect("Insertion should have succeed");
        let res = repository
            .list_all_raw_activities(&UserId::test_default())
            .await
            .expect("Should not err");

        assert_eq!(res.len(), 1);
        assert_eq!(
            res.first().unwrap().name(),
            format!("{}.fit", activity_1.id())
        )
    }

    mod test_list_activities {
        use chrono::Utc;

        use super::*;

        fn build_activity_with_name(name: &str) -> ActivityWithParsedData {
            ActivityWithParsedData::new(
                Activity::new(
                    ActivityId::new(),
                    UserId::test_default(),
                    Some(ActivityName::from(name)),
                    ActivityStartTime::from_timestamp(random_range(100..1200)).unwrap(),
                    ActivityDuration::default(),
                    Sport::Cycling,
                    ActivityRpe::empty(),
                    WorkoutType::empty(),
                    ActivityNutrition::empty(),
                    ActivityFeedback::empty(),
                ),
                ActivityTimeseries::default(),
                ActivityStatistics::default(),
                ActivityDurationCurves::default(),
            )
        }

        fn build_activity_for(user: UserId) -> ActivityWithParsedData {
            ActivityWithParsedData::new(
                Activity::new(
                    ActivityId::new(),
                    user,
                    ActivityName::empty(),
                    ActivityStartTime::from_timestamp(random_range(100..1200)).unwrap(),
                    ActivityDuration::default(),
                    Sport::Cycling,
                    ActivityRpe::empty(),
                    WorkoutType::empty(),
                    ActivityNutrition::empty(),
                    ActivityFeedback::empty(),
                ),
                ActivityTimeseries::default(),
                ActivityStatistics::default(),
                ActivityDurationCurves::default(),
            )
        }

        #[tokio::test]
        async fn test_list_activities_when_empty_db() {
            let db_file = NamedTempFile::new().unwrap();
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let (documents, remaining) = repository
                .list_activity_documents(10, 0)
                .await
                .expect("Should have succeeded");

            assert!(documents.is_empty());
            assert_eq!(remaining, RemainingDocuments::from(false));
        }

        #[tokio::test]
        async fn test_list_activities_returns_all_activities_in_insertion_order() {
            let db_file = NamedTempFile::new().unwrap();
            let now = Utc::now();
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                FakeClock::new(now),
            )
            .await
            .expect("repo should init");

            let activities = [
                build_activity_with_name("first"),
                build_activity_with_name("second"),
                build_activity_with_name("third"),
            ];
            for activity in &activities {
                repository
                    .save_activity(activity)
                    .await
                    .expect("Insertion should have succeeded");
            }

            let (documents, remaining) = repository
                .list_activity_documents(10, 0)
                .await
                .expect("Should have succeeded");

            assert_eq!(documents.len(), activities.len());
            assert_eq!(remaining, RemainingDocuments::from(false));

            // Documents are ordered by rowid, i.e. insertion order, and each activity is
            // turned into an "updated" search document.
            for (document, activity) in documents.iter().zip(activities.iter()) {
                assert_eq!(document.document_type(), &SearchDocumentType::Activity);
                assert_eq!(document.document_id(), activity.id().to_string());
                assert_eq!(document.user(), activity.user());
                assert_eq!(document.event(), &SearchDocumentEvent::Updated);
                assert_eq!(document.occurred_at(), &now);
            }
        }

        #[tokio::test]
        async fn test_list_activities_documents_include_activity_details() {
            let db_file = NamedTempFile::new().unwrap();
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let activity = build_activity();
            repository
                .save_activity(&activity)
                .await
                .expect("Save should have succeeded");

            let patched_activity = activity
                .patch(ActivityPatch::name(Some(ActivityName::from("Epic ride"))))
                .patch(ActivityPatch::feedback(Some(ActivityFeedback::from(
                    "Felt great",
                ))))
                .patch(ActivityPatch::nutrition(Some(ActivityNutrition::new(
                    BonkStatus::None,
                    Some("Drank early".to_string()),
                ))));
            repository
                .update_activity(&patched_activity.activity())
                .await
                .expect("Patch should have succeeded");

            let (documents, _remaining) = repository
                .list_activity_documents(10, 0)
                .await
                .expect("Should have succeeded");

            let document = documents.first().expect("Should contain one document");
            assert_eq!(document.document_id(), patched_activity.id().to_string());
            assert!(document.content().contains("Epic ride"));
            assert!(document.content().contains("Felt great"));
            assert!(document.content().contains("Drank early"));
        }

        #[tokio::test]
        async fn test_list_activities_includes_activities_from_all_users() {
            let db_file = NamedTempFile::new().unwrap();
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let default_user_activity = build_activity_with_name("default user ride");
            let other_user_activity = build_activity_for(UserId::from("another_user"));
            repository
                .save_activity(&default_user_activity)
                .await
                .expect("Insertion should have succeeded");
            repository
                .save_activity(&other_user_activity)
                .await
                .expect("Insertion should have succeeded");

            // Unlike `list_user_activities`, the snapshot spans every user's activities.
            let (documents, remaining) = repository
                .list_activity_documents(10, 0)
                .await
                .expect("Should have succeeded");

            assert_eq!(documents.len(), 2);
            assert_eq!(remaining, RemainingDocuments::from(false));
            let document_ids: Vec<String> = documents
                .iter()
                .map(|document| document.document_id().to_string())
                .collect();
            assert!(document_ids.contains(&default_user_activity.id().to_string()));
            assert!(document_ids.contains(&other_user_activity.id().to_string()));
        }

        #[tokio::test]
        async fn test_list_activities_skips_deleted_activities() {
            let db_file = NamedTempFile::new().unwrap();
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let kept = build_activity_with_name("kept ride");
            let deleted = build_activity_with_name("deleted ride");
            repository
                .save_activity(&kept)
                .await
                .expect("Insertion should have succeeded");
            repository
                .save_activity(&deleted)
                .await
                .expect("Insertion should have succeeded");
            repository
                .delete_activity(deleted.user(), deleted.id())
                .await
                .expect("Deletion should have succeeded");

            let (documents, remaining) = repository
                .list_activity_documents(10, 0)
                .await
                .expect("Should have succeeded");

            assert_eq!(documents.len(), 1);
            assert_eq!(remaining, RemainingDocuments::from(false));
            assert_eq!(
                documents
                    .first()
                    .expect("Should contain one document")
                    .document_id(),
                kept.id().to_string()
            );
        }

        #[tokio::test]
        async fn test_list_activities_page_with_more_remaining_returns_batch_size_documents() {
            let db_file = NamedTempFile::new().unwrap();
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let activities = [
                build_activity_with_name("activity-1"),
                build_activity_with_name("activity-2"),
                build_activity_with_name("activity-3"),
            ];
            for activity in &activities {
                repository
                    .save_activity(activity)
                    .await
                    .expect("Insertion should have succeeded");
            }

            // The repository fetches batch_size + 1 rows to detect a next page, but only
            // returns `batch_size` documents: the extra sentinel row is dropped.
            let (documents, remaining) = repository
                .list_activity_documents(2, 0)
                .await
                .expect("Should have succeeded");

            assert_eq!(documents.len(), 2);
            assert_eq!(remaining, RemainingDocuments::from(true));
            for (document, activity) in documents.iter().zip(activities.iter().take(2)) {
                assert_eq!(document.document_id(), activity.id().to_string());
            }
        }

        #[tokio::test]
        async fn test_list_activities_on_last_page_no_more_remaining() {
            let db_file = NamedTempFile::new().unwrap();
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let activities = [
                build_activity_with_name("activity-1"),
                build_activity_with_name("activity-2"),
                build_activity_with_name("activity-3"),
            ];
            for activity in &activities {
                repository
                    .save_activity(activity)
                    .await
                    .expect("Insertion should have succeeded");
            }

            // page 1 with batch_size 2 skips the first 2 rows and only the last activity remains.
            let (documents, remaining) = repository
                .list_activity_documents(2, 1)
                .await
                .expect("Should have succeeded");

            assert_eq!(documents.len(), 1);
            assert_eq!(remaining, RemainingDocuments::from(false));
            assert_eq!(
                documents
                    .first()
                    .expect("Should contain one document")
                    .document_id(),
                activities[2].id().to_string()
            );
        }

        #[tokio::test]
        async fn test_list_activities_pages_partition_all_activities_without_overlap() {
            let db_file = NamedTempFile::new().unwrap();
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let activities = [
                build_activity_with_name("activity-1"),
                build_activity_with_name("activity-2"),
                build_activity_with_name("activity-3"),
                build_activity_with_name("activity-4"),
                build_activity_with_name("activity-5"),
            ];
            for activity in &activities {
                repository
                    .save_activity(activity)
                    .await
                    .expect("Insertion should have succeeded");
            }

            // Walking through every page (incrementing the page until none remain) must
            // return each activity exactly once, in insertion order: pages do not overlap.
            let mut page_ids = Vec::new();
            let mut page = 0;
            loop {
                let (documents, remaining) = repository
                    .list_activity_documents(2, page)
                    .await
                    .expect("Should have succeeded");
                assert!(documents.len() <= 2);
                page_ids.extend(
                    documents
                        .iter()
                        .map(|document| document.document_id().to_string()),
                );
                if remaining == RemainingDocuments::from(false) {
                    break;
                }
                page += 1;
            }

            let expected_ids: Vec<String> = activities
                .iter()
                .map(|activity| activity.id().to_string())
                .collect();
            assert_eq!(page_ids, expected_ids);
        }

        #[tokio::test]
        async fn test_list_activities_offset_beyond_last_page_is_empty() {
            let db_file = NamedTempFile::new().unwrap();
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let activity = build_activity_with_name("activity-1");
            repository
                .save_activity(&activity)
                .await
                .expect("Insertion should have succeeded");

            // offset 10 with batch_size 2 skips 20 rows: nothing should be returned.
            let (documents, remaining) = repository
                .list_activity_documents(2, 10)
                .await
                .expect("Should have succeeded");

            assert!(documents.is_empty());
            assert_eq!(remaining, RemainingDocuments::from(false));
        }
    }

    mod test_sqlite_activity_repository_get_raw_activity {
        use super::*;

        #[tokio::test]
        async fn test_get_raw_activity() {
            let db_file = NamedTempFile::new().unwrap();
            let mut raw_data_repository = MockRawDataRepository::new();
            raw_data_repository
                .expect_get_raw_data()
                .times(1)
                .returning(|_| Ok(RawContent::new("fit".to_string(), vec![0, 1, 2])));
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                raw_data_repository,
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let activity = build_activity();
            repository
                .save_activity(&activity)
                .await
                .expect("Insertion should have succeed");

            let res = repository
                .get_raw_activity(activity.user(), activity.id())
                .await
                .expect("Should not err");
            assert_eq!(res.name(), format!("{}.fit", activity.id()));
            assert_eq!(res.content(), &[0, 1, 2]);
        }

        #[tokio::test]
        async fn test_get_raw_activity_activity_does_not_exist() {
            let db_file = NamedTempFile::new().unwrap();
            let raw_data_repository = MockRawDataRepository::new();

            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                raw_data_repository,
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let Err(GetRawActivityError::ActivityDoesNotExist(id)) = repository
                .get_raw_activity(&UserId::from("test_user"), &ActivityId::from("test_id"))
                .await
            else {
                unreachable!("Should Err(GetRawActivityError::ActivityDoesNotExist(id))")
            };
            assert_eq!(id, ActivityId::from("test_id"))
        }

        #[tokio::test]
        async fn test_get_raw_activity_activity_does_not_exist_for_that_user() {
            let db_file = NamedTempFile::new().unwrap();
            let raw_data_repository = MockRawDataRepository::new();
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                raw_data_repository,
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let activity = build_activity();
            repository
                .save_activity(&activity)
                .await
                .expect("Insertion should have succeed");

            let Err(GetRawActivityError::ActivityDoesNotExist(id)) = repository
                .get_raw_activity(&UserId::from("another_user"), activity.id())
                .await
            else {
                unreachable!("Should Err(GetRawActivityError::ActivityDoesNotExist(id))")
            };
            assert_eq!(id, *activity.id())
        }

        #[tokio::test]
        async fn test_get_raw_activity_activity_raw_file_does_not_exist() {
            let db_file = NamedTempFile::new().unwrap();
            let mut raw_data_repository = MockRawDataRepository::new();
            raw_data_repository
                .expect_get_raw_data()
                .times(1)
                .returning(|_| Err(GetRawDataError::Unknown));
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                raw_data_repository,
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let activity = build_activity();
            repository
                .save_activity(&activity)
                .await
                .expect("Insertion should have succeed");

            let Err(GetRawActivityError::Unknown(_err)) = repository
                .get_raw_activity(activity.user(), activity.id())
                .await
            else {
                unreachable!("Should Err(GetRawActivityError::Unknown(_))")
            };
        }
    }

    mod test_activity_metrics_v2 {
        use super::*;

        #[tokio::test]
        async fn test_update_activity_metric_value() {
            let db_file = NamedTempFile::new().unwrap();
            let repo = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");
            let activity = build_activity();

            repo.save_activity(&activity)
                .await
                .expect("Should have succeed");

            // No initial metric values
            let res = repo
                .get_activities_with_metrics(
                    activity.user(),
                    &ListActivitiesFilters::empty(),
                    &[ActivityMetric::AvgPower],
                )
                .await
                .unwrap();
            assert_eq!(res.len(), 1);
            let (_actvity, metrics) = res.first().unwrap();
            assert!(metrics.is_empty(),);

            // Insert a metric value
            repo.update_activity_metric(activity.id(), &ActivityMetric::AvgPower, &Some(1.2))
                .await
                .expect("Should have succeeded");

            let res = repo
                .get_activities_with_metrics(
                    activity.user(),
                    &ListActivitiesFilters::empty(),
                    &[ActivityMetric::AvgPower],
                )
                .await
                .unwrap();
            assert_eq!(res.len(), 1);
            let (_actvity, metrics) = res.first().unwrap();
            assert_eq!(
                metrics,
                &ActivityMetrics::new(HashMap::from([(ActivityMetric::AvgPower, Some(1.2))]))
            );

            // Update a metric value
            repo.update_activity_metric(activity.id(), &ActivityMetric::AvgPower, &None)
                .await
                .expect("Should have succeeded");

            let res = repo
                .get_activities_with_metrics(
                    activity.user(),
                    &ListActivitiesFilters::empty(),
                    &[ActivityMetric::AvgPower],
                )
                .await
                .unwrap();
            assert_eq!(res.len(), 1);
            let (_actvity, metrics) = res.first().unwrap();
            assert_eq!(
                metrics,
                &ActivityMetrics::new(HashMap::from([(ActivityMetric::AvgPower, None)]))
            );
        }

        #[tokio::test]
        async fn test_update_metric_value_activity_does_not_exist() {
            let db_file = NamedTempFile::new().unwrap();
            let repository = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let UpdateActivityMetricError::ActivityDoesNotExist(id) = repository
                .update_activity_metric(
                    &ActivityId::from("non-existing-activity"),
                    &ActivityMetric::AvgPower,
                    &Some(1.2),
                )
                .await
                .unwrap_err()
            else {
                unreachable!(
                    "Should have returned UpdateActivityMetricError::ActivityDoesNotExist(id)"
                )
            };
            assert_eq!(id, ActivityId::from("non-existing-activity"));
        }

        #[tokio::test]
        async fn test_get_activity_with_metrics_returns_metrics() {
            let db_file = NamedTempFile::new().unwrap();
            let repo = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");
            let activity = build_activity();

            repo.save_activity(&activity)
                .await
                .expect("Should have succeed");

            repo.update_activity_metric(activity.id(), &ActivityMetric::AvgPower, &Some(3.45))
                .await
                .expect("Should have succeeded");

            let (returned_activity, metrics) = repo
                .get_activity_with_metrics(
                    &UserId::test_default(),
                    activity.id(),
                    &[ActivityMetric::AvgPower],
                )
                .await
                .expect("Should have succeeded")
                .expect("Should not be None");

            assert_eq!(returned_activity.id(), activity.id());
            assert_eq!(
                metrics,
                ActivityMetrics::new(HashMap::from([(ActivityMetric::AvgPower, Some(3.45))]))
            );
        }

        #[tokio::test]
        async fn test_get_activity_with_metrics_no_metrics_stored() {
            let db_file = NamedTempFile::new().unwrap();
            let repo = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");
            let activity = build_activity();

            repo.save_activity(&activity)
                .await
                .expect("Should have succeed");

            let (_returned_activity, metrics) = repo
                .get_activity_with_metrics(
                    &UserId::test_default(),
                    activity.id(),
                    &[ActivityMetric::AvgPower],
                )
                .await
                .expect("Should have succeeded")
                .expect("Should not be None");

            assert!(metrics.is_empty());
        }

        #[tokio::test]
        async fn test_get_activity_with_metrics_activity_does_not_exist() {
            let db_file = NamedTempFile::new().unwrap();
            let repo = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");

            let err = repo
                .get_activity_with_metrics(
                    &UserId::test_default(),
                    &ActivityId::from("non-existing-activity"),
                    &[ActivityMetric::AvgPower],
                )
                .await
                .unwrap_err();

            let GetActivityError::ActivityDoesNotExist(id) = err else {
                unreachable!("Should have returned GetActivityError::ActivityDoesNotExist")
            };
            assert_eq!(id, ActivityId::from("non-existing-activity"));
        }

        #[tokio::test]
        async fn test_get_activity_with_metrics_only_requested_metrics_returned() {
            let db_file = NamedTempFile::new().unwrap();
            let repo = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");
            let activity = build_activity();

            repo.save_activity(&activity)
                .await
                .expect("Should have succeed");

            repo.update_activity_metric(activity.id(), &ActivityMetric::AvgPower, &Some(1.0))
                .await
                .expect("Should have succeeded");
            repo.update_activity_metric(activity.id(), &ActivityMetric::AvgHeartRate, &Some(150.0))
                .await
                .expect("Should have succeeded");

            let (_returned_activity, metrics) = repo
                .get_activity_with_metrics(
                    &UserId::test_default(),
                    activity.id(),
                    &[ActivityMetric::AvgPower],
                )
                .await
                .expect("Should have succeeded")
                .expect("Should not be None");

            assert_eq!(
                metrics,
                ActivityMetrics::new(HashMap::from([(ActivityMetric::AvgPower, Some(1.0))]))
            );
            assert!(!metrics.contains_key(&ActivityMetric::AvgHeartRate));
        }

        #[tokio::test]
        async fn test_get_activity_with_metrics_null_value() {
            let db_file = NamedTempFile::new().unwrap();
            let repo = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                Clock::new(),
            )
            .await
            .expect("repo should init");
            let activity = build_activity();

            repo.save_activity(&activity)
                .await
                .expect("Should have succeed");

            repo.update_activity_metric(activity.id(), &ActivityMetric::AvgPower, &None)
                .await
                .expect("Should have succeeded");

            let (_returned_activity, metrics) = repo
                .get_activity_with_metrics(
                    &UserId::test_default(),
                    activity.id(),
                    &[ActivityMetric::AvgPower],
                )
                .await
                .expect("Should have succeeded")
                .expect("Should not be None");

            assert_eq!(
                metrics,
                ActivityMetrics::new(HashMap::from([(ActivityMetric::AvgPower, None)]))
            );
        }
    }

    #[cfg(test)]
    mod test_t_outbox_activity_search {
        use chrono::Utc;

        use crate::domain::models::search::{SearchDocumentEvent, SearchDocumentType};

        use super::*;

        #[tokio::test]
        async fn test_save_activity_inserts_row_to_outbox_as_updated() {
            let db_file = NamedTempFile::new().unwrap();
            let now = Utc::now();
            let repo = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                FakeClock::new(now),
            )
            .await
            .expect("repo should init");
            let activity =
                build_activity().patch(ActivityPatch::name(Some(ActivityName::from("test name"))));

            // Outbox initially empty
            assert!(
                repo.get_outbox_documents_to_process()
                    .await
                    .expect("Get outbox documents should have succeeded")
                    .is_empty()
            );

            repo.save_activity(&activity)
                .await
                .expect("Should have succeeded");

            // Outbox contains row for the newly saved activity
            let document = repo
                .get_outbox_documents_to_process()
                .await
                .expect("Get outbox documents should have succeeded")
                .first()
                .cloned()
                .expect("Should contain at least one document");

            assert_eq!(document.document_type(), &SearchDocumentType::Activity);
            assert_eq!(document.document_id(), activity.id().to_string());
            assert_eq!(document.event(), &SearchDocumentEvent::Updated);
            assert_eq!(document.occurred_at(), &now);
            assert!(
                document.content().contains(
                    &activity
                        .name()
                        .as_ref()
                        .map(|n| n.to_string())
                        .unwrap_or_default()
                ),
            )
        }

        #[tokio::test]
        async fn test_delete_activity_inserts_row_to_outbox_as_deleted() {
            let db_file = NamedTempFile::new().unwrap();
            let now = Utc::now();
            let repo = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                FakeClock::new(now),
            )
            .await
            .expect("repo should init");
            let activity =
                build_activity().patch(ActivityPatch::name(Some(ActivityName::from("test name"))));

            // Outbox initially empty
            assert!(
                repo.get_outbox_documents_to_process()
                    .await
                    .expect("Get outbox documents should have succeeded")
                    .is_empty()
            );

            repo.delete_activity(activity.user(), activity.id())
                .await
                .expect("Should have succeeded");

            // Outbox contains row for the newly deleted activity
            let document = repo
                .get_outbox_documents_to_process()
                .await
                .expect("Get outbox documents should have succeeded")
                .first()
                .cloned()
                .expect("Should contain at least one document");

            assert_eq!(document.document_type(), &SearchDocumentType::Activity);
            assert_eq!(document.document_id(), activity.id().to_string());
            assert_eq!(document.event(), &SearchDocumentEvent::Deleted);
            assert_eq!(document.occurred_at(), &now);
            assert!(document.content().is_empty());
        }

        #[tokio::test]
        async fn test_mark_outbox_document_as_processed() {
            let db_file = NamedTempFile::new().unwrap();
            let now = Utc::now();
            let repo = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                FakeClock::new(now),
            )
            .await
            .expect("repo should init");
            let activity =
                build_activity().patch(ActivityPatch::name(Some(ActivityName::from("test name"))));

            // Outbox initially empty
            assert!(
                repo.get_outbox_documents_to_process()
                    .await
                    .expect("Get outbox documents should have succeeded")
                    .is_empty()
            );

            repo.save_activity(&activity)
                .await
                .expect("Should have succeeded");

            // Outbox contains row for the newly deleted activity
            let document = repo
                .get_outbox_documents_to_process()
                .await
                .expect("Get outbox documents should have succeeded")
                .first()
                .cloned()
                .expect("Should contain at least one document");

            // Mark document as processed
            let processed_at = chrono::Utc::now();
            repo.mark_outbox_document_as_processed(&document, processed_at.clone())
                .await
                .expect("Marking document as processes should have succeeded");

            // Outox should be empty
            assert!(
                repo.get_outbox_documents_to_process()
                    .await
                    .expect("Get outbox documents should have succeeded")
                    .is_empty()
            );

            // Marking the same document should be idempotent
            repo.mark_outbox_document_as_processed(&document, processed_at)
                .await
                .expect("Marking document as processes should be idempotent");
        }
    }

    mod test_t_outbox_duration_curve {

        use chrono::{DateTime, Utc};

        use super::*;

        type OutboxRow = (
            ActivityId,
            UserId,
            DurationCurveEvent,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
        );

        async fn outbox_rows<R, FP, C>(
            repo: &SqliteActivityRepository<R, FP, C>,
        ) -> Vec<OutboxRow> {
            sqlx::query_as(
                "SELECT activity_id, user_id, event, occurred_at, processed_at
                FROM t_outbox_duration_curve
                ORDER BY rowid;",
            )
            .fetch_all(&repo.readers)
            .await
            .expect("Reading the duration curve outbox should have succeeded")
        }

        async fn test_repository(
            now: DateTime<Utc>,
        ) -> (
            SqliteActivityRepository<
                crate::domain::ports::activity::test_utils::MockRawDataRepository,
                crate::inbound::parser::test_utils::MockFileParser,
                FakeClock,
            >,
            NamedTempFile,
        ) {
            let db_file = NamedTempFile::new().unwrap();
            let repo = SqliteActivityRepository::new(
                &db_file.path().to_string_lossy(),
                MockRawDataRepository::new(),
                MockFileParser::new(),
                FakeClock::new(now),
            )
            .await
            .expect("repo should init");

            (repo, db_file)
        }

        #[tokio::test]
        async fn test_save_duration_curve_posts_created_event() {
            let now = Utc::now();
            let (repo, _db_file) = test_repository(now).await;

            let activity_id = ActivityId::new();
            let user = UserId::test_default();
            let date = ActivityStartTime::from_timestamp(1000).unwrap();

            // Outbox initially empty
            assert!(outbox_rows(&repo).await.is_empty());

            let mut tx = repo.writer.begin().await.unwrap();
            repo.save_duration_curve(
                &mut tx,
                &activity_id,
                &user,
                &date,
                &build_duration_curve(DurationCurveType::Power),
            )
            .await
            .expect("Should have succeeded");
            tx.commit().await.unwrap();

            // Outbox contains a single created event for the curve
            let rows = outbox_rows(&repo).await;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].0, activity_id);
            assert_eq!(rows[0].1, user);
            assert_eq!(rows[0].2, DurationCurveEvent::Created);
            assert_eq!(rows[0].3, now);
            assert_eq!(rows[0].4, None); // not processed yet
        }

        #[tokio::test]
        async fn test_save_duration_curve_conflict_posts_no_event() {
            let now = Utc::now();
            let (repo, _db_file) = test_repository(now).await;

            let activity_id = ActivityId::new();
            let user = UserId::test_default();
            let date = ActivityStartTime::from_timestamp(1000).unwrap();
            let curve = build_duration_curve(DurationCurveType::Power);

            let mut tx = repo.writer.begin().await.unwrap();
            repo.save_duration_curve(&mut tx, &activity_id, &user, &date, &curve)
                .await
                .expect("Should have succeeded");

            // Saving the same curve again is a no-op (curves are immutable) and should
            // not post any event.
            repo.save_duration_curve(&mut tx, &activity_id, &user, &date, &curve)
                .await
                .expect("Should have succeeded");
            tx.commit().await.unwrap();

            // Outbox contains a single created event, for the first save only
            let rows = outbox_rows(&repo).await;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].0, activity_id);
            assert_eq!(rows[0].1, user);
            assert_eq!(rows[0].2, DurationCurveEvent::Created);
            assert_eq!(rows[0].3, now);
            assert_eq!(rows[0].4, None);
        }

        #[tokio::test]
        async fn test_save_activity_posts_created_event_per_curve() {
            let now = Utc::now();
            let (repo, _db_file) = test_repository(now).await;

            // Outbox initially empty
            assert!(outbox_rows(&repo).await.is_empty());

            let activity = build_activity_with_curves(vec![
                build_duration_curve(DurationCurveType::Power),
                build_duration_curve(DurationCurveType::Pace),
            ]);
            repo.save_activity(&activity)
                .await
                .expect("Should have succeeded");

            // Outbox contains one created event per saved curve
            let rows = outbox_rows(&repo).await;
            assert_eq!(rows.len(), 2);
            for row in rows.iter() {
                assert_eq!(row.0, *activity.id());
                assert_eq!(row.1, *activity.user());
                assert_eq!(row.2, DurationCurveEvent::Created);
                assert_eq!(row.3, now);
                assert_eq!(row.4, None);
            }
        }

        #[tokio::test]
        async fn test_delete_activity_posts_deleted_event_per_curve() {
            let now = Utc::now();
            let (repo, _db_file) = test_repository(now).await;

            let activity = build_activity_with_curves(vec![
                build_duration_curve(DurationCurveType::Power),
                build_duration_curve(DurationCurveType::Pace),
            ]);
            repo.save_activity(&activity)
                .await
                .expect("Should have succeeded");

            // The save posted one created event per curve
            let rows = outbox_rows(&repo).await;
            assert_eq!(rows.len(), 2);
            assert!(rows.iter().all(|row| row.2 == DurationCurveEvent::Created));

            repo.delete_activity(activity.user(), activity.id())
                .await
                .expect("Should have succeeded");

            // The deletion posted a single deleted event for the activity's curves
            let rows = outbox_rows(&repo).await;
            assert_eq!(rows.len(), 3);
            for row in rows
                .iter()
                .filter(|row| row.2 == DurationCurveEvent::Deleted)
            {
                assert_eq!(row.0, *activity.id());
                assert_eq!(row.1, *activity.user());
                assert_eq!(row.3, now);
                assert_eq!(row.4, None);
            }
        }
    }
}
