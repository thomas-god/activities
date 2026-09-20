use std::str::FromStr;

use anyhow::anyhow;
use chrono::{DateTime, FixedOffset, NaiveDate};
use sqlx::{
    ConnectOptions, QueryBuilder, Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

use crate::{
    domain::{
        models::{
            UserId,
            activity::{ActivityMetric, ActivityMetricSource},
            search::{SearchDocument, SearchDocumentEvent, SearchDocumentType},
            training::{
                ActivitySource, HooperIndex, SubjectiveScale, TrainingMetric,
                TrainingMetricActivityFilters, TrainingMetricAggregate, TrainingMetricDefinition,
                TrainingMetricGranularity, TrainingMetricGroupBy, TrainingMetricId,
                TrainingMetricName, TrainingMetricScope, TrainingMetricSource,
                TrainingMetricSummary, TrainingMetricTarget, TrainingMetricWindow,
                TrainingMetricsOrdering, TrainingNote, TrainingNoteContent, TrainingNoteDate,
                TrainingNoteId, TrainingNoteTitle, TrainingPeriod, TrainingPeriodId,
                TrainingPeriodSports, WeightAndNutrition,
            },
        },
        ports::{
            DateRange, IClock,
            search::RemainingDocuments,
            training::{
                DeleteTrainingMetricError, DeleteTrainingNoteError, DeleteTrainingPeriodError,
                GetTrainingMetricError, GetTrainingMetricsDefinitionsError,
                GetTrainingMetricsOrderingError, GetTrainingNoteError, HooperIndexError,
                SaveTrainingMetricError, SaveTrainingNoteError, SaveTrainingPeriodError,
                SetTrainingMetricsOrderingError, TrainingRepository, UpdateTrainingMetricNameError,
                UpdateTrainingPeriodDatesError, UpdateTrainingPeriodNameError,
                UpdateTrainingPeriodNoteError, WeightAndNutritionError,
            },
        },
    },
    outbound::sqlite::types::RepositoryTrainingMetricSource,
};

type DefinitionRow = (
    TrainingMetricId,
    Option<TrainingMetricName>,
    UserId,
    Option<ActivityMetricSource>,
    Option<RepositoryTrainingMetricSource>,
    Option<TrainingMetricGranularity>,
    Option<TrainingMetricAggregate>,
    TrainingMetricActivityFilters,
    Option<TrainingMetricGroupBy>,
    Option<TrainingPeriodId>,
    Option<TrainingMetricSummary>,
    Option<TrainingMetricTarget>,
);
type TrainingPeriodRow = (
    TrainingPeriodId,
    UserId,
    NaiveDate,
    Option<NaiveDate>,
    String,
    TrainingPeriodSports,
    Option<String>,
);
type TrainingNoteRow = (
    TrainingNoteId,
    UserId,
    Option<TrainingNoteTitle>,
    TrainingNoteContent,
    TrainingNoteDate,
    DateTime<FixedOffset>,
);

type SearchDocumentRow = (
    TrainingNoteId,
    UserId,
    SearchDocumentEvent,
    String,
    chrono::DateTime<chrono::Utc>,
);

type HooperIndexRow = (
    Option<SubjectiveScale>,
    Option<SubjectiveScale>,
    Option<SubjectiveScale>,
    Option<SubjectiveScale>,
    Option<SubjectiveScale>,
);

type WeightAndNutritionRow = (
    Option<f32>,
    Option<f32>,
    Option<f32>,
    Option<f32>,
    Option<f32>,
    Option<f32>,
    Option<f32>,
    Option<f32>,
    Option<f32>,
    Option<f32>,
);

#[derive(Debug, Clone)]
pub struct SqliteTrainingRepository<C> {
    writer: SqlitePool,
    readers: SqlitePool,
    clock: C,
}

impl<C> SqliteTrainingRepository<C> {
    pub async fn new(url: &str, clock: C) -> Result<Self, sqlx::Error> {
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
        sqlx::migrate!("migrations/training").run(&writer).await?;

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
            clock,
        })
    }

    #[tracing::instrument(skip_all, err)]
    async fn save_search_document(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        document: SearchDocument,
    ) -> Result<(), anyhow::Error> {
        sqlx::query(
            "
              INSERT INTO t_outbox_training_search (note_id, user, event, content, occurred_at)
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
                "Unable to save training note search document {}. {err}",
                document.document_id()
            )
        })
    }
}

