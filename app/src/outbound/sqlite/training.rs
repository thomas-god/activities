use std::{collections::HashMap, str::FromStr};

use anyhow::anyhow;
use chrono::NaiveDate;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

use crate::domain::{
    models::{
        UserId,
        training::{
            ActivityMetricSource, TrainingMetricAggregate, TrainingMetricBin,
            TrainingMetricDefinition, TrainingMetricFilters, TrainingMetricGranularity,
            TrainingMetricGroupBy, TrainingMetricId, TrainingMetricValue, TrainingMetricValues,
            TrainingPeriod, TrainingPeriodId, TrainingPeriodSports,
        },
    },
    ports::{
        DateRange, DeleteMetricError, GetDefinitionError, GetTrainingMetricValueError,
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
    Option<TrainingMetricGroupBy>,
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

const NONE_GROUP: &str = "no_group";

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
        sqlx::query(
            "INSERT INTO t_training_metrics_definitions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
        )
        .bind(definition.id())
        .bind(definition.user())
        .bind(definition.source())
        .bind(definition.granularity())
        .bind(definition.aggregate())
        .bind(definition.filters())
        .bind(definition.group_by())
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
        SELECT id, user_id, source, granularity, aggregate, filters, group_by
        FROM t_training_metrics_definitions
        WHERE id = ?1 LIMIT 1;",
        )
        .bind(metric)
        .fetch_one(&self.pool)
        .await
        {
            Ok((id, user_id, source, granularity, aggregate, filters, group_by)) => {
                Ok(Some(TrainingMetricDefinition::new(
                    id,
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

    async fn get_definitions(
        &self,
        user: &UserId,
    ) -> Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError> {
        sqlx::query_as::<_, DefinitionRow>(
            "SELECT id, user_id, source, granularity, aggregate, filters, group_by
            FROM t_training_metrics_definitions
            WHERE user_id = ?1;",
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(|err| GetTrainingMetricsDefinitionsError::Unknown(anyhow!(err)))
        .map(|rows| {
            rows.into_iter()
                .map(
                    |(id, user_id, source, granularity, aggregate, filters, group_by)| {
                        TrainingMetricDefinition::new(
                            id,
                            user_id,
                            source,
                            granularity,
                            aggregate,
                            filters,
                            group_by,
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

    async fn update_metric_value(
        &self,
        id: &TrainingMetricId,
        value: (TrainingMetricBin, TrainingMetricValue),
    ) -> Result<(), UpdateMetricError> {
        let query = sqlx::query("INSERT INTO t_training_metrics_values VALUES (?1, ?2, ?3, ?4);")
            .bind(id)
            .bind(value.0.granule())
            .bind(value.1)
            .bind(value.0.group().clone().unwrap_or(NONE_GROUP.to_string()));

        query
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
        bin_key: &TrainingMetricBin,
    ) -> Result<Option<TrainingMetricValue>, GetTrainingMetricValueError> {
        match sqlx::query_as::<_, (TrainingMetricValue,)>(
            "SELECT value FROM t_training_metrics_values
            WHERE definition_id = ?1 AND granule = ?2 AND bin_group = ?3;",
        )
        .bind(id)
        .bind(bin_key.granule())
        .bind(bin_key.group().clone().unwrap_or(NONE_GROUP.to_string()))
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
        date_range: &Option<DateRange>,
    ) -> Result<TrainingMetricValues, GetTrainingMetricValueError> {
        let mut builder = sqlx::QueryBuilder::<'_, sqlx::Sqlite>::new(
            "SELECT granule, bin_group, value
            FROM t_training_metrics_values
            WHERE definition_id = ",
        );
        builder.push_bind(id);

        if let Some(range) = date_range {
            builder.push(" AND granule >= ").push_bind(range.start());

            builder.push(" AND granule <= ").push_bind(range.end());
        }

        let query = builder.build_query_as::<'_, (String, Option<String>, TrainingMetricValue)>();

        query
            .fetch_all(&self.pool)
            .await
            .map_err(|err| GetTrainingMetricValueError::Unknown(anyhow!(err)))
            .map(|rows| {
                TrainingMetricValues::new(HashMap::from_iter(rows.into_iter().map(
                    |(granule, bin_group, value)| {
                        (
                            TrainingMetricBin::new(
                                granule,
                                bin_group.and_then(|group| {
                                    if group == NONE_GROUP {
                                        None
                                    } else {
                                        Some(group)
                                    }
                                }),
                            ),
                            value,
                        )
                    },
                )))
            })
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
            TrainingMetricGroupBy::none(),
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
            TrainingMetricGroupBy::none(),
        )
    }

    fn build_metric_definition_with_group_by() -> TrainingMetricDefinition {
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
            Some(TrainingMetricGroupBy::Sport),
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
    async fn test_save_training_metrics_with_group_by() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition_with_group_by();

        repository
            .save_definition(definition)
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
    async fn test_get_definition_with_group_by() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition_with_group_by();

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
    async fn test_get_definitions_with_filters() {
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
            .get_definitions(&UserId::test_default())
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
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let res = repository
            .get_definitions(&UserId::test_default())
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
        let new_value = (
            TrainingMetricBin::from_granule("2025-09-24"),
            TrainingMetricValue::Max(12.3),
        );
        repository
            .update_metric_value(definition.id(), new_value)
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

        let new_value = (
            TrainingMetricBin::from_granule("2025-09-24"),
            TrainingMetricValue::Max(12.3),
        );

        repository
            .update_metric_value(definition.id(), new_value)
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

        let initial_value = (
            TrainingMetricBin::new("2025-09-24".to_string(), None),
            TrainingMetricValue::Max(12.3),
        );
        repository
            .update_metric_value(definition.id(), initial_value)
            .await
            .expect("Should have return an err");

        let new_value = (
            TrainingMetricBin::new("2025-09-24".to_string(), None),
            TrainingMetricValue::Max(1342.8),
        );
        repository
            .update_metric_value(definition.id(), new_value)
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

        let new_value = (
            TrainingMetricBin::from_granule("2025-09-24"),
            TrainingMetricValue::Max(12.3),
        );
        let id = TrainingMetricId::new();
        let err = repository.update_metric_value(&id, new_value).await;

        let Err(UpdateMetricError::TrainingMetricDoesNotExists(err_id)) = err else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(err_id, id);
    }

    #[tokio::test]
    async fn test_insert_value_with_group() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition_with_group_by();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let new_value = (
            TrainingMetricBin::new("2025-09-24".to_string(), Some("Cycling".to_string())),
            TrainingMetricValue::Max(12.3),
        );

        repository
            .update_metric_value(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        assert_eq!(
            sqlx::query_scalar::<_, TrainingMetricValue>(
                "
            select value
            from t_training_metrics_values
            where definition_id = ?1 and granule = ?2 and bin_group = ?3;"
            )
            .bind(definition.id())
            .bind("2025-09-24")
            .bind("Cycling")
            .fetch_one(&repository.pool)
            .await
            .unwrap(),
            TrainingMetricValue::Max(12.3)
        );
    }

    #[tokio::test]
    async fn test_upsert_value_with_group() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition_with_group_by();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let initial_value = (
            TrainingMetricBin::new("2025-09-24".to_string(), Some("Cycling".to_string())),
            TrainingMetricValue::Max(12.3),
        );

        repository
            .update_metric_value(definition.id(), initial_value)
            .await
            .expect("Should have return an err");

        let new_value = (
            TrainingMetricBin::new("2025-09-24".to_string(), Some("Cycling".to_string())),
            TrainingMetricValue::Max(45.6),
        );

        repository
            .update_metric_value(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        assert_eq!(
            sqlx::query_scalar::<_, TrainingMetricValue>(
                "
            select value
            from t_training_metrics_values
            where definition_id = ?1 and granule = ?2 and bin_group = ?3;"
            )
            .bind(definition.id())
            .bind("2025-09-24")
            .bind("Cycling")
            .fetch_one(&repository.pool)
            .await
            .unwrap(),
            TrainingMetricValue::Max(45.6)
        );
    }

    #[tokio::test]
    async fn test_get_metric_values() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let new_value = (
            TrainingMetricBin::new("2025-09-24".to_string(), None),
            TrainingMetricValue::Max(12.3),
        );
        repository
            .update_metric_value(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        let new_value = (
            TrainingMetricBin::new("2025-09-25".to_string(), None),
            TrainingMetricValue::Max(10.1),
        );
        repository
            .update_metric_value(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        assert_eq!(
            repository
                .get_metric_values(definition.id(), &None)
                .await
                .expect("Should have returned OK")
                .as_hash_map(),
            HashMap::from_iter(vec![
                (
                    TrainingMetricBin::new("2025-09-24".to_string(), None),
                    TrainingMetricValue::Max(12.3)
                ),
                (
                    TrainingMetricBin::new("2025-09-25".to_string(), None),
                    TrainingMetricValue::Max(10.1)
                )
            ])
        );
    }

    #[tokio::test]
    async fn test_get_metric_values_with_group() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let new_value = (
            TrainingMetricBin::new("2025-09-24".to_string(), None),
            TrainingMetricValue::Max(12.3),
        );
        repository
            .update_metric_value(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        let new_value = (
            TrainingMetricBin::new("2025-09-24".to_string(), Some("Cycling".to_string())),
            TrainingMetricValue::Max(10.1),
        );
        repository
            .update_metric_value(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        assert_eq!(
            repository
                .get_metric_values(definition.id(), &None)
                .await
                .expect("Should have returned OK")
                .as_hash_map(),
            HashMap::from_iter(vec![
                (
                    TrainingMetricBin::new("2025-09-24".to_string(), None),
                    TrainingMetricValue::Max(12.3)
                ),
                (
                    TrainingMetricBin::new("2025-09-24".to_string(), Some("Cycling".to_string())),
                    TrainingMetricValue::Max(10.1)
                )
            ])
        );
    }

    #[tokio::test]
    async fn test_get_metric_values_with_date_range() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        // Add values for multiple dates
        let dates_values = vec![
            ("2025-09-20", 10.0),
            ("2025-09-24", 12.3),
            ("2025-09-25", 10.1),
            ("2025-09-28", 15.5),
        ];

        for (date, value) in dates_values {
            repository
                .update_metric_value(
                    definition.id(),
                    (
                        TrainingMetricBin::new(date.to_string(), None),
                        TrainingMetricValue::Max(value),
                    ),
                )
                .await
                .expect("Should have saved value");
        }

        // Test with date range filter (2025-09-24 to 2025-09-25)
        let date_range = DateRange::new(
            chrono::NaiveDate::from_ymd_opt(2025, 9, 24).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2025, 9, 25).unwrap(),
        );

        let values = repository
            .get_metric_values(definition.id(), &Some(date_range))
            .await
            .expect("Should have returned OK");

        // Convert to HashMap once
        let values_map = values.as_hash_map();

        // Should only return values within the date range
        assert_eq!(values_map.len(), 2);
        assert_eq!(
            values_map.get(&TrainingMetricBin::new("2025-09-24".to_string(), None)),
            Some(&TrainingMetricValue::Max(12.3))
        );
        assert_eq!(
            values_map.get(&TrainingMetricBin::new("2025-09-25".to_string(), None)),
            Some(&TrainingMetricValue::Max(10.1))
        );

        // Values outside range should not be included
        assert_eq!(
            values_map.get(&TrainingMetricBin::new("2025-09-20".to_string(), None)),
            None
        );
        assert_eq!(
            values_map.get(&TrainingMetricBin::new("2025-09-28".to_string(), None)),
            None
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

        let new_value = (
            TrainingMetricBin::new("2025-09-24".to_string(), None),
            TrainingMetricValue::Max(12.3),
        );
        repository
            .update_metric_value(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        assert_eq!(
            repository
                .get_metric_value(
                    definition.id(),
                    &TrainingMetricBin::new("2025-09-24".to_string(), None),
                )
                .await
                .expect("Should have returned OK")
                .expect("Should have returned Some"),
            TrainingMetricValue::Max(12.3)
        );
    }

    #[tokio::test]
    async fn test_get_metric_value_with_group() {
        let db_file = NamedTempFile::new().unwrap();
        let repository = SqliteTrainingRepository::new(&db_file.path().to_string_lossy())
            .await
            .expect("repo should init");

        let definition = build_metric_definition();
        repository
            .save_definition(definition.clone())
            .await
            .expect("Should have return Ok");

        let value_different_group = (
            TrainingMetricBin::new("2025-09-24".to_string(), Some("Cycling".to_string())),
            TrainingMetricValue::Max(45.6),
        );
        repository
            .update_metric_value(definition.id(), value_different_group)
            .await
            .expect("Should have return an err");

        let value = (
            TrainingMetricBin::new("2025-09-24".to_string(), Some("Running".to_string())),
            TrainingMetricValue::Max(12.3),
        );
        repository
            .update_metric_value(definition.id(), value)
            .await
            .expect("Should have return an err");

        assert_eq!(
            repository
                .get_metric_value(
                    definition.id(),
                    &TrainingMetricBin::new("2025-09-24".to_string(), Some("Running".to_string())),
                )
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

        let new_value = (
            TrainingMetricBin::from_granule("2025-09-24"),
            TrainingMetricValue::Max(12.3),
        );
        repository
            .update_metric_value(definition.id(), new_value)
            .await
            .expect("Should have return an err");

        assert!(
            repository
                .get_metric_value(
                    definition.id(),
                    &TrainingMetricBin::from_granule("2025-09-01"),
                )
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
}
