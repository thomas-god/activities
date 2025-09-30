use std::sync::Arc;

use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::domain::{
    models::{
        UserId,
        training_metrics::{TrainingMetricDefinition, TrainingMetricId, TrainingMetricValues},
    },
    ports::{
        ActivityRepository, CreateTrainingMetricError, CreateTrainingMetricRequest,
        DeleteTrainingMetricError, DeleteTrainingMetricRequest, ITrainingMetricService,
        ListActivitiesFilters, TrainingMetricsRepository, UpdateMetricsValuesRequest,
    },
};

///////////////////////////////////////////////////////////////////
/// TRAINING METRICS SERVICE
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Constructor)]
pub struct TrainingMetricService<TMR, AR>
where
    TMR: TrainingMetricsRepository,
    AR: ActivityRepository,
{
    metrics_repository: TMR,
    activity_repository: Arc<Mutex<AR>>,
}

impl<TMR, AR> ITrainingMetricService for TrainingMetricService<TMR, AR>
where
    TMR: TrainingMetricsRepository,
    AR: ActivityRepository,
{
    async fn create_metric(
        &self,
        req: CreateTrainingMetricRequest,
    ) -> Result<TrainingMetricId, CreateTrainingMetricError> {
        let id = TrainingMetricId::new();
        let metric_definition = TrainingMetricDefinition::new(
            id.clone(),
            req.user().clone(),
            req.source().clone(),
            req.granularity().clone(),
            req.aggregate().clone(),
        );
        self.metrics_repository
            .save_definitions(metric_definition)
            .await?;

        let _ = self
            .update_metrics_values(UpdateMetricsValuesRequest::new(
                req.user().clone(),
                Vec::new(),
            ))
            .await;

        Ok(id)
    }

    async fn update_metrics_values(&self, req: UpdateMetricsValuesRequest) -> Result<(), ()> {
        let metrics = self
            .metrics_repository
            .get_definitions(req.user())
            .await
            .unwrap();

        let activities = self
            .activity_repository
            .lock()
            .await
            .list_activities_with_timeseries(req.user(), &ListActivitiesFilters::empty())
            .await
            .unwrap();

        for metric in metrics {
            let values = metric.compute_values(&activities);
            tracing::info!("New value {:?} for metric {:?}", values, metric);
            for (key, value) in values.iter() {
                let _ = self
                    .metrics_repository
                    .update_metric_values(metric.id(), (key.clone(), *value))
                    .await;
            }
        }
        Ok(())
    }

    async fn get_training_metrics(
        &self,
        user: &UserId,
    ) -> Vec<(TrainingMetricDefinition, TrainingMetricValues)> {
        let Ok(definitions) = self.metrics_repository.get_definitions(user).await else {
            return vec![];
        };

        let mut res = vec![];
        for definition in definitions {
            let values = self
                .metrics_repository
                .get_metric_values(definition.id())
                .await
                .unwrap_or_default();
            res.push((definition.clone(), values.clone()))
        }

        res
    }

    async fn delete_metric(
        &self,
        req: DeleteTrainingMetricRequest,
    ) -> Result<(), DeleteTrainingMetricError> {
        let Some(definition) = self.metrics_repository.get_definition(req.metric()).await? else {
            return Err(DeleteTrainingMetricError::MetricDoesNotExist(
                req.metric().clone(),
            ));
        };

        if definition.user() != req.user() {
            return Err(DeleteTrainingMetricError::UserDoesNotOwnTrainingMetric(
                req.user().clone(),
                req.metric().clone(),
            ));
        }

        self.metrics_repository
            .delete_definition(req.metric())
            .await?;

        Ok(())
    }
}

///////////////////////////////////////////////////////////////////
// MOCK IMPLEMENTATIONS FOR TESTING
///////////////////////////////////////////////////////////////////

#[cfg(test)]
pub mod test_utils {

    use mockall::mock;

    use crate::domain::ports::{
        DeleteMetricError, GetDefinitionError, GetTrainingMetricValueError,
        GetTrainingMetricsDefinitionsError, SaveTrainingMetricError, UpdateMetricError,
    };

    use super::*;

    mock! {
        pub TrainingMetricService {}

        impl Clone for TrainingMetricService {
            fn clone(&self) -> Self;
        }

        impl ITrainingMetricService for TrainingMetricService {

            async fn create_metric(
                &self,
                req: CreateTrainingMetricRequest
            ) -> Result<TrainingMetricId, CreateTrainingMetricError>;

            async fn update_metrics_values(
                &self,
                req: UpdateMetricsValuesRequest,
            ) -> Result<(), ()>;

            async fn get_training_metrics(
                &self,
                user: &UserId,
            ) -> Vec<(TrainingMetricDefinition, TrainingMetricValues)>;

            async fn delete_metric(
                &self,
                req: DeleteTrainingMetricRequest,
            ) -> Result<(), DeleteTrainingMetricError>;
        }
    }

