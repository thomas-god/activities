use std::{collections::HashMap, str::FromStr};

use anyhow::anyhow;
use chrono::NaiveDate;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

use crate::domain::{
    models::{
        UserId,
        training::{
            ActivityMetricSource, TrainingMetricAggregate, TrainingMetricDefinition,
            TrainingMetricFilters, TrainingMetricGranularity, TrainingMetricId,
            TrainingMetricValue, TrainingMetricValues, TrainingPeriod, TrainingPeriodId,
            TrainingPeriodSports,
        },
    },
    ports::{
        DeleteMetricError, GetDefinitionError, GetTrainingMetricValueError,
        GetTrainingMetricsDefinitionsError, SaveTrainingMetricError, SaveTrainingPeriodError,
        TrainingRepository, UpdateMetricError,
    },
};

type DefinitionRow = (
    TrainingMetricId,
    UserId,
    ActivityMetricSource,
    TrainingMetricGranularity,
    TrainingMetricAggregate,
    TrainingMetricFilters,
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
    async fn save_definition(
        &self,
        definition: TrainingMetricDefinition,
    ) -> Result<(), SaveTrainingMetricError> {
        sqlx::query("INSERT INTO t_training_metrics_definitions VALUES (?1, ?2, ?3, ?4, ?5, ?6);")
            .bind(definition.id())
            .bind(definition.user())
            .bind(definition.source())
            .bind(definition.granularity())
            .bind(definition.aggregate())
            .bind(definition.filters())
            .execute(&self.pool)
            .await
            .map_err(|err| SaveTrainingMetricError::Unknown(anyhow!(err)))
            .map(|_| ())
    }

    async fn get_definition(
        &self,
        metric: &TrainingMetricId,
    ) -> Result<Option<TrainingMetricDefinition>, GetDefinitionError> {
        match sqlx::query_as::<_, DefinitionRow>(
            "
        SELECT id, user_id, source, granularity, aggregate, filters
        FROM t_training_metrics_definitions
        WHERE id = ?1 LIMIT 1;",
        )
        .bind(metric)
        .fetch_one(&self.pool)
        .await
        {
            Ok((id, user_id, source, granularity, aggregate, filters)) => Ok(Some(
                TrainingMetricDefinition::new(id, user_id, source, granularity, aggregate, filters),
            )),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(GetDefinitionError::Unknown(anyhow!(err))),
        }
    }

    async fn get_definitions(
        &self,
        user: &UserId,
    ) -> Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError> {
        sqlx::query_as::<_, DefinitionRow>(
            "SELECT id, user_id, source, granularity, aggregate, filters
            FROM t_training_metrics_definitions
            WHERE user_id = ?1;",
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(|err| GetTrainingMetricsDefinitionsError::Unknown(anyhow!(err)))
        .map(|rows| {
            rows.into_iter()
                .map(|(id, user_id, source, granularity, aggregate, filters)| {
                    TrainingMetricDefinition::new(
                        id,
                        user_id,
                        source,
                        granularity,
                        aggregate,
                        filters,
                    )
                })
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

    async fn update_metric_values(
        &self,
        id: &TrainingMetricId,
        values: (String, TrainingMetricValue),
    ) -> Result<(), UpdateMetricError> {
        sqlx::query("INSERT INTO t_training_metrics_values VALUES (?1, ?2, ?3);")
            .bind(id)
            .bind(values.0)
            .bind(values.1)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|err| match err {
                sqlx::Error::Database(err) if err.is_foreign_key_violation() => {
                    UpdateMetricError::TrainingMetricDoesNotExists(id.clone())
                }
                err => UpdateMetricError::Unknown(anyhow!(err)),
            })
    }

    async fn get_metric_value(
        &self,
        id: &TrainingMetricId,
        bin_key: &str,
    ) -> Result<Option<TrainingMetricValue>, GetTrainingMetricValueError> {
        match sqlx::query_as::<_, (TrainingMetricValue,)>(
            "SELECT value FROM t_training_metrics_values
            WHERE definition_id = ?1 AND granule = ?2;",
        )
        .bind(id)
        .bind(bin_key)
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => Ok(Some(row.0)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(GetTrainingMetricValueError::Unknown(anyhow!(err))),
        }
    }

    async fn get_metric_values(
        &self,
        id: &TrainingMetricId,
    ) -> Result<TrainingMetricValues, GetTrainingMetricValueError> {
        sqlx::query_as::<_, (String, TrainingMetricValue)>(
            "
        SELECT granule, value FROM t_training_metrics_values
        WHERE definition_id = ?1;",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|err| GetTrainingMetricValueError::Unknown(anyhow!(err)))
        .map(|rows| TrainingMetricValues::new(HashMap::from_iter(rows)))
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
}

#[cfg(test)]
mod test_sqlite_training_repository {

    use std::collections::HashMap;

    use chrono::NaiveDate;
    use tempfile::NamedTempFile;

    use crate::domain::models::{
        activity::{Sport, TimeseriesMetric},
        training::{
            ActivityMetricSource, SportFilter, TimeseriesAggregate, TrainingMetricAggregate,
            TrainingMetricFilters, TrainingMetricGranularity, TrainingPeriod, TrainingPeriodId,
            TrainingPeriodSports,
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

        sqlx::query("select count(*) from t_training_metrics_values;")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
    }

    fn build_metric_definition() -> TrainingMetricDefinition {
        TrainingMetricDefinition::new(
            TrainingMetricId::new(),
            UserId::test_default(),
            ActivityMetricSource::Timeseries((
                TimeseriesMetric::Altitude,
                TimeseriesAggregate::Max,
            )),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Max,
            TrainingMetricFilters::empty(),
        )
    }

    fn build_metric_definition_with_filters() -> TrainingMetricDefinition {
        TrainingMetricDefinition::new(
            TrainingMetricId::new(),
            UserId::test_default(),
            ActivityMetricSource::Timeseries((
                TimeseriesMetric::Altitude,
                TimeseriesAggregate::Max,
            )),
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Max,
            TrainingMetricFilters::new(Some(vec![SportFilter::Sport(Sport::Running)])),
        )
    }

    #[tokio::test]
    async fn test_save_training_metrics() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();

        repository
            .save_definition(definition)
            .await
            .expect("Should have return Ok");
    }

    #[tokio::test]
    async fn test_save_training_metrics_fails_if_duplicate() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();

        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");
        repository
            .save_definition(definition)
            .await
            .expect_err("Should have return Err");
    }

    #[tokio::test]
    async fn test_get_definition() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();

        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_definition(definition.id())
            .await
            .expect("Should have returned OK")
            .expect("Should have returned Some");

        assert_eq!(res, definition);
    }

    #[tokio::test]
    async fn test_get_definition_with_filters() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition_with_filters();

        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_definition(definition.id())
            .await
            .expect("Should have returned OK")
            .expect("Should have returned Some");

        assert_eq!(res, definition);
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

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");
        let definition_with_filters = build_metric_definition_with_filters();
        repository
            .save_definition(definition_with_filters.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_definitions(&UserId::test_default())
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

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_definitions(&UserId::from("another_user".to_string()))
            .await
            .expect("Should have returned OK");

        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_delete_definition_ok() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
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
    async fn test_delete_definition_with_values_cascade() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        // Create definition
        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        // Insert a value for this definition
        let new_value = ("2025-09-24".to_string(), TrainingMetricValue::Max(12.3));
        repository
            .update_metric_values(definition.id(), new_value)
            .await
            .expect("Should have return an err");
        assert_eq!(
            sqlx::query_scalar::<_, u64>(
                "
            select count(value)
            from t_training_metrics_values
            where definition_id = ?1;"
            )
            .bind(definition.id())
            .fetch_one(&repository.pool)
            .await
            .unwrap(),
            1
        );

        // Delete the parent definition: should succeed and delete the inserted value
        repository
            .delete_definition(definition.id())
            .await
            .expect("Should have returned OK");
        assert_eq!(
            sqlx::query_scalar::<_, u64>(
                "
            select count(value)
            from t_training_metrics_values
            where definition_id = ?1;"
            )
            .bind(definition.id())
            .fetch_one(&repository.pool)
            .await
            .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn test_insert_value() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let new_value = ("2025-09-24".to_string(), TrainingMetricValue::Max(12.3));

        repository
            .update_metric_values(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        assert_eq!(
            sqlx::query_scalar::<_, TrainingMetricValue>(
                "
            select value
            from t_training_metrics_values
            where definition_id = ?1 and granule = ?2;"
            )
            .bind(definition.id())
            .bind("2025-09-24")
            .fetch_one(&repository.pool)
            .await
            .unwrap(),
            TrainingMetricValue::Max(12.3)
        );
    }

    #[tokio::test]
    async fn test_insert_value_already_exist_replace() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let new_value = ("2025-09-24".to_string(), TrainingMetricValue::Max(12.3));
        repository
            .update_metric_values(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        let new_value = ("2025-09-24".to_string(), TrainingMetricValue::Max(1342.8));
        repository
            .update_metric_values(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        assert_eq!(
            sqlx::query_scalar::<_, TrainingMetricValue>(
                "
            select value
            from t_training_metrics_values
            where definition_id = ?1 and granule = ?2;"
            )
            .bind(definition.id())
            .bind("2025-09-24")
            .fetch_one(&repository.pool)
            .await
            .unwrap(),
            TrainingMetricValue::Max(1342.8)
        );
    }

    #[tokio::test]
    async fn test_insert_value_definition_does_not_exist() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let new_value = ("2025-09-24".to_string(), TrainingMetricValue::Max(12.3));
        let id = TrainingMetricId::new();
        let err = repository.update_metric_values(&id, new_value).await;

        let Err(UpdateMetricError::TrainingMetricDoesNotExists(err_id)) = err else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(err_id, id);
    }

    #[tokio::test]
    async fn test_metric_values() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let new_value = ("2025-09-24".to_string(), TrainingMetricValue::Max(12.3));
        repository
            .update_metric_values(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        let new_value = ("2025-09-25".to_string(), TrainingMetricValue::Max(10.1));
        repository
            .update_metric_values(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        assert_eq!(
            repository
                .get_metric_values(definition.id())
                .await
                .expect("Should have returned OK")
                .as_hash_map(),
            HashMap::from_iter(vec![
                ("2025-09-24".to_string(), TrainingMetricValue::Max(12.3)),
                ("2025-09-25".to_string(), TrainingMetricValue::Max(10.1))
            ])
        );
    }

    #[tokio::test]
    async fn test_get_metric_value() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let new_value = ("2025-09-24".to_string(), TrainingMetricValue::Max(12.3));
        repository
            .update_metric_values(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        assert_eq!(
            repository
                .get_metric_value(definition.id(), "2025-09-24")
                .await
                .expect("Should have returned OK")
                .expect("Should have returned Some"),
            TrainingMetricValue::Max(12.3)
        );
    }

    #[tokio::test]
    async fn test_get_metric_value_does_not_exist_for_bin() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let new_value = ("2025-09-24".to_string(), TrainingMetricValue::Max(12.3));
        repository
            .update_metric_values(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        assert!(
            repository
                .get_metric_value(definition.id(), "2025-09-01")
                .await
                .expect("Should have returned OK")
                .is_none()
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
}