impl<C> TrainingRepository for SqliteTrainingRepository<C>
where
    C: IClock,
{
    #[tracing::instrument(skip_all, err)]
    async fn save_metric(&self, metric: TrainingMetric) -> Result<(), SaveTrainingMetricError> {
        let definition = metric.definition();
        sqlx::query(
            "INSERT INTO t_training_metrics_definitions
                (id, user_id, activity_metric, granularity, aggregate, filters, group_by, name, training_period_id, summary, target)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT (id) DO UPDATE SET
                    user_id=excluded.user_id,
                    activity_metric=excluded.activity_metric,
                    granularity=excluded.granularity,
                    aggregate=excluded.aggregate,
                    filters=excluded.filters,
                    group_by=excluded.group_by,
                    name=excluded.name,
                    training_period_id=excluded.training_period_id,
                    summary=excluded.summary,
                    target=excluded.target;",

        )
        .bind(metric.id())
        .bind(definition.user())
        .bind(RepositoryTrainingMetricSource::from(definition.source()))
        .bind(definition.window().as_ref().map(|w| w.granularity()))
        .bind(definition.window().as_ref().map(|w| w.aggregate()))
        .bind(definition.filters())
        .bind(definition_group_by(definition))
        .bind(metric.name())
        .bind(metric.scope().period())
        .bind(definition.summary())
        .bind(definition.target())
        .execute(&self.writer)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(err) => {
                if err.is_foreign_key_violation() && metric.scope().period().is_some() {
                    SaveTrainingMetricError::TrainingPeriodDoesNotExist(metric.scope().period().unwrap())
                } else {
                    SaveTrainingMetricError::Unknown(anyhow!(err))
                }
            }
            err =>   SaveTrainingMetricError::Unknown(anyhow!(err))})
        .map(|_| ())
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_metric(
        &self,
        user: &UserId,
        metric: &TrainingMetricId,
    ) -> Result<Option<TrainingMetric>, GetTrainingMetricError> {
        match sqlx::query_as::<_, DefinitionRow>(
            "
        SELECT
            id,
            name,
            user_id,
            source,
            activity_metric,
            granularity,
            aggregate,
            filters,
            group_by,
            training_period_id,
            summary,
            target
        FROM t_training_metrics_definitions
        WHERE id = ?1 AND user_id = ?2 LIMIT 1;",
        )
        .bind(metric)
        .bind(user)
        .fetch_one(&self.readers)
        .await
        {
            Ok((
                id,
                name,
                user_id,
                source,
                metric,
                granularity,
                aggregate,
                filters,
                group_by,
                training_period_id,
                summary,
                target,
            )) => {
                let Some(metric) = parse_definition_row_metric(metric, source, group_by) else {
                    return Ok(None);
                };
                let window = match (granularity, aggregate) {
                    (Some(granularity), Some(aggregate)) => {
                        Some(TrainingMetricWindow::new(granularity, aggregate))
                    }
                    _ => None,
                };
                let scope = match training_period_id {
                    None => TrainingMetricScope::Global,
                    Some(period) => TrainingMetricScope::TrainingPeriod(period),
                };
                let definition = TrainingMetricDefinition::new(
                    user_id,
                    metric,
                    window,
                    filters,
                    summary.unwrap_or_else(TrainingMetricSummary::empty),
                    target,
                );
                Ok(Some(TrainingMetric::new(id, name, scope, definition)))
            }
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(GetTrainingMetricError::Unknown(anyhow!(err))),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_global_metrics(
        &self,
        user: &UserId,
    ) -> Result<Vec<TrainingMetric>, GetTrainingMetricsDefinitionsError> {
        sqlx::query_as::<_, DefinitionRow>(
            "SELECT
                id,
                name,
                user_id,
                source,
                activity_metric,
                granularity,
                aggregate,
                filters,
                group_by,
                training_period_id,
                summary,
                target
            FROM t_training_metrics_definitions
            WHERE user_id = ?1 AND training_period_id IS NULL;",
        )
        .bind(user)
        .fetch_all(&self.readers)
        .await
        .map_err(|err| GetTrainingMetricsDefinitionsError::Unknown(anyhow!(err)))
        .map(|rows| {
            rows.into_iter()
                .filter_map(
                    |(
                        id,
                        name,
                        user_id,
                        source,
                        metric,
                        granularity,
                        aggregate,
                        filters,
                        group_by,
                        training_period_id,
                        summary,
                        target,
                    )| {
                        let metric = parse_definition_row_metric(metric, source, group_by)?;
                        let window = match (granularity, aggregate) {
                            (Some(granularity), Some(aggregate)) => {
                                Some(TrainingMetricWindow::new(granularity, aggregate))
                            }
                            _ => None,
                        };
                        Some(TrainingMetric::new(
                            id,
                            name,
                            TrainingMetricScope::from(&training_period_id),
                            TrainingMetricDefinition::new(
                                user_id,
                                metric,
                                window,
                                filters,
                                summary.unwrap_or_else(TrainingMetricSummary::empty),
                                target,
                            ),
                        ))
                    },
                )
                .collect()
        })
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_period_metrics(
        &self,
        user: &UserId,
        period: &TrainingPeriodId,
    ) -> Result<Vec<TrainingMetric>, GetTrainingMetricsDefinitionsError> {
        sqlx::query_as::<_, DefinitionRow>(
            "SELECT
                id,
                name,
                user_id,
                source,
                activity_metric,
                granularity,
                aggregate,
                filters,
                group_by,
                training_period_id,
                summary,
                target
            FROM t_training_metrics_definitions
            WHERE user_id = ?1 AND training_period_id = ?2;",
        )
        .bind(user)
        .bind(period)
        .fetch_all(&self.readers)
        .await
        .map_err(|err| GetTrainingMetricsDefinitionsError::Unknown(anyhow!(err)))
        .map(|rows| {
            rows.into_iter()
                .filter_map(
                    |(
                        id,
                        name,
                        user_id,
                        source,
                        metric,
                        granularity,
                        aggregate,
                        filters,
                        group_by,
                        training_period_id,
                        summary,
                        target,
                    )| {
                        let metric = parse_definition_row_metric(metric, source, group_by)?;
                        let window = match (granularity, aggregate) {
                            (Some(granularity), Some(aggregate)) => {
                                Some(TrainingMetricWindow::new(granularity, aggregate))
                            }
                            _ => None,
                        };
                        Some(TrainingMetric::new(
                            id,
                            name,
                            TrainingMetricScope::from(&training_period_id),
                            TrainingMetricDefinition::new(
                                user_id,
                                metric,
                                window,
                                filters,
                                summary.unwrap_or_else(TrainingMetricSummary::empty),
                                target,
                            ),
                        ))
                    },
                )
                .collect()
        })
    }

    #[tracing::instrument(skip_all, err)]
    async fn delete_metric(
        &self,
        user: &UserId,
        metric: &TrainingMetricId,
    ) -> Result<(), DeleteTrainingMetricError> {
        match sqlx::query(
            "DELETE FROM t_training_metrics_definitions
        WHERE id = ?1 AND user_id = ?2;",
        )
        .bind(metric)
        .bind(user)
        .execute(&self.writer)
        .await
        {
            Ok(res) => {
                if res.rows_affected() == 1 {
                    Ok(())
                } else {
                    Err(DeleteTrainingMetricError::MetricDoesNotExist(
                        metric.clone(),
                    ))
                }
            }
            Err(err) => Err(DeleteTrainingMetricError::Unknown(anyhow!(err))),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn update_metric_name(
        &self,
        user: &UserId,
        metric_id: &TrainingMetricId,
        name: TrainingMetricName,
    ) -> Result<(), UpdateTrainingMetricNameError> {
        match sqlx::query(
            "UPDATE t_training_metrics_definitions SET name = ?1 WHERE id = ?2 AND user_id = ?3;",
        )
        .bind(name.to_string())
        .bind(metric_id)
        .bind(user)
        .execute(&self.writer)
        .await
        {
            Ok(res) if res.rows_affected() == 1 => Ok(()),
            Ok(_) => Err(UpdateTrainingMetricNameError::MetricDoesNotExist(
                metric_id.clone(),
            )),
            Err(err) => Err(UpdateTrainingMetricNameError::Unknown(anyhow!(err))),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn save_training_period(
        &self,
        period: crate::domain::models::training::TrainingPeriod,
    ) -> Result<(), crate::domain::ports::training::SaveTrainingPeriodError> {
        sqlx::query("INSERT INTO t_training_periods VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);")
            .bind(period.id())
            .bind(period.user())
            .bind(period.start())
            .bind(period.end())
            .bind(period.name())
            .bind(period.sports())
            .bind(period.note())
            .execute(&self.writer)
            .await
            .map_err(|err| SaveTrainingPeriodError::Unknown(anyhow!(err)))
            .map(|_| ())
    }

    #[tracing::instrument(skip_all)]
    async fn get_training_period(
        &self,
        user: &UserId,
        period: &TrainingPeriodId,
    ) -> Option<TrainingPeriod> {
        match sqlx::query_as::<_, TrainingPeriodRow>(
            "
        SELECT id, user_id, start, end, name, sports, note
        FROM t_training_periods
        WHERE id = ?1 AND user_id = ?2 LIMIT 1;",
        )
        .bind(period)
        .bind(user)
        .fetch_one(&self.readers)
        .await
        {
            Ok((id, user_id, start, end, name, sports, note)) => {
                TrainingPeriod::new(id, user_id, start, end, name, sports, note).ok()
            }
            Err(sqlx::Error::RowNotFound) => None,
            Err(_err) => None,
        }
    }

    #[tracing::instrument(skip_all)]
    async fn get_training_periods(
        &self,
        user: &UserId,
    ) -> Vec<crate::domain::models::training::TrainingPeriod> {
        sqlx::query_as::<_, TrainingPeriodRow>(
            "SELECT id, user_id, start, end, name, sports, note
            FROM t_training_periods
            WHERE user_id = ?1;",
        )
        .bind(user)
        .fetch_all(&self.readers)
        .await
        .map(|rows| {
            rows.into_iter()
                .filter_map(|(id, user_id, start, end, name, sports, note)| {
                    TrainingPeriod::new(id, user_id, start, end, name, sports, note).ok()
                })
                .collect()
        })
        .unwrap_or_default()
    }

    #[tracing::instrument(skip_all)]
    async fn get_active_training_periods(
        &self,
        user: &UserId,
        ref_date: &NaiveDate,
    ) -> Vec<crate::domain::models::training::TrainingPeriod> {
        sqlx::query_as::<_, TrainingPeriodRow>(
            "SELECT id, user_id, start, end, name, sports, note
            FROM t_training_periods
            WHERE user_id = ?1
              AND start <= ?2
              AND (end IS NULL OR end >= ?2);",
        )
        .bind(user)
        .bind(ref_date)
        .fetch_all(&self.readers)
        .await
        .map(|rows| {
            rows.into_iter()
                .filter_map(|(id, user_id, start, end, name, sports, note)| {
                    TrainingPeriod::new(id, user_id, start, end, name, sports, note).ok()
                })
                .collect()
        })
        .unwrap_or_default()
    }

    #[tracing::instrument(skip_all, err)]
    async fn delete_training_period(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
    ) -> Result<(), DeleteTrainingPeriodError> {
        match sqlx::query("DELETE FROM t_training_periods WHERE id = ?1 AND user_id = ?2;")
            .bind(period_id)
            .bind(user)
            .execute(&self.writer)
            .await
        {
            Ok(res) if res.rows_affected() == 1 => Ok(()),
            Ok(_) => Err(DeleteTrainingPeriodError::PeriodDoesNotExist(
                period_id.clone(),
            )),
            Err(err) => Err(DeleteTrainingPeriodError::Unknown(anyhow!(err))),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn update_training_period_name(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
        name: String,
    ) -> Result<(), UpdateTrainingPeriodNameError> {
        match sqlx::query("UPDATE t_training_periods SET name = ?1 WHERE id = ?2 AND user_id = ?3;")
            .bind(name)
            .bind(period_id)
            .bind(user)
            .execute(&self.writer)
            .await
        {
            Ok(res) if res.rows_affected() == 1 => Ok(()),
            Ok(_) => Err(UpdateTrainingPeriodNameError::PeriodDoesNotExist(
                period_id.clone(),
            )),
            Err(err) => Err(UpdateTrainingPeriodNameError::Unknown(anyhow!(err))),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn update_training_period_note(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
        note: Option<String>,
    ) -> Result<(), UpdateTrainingPeriodNoteError> {
        match sqlx::query("UPDATE t_training_periods SET note = ?1 WHERE id = ?2 AND user_id = ?3;")
            .bind(note)
            .bind(period_id)
            .bind(user)
            .execute(&self.writer)
            .await
        {
            Ok(res) if res.rows_affected() == 1 => Ok(()),
            Ok(_) => Err(UpdateTrainingPeriodNoteError::PeriodDoesNotExist(
                period_id.clone(),
            )),
            Err(err) => Err(UpdateTrainingPeriodNoteError::Unknown(anyhow!(err))),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn update_training_period_dates(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
        start: NaiveDate,
        end: Option<NaiveDate>,
    ) -> Result<(), UpdateTrainingPeriodDatesError> {
        match sqlx::query(
            "UPDATE t_training_periods SET start = ?1, end = ?2 WHERE id = ?3 AND user_id = ?4;",
        )
        .bind(start)
        .bind(end)
        .bind(period_id)
        .bind(user)
        .execute(&self.writer)
        .await
        {
            Ok(res) if res.rows_affected() == 1 => Ok(()),
            Ok(_) => Err(UpdateTrainingPeriodDatesError::PeriodDoesNotExist(
                period_id.clone(),
            )),
            Err(err) => Err(UpdateTrainingPeriodDatesError::Unknown(anyhow!(err))),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn save_training_note(&self, note: TrainingNote) -> Result<(), SaveTrainingNoteError> {
        let mut tx = self.writer.begin().await.map_err(|err| anyhow!(err))?;

        let _ = sqlx::query(
            "INSERT INTO t_training_notes (id, user_id, title, content, date, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT (id)
            DO UPDATE SET
                title=excluded.title,
                content=excluded.content,
                date=excluded.date;",
        )
        .bind(note.id().to_string())
        .bind(note.user().to_string())
        .bind(note.title().as_ref().map(|t| t.to_string()))
        .bind(note.content().to_string())
        .bind(note.date().to_string())
        .bind(note.created_at().to_rfc3339())
        .execute(&mut *tx)
        .await
        .map_err(|err| SaveTrainingNoteError::Unknown(anyhow!(err)))
        .map(|_| ());

        let document = note.to_search_document(SearchDocumentEvent::Updated, self.clock.now());
        self.save_search_document(&mut tx, document).await?;

        tx.commit()
            .await
            .map_err(|err| SaveTrainingNoteError::Unknown(anyhow!(err)))
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_training_note(
        &self,
        user: &UserId,
        note_id: &TrainingNoteId,
    ) -> Result<Option<TrainingNote>, GetTrainingNoteError> {
        match sqlx::query_as::<_, TrainingNoteRow>(
            "SELECT id, user_id, title, content, date, created_at FROM t_training_notes WHERE id = ?1 AND user_id = ?2 LIMIT 1;",
        )
        .bind(note_id.to_string())
        .bind(user)
        .fetch_one(&self.readers)
        .await
        {
            Ok((id, user_id, title, content, date, created_at)) => {
                Ok(Some(TrainingNote::new(
                    id,
                    user_id,
                    title,
                    content,
                    date,
                    created_at,
                )))
            }
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(GetTrainingNoteError::Unknown(anyhow!(err))),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn list_training_note_documents(
        &self,
        batch_size: i64,
        page: i64,
    ) -> Result<(Vec<SearchDocument>, RemainingDocuments), anyhow::Error> {
        let limit = batch_size + 1; // Extra sentinel row to detect if there is another page after
        let offset = page * batch_size;
        let rows = sqlx::query_as::<_, TrainingNoteRow>(
            "
            SELECT id, user_id, title, content, date, created_at
            FROM t_training_notes
            ORDER BY rowid
            LIMIT ?1 OFFSET ?2;",
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
            .map(|(id, user_id, title, content, date, created_at)| {
                TrainingNote::new(id, user_id, title, content, date, created_at)
                    .to_search_document(SearchDocumentEvent::Updated, self.clock.now())
            })
            .collect();

        Ok((documents, documents_remaining))
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_training_notes(
        &self,
        user: &UserId,
        date_range: &Option<DateRange>,
    ) -> Result<Vec<TrainingNote>, GetTrainingNoteError> {
        let mut builder = QueryBuilder::<'_, Sqlite>::new(
            "SELECT id, user_id, title, content, date, created_at FROM t_training_notes WHERE user_id = ",
        );
        builder.push_bind(user.to_string());

        if let Some(range) = date_range {
            builder.push(" AND date >= ").push_bind(range.start());
            builder.push(" AND date < ").push_bind(range.end());
        }

        builder.push(" ORDER BY created_at DESC");

        let rows = builder
            .build_query_as::<TrainingNoteRow>()
            .fetch_all(&self.readers)
            .await
            .map_err(|err| GetTrainingNoteError::Unknown(anyhow!(err)))?;

        rows.into_iter()
            .map(|(id, user_id, title, content, date, created_at)| {
                Ok(TrainingNote::new(
                    id, user_id, title, content, date, created_at,
                ))
            })
            .collect()
    }

    #[tracing::instrument(skip_all, err)]
    async fn delete_training_note(
        &self,
        user: &UserId,
        note_id: &TrainingNoteId,
    ) -> Result<(), DeleteTrainingNoteError> {
        let mut tx = self.writer.begin().await.map_err(|err| anyhow!(err))?;
        let _ = sqlx::query("DELETE FROM t_training_notes WHERE id = ?1 AND user_id = ?2;")
            .bind(note_id.to_string())
            .bind(user)
            .execute(&mut *tx)
            .await
            .map_err(|err| DeleteTrainingNoteError::Unknown(anyhow!(err)))
            .map(|_| ());

        let document = SearchDocument::new(
            SearchDocumentType::TrainingNote,
            note_id.to_string(),
            user.clone(),
            SearchDocumentEvent::Deleted,
            String::default(),
            self.clock.now(),
        );
        self.save_search_document(&mut tx, document).await?;

        tx.commit()
            .await
            .map_err(|err| DeleteTrainingNoteError::Unknown(anyhow!(err)))
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_training_metrics_ordering(
        &self,
        user: &UserId,
        scope: &TrainingMetricScope,
    ) -> Result<TrainingMetricsOrdering, GetTrainingMetricsOrderingError> {
        let period_id = match scope {
            TrainingMetricScope::Global => None,
            TrainingMetricScope::TrainingPeriod(id) => Some(id.to_string()),
        };

        let result = sqlx::query_as::<_, (String,)>(
            r#"
            SELECT metric_ids
            FROM t_training_metrics_ordering
            WHERE user_id = ?1 AND (training_period_id = ?2 OR (training_period_id IS NULL AND ?2 IS NULL))
            "#,
        )
        .bind(user.to_string())
        .bind(period_id)
        .fetch_optional(&self.readers)
        .await
        .map_err(|e| GetTrainingMetricsOrderingError::Unknown(e.into()))?;

        match result {
            Some((metric_ids_str,)) => {
                let metric_ids: Vec<TrainingMetricId> = metric_ids_str
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(TrainingMetricId::from)
                    .collect();

                TrainingMetricsOrdering::try_from(metric_ids).map_err(|_| {
                    GetTrainingMetricsOrderingError::Unknown(anyhow!(
                        "Invalid ordering data in database"
                    ))
                })
            }
            None => Ok(TrainingMetricsOrdering::try_from(vec![]).unwrap()),
        }
    }

    #[tracing::instrument(skip_all, err)]
    async fn set_training_metrics_ordering(
        &self,
        user: &UserId,
        scope: &TrainingMetricScope,
        ordering: TrainingMetricsOrdering,
    ) -> Result<(), SetTrainingMetricsOrderingError> {
        let period_id = match scope {
            TrainingMetricScope::Global => None,
            TrainingMetricScope::TrainingPeriod(id) => Some(id.to_string()),
        };

        // Convert ordering to comma-separated string
        let metric_ids_str = ordering
            .ids()
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        // First, try to update existing row
        let rows_affected = sqlx::query(
            r#"
            UPDATE t_training_metrics_ordering
            SET metric_ids = ?3
            WHERE user_id = ?1 AND (training_period_id = ?2 OR (training_period_id IS NULL AND ?2 IS NULL))
            "#,
        )
        .bind(user.to_string())
        .bind(&period_id)
        .bind(&metric_ids_str)
        .execute(&self.writer)
        .await
        .map_err(|e| SetTrainingMetricsOrderingError::Unknown(e.into()))?
        .rows_affected();

        // If no rows were updated, insert a new row
        if rows_affected == 0 {
            sqlx::query(
                r#"
                INSERT INTO t_training_metrics_ordering (user_id, training_period_id, metric_ids)
                VALUES (?1, ?2, ?3)
                "#,
            )
            .bind(user.to_string())
            .bind(period_id)
            .bind(metric_ids_str)
            .execute(&self.writer)
            .await
            .map_err(|e| SetTrainingMetricsOrderingError::Unknown(e.into()))?;
        }

        Ok(())
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_outbox_documents_to_process(&self) -> Result<Vec<SearchDocument>, anyhow::Error> {
        sqlx::query_as::<_, SearchDocumentRow>(
            "SELECT note_id, user, event, content, occurred_at
            FROM t_outbox_training_search
            WHERE processed_at IS NULL;",
        )
        .fetch_all(&self.readers)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(note, user, event, content, occurred_at)| {
                    SearchDocument::new(
                        SearchDocumentType::TrainingNote,
                        note.to_string(),
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
            "UPDATE t_outbox_training_search SET processed_at = ?1
            WHERE note_id = ?2 AND event = ?3 AND content = ?4 AND occurred_at = ?5;",
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

    async fn save_hooper_index(
        &self,
        user: &UserId,
        date: chrono::NaiveDate,
        value: &HooperIndex,
    ) -> Result<(), HooperIndexError> {
        sqlx::query(
            "
            INSERT INTO t_hooper_index (user, date, fatigue, pain, sleep, stress, mood)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT (user, date)
            DO UPDATE SET
                fatigue=excluded.fatigue,
                pain=excluded.pain,
                sleep=excluded.sleep,
                stress=excluded.stress,
                mood=excluded.mood",
        )
        .bind(user)
        .bind(date)
        .bind(value.fatigue())
        .bind(value.pain())
        .bind(value.sleep())
        .bind(value.stress())
        .bind(value.mood())
        .execute(&self.writer)
        .await
        .map(|_| ())
        .map_err(|err| HooperIndexError::Unknown(anyhow!(err)))
    }

    async fn get_hooper_index(
        &self,
        user: &UserId,
        date: chrono::NaiveDate,
    ) -> Result<Option<HooperIndex>, HooperIndexError> {
        let row = sqlx::query_as::<_, HooperIndexRow>(
            "
            SELECT fatigue, sleep, pain, stress, mood FROM t_hooper_index
            WHERE user=?1 AND date=?2;",
        )
        .bind(user)
        .bind(date)
        .fetch_optional(&self.readers)
        .await
        .map_err(|err| HooperIndexError::Unknown(anyhow!(err)))?;

        Ok(row.map(|(fatigue, sleep, pain, stress, mood)| {
            HooperIndex::new(fatigue, sleep, pain, stress, mood)
        }))
    }

    async fn get_hooper_indexes(
        &self,
        user: &UserId,
        range: &DateRange,
    ) -> Result<Vec<(chrono::NaiveDate, HooperIndex)>, HooperIndexError> {
        let rows = sqlx::query_as::<
            _,
            (
                chrono::NaiveDate,
                Option<SubjectiveScale>,
                Option<SubjectiveScale>,
                Option<SubjectiveScale>,
                Option<SubjectiveScale>,
                Option<SubjectiveScale>,
            ),
        >(
            "
              SELECT date, fatigue, sleep, pain, stress, mood FROM t_hooper_index
              WHERE user=?1 AND date >= ?2 AND date < ?3
              ORDER BY date ASC;",
        )
        .bind(user)
        .bind(range.start())
        .bind(range.end())
        .fetch_all(&self.readers)
        .await
        .map_err(|err| HooperIndexError::Unknown(anyhow!(err)))?;

        Ok(rows
            .into_iter()
            .map(|(date, fatigue, sleep, pain, stress, mood)| {
                (date, HooperIndex::new(fatigue, sleep, pain, stress, mood))
            })
            .collect())
    }

    async fn delete_hooper_index(
        &self,
        user: &UserId,
        date: chrono::NaiveDate,
    ) -> Result<(), HooperIndexError> {
        sqlx::query(
            "
            DELETE FROM t_hooper_index
            WHERE user=?1 AND date=?2",
        )
        .bind(user)
        .bind(date)
        .execute(&self.writer)
        .await
        .map(|_| ())
        .map_err(|err| HooperIndexError::Unknown(anyhow!(err)))
    }

    async fn save_weight_and_nutrition(
        &self,
        user: &UserId,
        date: chrono::NaiveDate,
        value: &WeightAndNutrition,
    ) -> Result<(), WeightAndNutritionError> {
        sqlx::query(
            "
            INSERT INTO t_weight_and_nutrition
                (user, date, weight, fat, muscle, bmi, calories, lipid, carbs, protein, water, alcohol)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT (user, date)
            DO UPDATE SET
                weight=excluded.weight,
                fat=excluded.fat,
                muscle=excluded.muscle,
                bmi=excluded.bmi,
                calories=excluded.calories,
                lipid=excluded.lipid,
                carbs=excluded.carbs,
                protein=excluded.protein,
                water=excluded.water,
                alcohol=excluded.alcohol;",
        )
        .bind(user)
        .bind(date)
        .bind(value.weight())
        .bind(value.fat())
        .bind(value.muscle())
        .bind(value.bmi())
        .bind(value.calories())
        .bind(value.lipid())
        .bind(value.carbs())
        .bind(value.protein())
        .bind(value.water())
        .bind(value.alcohol())
        .execute(&self.writer)
        .await
        .map(|_| ())
        .map_err(|err| WeightAndNutritionError::Unknown(anyhow!(err)))
    }

    async fn save_bulk_weight_and_nutrition(
        &self,
        user: &UserId,
        values: Vec<(chrono::NaiveDate, WeightAndNutrition)>,
    ) -> Result<(), WeightAndNutritionError> {
        // see https://www.sqlite.org/limits.html#max_variable_number
        // + we have 12 variables to bind per row
        let batch_size = 32766 / 12;
        let mut iter = values.into_iter();

        let mut batch = iter.by_ref().take(batch_size).peekable();
        while batch.peek().is_some() {
            let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                "INSERT INTO t_weight_and_nutrition
                    (user, date, weight, fat, muscle, bmi, calories, lipid, carbs, protein, water, alcohol) "
            );
            query_builder.push_values(batch, |mut b, (date, value)| {
                b.push_bind(user)
                    .push_bind(date)
                    .push_bind(value.weight())
                    .push_bind(value.fat())
                    .push_bind(value.muscle())
                    .push_bind(value.bmi())
                    .push_bind(value.calories())
                    .push_bind(value.lipid())
                    .push_bind(value.carbs())
                    .push_bind(value.protein())
                    .push_bind(value.water())
                    .push_bind(value.alcohol());
            });

            query_builder.push(
                "ON CONFLICT (user, date)
                DO UPDATE SET
                    weight=excluded.weight,
                    fat=excluded.fat,
                    muscle=excluded.muscle,
                    bmi=excluded.bmi,
                    calories=excluded.calories,
                    lipid=excluded.lipid,
                    carbs=excluded.carbs,
                    protein=excluded.protein,
                    water=excluded.water,
                    alcohol=excluded.alcohol;",
            );

            let query = query_builder.build();

            if let Err(err) = query.execute(&self.writer).await {
                return Err(WeightAndNutritionError::Unknown(anyhow!(err)));
            }

            batch = iter.by_ref().take(batch_size).peekable();
        }

        Ok(())
    }

    async fn get_weight_and_nutrition(
        &self,
        user: &UserId,
        date: chrono::NaiveDate,
    ) -> Result<Option<WeightAndNutrition>, WeightAndNutritionError> {
        let row = sqlx::query_as::<_, WeightAndNutritionRow>(
            "
            SELECT weight, fat, muscle, bmi, calories, lipid, carbs, protein, water, alcohol
            FROM t_weight_and_nutrition
            WHERE user=?1 AND date=?2;",
        )
        .bind(user)
        .bind(date)
        .fetch_optional(&self.readers)
        .await
        .map_err(|err| WeightAndNutritionError::Unknown(anyhow!(err)))?;

        Ok(row.map(
            |(weight, fat, muscle, bmi, calories, lipid, carbs, protein, water, alcohol)| {
                WeightAndNutrition::new(
                    weight, fat, muscle, bmi, calories, lipid, carbs, protein, water, alcohol,
                )
            },
        ))
    }

    async fn get_weight_and_nutritions(
        &self,
        user: &UserId,
        range: &DateRange,
    ) -> Result<Vec<(chrono::NaiveDate, WeightAndNutrition)>, WeightAndNutritionError> {
        let rows = sqlx::query_as::<
            _,
            (
                chrono::NaiveDate,
                Option<f32>,
                Option<f32>,
                Option<f32>,
                Option<f32>,
                Option<f32>,
                Option<f32>,
                Option<f32>,
                Option<f32>,
                Option<f32>,
                Option<f32>,
            ),
        >(
            "
              SELECT date, weight, fat, muscle, bmi, calories, lipid, carbs, protein, water, alcohol
              FROM t_weight_and_nutrition
              WHERE user=?1 AND date >= ?2 AND date < ?3
              ORDER BY date ASC;",
        )
        .bind(user)
        .bind(range.start())
        .bind(range.end())
        .fetch_all(&self.readers)
        .await
        .map_err(|err| WeightAndNutritionError::Unknown(anyhow!(err)))?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    date,
                    weight,
                    fat,
                    muscle,
                    bmi,
                    calories,
                    lipid,
                    carbs,
                    protein,
                    water,
                    alcohol,
                )| {
                    (
                        date,
                        WeightAndNutrition::new(
                            weight, fat, muscle, bmi, calories, lipid, carbs, protein, water,
                            alcohol,
                        ),
                    )
                },
            )
            .collect())
    }

    async fn delete_weight_and_nutrition(
        &self,
        user: &UserId,
        date: chrono::NaiveDate,
    ) -> Result<(), WeightAndNutritionError> {
        sqlx::query(
            "
            DELETE FROM t_weight_and_nutrition
            WHERE user=?1 AND date=?2",
        )
        .bind(user)
        .bind(date)
        .execute(&self.writer)
        .await
        .map(|_| ())
        .map_err(|err| WeightAndNutritionError::Unknown(anyhow!(err)))
    }
}

fn parse_definition_row_metric(
    metric: Option<RepositoryTrainingMetricSource>,
    source: Option<ActivityMetricSource>,
    group_by: Option<TrainingMetricGroupBy>,
) -> Option<TrainingMetricSource> {
    match (metric, source.map(ActivityMetric::try_from)) {
        (Some(metric), _) => Some(match metric {
            RepositoryTrainingMetricSource::Activity(metric) => {
                TrainingMetricSource::Activity(ActivitySource::new(metric, group_by))
            }
            RepositoryTrainingMetricSource::HooperIndex(source) => {
                TrainingMetricSource::HooperIndex(source)
            }
            RepositoryTrainingMetricSource::WeightAndNutrition(source) => {
                TrainingMetricSource::WeightAndNutrition(source)
            }
        }),
        (None, Some(Ok(metric))) => Some(TrainingMetricSource::Activity(ActivitySource::new(
            metric, group_by,
        ))),
        _ => None,
    }
}

fn definition_group_by(definition: &TrainingMetricDefinition) -> &Option<TrainingMetricGroupBy> {
    match definition.source() {
        TrainingMetricSource::Activity(source) => source.group_by(),
        _ => &None,
    }
}

#[cfg(test)]
mod test_sqlite_training_repository {

    use std::ops::Add;

    use chrono::{Days, NaiveDate, Utc};
    use tempfile::NamedTempFile;

    use crate::{
        clock::Clock,
        domain::models::{
            activity::{ActivityMetricSource, Sport, TimeseriesAggregate, TimeseriesMetric, Unit},
            training::{
                SportFilter, TrainingMetricActivityFilters, TrainingMetricAggregate,
                TrainingMetricDefinitionPatch, TrainingMetricGranularity, TrainingMetricPatch,
                TrainingMetricSummaryAverage, TrainingMetricTarget, TrainingNote,
                TrainingNoteContent, TrainingNoteId, TrainingNoteTitle, TrainingPeriod,
                TrainingPeriodId, TrainingPeriodSports, HooperIndexSource,
            },
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_init_table() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        sqlx::query("select count(*) from t_training_metrics_definitions;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();

        sqlx::query("select count(*) from t_training_periods;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();

        sqlx::query("select count(*) from t_training_notes;")
            .fetch_one(&repository.readers)
            .await
            .unwrap();
    }

    fn build_global_metric() -> TrainingMetric {
        TrainingMetric::new(
            TrainingMetricId::new(),
            None,
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                UserId::test_default(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::MaxAltitude,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Max,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        )
    }

    fn build_metric_scoped_to_period(period: &TrainingPeriodId) -> TrainingMetric {
        TrainingMetric::new(
            TrainingMetricId::new(),
            None,
            TrainingMetricScope::TrainingPeriod(period.clone()),
            TrainingMetricDefinition::new(
                UserId::test_default(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::MaxAltitude,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Max,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        )
    }

    fn build_metric_definition_with_filters() -> TrainingMetric {
        TrainingMetric::new(
            TrainingMetricId::new(),
            None,
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                UserId::test_default(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::MaxAltitude,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Max,
                )),
                TrainingMetricActivityFilters::new(
                    Some(vec![SportFilter::Sport(Sport::Running)]),
                    None,
                    None,
                    None,
                ),
                TrainingMetricSummary::empty(),
                None,
            ),
        )
    }

    fn build_metric_definition_with_group_by() -> TrainingMetric {
        TrainingMetric::new(
            TrainingMetricId::new(),
            None,
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                UserId::test_default(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::MaxAltitude,
                    Some(TrainingMetricGroupBy::Sport),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Max,
                )),
                TrainingMetricActivityFilters::new(
                    Some(vec![SportFilter::Sport(Sport::Running)]),
                    None,
                    None,
                    None,
                ),
                TrainingMetricSummary::empty(),
                None,
            ),
        )
    }
    fn build_metric_definition_with_target() -> TrainingMetric {
        TrainingMetric::new(
            TrainingMetricId::new(),
            None,
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                UserId::test_default(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::Distance,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Weekly,
                    TrainingMetricAggregate::Sum,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                Some(TrainingMetricTarget::new(100.0, Unit::Kilometer)),
            ),
        )
    }
    fn build_metric_definition_with_summary() -> TrainingMetric {
        TrainingMetric::new(
            TrainingMetricId::new(),
            None,
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                UserId::test_default(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::MaxAltitude,
                    Some(TrainingMetricGroupBy::Sport),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Max,
                )),
                TrainingMetricActivityFilters::new(
                    Some(vec![SportFilter::Sport(Sport::Running)]),
                    None,
                    None,
                    None,
                ),
                TrainingMetricSummary::new(Some(TrainingMetricSummaryAverage::new(true))),
                None,
            ),
        )
    }

    #[tokio::test]
    async fn test_save_training_metric_definition_scope_global() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let definition = build_global_metric();

        repository
            .save_metric(definition)
            .await
            .expect("Should have return Ok");
    }

    #[tokio::test]
    async fn test_save_training_metric_definition_without_window_round_trip() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("No Window Metric")),
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                UserId::test_default(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::Distance,
                    TrainingMetricGroupBy::none(),
                )),
                None,
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        );

        repository
            .save_metric(metric.clone())
            .await
            .expect("Should have return Ok");

        let saved_metric = repository
            .get_metric(metric.definition().user(), metric.id())
            .await
            .expect("Should have returned OK")
            .expect("Should have returned Some");
        assert_eq!(saved_metric.definition(), metric.definition());

        let saved_metrics = repository
            .get_global_metrics(metric.definition().user())
            .await
            .expect("Should have returned OK");
        assert_eq!(saved_metrics, vec![metric]);
    }

    #[tokio::test]
    async fn test_save_training_metric_definition_scope_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let period = build_training_period();
        let definition = build_metric_scoped_to_period(period.id());

        repository
            .save_training_period(period)
            .await
            .expect("Should have created the period");

        repository
            .save_metric(definition)
            .await
            .expect("Should have return Ok");
    }

    #[tokio::test]
    async fn test_save_training_metric_definition_scope_period_fails_if_period_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let definition =
            build_metric_scoped_to_period(&TrainingPeriodId::from("non-existing-period"));

        let Err(SaveTrainingMetricError::TrainingPeriodDoesNotExist(id)) =
            repository.save_metric(definition).await
        else {
            unreachable!("Should have returned err")
        };

        assert_eq!(id, TrainingPeriodId::from("non-existing-period"))
    }

    #[tokio::test]
    async fn test_save_training_metrics_with_group_by() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let definition = build_metric_definition_with_group_by();

        repository
            .save_metric(definition)
            .await
            .expect("Should have return Ok");

        assert_eq!(
            sqlx::query_scalar::<_, Option<TrainingMetricGroupBy>>(
                "select group_by from t_training_metrics_definitions limit 1;"
            )
            .fetch_one(&repository.readers)
            .await
            .unwrap(),
            Some(TrainingMetricGroupBy::Sport)
        );
    }

    #[tokio::test]
    async fn test_save_training_metric_definition_summary() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric = build_metric_definition_with_summary();

        repository
            .save_metric(metric.clone())
            .await
            .expect("Should have return Ok");

        let saved_metric = repository
            .get_metric(metric.definition().user(), metric.id())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(metric, saved_metric);
    }

    #[tokio::test]
    async fn test_save_training_metric_definition_target() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric = build_metric_definition_with_target();

        repository
            .save_metric(metric.clone())
            .await
            .expect("Should have return Ok");

        let raw_target = sqlx::query_scalar::<_, Option<TrainingMetricTarget>>(
            "select target from t_training_metrics_definitions limit 1;",
        )
        .fetch_one(&repository.readers)
        .await
        .expect("target column should exist");

        assert_eq!(raw_target, *metric.definition().target());

        let saved_metric = repository
            .get_metric(metric.definition().user(), metric.id())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(metric, saved_metric);
    }

    #[tokio::test]
    async fn test_save_existing_metric() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric = build_global_metric();

        repository
            .save_metric(metric.clone())
            .await
            .expect("Should have return Ok");

        let new_metric = metric.apply_patch(TrainingMetricPatch::new(
            TrainingMetricName::from("another-name"),
            TrainingMetricDefinitionPatch::new(
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::AvgPace,
                    TrainingMetricGroupBy::none(),
                )),
                None,
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        ));

        repository
            .save_metric(new_metric.clone())
            .await
            .expect("Should have return Ok");

        let saved_metric = repository
            .get_metric(new_metric.definition().user(), new_metric.id())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(saved_metric, new_metric);
    }

    #[tokio::test]
    async fn test_get_metric() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric = build_global_metric();

        repository
            .save_metric(metric.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_metric(metric.definition().user(), metric.id())
            .await
            .expect("Should have returned OK")
            .expect("Should have returned Some");

        assert_eq!(res, metric);
    }

    #[tokio::test]
    async fn test_get_metric_with_filters() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric = build_metric_definition_with_filters();

        repository
            .save_metric(metric.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_metric(metric.definition().user(), metric.id())
            .await
            .expect("Should have returned OK")
            .expect("Should have returned Some");

        assert_eq!(res, metric);
    }

    #[tokio::test]
    async fn test_get_metric_with_group_by() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric = build_metric_definition_with_group_by();

        repository
            .save_metric(metric.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_metric(metric.definition().user(), metric.id())
            .await
            .expect("Should have returned OK")
            .expect("Should have returned Some");

        assert_eq!(res, metric);
    }

    #[tokio::test]
    async fn test_get_metric_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let res = repository
            .get_metric(&UserId::test_default(), &TrainingMetricId::new())
            .await
            .expect("Should have returned OK");
        assert!(res.is_none());
    }

    #[tokio::test]
    async fn test_get_metrics_for_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let definition = build_global_metric();
        repository
            .save_metric(definition.clone())
            .await
            .expect("Should have return Ok");
        let definition_with_filters = build_metric_definition_with_filters();
        repository
            .save_metric(definition_with_filters.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_global_metrics(&UserId::test_default())
            .await
            .expect("Should have returned OK");

        assert_eq!(res.len(), 2);
        assert!(res.contains(&definition));
        assert!(res.contains(&definition_with_filters));
    }

    #[tokio::test]
    async fn test_get_metrics_for_user_only() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let definition = build_global_metric();
        repository
            .save_metric(definition.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_global_metrics(&UserId::from("another_user".to_string()))
            .await
            .expect("Should have returned OK");

        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_get_metrics_with_filters() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let definition = build_metric_definition_with_filters();

        repository
            .save_metric(definition.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_global_metrics(&UserId::test_default())
            .await
            .expect("Should have returned OK");

        assert_eq!(res, vec![definition]);
    }

    #[tokio::test]
    async fn test_get_metrics_with_group_by() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let definition = build_metric_definition_with_group_by();

        repository
            .save_metric(definition.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_global_metrics(&UserId::test_default())
            .await
            .expect("Should have returned OK");

        assert_eq!(res, vec![definition]);
    }

    #[tokio::test]
    async fn test_delete_definition_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric = build_global_metric();
        repository
            .save_metric(metric.clone())
            .await
            .expect("Should have return Ok");

        repository
            .delete_metric(metric.definition().user(), metric.id())
            .await
            .expect("Should have returned OK");
    }

    #[tokio::test]
    async fn test_delete_definition_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let id = TrainingMetricId::new();
        let err = repository.delete_metric(&UserId::test_default(), &id).await;

        let Err(DeleteTrainingMetricError::MetricDoesNotExist(err_id)) = err else {
            unreachable!("Should have been an err")
        };
        assert_eq!(err_id, id);
    }

    #[tokio::test]
    async fn test_update_training_metric_name_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create a metric
        let metric = build_global_metric();
        repository
            .save_metric(metric.clone())
            .await
            .expect("Should save metric");

        // Update the name
        let new_name = TrainingMetricName::from("Updated Metric Name");
        let result = repository
            .update_metric_name(metric.definition().user(), metric.id(), new_name.clone())
            .await;
        assert!(result.is_ok());

        // Verify the name was updated
        let metric = repository
            .get_metric(metric.definition().user(), metric.id())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            metric.name(),
            &Some(TrainingMetricName::from("Updated Metric Name"))
        );

        // Verify other fields unchanged
        assert_eq!(metric.definition().user(), metric.definition().user());
        assert_eq!(metric.definition().source(), metric.definition().source());
        assert_eq!(
            metric.definition().window().as_ref().unwrap().granularity(),
            metric.definition().window().as_ref().unwrap().granularity()
        );
        assert_eq!(
            metric.definition().window().as_ref().unwrap().aggregate(),
            metric.definition().window().as_ref().unwrap().aggregate()
        );
    }

    #[tokio::test]
    async fn test_update_training_metric_name_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Try to update a non-existent metric
        let metric_id = TrainingMetricId::new();
        let result = repository
            .update_metric_name(
                &UserId::test_default(),
                &metric_id,
                TrainingMetricName::from("New Name"),
            )
            .await
            .expect_err("Should fail");

        let UpdateTrainingMetricNameError::MetricDoesNotExist(id) = result else {
            unreachable!(" Should be UpdateTrainingMetricNameError::MetricDoesNotExist(id)")
        };
        assert_eq!(id, metric_id);
    }

    #[tokio::test]
    async fn test_update_training_metric_name_only_updates_specified_metric() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create two metrics
        let metric1 = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Metric 1")),
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                UserId::test_default(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::MaxAltitude,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Max,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        );
        let metric2 = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Metric 2")),
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                UserId::test_default(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::Distance,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Weekly,
                    TrainingMetricAggregate::Sum,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        );

        repository
            .save_metric(metric1.clone())
            .await
            .expect("Should save metric 1");
        repository
            .save_metric(metric2.clone())
            .await
            .expect("Should save metric 2");

        // Update only metric1's name
        let new_name = TrainingMetricName::from("Updated First Metric");
        let result = repository
            .update_metric_name(metric1.definition().user(), metric1.id(), new_name.clone())
            .await;
        assert!(result.is_ok());

        // Verify metric1's name was updated by fetching all metrics
        let all_metrics = repository
            .get_global_metrics(metric1.definition().user())
            .await
            .expect("Should fetch metrics");

        let fetched_metric1 = all_metrics.iter().find(|m| m.id() == metric1.id()).unwrap();
        assert_eq!(fetched_metric1.name(), &Some(new_name));

        // Verify metric2's name is unchanged
        let fetched_metric2 = all_metrics.iter().find(|m| m.id() == metric2.id()).unwrap();
        assert_eq!(fetched_metric2.name(), metric2.name());
    }

    #[tokio::test]
    async fn test_backward_compatibility_null_training_period_id() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Manually insert a metric with NULL training_period_id (testing backward compatibility for existing metrics)
        let metric_id = TrainingMetricId::new();
        let user_id = UserId::test_default();
        sqlx::query(
            "INSERT INTO t_training_metrics_definitions (id, user_id, source, granularity, aggregate, filters, group_by, name, training_period_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL);",
        )
        .bind(&metric_id)
        .bind(&user_id)
        .bind(ActivityMetricSource::Timeseries((
            TimeseriesMetric::Altitude,
            TimeseriesAggregate::Max,
        )))
        .bind(TrainingMetricGranularity::Daily)
        .bind(TrainingMetricAggregate::Max)
        .bind(TrainingMetricActivityFilters::empty())
        .bind(TrainingMetricGroupBy::none())
        .bind::<Option<String>>(None)
        .execute(&repository.writer)
        .await
        .expect("Should insert metric with NULL training_period_id");

        let metrics = repository
            .get_global_metrics(&user_id)
            .await
            .expect("Should fetch metrics");

        assert_eq!(metrics.len(), 1);
        let metric = &metrics[0];

        assert_eq!(metric.scope(), &TrainingMetricScope::Global);
        assert_eq!(metric.id(), &metric_id);
    }

    fn build_training_period() -> TrainingPeriod {
        TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-10-01".parse::<NaiveDate>().unwrap(),
            None,
            "test period".into(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_save_training_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let period = build_training_period();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should have return Ok");

        assert_eq!(
            sqlx::query_scalar::<_, i64>("select count(*) from t_training_periods where id = ?1")
                .bind(period.id())
                .fetch_one(&repository.readers)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_get_training_period_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let expected_period = build_training_period();
        repository
            .save_training_period(expected_period.clone())
            .await
            .expect("Should have return Ok");

        let period = repository
            .get_training_period(expected_period.user(), expected_period.id())
            .await
            .unwrap();

        assert_eq!(period, expected_period);
    }

    #[tokio::test]
    async fn test_get_training_period_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        assert!(
            repository
                .get_training_period(&UserId::test_default(), &TrainingPeriodId::new())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_get_training_period_does_not_match_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let initial_period = build_training_period();
        repository
            .save_training_period(initial_period.clone())
            .await
            .expect("Should have return Ok");

        assert!(
            repository
                .get_training_period(
                    &UserId::from("another_user".to_string()),
                    initial_period.id()
                )
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_get_training_periods_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let expected_period = build_training_period();
        repository
            .save_training_period(expected_period.clone())
            .await
            .expect("Should have return Ok");

        let periods = repository
            .get_training_periods(expected_period.user())
            .await;

        assert_eq!(periods, vec![expected_period]);
    }

    #[tokio::test]
    async fn test_get_training_periods_empty() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let periods = repository
            .get_training_periods(&UserId::test_default())
            .await;

        assert!(periods.is_empty());
    }

    #[tokio::test]
    async fn test_get_training_periods_exclude_periods_from_other_users() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let expected_period = build_training_period();
        repository
            .save_training_period(expected_period.clone())
            .await
            .expect("Should have return Ok");

        let periods = repository
            .get_training_periods(&UserId::from("another_user".to_string()))
            .await;

        assert!(periods.is_empty());
    }

    #[tokio::test]
    async fn test_get_active_training_periods_with_both_start_and_end() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create a period from 2025-10-01 to 2025-12-31
        let period = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-10-01".parse::<NaiveDate>().unwrap(),
            Some("2025-12-31".parse::<NaiveDate>().unwrap()),
            "Q4 2025".into(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        // Test ref_date within period (should return the period)
        let ref_date = "2025-11-15".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(period.user(), &ref_date)
            .await;
        assert_eq!(active_periods, vec![period.clone()]);

        // Test ref_date on start date (should return the period)
        let ref_date_start = "2025-10-01".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(period.user(), &ref_date_start)
            .await;
        assert_eq!(active_periods, vec![period.clone()]);

        // Test ref_date on end date (should return the period)
        let ref_date_end = "2025-12-31".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(period.user(), &ref_date_end)
            .await;
        assert_eq!(active_periods, vec![period.clone()]);

        // Test ref_date before start (should return empty)
        let ref_date_before = "2025-09-30".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(period.user(), &ref_date_before)
            .await;
        assert!(active_periods.is_empty());

        // Test ref_date after end (should return empty)
        let ref_date_after = "2026-01-01".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(period.user(), &ref_date_after)
            .await;
        assert!(active_periods.is_empty());
    }

    #[tokio::test]
    async fn test_get_active_training_periods_with_no_end_date() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create a period starting 2025-10-01 with no end date
        let period = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-10-01".parse::<NaiveDate>().unwrap(),
            None,
            "Open-ended period".into(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        // Test ref_date after start (should return the period)
        let ref_date = "2026-06-15".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(period.user(), &ref_date)
            .await;
        assert_eq!(active_periods, vec![period.clone()]);

        // Test ref_date on start date (should return the period)
        let ref_date_start = "2025-10-01".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(period.user(), &ref_date_start)
            .await;
        assert_eq!(active_periods, vec![period.clone()]);

        // Test ref_date before start (should return empty)
        let ref_date_before = "2025-09-30".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(period.user(), &ref_date_before)
            .await;
        assert!(active_periods.is_empty());
    }

    #[tokio::test]
    async fn test_get_active_training_periods_multiple_periods() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create multiple periods with different date ranges
        let period1 = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-01-01".parse::<NaiveDate>().unwrap(),
            Some("2025-03-31".parse::<NaiveDate>().unwrap()),
            "Q1 2025".into(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        let period2 = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-04-01".parse::<NaiveDate>().unwrap(),
            Some("2025-06-30".parse::<NaiveDate>().unwrap()),
            "Q2 2025".into(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        let period3 = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-07-01".parse::<NaiveDate>().unwrap(),
            None,
            "Q3 2025 onwards".into(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        repository
            .save_training_period(period1.clone())
            .await
            .expect("Should save period1");
        repository
            .save_training_period(period2.clone())
            .await
            .expect("Should save period2");
        repository
            .save_training_period(period3.clone())
            .await
            .expect("Should save period3");

        // Test ref_date in period1 (should return only period1)
        let ref_date = "2025-02-15".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(period1.user(), &ref_date)
            .await;
        assert_eq!(active_periods, vec![period1]);

        // Test ref_date in period2 (should return only period2)
        let ref_date = "2025-05-15".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(period2.user(), &ref_date)
            .await;
        assert_eq!(active_periods, vec![period2]);

        // Test ref_date in period3 (should return only period3)
        let ref_date = "2025-08-15".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(period3.user(), &ref_date)
            .await;
        assert_eq!(active_periods, vec![period3]);

        // Test ref_date before all periods (should return empty)
        let ref_date = "2024-12-31".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(&UserId::test_default(), &ref_date)
            .await;
        assert!(active_periods.is_empty());
    }

    #[tokio::test]
    async fn test_get_active_training_periods_exclude_other_users() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let period = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-10-01".parse::<NaiveDate>().unwrap(),
            Some("2025-12-31".parse::<NaiveDate>().unwrap()),
            "Q4 2025".into(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        // Test with another user (should return empty)
        let ref_date = "2025-11-15".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(&UserId::from("another_user".to_string()), &ref_date)
            .await;
        assert!(active_periods.is_empty());
    }

    #[tokio::test]
    async fn test_get_active_training_periods_empty() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Test with no periods saved
        let ref_date = "2025-11-15".parse::<NaiveDate>().unwrap();
        let active_periods = repository
            .get_active_training_periods(&UserId::test_default(), &ref_date)
            .await;
        assert!(active_periods.is_empty());
    }

    #[tokio::test]
    async fn test_delete_training_period_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let period = build_training_period();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        // Verify period exists
        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await;
        assert!(fetched.is_some());

        // Delete the period
        let result = repository
            .delete_training_period(period.user(), period.id())
            .await;
        assert!(result.is_ok());

        // Verify period is deleted
        let fetched_after = repository
            .get_training_period(period.user(), period.id())
            .await;
        assert!(fetched_after.is_none());
    }

    #[tokio::test]
    async fn test_delete_training_period_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let period_id = TrainingPeriodId::new();

        // Delete non-existent period (DELETE is idempotent)
        let result = repository
            .delete_training_period(&UserId::test_default(), &period_id)
            .await
            .expect_err("Should fail");

        let DeleteTrainingPeriodError::PeriodDoesNotExist(id) = result else {
            unreachable!(" Should be DeleteTrainingPeriodError::PeriodDoesNotExist(id)")
        };
        assert_eq!(id, period_id);
    }

    #[tokio::test]
    async fn test_delete_training_period_only_deletes_specified_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create two periods for the same user
        let period1 = build_training_period();
        let period2 = TrainingPeriod::new(
            TrainingPeriodId::new(),
            period1.user().clone(),
            "2025-11-01".parse::<NaiveDate>().unwrap(),
            Some("2025-11-15".parse::<NaiveDate>().unwrap()),
            "Another Period".to_string(),
            TrainingPeriodSports::new(Some(vec![SportFilter::Sport(Sport::Cycling)])),
            None,
        )
        .unwrap();

        repository
            .save_training_period(period1.clone())
            .await
            .expect("Should save period 1");
        repository
            .save_training_period(period2.clone())
            .await
            .expect("Should save period 2");

        // Delete only period1
        let result = repository
            .delete_training_period(period1.user(), period1.id())
            .await;
        assert!(result.is_ok());

        // Verify period1 is deleted
        let fetched1 = repository
            .get_training_period(period1.user(), period1.id())
            .await;
        assert!(fetched1.is_none());

        // Verify period2 still exists
        let fetched2 = repository
            .get_training_period(period2.user(), period2.id())
            .await;
        assert!(fetched2.is_some());
        assert_eq!(fetched2.unwrap().id(), period2.id());
    }

    #[tokio::test]
    async fn test_update_training_period_name_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create a period
        let period = build_training_period();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        // Update the name
        let new_name = "Updated Name".to_string();
        let result = repository
            .update_training_period_name(period.user(), period.id(), new_name.clone())
            .await;
        assert!(result.is_ok());

        // Verify the name was updated
        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await;
        assert!(fetched.is_some());
        let fetched_period = fetched.unwrap();
        assert_eq!(fetched_period.name(), &new_name);
        // Verify other fields unchanged
        assert_eq!(fetched_period.id(), period.id());
        assert_eq!(fetched_period.user(), period.user());
        assert_eq!(fetched_period.start(), period.start());
        assert_eq!(fetched_period.end(), period.end());
    }

    #[tokio::test]
    async fn test_update_training_period_name_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Try to update a non-existent period
        let period_id = TrainingPeriodId::new();
        let result = repository
            .update_training_period_name(
                &UserId::test_default(),
                &period_id,
                "New Name".to_string(),
            )
            .await
            .expect_err("Should fail");

        let UpdateTrainingPeriodNameError::PeriodDoesNotExist(id) = result else {
            unreachable!("Should be UpdateTrainingPeriodNameError::PeriodDoesNotExist")
        };
        assert_eq!(id, period_id);
    }

    #[tokio::test]
    async fn test_update_training_period_name_only_updates_specified_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create two periods
        let period1 = build_training_period();
        let period2 = TrainingPeriod::new(
            TrainingPeriodId::new(),
            period1.user().clone(),
            "2025-11-01".parse::<NaiveDate>().unwrap(),
            Some("2025-11-15".parse::<NaiveDate>().unwrap()),
            "Another Period".to_string(),
            TrainingPeriodSports::new(Some(vec![SportFilter::Sport(Sport::Cycling)])),
            None,
        )
        .unwrap();

        repository
            .save_training_period(period1.clone())
            .await
            .expect("Should save period 1");
        repository
            .save_training_period(period2.clone())
            .await
            .expect("Should save period 2");

        // Update only period1's name
        let new_name = "Updated First Period".to_string();
        let result = repository
            .update_training_period_name(period1.user(), period1.id(), new_name.clone())
            .await;
        assert!(result.is_ok());

        // Verify period1's name was updated
        let fetched1 = repository
            .get_training_period(period1.user(), period1.id())
            .await;
        assert!(fetched1.is_some());
        assert_eq!(fetched1.unwrap().name(), &new_name);

        // Verify period2's name is unchanged
        let fetched2 = repository
            .get_training_period(period2.user(), period2.id())
            .await;
        assert!(fetched2.is_some());
        assert_eq!(fetched2.unwrap().name(), period2.name());
    }

    #[tokio::test]
    async fn test_update_training_period_note_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create a period with an initial note
        let period = build_training_period();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        // Update the note
        let new_note = Some("This is an updated note content".to_string());
        let result = repository
            .update_training_period_note(period.user(), period.id(), new_note.clone())
            .await;
        assert!(result.is_ok());

        // Verify the note was updated
        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await;
        assert!(fetched.is_some());
        let fetched_period = fetched.unwrap();
        assert_eq!(fetched_period.note(), &new_note);
        // Verify other fields unchanged
        assert_eq!(fetched_period.id(), period.id());
        assert_eq!(fetched_period.user(), period.user());
        assert_eq!(fetched_period.start(), period.start());
        assert_eq!(fetched_period.end(), period.end());
        assert_eq!(fetched_period.name(), period.name());
    }

    #[tokio::test]
    async fn test_update_training_period_note_clear_note() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create a period with an initial note
        let mut period = build_training_period();
        period = TrainingPeriod::new(
            period.id().clone(),
            period.user().clone(),
            *period.start(),
            *period.end(),
            period.name().to_string(),
            period.sports().clone(),
            Some("Initial note".to_string()),
        )
        .unwrap();

        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        // Clear the note by setting it to None
        let result = repository
            .update_training_period_note(period.user(), period.id(), None)
            .await;
        assert!(result.is_ok());

        // Verify the note was cleared
        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await;
        assert!(fetched.is_some());
        let fetched_period = fetched.unwrap();
        assert_eq!(fetched_period.note(), &None);
    }

    #[tokio::test]
    async fn test_update_training_period_note_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Try to update a non-existent period
        let period_id = TrainingPeriodId::new();
        let result = repository
            .update_training_period_note(
                &UserId::test_default(),
                &period_id,
                Some("Note".to_string()),
            )
            .await;

        let UpdateTrainingPeriodNoteError::PeriodDoesNotExist(id) = result.expect_err("Should err")
        else {
            unreachable!(" Should be UpdateTrainingPeriodNoteError::PeriodDoesNotExist(id)")
        };
        assert_eq!(id, period_id);
    }

    #[tokio::test]
    async fn test_update_training_period_note_only_updates_specified_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create two periods with notes
        let period1 = TrainingPeriod::new(
            TrainingPeriodId::new(),
            UserId::test_default(),
            "2025-10-17".parse::<NaiveDate>().unwrap(),
            Some("2025-10-21".parse::<NaiveDate>().unwrap()),
            "First Period".to_string(),
            TrainingPeriodSports::new(None),
            Some("First note".to_string()),
        )
        .unwrap();

        let period2 = TrainingPeriod::new(
            TrainingPeriodId::new(),
            period1.user().clone(),
            "2025-11-01".parse::<NaiveDate>().unwrap(),
            Some("2025-11-15".parse::<NaiveDate>().unwrap()),
            "Another Period".to_string(),
            TrainingPeriodSports::new(Some(vec![SportFilter::Sport(Sport::Cycling)])),
            Some("Second note".to_string()),
        )
        .unwrap();

        repository
            .save_training_period(period1.clone())
            .await
            .expect("Should save period 1");
        repository
            .save_training_period(period2.clone())
            .await
            .expect("Should save period 2");

        // Update only period1's note
        let new_note = Some("Updated first note".to_string());
        let result = repository
            .update_training_period_note(period1.user(), period1.id(), new_note.clone())
            .await;
        assert!(result.is_ok());

        // Verify period1's note was updated
        let fetched1 = repository
            .get_training_period(period1.user(), period1.id())
            .await;
        assert!(fetched1.is_some());
        assert_eq!(fetched1.unwrap().note(), &new_note);

        // Verify period2's note is unchanged
        let fetched2 = repository
            .get_training_period(period2.user(), period2.id())
            .await;
        assert!(fetched2.is_some());
        assert_eq!(fetched2.unwrap().note(), period2.note());
    }

    #[tokio::test]
    async fn test_update_training_period_dates_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create a period
        let period = build_training_period();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        // Update the dates
        let new_start = "2025-12-01".parse::<NaiveDate>().unwrap();
        let new_end = Some("2025-12-31".parse::<NaiveDate>().unwrap());
        let result = repository
            .update_training_period_dates(period.user(), period.id(), new_start, new_end)
            .await;
        assert!(result.is_ok());

        // Verify the dates were updated
        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await;
        assert!(fetched.is_some());
        let fetched_period = fetched.unwrap();
        assert_eq!(fetched_period.start(), &new_start);
        assert_eq!(fetched_period.end(), &new_end);
        // Verify other fields unchanged
        assert_eq!(fetched_period.id(), period.id());
        assert_eq!(fetched_period.user(), period.user());
        assert_eq!(fetched_period.name(), period.name());
        assert_eq!(fetched_period.note(), period.note());
    }

    #[tokio::test]
    async fn test_update_training_period_dates_clear_end_date() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create a period with an end date
        let period = build_training_period();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        // Update dates and clear the end date
        let new_start = "2025-12-01".parse::<NaiveDate>().unwrap();
        let result = repository
            .update_training_period_dates(period.user(), period.id(), new_start, None)
            .await;
        assert!(result.is_ok());

        // Verify the dates were updated and end date is cleared
        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await;
        assert!(fetched.is_some());
        let fetched_period = fetched.unwrap();
        assert_eq!(fetched_period.start(), &new_start);
        assert_eq!(fetched_period.end(), &None);
        // Verify other fields unchanged
        assert_eq!(fetched_period.name(), period.name());
        assert_eq!(fetched_period.note(), period.note());
    }

    #[tokio::test]
    async fn test_update_training_period_dates_only_start() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create a period
        let period = build_training_period();
        let original_end = period.end();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        // Update only the start date, keeping the original end
        let new_start = "2025-10-15".parse::<NaiveDate>().unwrap();
        let result = repository
            .update_training_period_dates(period.user(), period.id(), new_start, *original_end)
            .await;
        assert!(result.is_ok());

        // Verify only start was updated
        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await;
        assert!(fetched.is_some());
        let fetched_period = fetched.unwrap();
        assert_eq!(fetched_period.start(), &new_start);
        assert_eq!(fetched_period.end(), original_end);
    }

    #[tokio::test]
    async fn test_update_training_period_dates_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Try to update a non-existent period
        let period_id = TrainingPeriodId::new();
        let new_start = "2025-12-01".parse::<NaiveDate>().unwrap();
        let new_end = Some("2025-12-31".parse::<NaiveDate>().unwrap());
        let result = repository
            .update_training_period_dates(&UserId::test_default(), &period_id, new_start, new_end)
            .await
            .expect_err("Should fail");

        let UpdateTrainingPeriodDatesError::PeriodDoesNotExist(id) = result else {
            unreachable!("Should be UpdateTrainingPeriodDatesError::PeriodDoesNotExist")
        };
        assert_eq!(id, period_id);
    }

    #[tokio::test]
    async fn test_update_training_period_dates_only_updates_specified_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Create two periods
        let period1 = build_training_period();
        let period2 = TrainingPeriod::new(
            TrainingPeriodId::new(),
            period1.user().clone(),
            "2025-11-01".parse::<NaiveDate>().unwrap(),
            Some("2025-11-15".parse::<NaiveDate>().unwrap()),
            "Another Period".to_string(),
            TrainingPeriodSports::new(Some(vec![SportFilter::Sport(Sport::Cycling)])),
            None,
        )
        .unwrap();

        repository
            .save_training_period(period1.clone())
            .await
            .expect("Should save period 1");
        repository
            .save_training_period(period2.clone())
            .await
            .expect("Should save period 2");

        // Update only period1's dates
        let new_start = "2025-12-01".parse::<NaiveDate>().unwrap();
        let new_end = Some("2025-12-31".parse::<NaiveDate>().unwrap());
        let result = repository
            .update_training_period_dates(period1.user(), period1.id(), new_start, new_end)
            .await;
        assert!(result.is_ok());

        // Verify period1's dates were updated
        let fetched1 = repository
            .get_training_period(period1.user(), period1.id())
            .await;
        assert!(fetched1.is_some());
        let updated_period1 = fetched1.unwrap();
        assert_eq!(updated_period1.start(), &new_start);
        assert_eq!(updated_period1.end(), &new_end);

        // Verify period2's dates are unchanged
        let fetched2 = repository
            .get_training_period(period2.user(), period2.id())
            .await;
        assert!(fetched2.is_some());
        let unchanged_period2 = fetched2.unwrap();
        assert_eq!(unchanged_period2.start(), period2.start());
        assert_eq!(unchanged_period2.end(), period2.end());
    }

    fn build_training_note() -> TrainingNote {
        use crate::domain::models::training::{
            TrainingNoteContent, TrainingNoteDate, TrainingNoteId,
        };
        use chrono::Utc;

        TrainingNote::new(
            TrainingNoteId::new(),
            UserId::test_default(),
            Some(TrainingNoteTitle::from("title")),
            TrainingNoteContent::from("Test training note"),
            TrainingNoteDate::today(),
            Utc::now().into(),
        )
    }

    #[tokio::test]
    async fn test_save_training_note() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let note = build_training_note();

        repository
            .save_training_note(note.clone())
            .await
            .expect("Should save note");

        let saved_note = repository
            .get_training_note(note.user(), note.id())
            .await
            .expect("Get should succeed")
            .expect("Note should be found");
        assert_eq!(note, saved_note);
    }

    #[tokio::test]
    async fn test_save_training_note_update_fields_if_existing() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        // Initial note
        let note = build_training_note();
        repository
            .save_training_note(note.clone())
            .await
            .expect("First save should succeed");

        // Upsert existing note
        let updated_note = TrainingNote::new(
            note.id().clone(),
            note.user().clone(),
            Some(TrainingNoteTitle::from("new title")),
            TrainingNoteContent::from("new content"),
            TrainingNoteDate::new(Utc::now().date_naive().add(Days::new(2))),
            note.created_at().clone(),
        );

        repository
            .save_training_note(updated_note.clone())
            .await
            .expect("Upsert should succeed");

        let saved_note = repository
            .get_training_note(note.user(), note.id())
            .await
            .expect("Get should succeed")
            .expect("Note should be found");
        assert_eq!(updated_note, saved_note);
    }

    #[tokio::test]
    async fn test_get_training_note_returns_note() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let note = build_training_note();
        repository
            .save_training_note(note.clone())
            .await
            .expect("Should save note");

        let retrieved = repository
            .get_training_note(note.user(), note.id())
            .await
            .expect("Should retrieve note")
            .expect("Note should exist");

        assert_eq!(retrieved.id(), note.id());
        assert_eq!(retrieved.user(), note.user());
        assert_eq!(retrieved.content(), note.content());
    }

    #[tokio::test]
    async fn test_get_training_note_returns_none_when_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let result = repository
            .get_training_note(&UserId::test_default(), &TrainingNoteId::new())
            .await
            .expect("Should not error");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_training_notes_returns_all_user_notes() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let user_id = UserId::test_default();
        let other_user_id = UserId::new();

        // Save notes for the test user
        let note1 = build_training_note();
        let note2 = TrainingNote::new(
            TrainingNoteId::new(),
            user_id.clone(),
            Some(TrainingNoteTitle::from("note title")),
            TrainingNoteContent::from("Second note"),
            TrainingNoteDate::today(),
            Utc::now().into(),
        );

        // Save note for another user
        let other_note = TrainingNote::new(
            TrainingNoteId::new(),
            other_user_id,
            Some(TrainingNoteTitle::from("note title")),
            TrainingNoteContent::from("Other user note"),
            TrainingNoteDate::today(),
            Utc::now().into(),
        );

        repository
            .save_training_note(note1.clone())
            .await
            .expect("Should save note1");
        repository
            .save_training_note(note2.clone())
            .await
            .expect("Should save note2");
        repository
            .save_training_note(other_note)
            .await
            .expect("Should save other_note");

        // Get notes for test user
        let notes = repository
            .get_training_notes(&user_id, &None)
            .await
            .expect("Should retrieve notes");

        assert_eq!(notes.len(), 2);
        assert!(notes.iter().any(|n| n.id() == note1.id()));
        assert!(notes.iter().any(|n| n.id() == note2.id()));
    }

    #[tokio::test]
    async fn test_get_training_notes_returns_empty_when_no_notes() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let notes = repository
            .get_training_notes(&UserId::new(), &None)
            .await
            .expect("Should not error");

        assert_eq!(notes.len(), 0);
    }

    #[tokio::test]
    async fn test_get_training_notes_orders_by_created_at_desc() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let user_id = UserId::test_default();

        // Create notes with different timestamps
        let older_note = TrainingNote::new(
            TrainingNoteId::new(),
            user_id.clone(),
            Some(TrainingNoteTitle::from("note title")),
            TrainingNoteContent::from("Older note"),
            TrainingNoteDate::today(),
            (Utc::now() - chrono::Duration::hours(2)).into(),
        );

        let newer_note = TrainingNote::new(
            TrainingNoteId::new(),
            user_id.clone(),
            Some(TrainingNoteTitle::from("another note title")),
            TrainingNoteContent::from("Newer note"),
            TrainingNoteDate::today(),
            Utc::now().into(),
        );

        // Save in random order
        repository
            .save_training_note(older_note.clone())
            .await
            .expect("Should save older note");
        repository
            .save_training_note(newer_note.clone())
            .await
            .expect("Should save newer note");

        let notes = repository
            .get_training_notes(&user_id, &None)
            .await
            .expect("Should retrieve notes");

        // Newer note should come first
        assert_eq!(notes[0].id(), newer_note.id());
        assert_eq!(notes[1].id(), older_note.id());
    }

    #[tokio::test]
    async fn test_get_training_notes_filters_by_date_range() {
        use crate::domain::ports::DateRange;

        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let user_id = UserId::test_default();

        // Create notes with different dates
        let note_jan_15 = TrainingNote::new(
            TrainingNoteId::new(),
            user_id.clone(),
            Some(TrainingNoteTitle::from("January 15")),
            TrainingNoteContent::from("Note on Jan 15"),
            TrainingNoteDate::try_from("2025-01-15").unwrap(),
            Utc::now().into(),
        );

        let note_jan_20 = TrainingNote::new(
            TrainingNoteId::new(),
            user_id.clone(),
            Some(TrainingNoteTitle::from("January 20")),
            TrainingNoteContent::from("Note on Jan 20"),
            TrainingNoteDate::try_from("2025-01-20").unwrap(),
            Utc::now().into(),
        );

        let note_jan_25 = TrainingNote::new(
            TrainingNoteId::new(),
            user_id.clone(),
            Some(TrainingNoteTitle::from("January 25")),
            TrainingNoteContent::from("Note on Jan 25"),
            TrainingNoteDate::try_from("2025-01-25").unwrap(),
            Utc::now().into(),
        );

        let note_feb_01 = TrainingNote::new(
            TrainingNoteId::new(),
            user_id.clone(),
            Some(TrainingNoteTitle::from("February 1")),
            TrainingNoteContent::from("Note on Feb 1"),
            TrainingNoteDate::try_from("2025-02-01").unwrap(),
            Utc::now().into(),
        );

        // Save all notes
        repository
            .save_training_note(note_jan_15.clone())
            .await
            .expect("Should save note");
        repository
            .save_training_note(note_jan_20.clone())
            .await
            .expect("Should save note");
        repository
            .save_training_note(note_jan_25.clone())
            .await
            .expect("Should save note");
        repository
            .save_training_note(note_feb_01.clone())
            .await
            .expect("Should save note");

        // Filter notes from Jan 18 to Jan 31
        let date_range = DateRange::new(
            "2025-01-18".parse::<NaiveDate>().unwrap(),
            "2025-01-31".parse::<NaiveDate>().unwrap(),
        );
        let notes = repository
            .get_training_notes(&user_id, &Some(date_range))
            .await
            .expect("Should retrieve notes");

        // Should only include note_jan_20 and note_jan_25
        assert_eq!(notes.len(), 2);
        assert!(notes.iter().any(|n| n.id() == note_jan_20.id()));
        assert!(notes.iter().any(|n| n.id() == note_jan_25.id()));
    }

    #[tokio::test]
    async fn test_get_training_notes_date_range_end_is_exclusive() {
        use crate::domain::ports::DateRange;

        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let user_id = UserId::test_default();

        // Create notes on different dates
        let note_jan_19 = TrainingNote::new(
            TrainingNoteId::new(),
            user_id.clone(),
            Some(TrainingNoteTitle::from("January 19")),
            TrainingNoteContent::from("Note on Jan 19"),
            TrainingNoteDate::try_from("2025-01-19").unwrap(),
            Utc::now().into(),
        );

        let note_jan_20 = TrainingNote::new(
            TrainingNoteId::new(),
            user_id.clone(),
            Some(TrainingNoteTitle::from("January 20")),
            TrainingNoteContent::from("Note on Jan 20"),
            TrainingNoteDate::try_from("2025-01-20").unwrap(),
            Utc::now().into(),
        );

        // Save notes
        repository
            .save_training_note(note_jan_19.clone())
            .await
            .expect("Should save note");
        repository
            .save_training_note(note_jan_20.clone())
            .await
            .expect("Should save note");

        // Filter with end date = Jan 20 (end should be exclusive)
        let date_range = DateRange::new(
            "2025-01-15".parse::<NaiveDate>().unwrap(),
            "2025-01-20".parse::<NaiveDate>().unwrap(),
        );
        let notes = repository
            .get_training_notes(&user_id, &Some(date_range))
            .await
            .expect("Should retrieve notes");

        // Should only include note_jan_19, not note_jan_20 (end is exclusive)
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id(), note_jan_19.id());
    }

    #[tokio::test]
    async fn test_delete_training_note_removes_note() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let note = build_training_note();
        repository
            .save_training_note(note.clone())
            .await
            .expect("Should save note");

        repository
            .delete_training_note(note.user(), note.id())
            .await
            .expect("Should delete note");

        // Verify note was deleted
        let result = repository
            .get_training_note(note.user(), note.id())
            .await
            .expect("Should not error");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_training_note_does_not_fail_when_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let result = repository
            .delete_training_note(&UserId::test_default(), &TrainingNoteId::new())
            .await;

        // Should not fail even if note doesn't exist
        assert!(result.is_ok());
    }

    mod test_list_training_note_documents {
        use chrono::Utc;

        use crate::clock::clock_test_utils::FakeClock;

        use super::*;

        fn build_training_note_with(title: &str, content: &str) -> TrainingNote {
            TrainingNote::new(
                TrainingNoteId::new(),
                UserId::test_default(),
                Some(TrainingNoteTitle::from(title)),
                TrainingNoteContent::from(content),
                TrainingNoteDate::today(),
                Utc::now().into(),
            )
        }

        fn build_training_note_for(user: UserId) -> TrainingNote {
            TrainingNote::new(
                TrainingNoteId::new(),
                user,
                Some(TrainingNoteTitle::from("note title")),
                TrainingNoteContent::from("A note from another user"),
                TrainingNoteDate::today(),
                Utc::now().into(),
            )
        }

        #[tokio::test]
        async fn test_list_training_note_documents_when_empty_db() {
            let db_file = NamedTempFile::new().unwrap();
            let repository =
                SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                    .await
                    .expect("repo should init");

            let (documents, remaining) = repository
                .list_training_note_documents(10, 0)
                .await
                .expect("Should have succeeded");

            assert!(documents.is_empty());
            assert_eq!(remaining, RemainingDocuments::from(false));
        }

        #[tokio::test]
        async fn test_list_training_note_documents_returns_all_notes_in_insertion_order() {
            let db_file = NamedTempFile::new().unwrap();
            let now = Utc::now();
            let repository = SqliteTrainingRepository::new(
                &db_file.path().to_string_lossy(),
                FakeClock::new(now),
            )
            .await
            .expect("repo should init");

            let notes = [
                build_training_note_with("first", "first note content"),
                build_training_note_with("second", "second note content"),
                build_training_note_with("third", "third note content"),
            ];
            for note in &notes {
                repository
                    .save_training_note(note.clone())
                    .await
                    .expect("Insertion should have succeeded");
            }

            let (documents, remaining) = repository
                .list_training_note_documents(10, 0)
                .await
                .expect("Should have succeeded");

            assert_eq!(documents.len(), notes.len());
            assert_eq!(remaining, RemainingDocuments::from(false));

            // Documents are ordered by rowid, i.e. insertion order, and each note is turned
            // into an "updated" search document.
            for (document, note) in documents.iter().zip(notes.iter()) {
                assert_eq!(document.document_type(), &SearchDocumentType::TrainingNote);
                assert_eq!(document.document_id(), note.id().to_string());
                assert_eq!(document.user(), note.user());
                assert_eq!(document.event(), &SearchDocumentEvent::Updated);
                assert_eq!(document.occurred_at(), &now);
            }
        }

        #[tokio::test]
        async fn test_list_training_note_documents_include_title_and_content() {
            let db_file = NamedTempFile::new().unwrap();
            let repository =
                SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                    .await
                    .expect("repo should init");

            let note = build_training_note_with("Epic ride", "Felt great legs");
            repository
                .save_training_note(note.clone())
                .await
                .expect("Insertion should have succeeded");

            let (documents, _remaining) = repository
                .list_training_note_documents(10, 0)
                .await
                .expect("Should have succeeded");

            let document = documents.first().expect("Should contain one document");
            assert_eq!(document.document_id(), note.id().to_string());
            assert!(document.content().contains("Epic ride"));
            assert!(document.content().contains("Felt great legs"));
        }

        #[tokio::test]
        async fn test_list_training_note_documents_includes_notes_from_all_users() {
            let db_file = NamedTempFile::new().unwrap();
            let repository =
                SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                    .await
                    .expect("repo should init");

            let default_user_note = build_training_note_with("default user note", "content");
            let other_user_note = build_training_note_for(UserId::new());
            repository
                .save_training_note(default_user_note.clone())
                .await
                .expect("Insertion should have succeeded");
            repository
                .save_training_note(other_user_note.clone())
                .await
                .expect("Insertion should have succeeded");

            // Unlike `get_training_notes`, the snapshot spans every user's notes.
            let (documents, remaining) = repository
                .list_training_note_documents(10, 0)
                .await
                .expect("Should have succeeded");

            assert_eq!(documents.len(), 2);
            assert_eq!(remaining, RemainingDocuments::from(false));
            let document_ids: Vec<String> = documents
                .iter()
                .map(|document| document.document_id().to_string())
                .collect();
            assert!(document_ids.contains(&default_user_note.id().to_string()));
            assert!(document_ids.contains(&other_user_note.id().to_string()));
        }

        #[tokio::test]
        async fn test_list_training_note_documents_skips_deleted_notes() {
            let db_file = NamedTempFile::new().unwrap();
            let repository =
                SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                    .await
                    .expect("repo should init");

            let kept = build_training_note_with("kept note", "content");
            let deleted = build_training_note_with("deleted note", "content");
            repository
                .save_training_note(kept.clone())
                .await
                .expect("Insertion should have succeeded");
            repository
                .save_training_note(deleted.clone())
                .await
                .expect("Insertion should have succeeded");
            repository
                .delete_training_note(deleted.user(), deleted.id())
                .await
                .expect("Deletion should have succeeded");

            let (documents, remaining) = repository
                .list_training_note_documents(10, 0)
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
        async fn test_list_training_note_documents_page_with_more_remaining_returns_batch_size_documents()
         {
            let db_file = NamedTempFile::new().unwrap();
            let repository =
                SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                    .await
                    .expect("repo should init");

            let notes = [
                build_training_note_with("note-1", "content-1"),
                build_training_note_with("note-2", "content-2"),
                build_training_note_with("note-3", "content-3"),
            ];
            for note in &notes {
                repository
                    .save_training_note(note.clone())
                    .await
                    .expect("Insertion should have succeeded");
            }

            // The repository fetches batch_size + 1 rows to detect a next page, but only
            // returns `batch_size` documents: the extra sentinel row is dropped.
            let (documents, remaining) = repository
                .list_training_note_documents(2, 0)
                .await
                .expect("Should have succeeded");

            assert_eq!(documents.len(), 2);
            assert_eq!(remaining, RemainingDocuments::from(true));
            for (document, note) in documents.iter().zip(notes.iter().take(2)) {
                assert_eq!(document.document_id(), note.id().to_string());
            }
        }

        #[tokio::test]
        async fn test_list_training_note_documents_pages_partition_all_notes_without_overlap() {
            let db_file = NamedTempFile::new().unwrap();
            let repository =
                SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                    .await
                    .expect("repo should init");

            let notes = [
                build_training_note_with("note-1", "content-1"),
                build_training_note_with("note-2", "content-2"),
                build_training_note_with("note-3", "content-3"),
                build_training_note_with("note-4", "content-4"),
                build_training_note_with("note-5", "content-5"),
            ];
            for note in &notes {
                repository
                    .save_training_note(note.clone())
                    .await
                    .expect("Insertion should have succeeded");
            }

            // Walking through every page (incrementing the page until none remain) must
            // return each note exactly once, in insertion order: pages do not overlap.
            let mut page_ids = Vec::new();
            let mut page = 0;
            loop {
                let (documents, remaining) = repository
                    .list_training_note_documents(2, page)
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

            let expected_ids: Vec<String> =
                notes.iter().map(|note| note.id().to_string()).collect();
            assert_eq!(page_ids, expected_ids);
        }

        #[tokio::test]
        async fn test_list_training_note_documents_on_last_page_no_more_remaining() {
            let db_file = NamedTempFile::new().unwrap();
            let repository =
                SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                    .await
                    .expect("repo should init");

            let notes = [
                build_training_note_with("note-1", "content-1"),
                build_training_note_with("note-2", "content-2"),
                build_training_note_with("note-3", "content-3"),
            ];
            for note in &notes {
                repository
                    .save_training_note(note.clone())
                    .await
                    .expect("Insertion should have succeeded");
            }

            // offset 1 with batch_size 2 skips the first 2 rows and only the last note remains.
            let (documents, remaining) = repository
                .list_training_note_documents(2, 1)
                .await
                .expect("Should have succeeded");

            assert_eq!(documents.len(), 1);
            assert_eq!(remaining, RemainingDocuments::from(false));
            assert_eq!(
                documents
                    .first()
                    .expect("Should contain one document")
                    .document_id(),
                notes[2].id().to_string()
            );
        }

        #[tokio::test]
        async fn test_list_training_note_documents_offset_beyond_last_page_is_empty() {
            let db_file = NamedTempFile::new().unwrap();
            let repository =
                SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                    .await
                    .expect("repo should init");

            let note = build_training_note_with("note-1", "content-1");
            repository
                .save_training_note(note.clone())
                .await
                .expect("Insertion should have succeeded");

            // offset 10 with batch_size 2 skips 20 rows: nothing should be returned.
            let (documents, remaining) = repository
                .list_training_note_documents(2, 10)
                .await
                .expect("Should have succeeded");

            assert!(documents.is_empty());
            assert_eq!(remaining, RemainingDocuments::from(false));
        }
    }

    #[tokio::test]
    async fn test_save_and_retrieve_metric_with_name() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric_with_name = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::new("My Custom Metric")),
            TrainingMetricScope::Global,
            build_global_metric().definition().clone(),
        );

        repository
            .save_metric(metric_with_name.clone())
            .await
            .expect("Should save metric with name");

        let metrics = repository
            .get_global_metrics(&UserId::test_default())
            .await
            .expect("Should retrieve metrics");

        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name(), metric_with_name.name());
        assert_eq!(
            metrics[0].name().as_ref().map(|n| n.as_str()),
            Some("My Custom Metric")
        );
    }

    #[tokio::test]
    async fn test_save_and_retrieve_metric_without_name() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric_without_name = build_global_metric(); // None for name

        repository
            .save_metric(metric_without_name.clone())
            .await
            .expect("Should save metric without name");

        let metrics = repository
            .get_global_metrics(&UserId::test_default())
            .await
            .expect("Should retrieve metrics");

        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name(), &None);
    }

    #[tokio::test]
    async fn test_get_metrics_with_global_scope_filter() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let user_id = UserId::test_default();

        // Create a global metric
        let global_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Global Metric")),
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                user_id.clone(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::Distance,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Weekly,
                    TrainingMetricAggregate::Sum,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        );

        // Create a period-scoped metric
        let period = build_training_period();
        let period_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Period Metric")),
            TrainingMetricScope::TrainingPeriod(period.id().clone()),
            TrainingMetricDefinition::new(
                user_id.clone(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::Distance,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Weekly,
                    TrainingMetricAggregate::Sum,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        );

        repository
            .save_metric(global_metric.clone())
            .await
            .expect("Should save global metric");

        repository
            .save_training_period(period.clone())
            .await
            .expect("Should have created the period");
        repository
            .save_metric(period_metric.clone())
            .await
            .expect("Should save period metric");

        // Test: Filter by Global scope should return only global metrics
        let metrics = repository
            .get_global_metrics(&user_id)
            .await
            .expect("Should retrieve metrics");

        assert_eq!(metrics.len(), 1);
        assert_eq!(
            metrics[0].name(),
            &Some(TrainingMetricName::from("Global Metric"))
        );
        assert_eq!(metrics[0].scope(), &TrainingMetricScope::Global);
    }

    #[tokio::test]
    async fn test_get_metrics_with_training_period_scope_filter() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let user_id = UserId::test_default();

        // Create a global metric
        let global_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Global Metric")),
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                user_id.clone(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::Distance,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Weekly,
                    TrainingMetricAggregate::Sum,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        );

        // Create a period-scoped metric for our period
        let period_1 = build_training_period();
        let period_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Period Metric")),
            TrainingMetricScope::TrainingPeriod(period_1.id().clone()),
            TrainingMetricDefinition::new(
                user_id.clone(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::Distance,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Weekly,
                    TrainingMetricAggregate::Sum,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        );

        // Create a period-scoped metric for a different period
        let period_2 = build_training_period();
        let other_period_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Other Period Metric")),
            TrainingMetricScope::TrainingPeriod(period_2.id().clone()),
            TrainingMetricDefinition::new(
                user_id.clone(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::Distance,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Weekly,
                    TrainingMetricAggregate::Sum,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        );

        repository
            .save_metric(global_metric.clone())
            .await
            .expect("Should save global metric");

        repository
            .save_training_period(period_1.clone())
            .await
            .expect("Should have created period 1");
        repository
            .save_metric(period_metric.clone())
            .await
            .expect("Should save period metric");

        repository
            .save_training_period(period_2.clone())
            .await
            .expect("Should have created period 2");
        repository
            .save_metric(other_period_metric.clone())
            .await
            .expect("Should save other period metric");

        // Test: Filter by TrainingPeriod scope should return global metrics + metrics for that period
        let mut global_metrics = repository
            .get_global_metrics(&user_id)
            .await
            .expect("Should retrieve global metrics");

        let mut period_metrics = repository
            .get_period_metrics(&user_id, period_1.id())
            .await
            .expect("Should retrieve period metrics");

        global_metrics.append(&mut period_metrics);
        let metrics = global_metrics;

        assert_eq!(metrics.len(), 2);

        // Should contain the global metric
        assert!(
            metrics
                .iter()
                .any(|m| m.name() == &Some(TrainingMetricName::from("Global Metric")))
        );

        // Should contain the period metric for our period
        assert!(
            metrics
                .iter()
                .any(|m| m.name() == &Some(TrainingMetricName::from("Period Metric")))
        );

        // Should NOT contain the metric for the other period
        assert!(
            !metrics
                .iter()
                .any(|m| m.name() == &Some(TrainingMetricName::from("Other Period Metric")))
        );
    }

    #[tokio::test]
    async fn test_get_metrics_without_scope_filter_returns_all() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let user_id = UserId::test_default();

        // Create a global metric
        let global_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Global Metric")),
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                user_id.clone(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::Distance,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Weekly,
                    TrainingMetricAggregate::Sum,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        );

        // Create a period-scoped metric
        let period = build_training_period();
        let period_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Period Metric")),
            TrainingMetricScope::TrainingPeriod(period.id().clone()),
            TrainingMetricDefinition::new(
                user_id.clone(),
                TrainingMetricSource::Activity(ActivitySource::new(
                    ActivityMetric::Distance,
                    TrainingMetricGroupBy::none(),
                )),
                Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Weekly,
                    TrainingMetricAggregate::Sum,
                )),
                TrainingMetricActivityFilters::empty(),
                TrainingMetricSummary::empty(),
                None,
            ),
        );

        repository
            .save_metric(global_metric.clone())
            .await
            .expect("Should save global metric");

        repository
            .save_training_period(period.clone())
            .await
            .expect("Should have created the period");
        repository
            .save_metric(period_metric.clone())
            .await
            .expect("Should save period metric");

        // Test: We can fetch both global and period metrics separately
        let global_metrics = repository
            .get_global_metrics(&user_id)
            .await
            .expect("Should retrieve global metrics");

        let period_metrics = repository
            .get_period_metrics(&user_id, period.id())
            .await
            .expect("Should retrieve period metrics");

        assert_eq!(global_metrics.len(), 1);
        assert_eq!(period_metrics.len(), 1);
        assert!(
            global_metrics
                .iter()
                .any(|m| m.name() == &Some(TrainingMetricName::from("Global Metric")))
        );
        assert!(
            period_metrics
                .iter()
                .any(|m| m.name() == &Some(TrainingMetricName::from("Period Metric")))
        );
    }

    #[tokio::test]
    async fn test_get_training_metrics_ordering_returns_empty_when_not_set() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let user_id = UserId::test_default();
        let scope = TrainingMetricScope::Global;

        let ordering = repository
            .get_training_metrics_ordering(&user_id, &scope)
            .await
            .expect("Should retrieve empty ordering");

        assert_eq!(ordering.ids().len(), 0);
    }

    #[tokio::test]
    async fn test_set_and_get_training_metrics_ordering_global_scope() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let user_id = UserId::test_default();
        let scope = TrainingMetricScope::Global;

        // Create ordering with some metric IDs
        let id1 = TrainingMetricId::new();
        let id2 = TrainingMetricId::new();
        let id3 = TrainingMetricId::new();
        let ordering =
            TrainingMetricsOrdering::try_from(vec![id1.clone(), id2.clone(), id3.clone()])
                .expect("Should create ordering");

        // Save ordering
        repository
            .set_training_metrics_ordering(&user_id, &scope, ordering.clone())
            .await
            .expect("Should save ordering");

        // Retrieve ordering
        let retrieved_ordering = repository
            .get_training_metrics_ordering(&user_id, &scope)
            .await
            .expect("Should retrieve ordering");

        assert_eq!(retrieved_ordering.ids().len(), 3);
        assert_eq!(retrieved_ordering.ids()[0], id1);
        assert_eq!(retrieved_ordering.ids()[1], id2);
        assert_eq!(retrieved_ordering.ids()[2], id3);
    }

    #[tokio::test]
    async fn test_set_and_get_training_metrics_ordering_period_scope() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let user_id = UserId::test_default();
        let period_id = TrainingPeriodId::new();
        let scope = TrainingMetricScope::TrainingPeriod(period_id);

        // Create ordering with some metric IDs
        let id1 = TrainingMetricId::new();
        let id2 = TrainingMetricId::new();
        let ordering = TrainingMetricsOrdering::try_from(vec![id1.clone(), id2.clone()])
            .expect("Should create ordering");

        // Save ordering
        repository
            .set_training_metrics_ordering(&user_id, &scope, ordering.clone())
            .await
            .expect("Should save ordering");

        // Retrieve ordering
        let retrieved_ordering = repository
            .get_training_metrics_ordering(&user_id, &scope)
            .await
            .expect("Should retrieve ordering");

        assert_eq!(retrieved_ordering.ids().len(), 2);
        assert_eq!(retrieved_ordering.ids()[0], id1);
        assert_eq!(retrieved_ordering.ids()[1], id2);
    }

    #[tokio::test]
    async fn test_update_training_metrics_ordering_scope_global() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let user_id = UserId::test_default();
        let scope = TrainingMetricScope::Global;

        // Create and save initial ordering
        let id1 = TrainingMetricId::new();
        let id2 = TrainingMetricId::new();
        let ordering1 = TrainingMetricsOrdering::try_from(vec![id1.clone(), id2.clone()])
            .expect("Should create ordering");

        repository
            .set_training_metrics_ordering(&user_id, &scope, ordering1)
            .await
            .expect("Should save ordering");

        // Update with new ordering
        let id3 = TrainingMetricId::new();
        let ordering2 = TrainingMetricsOrdering::try_from(vec![id3.clone(), id1.clone()])
            .expect("Should create ordering");

        repository
            .set_training_metrics_ordering(&user_id, &scope, ordering2)
            .await
            .expect("Should update ordering");

        // Retrieve and verify updated ordering
        let retrieved_ordering = repository
            .get_training_metrics_ordering(&user_id, &scope)
            .await
            .expect("Should retrieve ordering");

        assert_eq!(retrieved_ordering.ids().len(), 2);
        assert_eq!(retrieved_ordering.ids()[0], id3);
        assert_eq!(retrieved_ordering.ids()[1], id1);
    }

    #[tokio::test]
    async fn test_update_training_metrics_ordering_scope_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let user_id = UserId::test_default();
        let period_id = TrainingPeriodId::new();
        let scope = TrainingMetricScope::TrainingPeriod(period_id);

        // Create and save initial ordering
        let id1 = TrainingMetricId::new();
        let id2 = TrainingMetricId::new();
        let ordering1 = TrainingMetricsOrdering::try_from(vec![id1.clone(), id2.clone()])
            .expect("Should create ordering");

        repository
            .set_training_metrics_ordering(&user_id, &scope, ordering1)
            .await
            .expect("Should save ordering");

        // Update with new ordering
        let id3 = TrainingMetricId::new();
        let ordering2 = TrainingMetricsOrdering::try_from(vec![id3.clone(), id1.clone()])
            .expect("Should create ordering");

        repository
            .set_training_metrics_ordering(&user_id, &scope, ordering2)
            .await
            .expect("Should update ordering");

        // Retrieve and verify updated ordering
        let retrieved_ordering = repository
            .get_training_metrics_ordering(&user_id, &scope)
            .await
            .expect("Should retrieve ordering");

        assert_eq!(retrieved_ordering.ids().len(), 2);
        assert_eq!(retrieved_ordering.ids()[0], id3);
        assert_eq!(retrieved_ordering.ids()[1], id1);
    }

    #[tokio::test]
    async fn test_training_metrics_ordering_scopes_are_independent() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let user_id = UserId::test_default();
        let period_id = TrainingPeriodId::new();

        // Set global scope ordering
        let global_id1 = TrainingMetricId::new();
        let global_id2 = TrainingMetricId::new();
        let global_ordering =
            TrainingMetricsOrdering::try_from(vec![global_id1.clone(), global_id2.clone()])
                .expect("Should create ordering");

        repository
            .set_training_metrics_ordering(&user_id, &TrainingMetricScope::Global, global_ordering)
            .await
            .expect("Should save global ordering");

        // Set period scope ordering
        let period_id1 = TrainingMetricId::new();
        let period_id2 = TrainingMetricId::new();
        let period_ordering =
            TrainingMetricsOrdering::try_from(vec![period_id1.clone(), period_id2.clone()])
                .expect("Should create ordering");

        repository
            .set_training_metrics_ordering(
                &user_id,
                &TrainingMetricScope::TrainingPeriod(period_id.clone()),
                period_ordering,
            )
            .await
            .expect("Should save period ordering");

        // Verify both orderings are independent
        let retrieved_global = repository
            .get_training_metrics_ordering(&user_id, &TrainingMetricScope::Global)
            .await
            .expect("Should retrieve global ordering");

        let retrieved_period = repository
            .get_training_metrics_ordering(
                &user_id,
                &TrainingMetricScope::TrainingPeriod(period_id),
            )
            .await
            .expect("Should retrieve period ordering");

        assert_eq!(retrieved_global.ids().len(), 2);
        assert_eq!(retrieved_global.ids()[0], global_id1);
        assert_eq!(retrieved_global.ids()[1], global_id2);

        assert_eq!(retrieved_period.ids().len(), 2);
        assert_eq!(retrieved_period.ids()[0], period_id1);
        assert_eq!(retrieved_period.ids()[1], period_id2);
    }

    #[tokio::test]
    async fn test_training_metrics_ordering_users_are_isolated() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let user1_id = UserId::test_default();
        let user2_id = UserId::new();
        let scope = TrainingMetricScope::Global;

        // Set ordering for user1
        let user1_id1 = TrainingMetricId::new();
        let user1_ordering = TrainingMetricsOrdering::try_from(vec![user1_id1.clone()])
            .expect("Should create ordering");

        repository
            .set_training_metrics_ordering(&user1_id, &scope, user1_ordering)
            .await
            .expect("Should save user1 ordering");

        // Set ordering for user2
        let user2_id1 = TrainingMetricId::new();
        let user2_ordering = TrainingMetricsOrdering::try_from(vec![user2_id1.clone()])
            .expect("Should create ordering");

        repository
            .set_training_metrics_ordering(&user2_id, &scope, user2_ordering)
            .await
            .expect("Should save user2 ordering");

        // Verify user1 ordering is not affected by user2
        let retrieved_user1 = repository
            .get_training_metrics_ordering(&user1_id, &scope)
            .await
            .expect("Should retrieve user1 ordering");

        assert_eq!(retrieved_user1.ids().len(), 1);
        assert_eq!(retrieved_user1.ids()[0], user1_id1);

        // Verify user2 has their own ordering
        let retrieved_user2 = repository
            .get_training_metrics_ordering(&user2_id, &scope)
            .await
            .expect("Should retrieve user2 ordering");

        assert_eq!(retrieved_user2.ids().len(), 1);
        assert_eq!(retrieved_user2.ids()[0], user2_id1);
    }

    #[tokio::test]
    async fn test_set_empty_training_metrics_ordering() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");
        let user_id = UserId::test_default();
        let scope = TrainingMetricScope::Global;

        // Set ordering with metrics first
        let id1 = TrainingMetricId::new();
        let ordering1 =
            TrainingMetricsOrdering::try_from(vec![id1.clone()]).expect("Should create ordering");

        repository
            .set_training_metrics_ordering(&user_id, &scope, ordering1)
            .await
            .expect("Should save ordering");

        // Now clear it by setting empty ordering
        let empty_ordering =
            TrainingMetricsOrdering::try_from(vec![]).expect("Should create empty ordering");

        repository
            .set_training_metrics_ordering(&user_id, &scope, empty_ordering)
            .await
            .expect("Should save empty ordering");

        // Verify it's empty
        let retrieved = repository
            .get_training_metrics_ordering(&user_id, &scope)
            .await
            .expect("Should retrieve ordering");

        assert_eq!(retrieved.ids().len(), 0);
    }

    #[test]
    fn test_parse_definition_row_metric_returns_metric_when_some() {
        let metric = Some(RepositoryTrainingMetricSource::Activity(
            ActivityMetric::Distance,
        ));
        let result = parse_definition_row_metric(
            metric,
            None,
            Some(TrainingMetricGroupBy::Sport),
        );
        assert_eq!(
            result,
            Some(TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::Distance,
                Some(TrainingMetricGroupBy::Sport)
            )))
        );
    }

    #[test]
    fn test_parse_definition_row_metric_metric_takes_priority_over_source() {
        let metric = Some(RepositoryTrainingMetricSource::Activity(
            ActivityMetric::Distance,
        ));
        let source = Some(ActivityMetricSource::Timeseries((
            TimeseriesMetric::Altitude,
            TimeseriesAggregate::Max,
        )));
        let result = parse_definition_row_metric(
            metric,
            source,
            Some(TrainingMetricGroupBy::WorkoutType),
        );
        assert_eq!(
            result,
            Some(TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::Distance,
                Some(TrainingMetricGroupBy::WorkoutType)
            )))
        );
    }

    #[test]
    fn test_parse_definition_row_metric_falls_back_to_source_when_metric_none() {
        let metric = None;
        let source = Some(ActivityMetricSource::Timeseries((
            TimeseriesMetric::Altitude,
            TimeseriesAggregate::Max,
        )));
        let result = parse_definition_row_metric(
            metric,
            source,
            Some(TrainingMetricGroupBy::SportCategory),
        );
        assert_eq!(
            result,
            Some(TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::MaxAltitude,
                Some(TrainingMetricGroupBy::SportCategory)
            )))
        );
    }

    #[test]
    fn test_parse_definition_row_metric_group_by_none() {
        let metric = Some(RepositoryTrainingMetricSource::Activity(
            ActivityMetric::Distance,
        ));
        let result = parse_definition_row_metric(metric, None, TrainingMetricGroupBy::none());
        assert_eq!(
            result,
            Some(TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::Distance,
                TrainingMetricGroupBy::none()
            )))
        );
    }

    #[test]
    fn test_parse_definition_row_metric_group_by_ignored_for_non_activity_sources() {
        let metric = Some(RepositoryTrainingMetricSource::HooperIndex(
            HooperIndexSource::Sleep,
        ));
        let result = parse_definition_row_metric(
            metric,
            None,
            Some(TrainingMetricGroupBy::Sport),
        );
        assert_eq!(
            result,
            Some(TrainingMetricSource::HooperIndex(HooperIndexSource::Sleep))
        );
    }

    #[test]
    fn test_parse_definition_row_metric_returns_none_when_both_none() {
        let metric = None;
        let source = None;
        let result = parse_definition_row_metric(metric, source, TrainingMetricGroupBy::none());
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_definition_row_metric_returns_none_when_source_conversion_fails() {
        let metric = None;
        // Latitude has no mapping in TryFrom<ActivityMetricSource> for ActivityMetricV2
        let source = Some(ActivityMetricSource::Timeseries((
            TimeseriesMetric::Latitude,
            TimeseriesAggregate::Max,
        )));
        let result = parse_definition_row_metric(metric, source, TrainingMetricGroupBy::none());
        assert_eq!(result, None);
    }

    // --- User isolation tests for methods accepting &UserId ---

    #[tokio::test]
    async fn test_get_training_note_returns_none_for_wrong_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let note = build_training_note();
        repository
            .save_training_note(note.clone())
            .await
            .expect("Should save note");

        let result = repository
            .get_training_note(&UserId::from("another_user".to_string()), note.id())
            .await
            .expect("Should not error");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_training_note_does_not_delete_for_wrong_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let note = build_training_note();
        repository
            .save_training_note(note.clone())
            .await
            .expect("Should save note");

        repository
            .delete_training_note(&UserId::from("another_user".to_string()), note.id())
            .await
            .expect("Should not fail");

        let retrieved = repository
            .get_training_note(note.user(), note.id())
            .await
            .expect("Should not error");
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_delete_definition_does_not_delete_for_wrong_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric = build_global_metric();
        repository
            .save_metric(metric.clone())
            .await
            .expect("Should save metric");

        let err = repository
            .delete_metric(&UserId::from("another_user".to_string()), metric.id())
            .await;

        let Err(DeleteTrainingMetricError::MetricDoesNotExist(_)) = err else {
            unreachable!("Should have been a DeleteTrainingMetricError::MetricDoesNotExist error");
        };

        let fetched = repository
            .get_metric(metric.definition().user(), metric.id())
            .await
            .expect("Should not error");
        assert!(fetched.is_some());
    }

    #[tokio::test]
    async fn test_update_training_metric_name_does_not_update_for_wrong_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Original Name")),
            TrainingMetricScope::Global,
            build_global_metric().definition().clone(),
        );
        repository
            .save_metric(metric.clone())
            .await
            .expect("Should save metric");

        let res = repository
            .update_metric_name(
                &UserId::from("another_user".to_string()),
                metric.id(),
                TrainingMetricName::from("Updated Name"),
            )
            .await
            .expect_err("Should fail");

        let UpdateTrainingMetricNameError::MetricDoesNotExist(id) = res else {
            unreachable!("Should be UpdateTrainingMetricNameError::MetricDoesNotExist")
        };
        assert_eq!(&id, metric.id());

        let metrics = repository
            .get_global_metrics(metric.definition().user())
            .await
            .expect("Should retrieve metrics");
        let fetched = metrics.iter().find(|m| m.id() == metric.id()).unwrap();
        assert_eq!(fetched.name(), metric.name());
    }

    #[tokio::test]
    async fn test_delete_training_period_does_not_delete_for_wrong_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let period = build_training_period();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        let res = repository
            .delete_training_period(&UserId::from("another_user".to_string()), period.id())
            .await
            .expect_err("Should fail");

        let DeleteTrainingPeriodError::PeriodDoesNotExist(id) = res else {
            unreachable!(" Should be DeleteTrainingPeriodError::PeriodDoesNotExist(id)")
        };
        assert_eq!(&id, period.id());

        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await;
        assert!(fetched.is_some());
    }

    #[tokio::test]
    async fn test_update_training_period_name_does_not_update_for_wrong_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let period = build_training_period();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        let res = repository
            .update_training_period_name(
                &UserId::from("another_user".to_string()),
                period.id(),
                "Updated Name".to_string(),
            )
            .await
            .expect_err("Should fail");

        let UpdateTrainingPeriodNameError::PeriodDoesNotExist(id) = res else {
            unreachable!("Should be UpdateTrainingPeriodNameError::PeriodDoesNotExist")
        };
        assert_eq!(&id, period.id());

        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await
            .expect("Period should still exist");
        assert_eq!(fetched.name(), period.name());
    }

    #[tokio::test]
    async fn test_update_training_period_note_does_not_update_for_wrong_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let period = build_training_period();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        let res = repository
            .update_training_period_note(
                &UserId::from("another_user".to_string()),
                period.id(),
                Some("Updated note".to_string()),
            )
            .await
            .expect_err("Should fail");

        let UpdateTrainingPeriodNoteError::PeriodDoesNotExist(id) = res else {
            unreachable!("Should be UpdateTrainingPeriodNoteError::PeriodDoesNotExist")
        };
        assert_eq!(&id, period.id());

        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await
            .expect("Period should still exist");
        assert_eq!(fetched.note(), period.note());
    }

    #[tokio::test]
    async fn test_update_training_period_dates_does_not_update_for_wrong_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository =
            SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                .await
                .expect("repo should init");

        let period = build_training_period();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        let new_start = "2025-12-01".parse::<NaiveDate>().unwrap();
        let new_end = Some("2025-12-31".parse::<NaiveDate>().unwrap());
        let res = repository
            .update_training_period_dates(
                &UserId::from("another_user".to_string()),
                period.id(),
                new_start,
                new_end,
            )
            .await
            .expect_err("Should fail");

        let UpdateTrainingPeriodDatesError::PeriodDoesNotExist(id) = res else {
            unreachable!("Should be UpdateTrainingPeriodDatesError::PeriodDoesNotExist")
        };
        assert_eq!(&id, period.id());

        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await
            .expect("Period should still exist");
        assert_eq!(fetched.start(), period.start());
        assert_eq!(fetched.end(), period.end());
    }

    #[cfg(test)]
    mod test_t_outbox_training_search {
        use chrono::Utc;

        use crate::{
            clock::clock_test_utils::FakeClock,
            domain::models::search::{SearchDocumentEvent, SearchDocumentType},
        };

        use super::*;

        #[tokio::test]
        async fn test_save_training_note_inserts_row_to_outbox_as_updated() {
            let db_file = NamedTempFile::new().unwrap();
            let now = Utc::now();
            let repo = SqliteTrainingRepository::new(
                &db_file.path().to_string_lossy(),
                FakeClock::new(now),
            )
            .await
            .expect("repo should init");
            let training_note = build_training_note();

            // Outbox initially empty
            assert!(
                repo.get_outbox_documents_to_process()
                    .await
                    .expect("Get outbox documents should have succeeded")
                    .is_empty()
            );

            repo.save_training_note(training_note.clone())
                .await
                .expect("Should have succeeded");

            // Outbox contains row for the newly saved note
            let document = repo
                .get_outbox_documents_to_process()
                .await
                .expect("Get outbox documents should have succeeded")
                .first()
                .cloned()
                .expect("Should contain at least one document");

            assert_eq!(document.document_type(), &SearchDocumentType::TrainingNote);
            assert_eq!(document.document_id(), training_note.id().to_string());
            assert_eq!(document.event(), &SearchDocumentEvent::Updated);
            assert_eq!(document.occurred_at(), &now);
            assert!(
                document
                    .content()
                    .contains(&training_note.content().to_string())
            )
        }

        #[tokio::test]
        async fn test_delete_training_note_inserts_row_to_outbox_as_deleted() {
            let db_file = NamedTempFile::new().unwrap();
            let now = Utc::now();
            let repo = SqliteTrainingRepository::new(
                &db_file.path().to_string_lossy(),
                FakeClock::new(now),
            )
            .await
            .expect("repo should init");
            let training_note = build_training_note();

            // Outbox initially empty
            assert!(
                repo.get_outbox_documents_to_process()
                    .await
                    .expect("Get outbox documents should have succeeded")
                    .is_empty()
            );

            repo.delete_training_note(training_note.user(), training_note.id())
                .await
                .expect("Should have succeeded");

            // Outbox contains row for the newly deleted training note
            let document = repo
                .get_outbox_documents_to_process()
                .await
                .expect("Get outbox documents should have succeeded")
                .first()
                .cloned()
                .expect("Should contain at least one document");

            assert_eq!(document.document_type(), &SearchDocumentType::TrainingNote);
            assert_eq!(document.document_id(), training_note.id().to_string());
            assert_eq!(document.event(), &SearchDocumentEvent::Deleted);
            assert_eq!(document.occurred_at(), &now);
            assert!(document.content().is_empty());
        }

        #[tokio::test]
        async fn test_mark_outbox_document_as_processed() {
            let db_file = NamedTempFile::new().unwrap();
            let now = Utc::now();
            let repo = SqliteTrainingRepository::new(
                &db_file.path().to_string_lossy(),
                FakeClock::new(now),
            )
            .await
            .expect("repo should init");
            let training_note = build_training_note();

            // Outbox initially empty
            assert!(
                repo.get_outbox_documents_to_process()
                    .await
                    .expect("Get outbox documents should have succeeded")
                    .is_empty()
            );

            repo.save_training_note(training_note.clone())
                .await
                .expect("Should have succeeded");

            // Outbox contains row for the newly saved training note
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

    #[cfg(test)]
    mod test_hooper_index {
        use super::*;

        fn scale(value: u8) -> SubjectiveScale {
            SubjectiveScale::try_from(value).unwrap()
        }

        fn test_date() -> NaiveDate {
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()
        }

        async fn setup() -> (NamedTempFile, SqliteTrainingRepository<Clock>) {
            let db_file = NamedTempFile::new().unwrap();
            let repository =
                SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                    .await
                    .expect("repo should init");
            (db_file, repository)
        }

        async fn count_rows(
            repository: &SqliteTrainingRepository<Clock>,
            user: &UserId,
            date: NaiveDate,
        ) -> i64 {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM t_hooper_index WHERE user=?1 AND date=?2",
            )
            .bind(user)
            .bind(date)
            .fetch_one(&repository.readers)
            .await
            .unwrap()
        }

        #[tokio::test]
        async fn test_save_hooper_index_round_trip() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();
            let value = HooperIndex::new(
                Some(scale(1)),
                Some(scale(2)),
                Some(scale(3)),
                Some(scale(4)),
                Some(scale(5)),
            );

            repository
                .save_hooper_index(&user, date, &value)
                .await
                .expect("Save should succeed");

            let saved = repository
                .get_hooper_index(&user, date)
                .await
                .expect("Get should succeed")
                .expect("Hooper index should exist");

            assert_eq!(saved.fatigue(), &Some(scale(1)));
            assert_eq!(saved.sleep(), &Some(scale(2)));
            assert_eq!(saved.pain(), &Some(scale(3)));
            assert_eq!(saved.stress(), &Some(scale(4)));
            assert_eq!(saved.mood(), &Some(scale(5)));
        }

        #[tokio::test]
        async fn test_save_hooper_index_upserts_existing_row() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            let initial = HooperIndex::new(
                Some(scale(1)),
                Some(scale(2)),
                Some(scale(3)),
                Some(scale(4)),
                Some(scale(5)),
            );
            repository
                .save_hooper_index(&user, date, &initial)
                .await
                .expect("Initial save should succeed");

            let updated = HooperIndex::new(
                Some(scale(6)),
                Some(scale(7)),
                Some(scale(8)),
                Some(scale(9)),
                Some(scale(10)),
            );
            repository
                .save_hooper_index(&user, date, &updated)
                .await
                .expect("Upsert should succeed");

            // Only one row should exist for the (user, date) pair.
            assert_eq!(count_rows(&repository, &user, date).await, 1);

            let saved = repository
                .get_hooper_index(&user, date)
                .await
                .expect("Get should succeed")
                .expect("Hooper index should exist");

            assert_eq!(saved.fatigue(), &Some(scale(6)));
            assert_eq!(saved.sleep(), &Some(scale(7)));
            assert_eq!(saved.pain(), &Some(scale(8)));
            assert_eq!(saved.stress(), &Some(scale(9)));
            assert_eq!(saved.mood(), &Some(scale(10)));
        }

        #[tokio::test]
        async fn test_save_hooper_index_allows_missing_values() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            repository
                .save_hooper_index(&user, date, &HooperIndex::default())
                .await
                .expect("Save should succeed");

            let saved = repository
                .get_hooper_index(&user, date)
                .await
                .expect("Get should succeed")
                .expect("Hooper index should exist");

            assert_eq!(saved.fatigue(), &None);
            assert_eq!(saved.sleep(), &None);
            assert_eq!(saved.pain(), &None);
            assert_eq!(saved.stress(), &None);
            assert_eq!(saved.mood(), &None);
        }

        #[tokio::test]
        async fn test_save_hooper_index_is_scoped_by_user_and_date() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let other_user = UserId::from("user2");
            let date = test_date();
            let other_date = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

            repository
                .save_hooper_index(
                    &user,
                    date,
                    &HooperIndex::new(Some(scale(1)), None, None, None, None),
                )
                .await
                .expect("Save should succeed");

            assert!(
                repository
                    .get_hooper_index(&other_user, date)
                    .await
                    .expect("Get should succeed")
                    .is_none()
            );
            assert!(
                repository
                    .get_hooper_index(&user, other_date)
                    .await
                    .expect("Get should succeed")
                    .is_none()
            );
        }

        #[tokio::test]
        async fn test_get_hooper_index_returns_none_when_not_found() {
            let (_db_file, repository) = setup().await;

            let result = repository
                .get_hooper_index(&UserId::from("user1"), test_date())
                .await
                .expect("Get should succeed");

            assert!(result.is_none());
        }

        #[tokio::test]
        async fn test_get_hooper_index_returns_matching_row() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();
            let other_date = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

            repository
                .save_hooper_index(
                    &user,
                    date,
                    &HooperIndex::new(
                        Some(scale(1)),
                        Some(scale(2)),
                        Some(scale(3)),
                        Some(scale(4)),
                        Some(scale(5)),
                    ),
                )
                .await
                .expect("Save should succeed");
            repository
                .save_hooper_index(
                    &user,
                    other_date,
                    &HooperIndex::new(
                        Some(scale(6)),
                        Some(scale(7)),
                        Some(scale(8)),
                        Some(scale(9)),
                        Some(scale(10)),
                    ),
                )
                .await
                .expect("Save should succeed");

            let saved = repository
                .get_hooper_index(&user, date)
                .await
                .expect("Get should succeed")
                .expect("Hooper index should exist");

            assert_eq!(saved.fatigue(), &Some(scale(1)));
            assert_eq!(saved.sleep(), &Some(scale(2)));
            assert_eq!(saved.pain(), &Some(scale(3)));
            assert_eq!(saved.stress(), &Some(scale(4)));
            assert_eq!(saved.mood(), &Some(scale(5)));
        }

        #[tokio::test]
        async fn test_get_hooper_indexes_returns_empty_when_no_rows() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let range = DateRange::new(
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            );

            let result = repository
                .get_hooper_indexes(&user, &range)
                .await
                .expect("Get should succeed");

            assert!(result.is_empty());
        }

        #[tokio::test]
        async fn test_get_hooper_indexes_returns_all_values() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            repository
                .save_hooper_index(
                    &user,
                    date,
                    &HooperIndex::new(
                        Some(scale(1)),
                        Some(scale(2)),
                        Some(scale(3)),
                        Some(scale(4)),
                        Some(scale(5)),
                    ),
                )
                .await
                .expect("Save should succeed");

            let result = repository
                .get_hooper_indexes(
                    &user,
                    &DateRange::new(
                        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                        NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                    ),
                )
                .await
                .expect("Get should succeed");

            assert_eq!(result.len(), 1);
            let (returned_date, value) = &result[0];
            assert_eq!(*returned_date, date);
            assert_eq!(value.fatigue(), &Some(scale(1)));
            assert_eq!(value.sleep(), &Some(scale(2)));
            assert_eq!(value.pain(), &Some(scale(3)));
            assert_eq!(value.stress(), &Some(scale(4)));
            assert_eq!(value.mood(), &Some(scale(5)));
        }

        #[tokio::test]
        async fn test_get_hooper_indexes_returns_rows_ordered_ascending() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let earlier = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
            let later = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

            // Save out of order to make sure the repository sorts the result.
            repository
                .save_hooper_index(
                    &user,
                    later,
                    &HooperIndex::new(Some(scale(5)), None, None, None, None),
                )
                .await
                .expect("Save should succeed");
            repository
                .save_hooper_index(
                    &user,
                    earlier,
                    &HooperIndex::new(Some(scale(1)), None, None, None, None),
                )
                .await
                .expect("Save should succeed");

            let result = repository
                .get_hooper_indexes(
                    &user,
                    &DateRange::new(
                        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                        NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                    ),
                )
                .await
                .expect("Get should succeed");

            let dates: Vec<NaiveDate> = result.iter().map(|(date, _)| *date).collect();
            assert_eq!(dates, vec![earlier, later]);
            assert_eq!(result[0].1.fatigue(), &Some(scale(1)));
            assert_eq!(result[1].1.fatigue(), &Some(scale(5)));
        }

        #[tokio::test]
        async fn test_get_hooper_indexes_range_is_start_inclusive_end_exclusive() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let start = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
            let end = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
            let before = start.pred_opt().unwrap();
            let inside = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
            let after = end.succ_opt().unwrap();

            for date in [before, start, inside, end, after] {
                repository
                    .save_hooper_index(
                        &user,
                        date,
                        &HooperIndex::new(Some(scale(1)), None, None, None, None),
                    )
                    .await
                    .expect("Save should succeed");
            }

            let result = repository
                .get_hooper_indexes(&user, &DateRange::new(start, end))
                .await
                .expect("Get should succeed");

            let dates: Vec<NaiveDate> = result.iter().map(|(date, _)| *date).collect();
            assert_eq!(dates, vec![start, inside]);
        }

        #[tokio::test]
        async fn test_get_hooper_indexes_is_scoped_by_user() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let other_user = UserId::from("user2");
            let date = test_date();

            repository
                .save_hooper_index(
                    &user,
                    date,
                    &HooperIndex::new(Some(scale(1)), None, None, None, None),
                )
                .await
                .expect("Save should succeed");
            repository
                .save_hooper_index(
                    &other_user,
                    date,
                    &HooperIndex::new(Some(scale(2)), None, None, None, None),
                )
                .await
                .expect("Save should succeed");

            let result = repository
                .get_hooper_indexes(
                    &user,
                    &DateRange::new(
                        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                        NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                    ),
                )
                .await
                .expect("Get should succeed");

            assert_eq!(result.len(), 1);
            assert_eq!(result[0].1.fatigue(), &Some(scale(1)));
        }

        #[tokio::test]
        async fn test_get_hooper_indexes_handles_missing_values() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            repository
                .save_hooper_index(&user, date, &HooperIndex::default())
                .await
                .expect("Save should succeed");

            let result = repository
                .get_hooper_indexes(
                    &user,
                    &DateRange::new(
                        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                        NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                    ),
                )
                .await
                .expect("Get should succeed");

            assert_eq!(result.len(), 1);
            let (_, value) = &result[0];
            assert_eq!(value.fatigue(), &None);
            assert_eq!(value.sleep(), &None);
            assert_eq!(value.pain(), &None);
            assert_eq!(value.stress(), &None);
            assert_eq!(value.mood(), &None);
        }

        #[tokio::test]
        async fn test_delete_hooper_index_removes_row() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            repository
                .save_hooper_index(
                    &user,
                    date,
                    &HooperIndex::new(Some(scale(1)), None, None, None, None),
                )
                .await
                .expect("Save should succeed");

            repository
                .delete_hooper_index(&user, date)
                .await
                .expect("Delete should succeed");

            assert!(
                repository
                    .get_hooper_index(&user, date)
                    .await
                    .expect("Get should succeed")
                    .is_none()
            );
            assert_eq!(count_rows(&repository, &user, date).await, 0);
        }

        #[tokio::test]
        async fn test_delete_hooper_index_is_ok_when_not_found() {
            let (_db_file, repository) = setup().await;

            let result = repository
                .delete_hooper_index(&UserId::from("user1"), test_date())
                .await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_delete_hooper_index_only_deletes_matching_row() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();
            let other_date = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

            repository
                .save_hooper_index(
                    &user,
                    date,
                    &HooperIndex::new(Some(scale(1)), None, None, None, None),
                )
                .await
                .expect("Save should succeed");
            repository
                .save_hooper_index(
                    &user,
                    other_date,
                    &HooperIndex::new(Some(scale(2)), None, None, None, None),
                )
                .await
                .expect("Save should succeed");

            repository
                .delete_hooper_index(&user, date)
                .await
                .expect("Delete should succeed");

            assert!(
                repository
                    .get_hooper_index(&user, date)
                    .await
                    .expect("Get should succeed")
                    .is_none()
            );
            let kept = repository
                .get_hooper_index(&user, other_date)
                .await
                .expect("Get should succeed")
                .expect("Other date should be kept");
            assert_eq!(kept.fatigue(), &Some(scale(2)));
        }

        #[tokio::test]
        async fn test_delete_hooper_index_does_not_delete_other_user() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let other_user = UserId::from("user2");
            let date = test_date();

            repository
                .save_hooper_index(
                    &user,
                    date,
                    &HooperIndex::new(Some(scale(1)), None, None, None, None),
                )
                .await
                .expect("Save should succeed");
            repository
                .save_hooper_index(
                    &other_user,
                    date,
                    &HooperIndex::new(Some(scale(2)), None, None, None, None),
                )
                .await
                .expect("Save should succeed");

            repository
                .delete_hooper_index(&user, date)
                .await
                .expect("Delete should succeed");

            let kept = repository
                .get_hooper_index(&other_user, date)
                .await
                .expect("Get should succeed")
                .expect("Other user row should be kept");
            assert_eq!(kept.fatigue(), &Some(scale(2)));
        }
    }

    #[cfg(test)]
    mod test_weight_and_nutrition {
        use super::*;

        fn test_date() -> NaiveDate {
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()
        }

        fn sample() -> WeightAndNutrition {
            WeightAndNutrition::new(
                Some(70.0),
                Some(15.0),
                Some(30.0),
                Some(22.0),
                Some(2000.0),
                Some(50.0),
                Some(250.0),
                Some(150.0),
                Some(2.5),
                Some(0.0),
            )
        }

        async fn setup() -> (NamedTempFile, SqliteTrainingRepository<Clock>) {
            let db_file = NamedTempFile::new().unwrap();
            let repository =
                SqliteTrainingRepository::new(&db_file.path().to_string_lossy(), Clock::new())
                    .await
                    .expect("repo should init");
            (db_file, repository)
        }

        async fn count_rows(
            repository: &SqliteTrainingRepository<Clock>,
            user: &UserId,
            date: NaiveDate,
        ) -> i64 {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM t_weight_and_nutrition WHERE user=?1 AND date=?2",
            )
            .bind(user)
            .bind(date)
            .fetch_one(&repository.readers)
            .await
            .unwrap()
        }

        #[tokio::test]
        async fn test_save_weight_and_nutrition_round_trip() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            repository
                .save_weight_and_nutrition(&user, date, &sample())
                .await
                .expect("Save should succeed");

            let saved = repository
                .get_weight_and_nutrition(&user, date)
                .await
                .expect("Get should succeed")
                .expect("Weight and nutrition should exist");

            assert_eq!(saved.weight(), Some(70.0));
            assert_eq!(saved.fat(), Some(15.0));
            assert_eq!(saved.muscle(), Some(30.0));
            assert_eq!(saved.bmi(), Some(22.0));
            assert_eq!(saved.calories(), Some(2000.0));
            assert_eq!(saved.lipid(), Some(50.0));
            assert_eq!(saved.carbs(), Some(250.0));
            assert_eq!(saved.protein(), Some(150.0));
            assert_eq!(saved.water(), Some(2.5));
            assert_eq!(saved.alcohol(), Some(0.0));
        }

        #[tokio::test]
        async fn test_save_weight_and_nutrition_upserts_existing_row() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            repository
                .save_weight_and_nutrition(&user, date, &sample())
                .await
                .expect("Initial save should succeed");

            let updated = WeightAndNutrition::new(
                Some(72.0),
                Some(16.0),
                Some(31.0),
                Some(23.0),
                Some(2100.0),
                Some(55.0),
                Some(260.0),
                Some(160.0),
                Some(3.0),
                Some(1.0),
            );
            repository
                .save_weight_and_nutrition(&user, date, &updated)
                .await
                .expect("Upsert should succeed");

            // Only one row should exist for the (user, date) pair.
            assert_eq!(count_rows(&repository, &user, date).await, 1);

            let saved = repository
                .get_weight_and_nutrition(&user, date)
                .await
                .expect("Get should succeed")
                .expect("Weight and nutrition should exist");

            assert_eq!(saved.weight(), Some(72.0));
            assert_eq!(saved.fat(), Some(16.0));
            assert_eq!(saved.muscle(), Some(31.0));
            assert_eq!(saved.bmi(), Some(23.0));
            assert_eq!(saved.calories(), Some(2100.0));
            assert_eq!(saved.lipid(), Some(55.0));
            assert_eq!(saved.carbs(), Some(260.0));
            assert_eq!(saved.protein(), Some(160.0));
            assert_eq!(saved.water(), Some(3.0));
            assert_eq!(saved.alcohol(), Some(1.0));
        }

        #[tokio::test]
        async fn test_save_weight_and_nutrition_allows_missing_values() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            repository
                .save_weight_and_nutrition(&user, date, &WeightAndNutrition::default())
                .await
                .expect("Save should succeed");

            let saved = repository
                .get_weight_and_nutrition(&user, date)
                .await
                .expect("Get should succeed")
                .expect("Weight and nutrition should exist");

            assert_eq!(saved.weight(), None);
            assert_eq!(saved.fat(), None);
            assert_eq!(saved.muscle(), None);
            assert_eq!(saved.bmi(), None);
            assert_eq!(saved.calories(), None);
            assert_eq!(saved.lipid(), None);
            assert_eq!(saved.carbs(), None);
            assert_eq!(saved.protein(), None);
            assert_eq!(saved.water(), None);
            assert_eq!(saved.alcohol(), None);
        }

        #[tokio::test]
        async fn test_save_weight_and_nutrition_is_scoped_by_user_and_date() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let other_user = UserId::from("user2");
            let date = test_date();
            let other_date = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

            repository
                .save_weight_and_nutrition(&user, date, &sample())
                .await
                .expect("Save should succeed");

            assert!(
                repository
                    .get_weight_and_nutrition(&other_user, date)
                    .await
                    .expect("Get should succeed")
                    .is_none()
            );
            assert!(
                repository
                    .get_weight_and_nutrition(&user, other_date)
                    .await
                    .expect("Get should succeed")
                    .is_none()
            );
        }

        #[tokio::test]
        async fn test_get_weight_and_nutrition_returns_none_when_not_found() {
            let (_db_file, repository) = setup().await;

            let result = repository
                .get_weight_and_nutrition(&UserId::from("user1"), test_date())
                .await
                .expect("Get should succeed");

            assert!(result.is_none());
        }

        #[tokio::test]
        async fn test_get_weight_and_nutrition_returns_matching_row() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();
            let other_date = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

            repository
                .save_weight_and_nutrition(&user, date, &sample())
                .await
                .expect("Save should succeed");
            repository
                .save_weight_and_nutrition(
                    &user,
                    other_date,
                    &WeightAndNutrition::new(
                        Some(80.0),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                )
                .await
                .expect("Save should succeed");

            let saved = repository
                .get_weight_and_nutrition(&user, date)
                .await
                .expect("Get should succeed")
                .expect("Weight and nutrition should exist");

            assert_eq!(saved.weight(), Some(70.0));
            assert_eq!(saved.calories(), Some(2000.0));
        }

        #[tokio::test]
        async fn test_get_weight_and_nutritions_returns_empty_when_no_rows() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let range = DateRange::new(
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            );

            let result = repository
                .get_weight_and_nutritions(&user, &range)
                .await
                .expect("Get should succeed");

            assert!(result.is_empty());
        }

        #[tokio::test]
        async fn test_get_weight_and_nutritions_returns_all_values() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            repository
                .save_weight_and_nutrition(&user, date, &sample())
                .await
                .expect("Save should succeed");

            let result = repository
                .get_weight_and_nutritions(
                    &user,
                    &DateRange::new(
                        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                        NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                    ),
                )
                .await
                .expect("Get should succeed");

            assert_eq!(result.len(), 1);
            let (returned_date, value) = &result[0];
            assert_eq!(*returned_date, date);
            assert_eq!(value.weight(), Some(70.0));
            assert_eq!(value.calories(), Some(2000.0));
        }

        #[tokio::test]
        async fn test_get_weight_and_nutritions_returns_rows_ordered_ascending() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let earlier = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
            let later = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

            // Save out of order to make sure the repository sorts the result.
            repository
                .save_weight_and_nutrition(
                    &user,
                    later,
                    &WeightAndNutrition::new(
                        Some(75.0),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                )
                .await
                .expect("Save should succeed");
            repository
                .save_weight_and_nutrition(
                    &user,
                    earlier,
                    &WeightAndNutrition::new(
                        Some(70.0),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                )
                .await
                .expect("Save should succeed");

            let result = repository
                .get_weight_and_nutritions(
                    &user,
                    &DateRange::new(
                        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                        NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                    ),
                )
                .await
                .expect("Get should succeed");

            let dates: Vec<NaiveDate> = result.iter().map(|(date, _)| *date).collect();
            assert_eq!(dates, vec![earlier, later]);
            assert_eq!(result[0].1.weight(), Some(70.0));
            assert_eq!(result[1].1.weight(), Some(75.0));
        }

        #[tokio::test]
        async fn test_get_weight_and_nutritions_range_is_start_inclusive_end_exclusive() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let start = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
            let end = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
            let before = start.pred_opt().unwrap();
            let inside = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
            let after = end.succ_opt().unwrap();

            for date in [before, start, inside, end, after] {
                repository
                    .save_weight_and_nutrition(
                        &user,
                        date,
                        &WeightAndNutrition::new(
                            Some(70.0),
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                        ),
                    )
                    .await
                    .expect("Save should succeed");
            }

            let result = repository
                .get_weight_and_nutritions(&user, &DateRange::new(start, end))
                .await
                .expect("Get should succeed");

            let dates: Vec<NaiveDate> = result.iter().map(|(date, _)| *date).collect();
            assert_eq!(dates, vec![start, inside]);
        }

        #[tokio::test]
        async fn test_get_weight_and_nutritions_is_scoped_by_user() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let other_user = UserId::from("user2");
            let date = test_date();

            repository
                .save_weight_and_nutrition(
                    &user,
                    date,
                    &WeightAndNutrition::new(
                        Some(70.0),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                )
                .await
                .expect("Save should succeed");
            repository
                .save_weight_and_nutrition(
                    &other_user,
                    date,
                    &WeightAndNutrition::new(
                        Some(80.0),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                )
                .await
                .expect("Save should succeed");

            let result = repository
                .get_weight_and_nutritions(
                    &user,
                    &DateRange::new(
                        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                        NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                    ),
                )
                .await
                .expect("Get should succeed");

            assert_eq!(result.len(), 1);
            assert_eq!(result[0].1.weight(), Some(70.0));
        }

        #[tokio::test]
        async fn test_get_weight_and_nutritions_handles_missing_values() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            repository
                .save_weight_and_nutrition(&user, date, &WeightAndNutrition::default())
                .await
                .expect("Save should succeed");

            let result = repository
                .get_weight_and_nutritions(
                    &user,
                    &DateRange::new(
                        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                        NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                    ),
                )
                .await
                .expect("Get should succeed");

            assert_eq!(result.len(), 1);
            let (_, value) = &result[0];
            assert_eq!(value.weight(), None);
            assert_eq!(value.calories(), None);
            assert_eq!(value.alcohol(), None);
        }

        #[tokio::test]
        async fn test_delete_weight_and_nutrition_removes_row() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            repository
                .save_weight_and_nutrition(&user, date, &sample())
                .await
                .expect("Save should succeed");

            repository
                .delete_weight_and_nutrition(&user, date)
                .await
                .expect("Delete should succeed");

            assert!(
                repository
                    .get_weight_and_nutrition(&user, date)
                    .await
                    .expect("Get should succeed")
                    .is_none()
            );
            assert_eq!(count_rows(&repository, &user, date).await, 0);
        }

        #[tokio::test]
        async fn test_delete_weight_and_nutrition_is_ok_when_not_found() {
            let (_db_file, repository) = setup().await;

            let result = repository
                .delete_weight_and_nutrition(&UserId::from("user1"), test_date())
                .await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_delete_weight_and_nutrition_only_deletes_matching_row() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();
            let other_date = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

            repository
                .save_weight_and_nutrition(
                    &user,
                    date,
                    &WeightAndNutrition::new(
                        Some(70.0),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                )
                .await
                .expect("Save should succeed");
            repository
                .save_weight_and_nutrition(
                    &user,
                    other_date,
                    &WeightAndNutrition::new(
                        Some(80.0),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                )
                .await
                .expect("Save should succeed");

            repository
                .delete_weight_and_nutrition(&user, date)
                .await
                .expect("Delete should succeed");

            assert!(
                repository
                    .get_weight_and_nutrition(&user, date)
                    .await
                    .expect("Get should succeed")
                    .is_none()
            );
            let kept = repository
                .get_weight_and_nutrition(&user, other_date)
                .await
                .expect("Get should succeed")
                .expect("Other date should be kept");
            assert_eq!(kept.weight(), Some(80.0));
        }

        #[tokio::test]
        async fn test_delete_weight_and_nutrition_does_not_delete_other_user() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let other_user = UserId::from("user2");
            let date = test_date();

            repository
                .save_weight_and_nutrition(
                    &user,
                    date,
                    &WeightAndNutrition::new(
                        Some(70.0),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                )
                .await
                .expect("Save should succeed");
            repository
                .save_weight_and_nutrition(
                    &other_user,
                    date,
                    &WeightAndNutrition::new(
                        Some(80.0),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                )
                .await
                .expect("Save should succeed");

            repository
                .delete_weight_and_nutrition(&user, date)
                .await
                .expect("Delete should succeed");

            let kept = repository
                .get_weight_and_nutrition(&other_user, date)
                .await
                .expect("Get should succeed")
                .expect("Other user row should be kept");
            assert_eq!(kept.weight(), Some(80.0));
        }

        async fn count_all_rows(
            repository: &SqliteTrainingRepository<Clock>,
            user: &UserId,
        ) -> i64 {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM t_weight_and_nutrition WHERE user=?1",
            )
            .bind(user)
            .fetch_one(&repository.readers)
            .await
            .unwrap()
        }

        fn value_with_weight(weight: f32) -> WeightAndNutrition {
            WeightAndNutrition::new(
                Some(weight),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        }

        #[tokio::test]
        async fn test_save_bulk_weight_and_nutrition_empty_is_noop() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");

            repository
                .save_bulk_weight_and_nutrition(&user, Vec::new())
                .await
                .expect("Bulk save should succeed");

            assert_eq!(count_all_rows(&repository, &user).await, 0);
        }

        #[tokio::test]
        async fn test_save_bulk_weight_and_nutrition_round_trip() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");

            let day_1 = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
            let day_2 = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
            let day_3 = NaiveDate::from_ymd_opt(2026, 1, 3).unwrap();

            repository
                .save_bulk_weight_and_nutrition(
                    &user,
                    vec![
                        (day_1, value_with_weight(70.0)),
                        (day_2, sample()),
                        (day_3, WeightAndNutrition::default()),
                    ],
                )
                .await
                .expect("Bulk save should succeed");

            assert_eq!(count_all_rows(&repository, &user).await, 3);

            let first = repository
                .get_weight_and_nutrition(&user, day_1)
                .await
                .expect("Get should succeed")
                .expect("First row should exist");
            assert_eq!(first.weight(), Some(70.0));
            assert_eq!(first.fat(), None);

            let second = repository
                .get_weight_and_nutrition(&user, day_2)
                .await
                .expect("Get should succeed")
                .expect("Second row should exist");
            assert_eq!(second.weight(), Some(70.0));
            assert_eq!(second.fat(), Some(15.0));
            assert_eq!(second.muscle(), Some(30.0));
            assert_eq!(second.bmi(), Some(22.0));
            assert_eq!(second.calories(), Some(2000.0));
            assert_eq!(second.lipid(), Some(50.0));
            assert_eq!(second.carbs(), Some(250.0));
            assert_eq!(second.protein(), Some(150.0));
            assert_eq!(second.water(), Some(2.5));
            assert_eq!(second.alcohol(), Some(0.0));

            let third = repository
                .get_weight_and_nutrition(&user, day_3)
                .await
                .expect("Get should succeed")
                .expect("Third row should exist");
            assert_eq!(third.weight(), None);
            assert_eq!(third.calories(), None);
        }

        #[tokio::test]
        async fn test_save_bulk_weight_and_nutrition_does_not_affect_other_users() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let other_user = UserId::from("user2");
            let date = test_date();

            repository
                .save_bulk_weight_and_nutrition(&user, vec![(date, sample())])
                .await
                .expect("Bulk save should succeed");

            let other = repository
                .get_weight_and_nutrition(&other_user, date)
                .await
                .expect("Get should succeed");
            assert!(other.is_none());
            assert_eq!(count_all_rows(&repository, &other_user).await, 0);
        }

        #[tokio::test]
        async fn test_save_bulk_weight_and_nutrition_persists_all_batches() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let base = test_date();

            // The implementation caps each statement at 32766 / 12 = 2730 bound
            // values, so 2735 rows forces the loop to execute twice.
            let total = 2735u64;
            let values = (0..total)
                .map(|i| (base.add(Days::new(i)), value_with_weight(i as f32)))
                .collect::<Vec<_>>();

            repository
                .save_bulk_weight_and_nutrition(&user, values)
                .await
                .expect("Bulk save should succeed");

            assert_eq!(count_all_rows(&repository, &user).await, total as i64);

            let stored = repository
                .get_weight_and_nutritions(&user, &DateRange::new(base, base.add(Days::new(total))))
                .await
                .expect("Get should succeed");
            assert_eq!(stored.len(), total as usize);
            assert_eq!(stored.first().unwrap().1.weight(), Some(0.0));
            assert_eq!(stored.last().unwrap().1.weight(), Some((total - 1) as f32));
        }

        #[tokio::test]
        async fn test_save_bulk_weight_and_nutrition_upserts_existing_rows() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let day_1 = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
            let day_2 = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();

            repository
                .save_weight_and_nutrition(&user, day_1, &sample())
                .await
                .expect("Save should succeed");
            repository
                .save_weight_and_nutrition(&user, day_2, &sample())
                .await
                .expect("Save should succeed");

            let updated = WeightAndNutrition::new(
                Some(65.0),
                Some(12.0),
                Some(28.0),
                Some(21.0),
                Some(1800.0),
                Some(40.0),
                Some(200.0),
                Some(140.0),
                Some(3.0),
                Some(5.0),
            );

            repository
                .save_bulk_weight_and_nutrition(
                    &user,
                    vec![(day_1, updated.clone()), (day_2, updated.clone())],
                )
                .await
                .expect("Bulk save should succeed");

            assert_eq!(count_all_rows(&repository, &user).await, 2);

            for day in [day_1, day_2] {
                let saved = repository
                    .get_weight_and_nutrition(&user, day)
                    .await
                    .expect("Get should succeed")
                    .expect("Row should exist");
                assert_eq!(saved.weight(), Some(65.0));
                assert_eq!(saved.fat(), Some(12.0));
                assert_eq!(saved.muscle(), Some(28.0));
                assert_eq!(saved.bmi(), Some(21.0));
                assert_eq!(saved.calories(), Some(1800.0));
                assert_eq!(saved.lipid(), Some(40.0));
                assert_eq!(saved.carbs(), Some(200.0));
                assert_eq!(saved.protein(), Some(140.0));
                assert_eq!(saved.water(), Some(3.0));
                assert_eq!(saved.alcohol(), Some(5.0));
            }
        }

        #[tokio::test]
        async fn test_save_bulk_weight_and_nutrition_duplicate_dates_last_value_wins() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let date = test_date();

            repository
                .save_bulk_weight_and_nutrition(
                    &user,
                    vec![
                        (date, value_with_weight(70.0)),
                        (date, value_with_weight(80.0)),
                    ],
                )
                .await
                .expect("Bulk save should succeed");

            assert_eq!(count_all_rows(&repository, &user).await, 1);

            let saved = repository
                .get_weight_and_nutrition(&user, date)
                .await
                .expect("Get should succeed")
                .expect("Row should exist");
            assert_eq!(saved.weight(), Some(80.0));
        }

        #[tokio::test]
        async fn test_save_bulk_weight_and_nutrition_upserts_across_batches() {
            let (_db_file, repository) = setup().await;
            let user = UserId::from("user1");
            let base = test_date();

            // The implementation caps each statement at 32766 / 12 = 2730 bound
            // values, so 2735 rows forces the loop to execute twice.
            let total = 2735u64;
            let seed = (0..total)
                .map(|i| (base.add(Days::new(i)), value_with_weight(0.0)))
                .collect::<Vec<_>>();
            repository
                .save_bulk_weight_and_nutrition(&user, seed)
                .await
                .expect("Bulk save should succeed");

            let updates = (0..total)
                .map(|i| (base.add(Days::new(i)), value_with_weight((i + 1) as f32)))
                .collect::<Vec<_>>();
            repository
                .save_bulk_weight_and_nutrition(&user, updates)
                .await
                .expect("Bulk upsert should succeed");

            assert_eq!(count_all_rows(&repository, &user).await, total as i64);

            let stored = repository
                .get_weight_and_nutritions(&user, &DateRange::new(base, base.add(Days::new(total))))
                .await
                .expect("Get should succeed");
            assert_eq!(stored.len(), total as usize);
            assert_eq!(stored.first().unwrap().1.weight(), Some(1.0));
            assert_eq!(stored.last().unwrap().1.weight(), Some(total as f32));
        }
    }
}
