use std::str::FromStr;

use anyhow::anyhow;
use chrono::{DateTime, FixedOffset, NaiveDate};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

use crate::domain::{
    models::{
        UserId,
        training::{
            ActivityMetricSource, TrainingMetric, TrainingMetricAggregate,
            TrainingMetricDefinition, TrainingMetricFilters, TrainingMetricGranularity,
            TrainingMetricGroupBy, TrainingMetricId, TrainingMetricName, TrainingMetricScope,
            TrainingMetricsOrdering, TrainingNote, TrainingNoteContent, TrainingNoteDate,
            TrainingNoteId, TrainingNoteTitle, TrainingPeriod, TrainingPeriodId,
            TrainingPeriodSports,
        },
    },
    ports::{
        DeleteMetricError, DeleteTrainingNoteError, GetDefinitionError,
        GetTrainingMetricsDefinitionsError, GetTrainingMetricsOrderingError, GetTrainingNoteError,
        SaveTrainingMetricError, SaveTrainingNoteError, SaveTrainingPeriodError,
        SetTrainingMetricsOrderingError, TrainingRepository,
        UpdateTrainingMetricScopeRepositoryError, UpdateTrainingNoteError,
    },
};

type DefinitionRow = (
    TrainingMetricId,
    Option<TrainingMetricName>,
    UserId,
    ActivityMetricSource,
    TrainingMetricGranularity,
    TrainingMetricAggregate,
    TrainingMetricFilters,
    Option<TrainingMetricGroupBy>,
    Option<TrainingPeriodId>,
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

#[derive(Debug, Clone)]
pub struct SqliteTrainingRepository {
    pool: SqlitePool,
}

impl SqliteTrainingRepository {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migrations
        sqlx::migrate!("migrations/training").run(&pool).await?;

        Ok(Self { pool })
    }
}

impl TrainingRepository for SqliteTrainingRepository {
    async fn save_training_metric_definition(
        &self,
        metric: TrainingMetric,
    ) -> Result<(), SaveTrainingMetricError> {
        let definition = metric.definition();
        sqlx::query(
            "INSERT INTO t_training_metrics_definitions (id, user_id, source, granularity, aggregate, filters, group_by, name, training_period_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
        )
        .bind(metric.id())
        .bind(definition.user())
        .bind(definition.source())
        .bind(definition.granularity())
        .bind(definition.aggregate())
        .bind(definition.filters())
        .bind(definition.group_by())
        .bind(metric.name())
        .bind(metric.scope().period())
        .execute(&self.pool)
        .await
        .map_err(|err| SaveTrainingMetricError::Unknown(anyhow!(err)))
        .map(|_| ())
    }

    async fn update_training_metric_scope(
        &self,
        metric: &TrainingMetricId,
        scope: &TrainingMetricScope,
    ) -> Result<(), UpdateTrainingMetricScopeRepositoryError> {
        let period_id: Option<TrainingPeriodId> = scope.into();
        sqlx::query(
            "UPDATE t_training_metrics_definitions SET training_period_id = ?1 WHERE id = ?2;",
        )
        .bind(period_id)
        .bind(metric)
        .execute(&self.pool)
        .await
        .map_err(|err| UpdateTrainingMetricScopeRepositoryError::Unknown(anyhow!(err)))
        .map(|_| ())
    }

    async fn get_definition(
        &self,
        metric: &TrainingMetricId,
    ) -> Result<Option<TrainingMetricDefinition>, GetDefinitionError> {
        match sqlx::query_as::<_, DefinitionRow>(
            "
        SELECT id, name, user_id, source, granularity, aggregate, filters, group_by, training_period_id
        FROM t_training_metrics_definitions
        WHERE id = ?1 LIMIT 1;",
        )
        .bind(metric)
        .fetch_one(&self.pool)
        .await
        {
            Ok((_id, _name, user_id, source, granularity, aggregate, filters, group_by, _training_period_id)) => {
                Ok(Some(TrainingMetricDefinition::new(
                    user_id,
                    source,
                    granularity,
                    aggregate,
                    filters,
                    group_by,
                )))
            }
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(GetDefinitionError::Unknown(anyhow!(err))),
        }
    }