    impl MockTrainingMetricService {
        pub fn test_default() -> Self {
            let mut mock = Self::new();

            mock.expect_create_metric()
                .returning(|_| Ok(TrainingMetricId::default()));
            mock.expect_update_metrics_values().returning(|_| Ok(()));
            mock.expect_get_training_metrics().returning(|_| vec![]);
            mock.expect_delete_metric().returning(|_| Ok(()));

            mock
        }
    }

    mock! {
        pub TrainingMetricsRepository {}

        impl Clone for TrainingMetricsRepository {
            fn clone(&self) -> Self;
        }

        impl TrainingMetricsRepository for TrainingMetricsRepository {
            async fn save_definitions(
                &self,
                definition: TrainingMetricDefinition,
            ) -> Result<(), SaveTrainingMetricError>;

            async fn get_definitions(
                &self,
                user: &UserId,
            ) -> Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError>;

            async fn update_metric_values(
                &self,
                id: &TrainingMetricId,
                values: (String, f64),
            ) -> Result<(), UpdateMetricError>;

            async fn get_metric_values(
                &self,
                id: &TrainingMetricId,
            ) -> Result<TrainingMetricValues, GetTrainingMetricValueError>;

            async fn get_definition(
                &self,
                metric: &TrainingMetricId,
            ) -> Result<Option<TrainingMetricDefinition>, GetDefinitionError>;

            async fn delete_definition(
                &self,
                metric: &TrainingMetricId,
            ) -> Result<(), DeleteMetricError>;
        }
    }
}

#[cfg(test)]
mod tests_training_metrics_service {
    use std::{collections::HashMap, sync::Arc};

    use anyhow::anyhow;
    use tokio::sync::Mutex;

    use crate::domain::{
        models::{
            activity::ActivityStatistic,
            training_metrics::{
                TrainingMetricAggregate, TrainingMetricDefinition, TrainingMetricGranularity,
                TrainingMetricId, TrainingMetricSource, TrainingMetricValues,
            },
        },
        ports::GetTrainingMetricsDefinitionsError,
        services::{
            activity::test_utils::MockActivityRepository,
            training_metrics::test_utils::MockTrainingMetricsRepository,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_when_get_definitions_err() {
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definitions().returning(|_| {
            Err(GetTrainingMetricsDefinitionsError::Unknown(anyhow!(
                "an error"
            )))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

        let res = service.get_training_metrics(&UserId::test_default()).await;
        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_def_without_values() {
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definitions().returning(|_| {
            Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )])
        });
        repository
            .expect_get_metric_values()
            .returning(|_| Ok(TrainingMetricValues::new(HashMap::new())));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

        let res = service.get_training_metrics(&UserId::test_default()).await;

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )
        );
        assert!(value.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_map_def_with_its_values() {
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definitions().returning(|_| {
            Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )])
        });
        repository.expect_get_metric_values().returning(|_| {
            Ok(TrainingMetricValues::new(HashMap::from([(
                "toto".to_string(),
                0.3,
            )])))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

        let res = service.get_training_metrics(&UserId::test_default()).await;

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )
        );
        assert_eq!(*value.get("toto").unwrap(), 0.3);
    }

    #[tokio::test]
    async fn test_training_service_delete_metric_does_not_exist() {
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definition().returning(|_| Ok(None));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

        let req = DeleteTrainingMetricRequest::new(
            "user".to_string().into(),
            TrainingMetricId::from("test"),
        );

        let res = service.delete_metric(req).await;

        let Err(DeleteTrainingMetricError::MetricDoesNotExist(metric)) = res else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(metric, TrainingMetricId::from("test"));
    }

    #[tokio::test]
    async fn test_training_service_delete_metric_wrong_user() {
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definition().returning(|_| {
            Ok(Some(TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                "other_user".to_string().into(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )))
        });

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

        let req = DeleteTrainingMetricRequest::new(
            "user".to_string().into(),
            TrainingMetricId::from("test"),
        );

        let res = service.delete_metric(req).await;

        let Err(DeleteTrainingMetricError::UserDoesNotOwnTrainingMetric(user, metric)) = res else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(user, "user".to_string().into());
        assert_eq!(metric, TrainingMetricId::from("test"));
    }

    #[tokio::test]
    async fn test_training_service_delete_metric() {
        let mut repository = MockTrainingMetricsRepository::new();
        repository.expect_get_definition().returning(|_| {
            Ok(Some(TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                "user".to_string().into(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )))
        });
        repository
            .expect_delete_definition()
            .times(1)
            .withf(|id| id == &TrainingMetricId::from("test"))
            .returning(|_| Ok(()));

        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

        let req = DeleteTrainingMetricRequest::new(
            "user".to_string().into(),
            TrainingMetricId::from("test"),
        );

        let res = service.delete_metric(req).await;

        assert!(res.is_ok());
    }
}
