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
                Activity, ActivityDuration, ActivityFeedback, ActivityId, ActivityMetricV2,
                ActivityMetricsV2, ActivityName, ActivityNaturalKey, ActivityNutrition,
                ActivityRpe, ActivityStartTime, ActivityWithParsedData, Sport, WorkoutType,
            },
            search::{SearchDocument, SearchDocumentEvent, SearchDocumentType},
        },
        ports::{
            DateTimeRange, IClock,
            activity::{
                ActivityRepository, GetActivityError, GetRawActivityError, ListActivitiesError,
                ListActivitiesFilters, RawActivity, RawDataRepository, SaveActivityError,
                SimilarActivityError, UpdateActivityMetricError,
            },
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

type SearchDocumentRow = (
    ActivityId,
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
    pub async fn metric_rowid(&self, metric: &ActivityMetricV2) -> Result<i64, anyhow::Error> {
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

        Ok(ActivityWithParsedData::new(
            activity,
            parsed_content.timeseries().clone(),
            parsed_content.statistics().clone(),
        ))
    }

    async fn save_search_document(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        document: SearchDocument,
    ) -> Result<(), anyhow::Error> {
        sqlx::query(
            "
            INSERT INTO t_outbox_activity_search (activity_id, event, content, occurred_at)
            VALUES (?1, ?2, ?3, ?4);",
        )
        .bind(document.document_id())
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
}

impl<R, FP, C> ActivityRepository for SqliteActivityRepository<R, FP, C>
where
    R: RawDataRepository,
    FP: ParseFile,
    C: IClock,
{
    #[tracing::instrument(skip_all, err)]
    async fn delete_activity(&self, activity: &ActivityId) -> Result<(), anyhow::Error> {
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
            SearchDocumentEvent::Deleted,
            String::default(),
            self.clock.now(),
        );

        self.save_search_document(&mut tx, search_document).await?;

        tx.commit().await.map_err(|err| anyhow!(err))
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_activity(&self, id: &ActivityId) -> Result<Option<Activity>, GetActivityError> {
        match sqlx::query_as::<_, ActivityRow>(
            "SELECT id, user_id, name, start_time, duration, sport, rpe, workout_type, nutrition, feedback
            FROM t_activities_v2
            WHERE id = ?1
            LIMIT 1;",
        )
        .bind(id)
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
        id: &ActivityId,
        metrics: &[ActivityMetricV2],
    ) -> Result<Option<(Activity, ActivityMetricsV2)>, GetActivityError> {
        let mut builder = sqlx::QueryBuilder::<'_, Sqlite>::new("
        SELECT
            t_activities_v2.id,
            t_activities_metrics.metric,
            t_activities_metrics_values.value
        FROM t_activities_v2
        JOIN t_activities_metrics_values ON t_activities_metrics_values.activity_rowid = t_activities_v2.rowid
        JOIN t_activities_metrics ON t_activities_metrics_values.metric_rowid = t_activities_metrics.rowid");

        builder.push(" WHERE t_activities_v2.id = ").push_bind(id);

        builder.push(" AND t_activities_metrics.metric IN (");
        for (idx, metric) in metrics.iter().enumerate() {
            builder.push(" ").push_bind(metric);
            if idx < metrics.len() - 1 {
                builder.push(",");
            }
        }
        builder.push(") ");

        let query = builder.build_query_as::<'_, (ActivityId, ActivityMetricV2, Option<f64>)>();
        let mut metrics_values: Vec<(ActivityMetricV2, Option<f64>)> = Vec::new();
        for (_activity, metric, value) in query
            .fetch_all(&self.readers)
            .await
            .map_err(|err| GetActivityError::Unknown(anyhow!(err)))?
        {
            metrics_values.push((metric, value));
        }

        let Some(activity) = self.get_activity(id).await? else {
            return Err(GetActivityError::ActivityDoesNotExist(id.clone()));
        };

        Ok(Some((
            activity,
            ActivityMetricsV2::new(HashMap::from_iter(metrics_values)),
        )))
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_activity_with_parsed_data(
        &self,
        id: &ActivityId,
    ) -> Result<Option<ActivityWithParsedData>, GetActivityError> {
        let activity = match self.get_activity(id).await {
            Ok(Some(activity)) => activity,
            Ok(None) => return Err(GetActivityError::ActivityDoesNotExist(id.clone())),
            Err(err) => return Err(GetActivityError::Unknown(anyhow!(err))),
        };

        let activity_with_parsed_data = match self.load_timeseries(id, activity).await {
            Ok(value) => value,
            Err(err) => return Err(GetActivityError::Unknown(anyhow!(err))),
        };

        Ok(Some(activity_with_parsed_data))
    }

    #[tracing::instrument(skip_all, err)]
    async fn list_activities(
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
        let activities = self.list_activities(user, filters).await?;

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
        metric: &ActivityMetricV2,
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
        metrics: &[ActivityMetricV2],
    ) -> Result<Vec<(Activity, ActivityMetricsV2)>, ListActivitiesError> {
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
        let query = builder.build_query_as::<'_, (ActivityId, ActivityMetricV2, Option<f64>)>();
        let mut metrics_values: HashMap<ActivityId, Vec<(ActivityMetricV2, Option<f64>)>> =
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
        for activity in self.list_activities(user, filters).await? {
            let metrics = metrics_values.remove(activity.id()).unwrap_or_default();
            res.push((
                activity,
                ActivityMetricsV2::new(HashMap::from_iter(metrics)),
            ));
        }

        Ok(res)
    }

    #[tracing::instrument(skip_all, err)]
    async fn save_activity(&self, activity: &Activity) -> Result<(), SaveActivityError> {
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

        let search_document =
            activity.to_search_document(SearchDocumentEvent::Updated, self.clock.now());

        self.save_search_document(&mut tx, search_document).await?;

        tx.commit()
            .await
            .map_err(|err| SaveActivityError::Unknown(err.into()))
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
            "SELECT activity_id, event, content, occurred_at
            FROM t_outbox_activity_search
            WHERE processed_at IS NULL;",
        )
        .fetch_all(&self.readers)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(activity, event, content, occurred_at)| {
                    SearchDocument::new(
                        SearchDocumentType::Activity,
                        activity.to_string(),
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
                    ActiveTime, ActivityDuration, ActivityPatch, ActivityStartTime,
                    ActivityStatistics, ActivityTimeseries, BonkStatus, Sport, Timeseries,
                    TimeseriesActiveTime, TimeseriesMetric, TimeseriesTime, TimeseriesValue,
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

    fn build_activity() -> Activity {
        Activity::new_empty(
            ActivityId::new(),
            UserId::test_default(),
            ActivityStartTime::from_timestamp(random_range(100..1200)).unwrap(),
            ActivityDuration::default(),
            Sport::Cycling,
        )
    }

    fn build_activity_starting_at(start: &DateTime<FixedOffset>) -> Activity {
        Activity::new_empty(
            ActivityId::new(),
            UserId::test_default(),
            ActivityStartTime::new(*start),
            ActivityDuration::default(),
            Sport::Cycling,
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

    #[tokio::test]
    async fn test_save_existing_activity_id_updates_optional_fields() {
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

        let patched_activity = activity.apply_patch(ActivityPatch::new(
            Some(Some(ActivityName::from("Another name"))),
            Some(Some(ActivityRpe::Eight)),
            Some(Some(WorkoutType::CrossTraining)),
            Some(Some(ActivityNutrition::new(BonkStatus::None, None))),
            Some(Some(ActivityFeedback::from("Another feedback"))),
        ));

        repository
            .save_activity(&patched_activity)
            .await
            .expect("Should have succeeded");

        let activity = repository
            .get_activity(patched_activity.id())
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
            .delete_activity(activity.id())
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
            .delete_activity(activity.id())
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
            .get_activity(activity.id())
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
            .get_activity(activity.id())
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
            .list_activities(&UserId::test_default(), &ListActivitiesFilters::empty())
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
            .list_activities(
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
            .list_activities(
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
            .list_activities(
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
            .list_activities(
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
            .get_activity_with_parsed_data(activity.id())
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
            .get_activity_with_parsed_data(activity.id())
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
            .get_activity_with_parsed_data(activity.id())
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
                    &[ActivityMetricV2::AvgPower],
                )
                .await
                .unwrap();
            assert_eq!(res.len(), 1);
            let (_actvity, metrics) = res.first().unwrap();
            assert!(metrics.is_empty(),);

            // Insert a metric value
            repo.update_activity_metric(activity.id(), &ActivityMetricV2::AvgPower, &Some(1.2))
                .await
                .expect("Should have succeeded");

            let res = repo
                .get_activities_with_metrics(
                    activity.user(),
                    &ListActivitiesFilters::empty(),
                    &[ActivityMetricV2::AvgPower],
                )
                .await
                .unwrap();
            assert_eq!(res.len(), 1);
            let (_actvity, metrics) = res.first().unwrap();
            assert_eq!(
                metrics,
                &ActivityMetricsV2::new(HashMap::from([(ActivityMetricV2::AvgPower, Some(1.2))]))
            );

            // Update a metric value
            repo.update_activity_metric(activity.id(), &ActivityMetricV2::AvgPower, &None)
                .await
                .expect("Should have succeeded");

            let res = repo
                .get_activities_with_metrics(
                    activity.user(),
                    &ListActivitiesFilters::empty(),
                    &[ActivityMetricV2::AvgPower],
                )
                .await
                .unwrap();
            assert_eq!(res.len(), 1);
            let (_actvity, metrics) = res.first().unwrap();
            assert_eq!(
                metrics,
                &ActivityMetricsV2::new(HashMap::from([(ActivityMetricV2::AvgPower, None)]))
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
                    &ActivityMetricV2::AvgPower,
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

            repo.update_activity_metric(activity.id(), &ActivityMetricV2::AvgPower, &Some(3.45))
                .await
                .expect("Should have succeeded");

            let (returned_activity, metrics) = repo
                .get_activity_with_metrics(activity.id(), &[ActivityMetricV2::AvgPower])
                .await
                .expect("Should have succeeded")
                .expect("Should not be None");

            assert_eq!(returned_activity.id(), activity.id());
            assert_eq!(
                metrics,
                ActivityMetricsV2::new(HashMap::from([(ActivityMetricV2::AvgPower, Some(3.45))]))
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
                .get_activity_with_metrics(activity.id(), &[ActivityMetricV2::AvgPower])
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
                    &ActivityId::from("non-existing-activity"),
                    &[ActivityMetricV2::AvgPower],
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

            repo.update_activity_metric(activity.id(), &ActivityMetricV2::AvgPower, &Some(1.0))
                .await
                .expect("Should have succeeded");
            repo.update_activity_metric(
                activity.id(),
                &ActivityMetricV2::AvgHeartRate,
                &Some(150.0),
            )
            .await
            .expect("Should have succeeded");

            let (_returned_activity, metrics) = repo
                .get_activity_with_metrics(activity.id(), &[ActivityMetricV2::AvgPower])
                .await
                .expect("Should have succeeded")
                .expect("Should not be None");

            assert_eq!(
                metrics,
                ActivityMetricsV2::new(HashMap::from([(ActivityMetricV2::AvgPower, Some(1.0))]))
            );
            assert!(!metrics.contains_key(&ActivityMetricV2::AvgHeartRate));
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

            repo.update_activity_metric(activity.id(), &ActivityMetricV2::AvgPower, &None)
                .await
                .expect("Should have succeeded");

            let (_returned_activity, metrics) = repo
                .get_activity_with_metrics(activity.id(), &[ActivityMetricV2::AvgPower])
                .await
                .expect("Should have succeeded")
                .expect("Should not be None");

            assert_eq!(
                metrics,
                ActivityMetricsV2::new(HashMap::from([(ActivityMetricV2::AvgPower, None)]))
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
            let activity = build_activity()
                .apply_patch(ActivityPatch::name(Some(ActivityName::from("test name"))));

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
            let activity = build_activity()
                .apply_patch(ActivityPatch::name(Some(ActivityName::from("test name"))));

            // Outbox initially empty
            assert!(
                repo.get_outbox_documents_to_process()
                    .await
                    .expect("Get outbox documents should have succeeded")
                    .is_empty()
            );

            repo.delete_activity(activity.id())
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
            let activity = build_activity()
                .apply_patch(ActivityPatch::name(Some(ActivityName::from("test name"))));

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
}
