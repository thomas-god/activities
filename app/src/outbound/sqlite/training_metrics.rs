use std::{collections::HashMap, str::FromStr};

use anyhow::anyhow;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

use crate::domain::{
    models::{
        UserId,
        training_metrics::{
            ActivityMetricSource, TrainingMetricAggregate, TrainingMetricDefinition,
            TrainingMetricGranularity, TrainingMetricId, TrainingMetricValue, TrainingMetricValues,
        },
    },
    ports::{
        DeleteMetricError, GetDefinitionError, GetTrainingMetricValueError,
        GetTrainingMetricsDefinitionsError, SaveTrainingMetricError, TrainingMetricsRepository,
        UpdateMetricError,
    },
};

type DefinitionRow = (
    TrainingMetricId,
    UserId,
    ActivityMetricSource,
    TrainingMetricGranularity,
    TrainingMetricAggregate,
);

#[derive(Debug, Clone)]
pub struct SqliteTrainingMetricsRepository {
    pool: SqlitePool,
}

impl SqliteTrainingMetricsRepository {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migration here for now
        sqlx::query(
            r#"
        CREATE TABLE IF NOT EXISTS t_training_metrics_definitions (
            id TEXT UNIQUE,
            user_id TEXT,
            source BLOB,
            granularity TEXT,
            aggregate TEXT
        );

        CREATE TABLE IF NOT EXISTS t_training_metrics_values (
            definition_id TEXT,
            granule TEXT,
            value BLOB,
            FOREIGN KEY(definition_id)
                REFERENCES t_training_metrics_definitions(id)
                ON DELETE CASCADE,
            CONSTRAINT
                t_training_metrics_values_unique_id_granule
                UNIQUE(definition_id, granule)
                ON CONFLICT REPLACE
        );"#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }
}

impl TrainingMetricsRepository for SqliteTrainingMetricsRepository {
    async fn save_definition(
        &self,
        definition: TrainingMetricDefinition,
    ) -> Result<(), SaveTrainingMetricError> {
        sqlx::query("INSERT INTO t_training_metrics_definitions VALUES (?1, ?2, ?3, ?4, ?5);")
            .bind(definition.id())
            .bind(definition.user())
            .bind(definition.source())
            .bind(definition.granularity())
            .bind(definition.aggregate())
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
        SELECT id, user_id, source, granularity, aggregate
        FROM t_training_metrics_definitions
        WHERE id = ?1 LIMIT 1;",
        )
        .bind(metric)
        .fetch_one(&self.pool)
        .await
        {
            Ok((id, user_id, source, granularity, aggregate)) => Ok(Some(
                TrainingMetricDefinition::new(id, user_id, source, granularity, aggregate),
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
            "SELECT id, user_id, source, granularity, aggregate
            FROM t_training_metrics_definitions
            WHERE user_id = ?1;",
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(|err| GetTrainingMetricsDefinitionsError::Unknown(anyhow!(err)))
        .map(|rows| {
            rows.into_iter()
                .map(|(id, user_id, source, granularity, aggregate)| {
                    TrainingMetricDefinition::new(id, user_id, source, granularity, aggregate)
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
}

#[cfg(test)]
mod test_sqlite_activity_repository {

    use std::collections::HashMap;

    use tempfile::NamedTempFile;

    use crate::domain::models::{
        activity::TimeseriesMetric,
        training_metrics::{
            ActivityMetricSource, TimeseriesAggregate, TrainingMetricAggregate,
            TrainingMetricGranularity,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_init_table() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        )
    }

    #[tokio::test]
    async fn test_save_training_metrics() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
    async fn test_get_definition_not_found() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");
        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_definitions(&UserId::test_default())
            .await
            .expect("Should have returned OK");

        assert_eq!(res.len(), 2);
    }

    #[tokio::test]
    async fn test_get_definitions_for_user_only() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let id = TrainingMetricId::new();
        let err = repository.delete_definition(&id).await;

        dbg!(&err);

        let Err(DeleteMetricError::TrainingMetricDoesNotExists(err_id)) = err else {
            unreachable!("Should have been an err")
        };
        assert_eq!(err_id, id);
    }

    #[tokio::test]
    async fn test_delete_definition_with_values_cascade() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
        let repository = SqliteTrainingMetricsRepository::new(&db_file.path().to_string_lossy())
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
}