    async fn get_global_metrics(
        &self,
        user: &UserId,
    ) -> Result<Vec<TrainingMetric>, GetTrainingMetricsDefinitionsError> {
        sqlx::query_as::<_, DefinitionRow>(
            "SELECT id, name, user_id, source, granularity, aggregate, filters, group_by, training_period_id
            FROM t_training_metrics_definitions
            WHERE user_id = ?1 AND training_period_id IS NULL;",
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(|err| GetTrainingMetricsDefinitionsError::Unknown(anyhow!(err)))
        .map(|rows| {
            rows.into_iter()
                .map(
                    |(id, name, user_id, source, granularity, aggregate, filters, group_by, training_period_id)| {
                        TrainingMetric::new(
                            id,
                            name,
                            TrainingMetricScope::from(&training_period_id),
                            TrainingMetricDefinition::new(
                                user_id,
                                source,
                                granularity,
                                aggregate,
                                filters,
                                group_by,
                            ),
                        )
                    },
                )
                .collect()
        })
    }

    async fn get_period_metrics(
        &self,
        user: &UserId,
        period: &TrainingPeriodId,
    ) -> Result<Vec<TrainingMetric>, GetTrainingMetricsDefinitionsError> {
        sqlx::query_as::<_, DefinitionRow>(
            "SELECT id, name, user_id, source, granularity, aggregate, filters, group_by, training_period_id
            FROM t_training_metrics_definitions
            WHERE user_id = ?1 AND training_period_id = ?2;",
        )
        .bind(user)
        .bind(period)
        .fetch_all(&self.pool)
        .await
        .map_err(|err| GetTrainingMetricsDefinitionsError::Unknown(anyhow!(err)))
        .map(|rows| {
            rows.into_iter()
                .map(
                    |(id, name, user_id, source, granularity, aggregate, filters, group_by, training_period_id)| {
                        TrainingMetric::new(
                            id,
                            name,
                            TrainingMetricScope::from(&training_period_id),
                            TrainingMetricDefinition::new(
                                user_id,
                                source,
                                granularity,
                                aggregate,
                                filters,
                                group_by,
                            ),
                        )
                    },
                )
                .collect()
        })
    }

    async fn delete_definition(&self, metric: &TrainingMetricId) -> Result<(), DeleteMetricError> {
        match sqlx::query(
            "DELETE FROM t_training_metrics_definitions
        WHERE id = ?1;",
        )
        .bind(metric)
        .execute(&self.pool)
        .await
        {
            Ok(res) => {
                if res.rows_affected() == 1 {
                    Ok(())
                } else {
                    Err(DeleteMetricError::TrainingMetricDoesNotExists(
                        metric.clone(),
                    ))
                }
            }
            Err(err) => Err(DeleteMetricError::Unknown(anyhow!(err))),
        }
    }

    async fn update_training_metric_name(
        &self,
        metric_id: &TrainingMetricId,
        name: TrainingMetricName,
    ) -> Result<(), anyhow::Error> {
        sqlx::query("UPDATE t_training_metrics_definitions SET name = ?1 WHERE id = ?2;")
            .bind(name.to_string())
            .bind(metric_id)
            .execute(&self.pool)
            .await
            .map_err(|err| anyhow!(err))
            .map(|_| ())
    }

    async fn save_training_period(
        &self,
        period: crate::domain::models::training::TrainingPeriod,
    ) -> Result<(), crate::domain::ports::SaveTrainingPeriodError> {
        sqlx::query("INSERT INTO t_training_periods VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);")
            .bind(period.id())
            .bind(period.user())
            .bind(period.start())
            .bind(period.end())
            .bind(period.name())
            .bind(period.sports())
            .bind(period.note())
            .execute(&self.pool)
            .await
            .map_err(|err| SaveTrainingPeriodError::Unknown(anyhow!(err)))
            .map(|_| ())
    }

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
        .fetch_one(&self.pool)
        .await
        {
            Ok((id, user_id, start, end, name, sports, note)) => {
                TrainingPeriod::new(id, user_id, start, end, name, sports, note).ok()
            }
            Err(sqlx::Error::RowNotFound) => None,
            Err(_err) => None,
        }
    }

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
        .fetch_all(&self.pool)
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

    async fn delete_training_period(
        &self,
        period_id: &TrainingPeriodId,
    ) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM t_training_periods WHERE id = ?1;")
            .bind(period_id)
            .execute(&self.pool)
            .await
            .map_err(|err| anyhow!(err))
            .map(|_| ())
    }

    async fn update_training_period_name(
        &self,
        period_id: &TrainingPeriodId,
        name: String,
    ) -> Result<(), anyhow::Error> {
        sqlx::query("UPDATE t_training_periods SET name = ?1 WHERE id = ?2;")
            .bind(name)
            .bind(period_id)
            .execute(&self.pool)
            .await
            .map_err(|err| anyhow!(err))
            .map(|_| ())
    }

    async fn update_training_period_note(
        &self,
        period_id: &TrainingPeriodId,
        note: Option<String>,
    ) -> Result<(), anyhow::Error> {
        sqlx::query("UPDATE t_training_periods SET note = ?1 WHERE id = ?2;")
            .bind(note)
            .bind(period_id)
            .execute(&self.pool)
            .await
            .map_err(|err| anyhow!(err))
            .map(|_| ())
    }

    async fn update_training_period_dates(
        &self,
        period_id: &TrainingPeriodId,
        start: NaiveDate,
        end: Option<NaiveDate>,
    ) -> Result<(), anyhow::Error> {
        sqlx::query("UPDATE t_training_periods SET start = ?1, end = ?2 WHERE id = ?3;")
            .bind(start)
            .bind(end)
            .bind(period_id)
            .execute(&self.pool)
            .await
            .map_err(|err| anyhow!(err))
            .map(|_| ())
    }

    async fn save_training_note(&self, note: TrainingNote) -> Result<(), SaveTrainingNoteError> {
        sqlx::query(
            "INSERT INTO t_training_notes (id, user_id, title, content, date, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
        )
        .bind(note.id().to_string())
        .bind(note.user().to_string())
        .bind(note.title().as_ref().map(|t| t.to_string()))
        .bind(note.content().to_string())
        .bind(note.date().to_string())
        .bind(note.created_at().to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|err| SaveTrainingNoteError::Unknown(anyhow!(err)))
        .map(|_| ())
    }

    async fn get_training_note(
        &self,
        note_id: &TrainingNoteId,
    ) -> Result<Option<TrainingNote>, GetTrainingNoteError> {
        match sqlx::query_as::<_, TrainingNoteRow>(
            "SELECT id, user_id, title, content, date, created_at FROM t_training_notes WHERE id = ?1 LIMIT 1;",
        )
        .bind(note_id.to_string())
        .fetch_one(&self.pool)
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

    async fn get_training_notes(
        &self,
        user: &UserId,
    ) -> Result<Vec<TrainingNote>, GetTrainingNoteError> {
        let rows = sqlx::query_as::<_, TrainingNoteRow>(
            "SELECT id, user_id, title, content, date, created_at FROM t_training_notes WHERE user_id = ?1 ORDER BY created_at DESC;",
        )
        .bind(user.to_string())
        .fetch_all(&self.pool)
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

    async fn update_training_note(
        &self,
        note_id: &TrainingNoteId,
        title: Option<TrainingNoteTitle>,
        content: TrainingNoteContent,
        date: TrainingNoteDate,
    ) -> Result<(), UpdateTrainingNoteError> {
        sqlx::query(
            "UPDATE t_training_notes SET title = ?1, content = ?2, date = ?3 WHERE id = ?4;",
        )
        .bind(title.as_ref().map(|t| t.to_string()))
        .bind(content.to_string())
        .bind(date.to_string())
        .bind(note_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|err| UpdateTrainingNoteError::Unknown(anyhow!(err)))
        .map(|_| ())
    }

    async fn delete_training_note(
        &self,
        note_id: &TrainingNoteId,
    ) -> Result<(), DeleteTrainingNoteError> {
        sqlx::query("DELETE FROM t_training_notes WHERE id = ?1;")
            .bind(note_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|err| DeleteTrainingNoteError::Unknown(anyhow!(err)))
            .map(|_| ())
    }

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| GetTrainingMetricsOrderingError::Unknown(e.into()))?;

        match result {
            Some((metric_ids_str,)) => {
                let metric_ids: Vec<TrainingMetricId> = metric_ids_str
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|id| TrainingMetricId::from(id))
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
        .execute(&self.pool)
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
            .execute(&self.pool)
            .await
            .map_err(|e| SetTrainingMetricsOrderingError::Unknown(e.into()))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test_sqlite_training_repository {

    use chrono::{NaiveDate, Utc};
    use tempfile::NamedTempFile;

    use crate::domain::models::{
        activity::{ActivityStatistic, Sport, TimeseriesAggregate, TimeseriesMetric},
        training::{
            ActivityMetricSource, SportFilter, TrainingMetricAggregate, TrainingMetricFilters,
            TrainingMetricGranularity, TrainingNote, TrainingNoteContent, TrainingNoteId,
            TrainingNoteTitle, TrainingPeriod, TrainingPeriodId, TrainingPeriodSports,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_init_table() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        sqlx::query("select count(*) from t_training_metrics_definitions;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();

        sqlx::query("select count(*) from t_training_periods;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();

        sqlx::query("select count(*) from t_training_notes;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
    }

    fn build_metric() -> TrainingMetric {
        TrainingMetric::new(
            TrainingMetricId::new(),
            None,
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                UserId::test_default(),
                ActivityMetricSource::Timeseries((
                    TimeseriesMetric::Altitude,
                    TimeseriesAggregate::Max,
                )),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Max,
                TrainingMetricFilters::empty(),
                TrainingMetricGroupBy::none(),
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
                ActivityMetricSource::Timeseries((
                    TimeseriesMetric::Altitude,
                    TimeseriesAggregate::Max,
                )),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Max,
                TrainingMetricFilters::new(Some(vec![SportFilter::Sport(Sport::Running)])),
                TrainingMetricGroupBy::none(),
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
                ActivityMetricSource::Timeseries((
                    TimeseriesMetric::Altitude,
                    TimeseriesAggregate::Max,
                )),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Max,
                TrainingMetricFilters::new(Some(vec![SportFilter::Sport(Sport::Running)])),
                Some(TrainingMetricGroupBy::Sport),
            ),
        )
    }

    #[tokio::test]
    async fn test_save_training_metrics() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric();

        repository
            .save_training_metric_definition(definition)
            .await
            .expect("Should have return Ok");
    }

    #[tokio::test]
    async fn test_save_training_metrics_with_group_by() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition_with_group_by();

        repository
            .save_training_metric_definition(definition)
            .await
            .expect("Should have return Ok");

        assert_eq!(
            sqlx::query_scalar::<_, Option<TrainingMetricGroupBy>>(
                "select group_by from t_training_metrics_definitions limit 1;"
            )
            .fetch_one(&repository.pool)
            .await
            .unwrap(),
            Some(TrainingMetricGroupBy::Sport)
        );
    }

    #[tokio::test]
    async fn test_save_training_metrics_fails_if_duplicate() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric();

        repository
            .save_training_metric_definition(definition.clone())
            .await
            .expect("Should have return Ok");
        repository
            .save_training_metric_definition(definition)
            .await
            .expect_err("Should have return Err");
    }

    #[tokio::test]
    async fn test_get_definition() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let metric = build_metric();

        repository
            .save_training_metric_definition(metric.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_definition(metric.id())
            .await
            .expect("Should have returned OK")
            .expect("Should have returned Some");

        assert_eq!(res, *metric.definition());
    }

    #[tokio::test]
    async fn test_get_definition_with_filters() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let metric = build_metric_definition_with_filters();

        repository
            .save_training_metric_definition(metric.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_definition(metric.id())
            .await
            .expect("Should have returned OK")
            .expect("Should have returned Some");

        assert_eq!(res, *metric.definition());
    }

    #[tokio::test]
    async fn test_get_definition_with_group_by() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let metric = build_metric_definition_with_group_by();

        repository
            .save_training_metric_definition(metric.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_definition(metric.id())
            .await
            .expect("Should have returned OK")
            .expect("Should have returned Some");

        assert_eq!(res, *metric.definition());
    }

    #[tokio::test]
    async fn test_get_definition_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let res = repository
            .get_definition(&TrainingMetricId::new())
            .await
            .expect("Should have returned OK");
        assert!(res.is_none());
    }

    #[tokio::test]
    async fn test_get_definitions_for_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric();
        repository
            .save_training_metric_definition(definition.clone())
            .await
            .expect("Should have return Ok");
        let definition_with_filters = build_metric_definition_with_filters();
        repository
            .save_training_metric_definition(definition_with_filters.clone())
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
    async fn test_get_definitions_for_user_only() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric();
        repository
            .save_training_metric_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_global_metrics(&UserId::from("another_user".to_string()))
            .await
            .expect("Should have returned OK");

        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_get_definitions_with_filters() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition_with_filters();

        repository
            .save_training_metric_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_global_metrics(&UserId::test_default())
            .await
            .expect("Should have returned OK");

        assert_eq!(res, vec![definition]);
    }

    #[tokio::test]
    async fn test_get_definitions_with_group_by() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition_with_group_by();

        repository
            .save_training_metric_definition(definition.clone())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric();
        repository
            .save_training_metric_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        repository
            .delete_definition(definition.id())
            .await
            .expect("Should have returned OK");
    }

    #[tokio::test]
    async fn test_delete_definition_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let id = TrainingMetricId::new();
        let err = repository.delete_definition(&id).await;

        let Err(DeleteMetricError::TrainingMetricDoesNotExists(err_id)) = err else {
            unreachable!("Should have been an err")
        };
        assert_eq!(err_id, id);
    }

    #[tokio::test]
    async fn test_update_training_metric_name_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Create a metric
        let metric = build_metric();
        repository
            .save_training_metric_definition(metric.clone())
            .await
            .expect("Should save metric");

        // Update the name
        let new_name = TrainingMetricName::from("Updated Metric Name");
        let result = repository
            .update_training_metric_name(metric.id(), new_name.clone())
            .await;
        assert!(result.is_ok());

        // Verify the name was updated
        let fetched = repository.get_definition(metric.id()).await;
        assert!(fetched.is_ok());
        let fetched_def = fetched.unwrap();
        assert!(fetched_def.is_some());
        let definition = fetched_def.unwrap();

        // Verify other fields unchanged
        assert_eq!(definition.user(), metric.definition().user());
        assert_eq!(definition.source(), metric.definition().source());
        assert_eq!(definition.granularity(), metric.definition().granularity());
        assert_eq!(definition.aggregate(), metric.definition().aggregate());
    }

    #[tokio::test]
    async fn test_update_training_metric_name_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Try to update a non-existent metric
        let metric_id = TrainingMetricId::new();
        let result = repository
            .update_training_metric_name(&metric_id, TrainingMetricName::from("New Name"))
            .await;

        // Should succeed (no rows affected, but no error)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_training_metric_name_only_updates_specified_metric() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Create two metrics
        let metric1 = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Metric 1")),
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                UserId::test_default(),
                ActivityMetricSource::Timeseries((
                    TimeseriesMetric::Altitude,
                    TimeseriesAggregate::Max,
                )),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Max,
                TrainingMetricFilters::empty(),
                TrainingMetricGroupBy::none(),
            ),
        );
        let metric2 = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Metric 2")),
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                UserId::test_default(),
                ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                TrainingMetricGranularity::Weekly,
                TrainingMetricAggregate::Sum,
                TrainingMetricFilters::empty(),
                TrainingMetricGroupBy::none(),
            ),
        );

        repository
            .save_training_metric_definition(metric1.clone())
            .await
            .expect("Should save metric 1");
        repository
            .save_training_metric_definition(metric2.clone())
            .await
            .expect("Should save metric 2");

        // Update only metric1's name
        let new_name = TrainingMetricName::from("Updated First Metric");
        let result = repository
            .update_training_metric_name(metric1.id(), new_name.clone())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        .bind(TrainingMetricFilters::empty())
        .bind(TrainingMetricGroupBy::none())
        .bind::<Option<String>>(None)
        .execute(&repository.pool)
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

    #[tokio::test]
    async fn test_update_training_metric_scope_to_global() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Create a metric with a training period
        let period_id = TrainingPeriodId::new();
        let metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Test Metric")),
            TrainingMetricScope::TrainingPeriod(period_id.clone()),
            TrainingMetricDefinition::new(
                UserId::test_default(),
                ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Sum,
                TrainingMetricFilters::empty(),
                TrainingMetricGroupBy::none(),
            ),
        );

        repository
            .save_training_metric_definition(metric.clone())
            .await
            .expect("Should save metric");

        // Update scope to Global
        let result = repository
            .update_training_metric_scope(metric.id(), &TrainingMetricScope::Global)
            .await;
        assert!(result.is_ok());

        // Verify the scope was updated
        let metrics = repository
            .get_global_metrics(metric.definition().user())
            .await
            .expect("Should fetch metrics");

        let fetched_metric = metrics.iter().find(|m| m.id() == metric.id()).unwrap();
        assert_eq!(fetched_metric.scope(), &TrainingMetricScope::Global);
    }

    #[tokio::test]
    async fn test_update_training_metric_scope_to_training_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Create a metric with Global scope
        let metric = build_metric();
        repository
            .save_training_metric_definition(metric.clone())
            .await
            .expect("Should save metric");

        // Update scope to TrainingPeriod
        let period_id = TrainingPeriodId::new();
        let new_scope = TrainingMetricScope::TrainingPeriod(period_id.clone());
        let result = repository
            .update_training_metric_scope(metric.id(), &new_scope)
            .await;
        assert!(result.is_ok());

        // Verify the scope was updated
        let metrics = repository
            .get_period_metrics(metric.definition().user(), &period_id)
            .await
            .expect("Should fetch metrics");

        let fetched_metric = metrics.iter().find(|m| m.id() == metric.id()).unwrap();
        assert_eq!(fetched_metric.scope(), &new_scope);
    }

    #[tokio::test]
    async fn test_update_training_metric_scope_change_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Create a metric with one training period
        let period_id_1 = TrainingPeriodId::new();
        let metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Test Metric")),
            TrainingMetricScope::TrainingPeriod(period_id_1.clone()),
            TrainingMetricDefinition::new(
                UserId::test_default(),
                ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Sum,
                TrainingMetricFilters::empty(),
                TrainingMetricGroupBy::none(),
            ),
        );

        repository
            .save_training_metric_definition(metric.clone())
            .await
            .expect("Should save metric");

        // Update scope to a different training period
        let period_id_2 = TrainingPeriodId::new();
        let new_scope = TrainingMetricScope::TrainingPeriod(period_id_2.clone());
        let result = repository
            .update_training_metric_scope(metric.id(), &new_scope)
            .await;
        assert!(result.is_ok());

        // Verify the scope was updated to the new period
        let metrics = repository
            .get_period_metrics(metric.definition().user(), &period_id_2)
            .await
            .expect("Should fetch metrics");

        let fetched_metric = metrics.iter().find(|m| m.id() == metric.id()).unwrap();
        assert_eq!(fetched_metric.scope(), &new_scope);
        assert_ne!(
            fetched_metric.scope(),
            &TrainingMetricScope::TrainingPeriod(period_id_1)
        );
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
                .fetch_one(&repository.pool)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_get_training_period_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        assert!(
            repository
                .get_training_period(&&UserId::test_default(), &TrainingPeriodId::new())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_get_training_period_does_not_match_user() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
    async fn test_delete_training_period_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let result = repository.delete_training_period(period.id()).await;
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let period_id = TrainingPeriodId::new();

        // Delete non-existent period should succeed (DELETE is idempotent)
        let result = repository.delete_training_period(&period_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_training_period_only_deletes_specified_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let result = repository.delete_training_period(period1.id()).await;
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
            .update_training_period_name(period.id(), new_name.clone())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Try to update a non-existent period
        let period_id = TrainingPeriodId::new();
        let result = repository
            .update_training_period_name(&period_id, "New Name".to_string())
            .await;

        // Should succeed (no rows affected, but no error)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_training_period_name_only_updates_specified_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
            .update_training_period_name(period1.id(), new_name.clone())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
            .update_training_period_note(period.id(), new_note.clone())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
            .update_training_period_note(period.id(), None)
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Try to update a non-existent period
        let period_id = TrainingPeriodId::new();
        let result = repository
            .update_training_period_note(&period_id, Some("Note".to_string()))
            .await;

        // Should succeed (no rows affected, but no error)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_training_period_note_only_updates_specified_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
            .update_training_period_note(period1.id(), new_note.clone())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
            .update_training_period_dates(period.id(), new_start, new_end.clone())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
            .update_training_period_dates(period.id(), new_start, None)
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Create a period
        let period = build_training_period();
        let original_end = period.end().clone();
        repository
            .save_training_period(period.clone())
            .await
            .expect("Should save period");

        // Update only the start date, keeping the original end
        let new_start = "2025-10-15".parse::<NaiveDate>().unwrap();
        let result = repository
            .update_training_period_dates(period.id(), new_start, original_end.clone())
            .await;
        assert!(result.is_ok());

        // Verify only start was updated
        let fetched = repository
            .get_training_period(period.user(), period.id())
            .await;
        assert!(fetched.is_some());
        let fetched_period = fetched.unwrap();
        assert_eq!(fetched_period.start(), &new_start);
        assert_eq!(fetched_period.end(), &original_end);
    }

    #[tokio::test]
    async fn test_update_training_period_dates_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Try to update a non-existent period
        let period_id = TrainingPeriodId::new();
        let new_start = "2025-12-01".parse::<NaiveDate>().unwrap();
        let new_end = Some("2025-12-31".parse::<NaiveDate>().unwrap());
        let result = repository
            .update_training_period_dates(&period_id, new_start, new_end)
            .await;

        // Should succeed (no rows affected, but no error)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_training_period_dates_only_updates_specified_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
            .update_training_period_dates(period1.id(), new_start, new_end.clone())
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
    async fn test_save_training_note_without_period() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let note = build_training_note();
        repository
            .save_training_note(note.clone())
            .await
            .expect("Should save note");

        // Verify note was saved
        assert_eq!(
            sqlx::query_scalar::<_, i64>("select count(*) from t_training_notes where id = ?1")
                .bind(note.id().to_string())
                .fetch_one(&repository.pool)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_save_training_note_stores_all_fields() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let note = build_training_note();
        repository
            .save_training_note(note.clone())
            .await
            .expect("Should save note");

        // Verify all fields were stored correctly
        let (stored_id, stored_user, stored_content, _stored_created_at) =
            sqlx::query_as::<_, (String, String, String, String)>(
                "select id, user_id, content, created_at from t_training_notes where id = ?1",
            )
            .bind(note.id().to_string())
            .fetch_one(&repository.pool)
            .await
            .unwrap();

        assert_eq!(stored_id, note.id().to_string());
        assert_eq!(stored_user, note.user().to_string());
        assert_eq!(stored_content, note.content().to_string());
    }

    #[tokio::test]
    async fn test_save_training_note_duplicate_fails() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let note = build_training_note();

        // First save should succeed
        repository
            .save_training_note(note.clone())
            .await
            .expect("First save should succeed");

        // Second save with same ID should fail
        let result = repository.save_training_note(note.clone()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_training_note_returns_note() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let note = build_training_note();
        repository
            .save_training_note(note.clone())
            .await
            .expect("Should save note");

        let retrieved = repository
            .get_training_note(note.id())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let result = repository
            .get_training_note(&TrainingNoteId::new())
            .await
            .expect("Should not error");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_training_notes_returns_all_user_notes() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
            .get_training_notes(&user_id)
            .await
            .expect("Should retrieve notes");

        assert_eq!(notes.len(), 2);
        assert!(notes.iter().any(|n| n.id() == note1.id()));
        assert!(notes.iter().any(|n| n.id() == note2.id()));
    }

    #[tokio::test]
    async fn test_get_training_notes_returns_empty_when_no_notes() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let notes = repository
            .get_training_notes(&UserId::new())
            .await
            .expect("Should not error");

        assert_eq!(notes.len(), 0);
    }

    #[tokio::test]
    async fn test_get_training_notes_orders_by_created_at_desc() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
            .get_training_notes(&user_id)
            .await
            .expect("Should retrieve notes");

        // Newer note should come first
        assert_eq!(notes[0].id(), newer_note.id());
        assert_eq!(notes[1].id(), older_note.id());
    }

    #[tokio::test]
    async fn test_update_training_note_updates_content() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let note = build_training_note();
        repository
            .save_training_note(note.clone())
            .await
            .expect("Should save note");

        let new_title = Some(TrainingNoteTitle::from("Updated title"));
        let new_content = TrainingNoteContent::from("Updated content");
        let new_date = TrainingNoteDate::try_from("2025-01-15").unwrap();
        repository
            .update_training_note(
                note.id(),
                new_title.clone(),
                new_content.clone(),
                new_date.clone(),
            )
            .await
            .expect("Should update note");

        // Verify content, title, and date were updated
        let updated_note = repository
            .get_training_note(note.id())
            .await
            .expect("Should retrieve note")
            .expect("Note should exist");

        assert_eq!(updated_note.content(), &new_content);
        assert_eq!(updated_note.title(), &new_title);
        assert_eq!(updated_note.date(), &new_date);
        assert_eq!(updated_note.id(), note.id());
        assert_eq!(updated_note.user(), note.user());
    }

    #[tokio::test]
    async fn test_update_training_note_does_not_fail_when_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let result = repository
            .update_training_note(
                &TrainingNoteId::new(),
                None,
                TrainingNoteContent::from("Content"),
                TrainingNoteDate::today(),
            )
            .await;

        // Should not fail even if note doesn't exist
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_training_note_removes_note() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let note = build_training_note();
        repository
            .save_training_note(note.clone())
            .await
            .expect("Should save note");

        repository
            .delete_training_note(note.id())
            .await
            .expect("Should delete note");

        // Verify note was deleted
        let result = repository
            .get_training_note(note.id())
            .await
            .expect("Should not error");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_training_note_does_not_fail_when_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let result = repository
            .delete_training_note(&TrainingNoteId::new())
            .await;

        // Should not fail even if note doesn't exist
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_save_and_retrieve_metric_with_name() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let metric_with_name = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::new("My Custom Metric")),
            TrainingMetricScope::Global,
            build_metric().definition().clone(),
        );

        repository
            .save_training_metric_definition(metric_with_name.clone())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let metric_without_name = build_metric(); // None for name

        repository
            .save_training_metric_definition(metric_without_name.clone())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let user_id = UserId::test_default();
        let period_id = TrainingPeriodId::new();

        // Create a global metric
        let global_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Global Metric")),
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                user_id.clone(),
                ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                TrainingMetricGranularity::Weekly,
                TrainingMetricAggregate::Sum,
                TrainingMetricFilters::empty(),
                None,
            ),
        );

        // Create a period-scoped metric
        let period_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Period Metric")),
            TrainingMetricScope::TrainingPeriod(period_id.clone()),
            TrainingMetricDefinition::new(
                user_id.clone(),
                ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                TrainingMetricGranularity::Weekly,
                TrainingMetricAggregate::Sum,
                TrainingMetricFilters::empty(),
                None,
            ),
        );

        repository
            .save_training_metric_definition(global_metric.clone())
            .await
            .expect("Should save global metric");

        repository
            .save_training_metric_definition(period_metric.clone())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let user_id = UserId::test_default();
        let period_id = TrainingPeriodId::new();

        // Create a global metric
        let global_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Global Metric")),
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                user_id.clone(),
                ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                TrainingMetricGranularity::Weekly,
                TrainingMetricAggregate::Sum,
                TrainingMetricFilters::empty(),
                None,
            ),
        );

        // Create a period-scoped metric for our period
        let period_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Period Metric")),
            TrainingMetricScope::TrainingPeriod(period_id.clone()),
            TrainingMetricDefinition::new(
                user_id.clone(),
                ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                TrainingMetricGranularity::Weekly,
                TrainingMetricAggregate::Sum,
                TrainingMetricFilters::empty(),
                None,
            ),
        );

        // Create a period-scoped metric for a different period
        let other_period_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Other Period Metric")),
            TrainingMetricScope::TrainingPeriod(TrainingPeriodId::new()),
            TrainingMetricDefinition::new(
                user_id.clone(),
                ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                TrainingMetricGranularity::Weekly,
                TrainingMetricAggregate::Sum,
                TrainingMetricFilters::empty(),
                None,
            ),
        );

        repository
            .save_training_metric_definition(global_metric.clone())
            .await
            .expect("Should save global metric");

        repository
            .save_training_metric_definition(period_metric.clone())
            .await
            .expect("Should save period metric");

        repository
            .save_training_metric_definition(other_period_metric.clone())
            .await
            .expect("Should save other period metric");

        // Test: Filter by TrainingPeriod scope should return global metrics + metrics for that period
        let mut global_metrics = repository
            .get_global_metrics(&user_id)
            .await
            .expect("Should retrieve global metrics");

        let mut period_metrics = repository
            .get_period_metrics(&user_id, &period_id)
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");
        let user_id = UserId::test_default();
        let period_id = TrainingPeriodId::new();

        // Create a global metric
        let global_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Global Metric")),
            TrainingMetricScope::Global,
            TrainingMetricDefinition::new(
                user_id.clone(),
                ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                TrainingMetricGranularity::Weekly,
                TrainingMetricAggregate::Sum,
                TrainingMetricFilters::empty(),
                None,
            ),
        );

        // Create a period-scoped metric
        let period_metric = TrainingMetric::new(
            TrainingMetricId::new(),
            Some(TrainingMetricName::from("Period Metric")),
            TrainingMetricScope::TrainingPeriod(period_id.clone()),
            TrainingMetricDefinition::new(
                user_id.clone(),
                ActivityMetricSource::Statistic(ActivityStatistic::Distance),
                TrainingMetricGranularity::Weekly,
                TrainingMetricAggregate::Sum,
                TrainingMetricFilters::empty(),
                None,
            ),
        );

        repository
            .save_training_metric_definition(global_metric.clone())
            .await
            .expect("Should save global metric");

        repository
            .save_training_metric_definition(period_metric.clone())
            .await
            .expect("Should save period metric");

        // Test: We can fetch both global and period metrics separately
        let global_metrics = repository
            .get_global_metrics(&user_id)
            .await
            .expect("Should retrieve global metrics");

        let period_metrics = repository
            .get_period_metrics(&user_id, &period_id)
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
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
}
