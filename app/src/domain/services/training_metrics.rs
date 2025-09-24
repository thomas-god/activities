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
        RecomputeMetricRequest, TrainingMetricsRepository,
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
            .recompute_metric(RecomputeMetricRequest::new(req.user().clone(), None))
            .await;

        Ok(id)
    }

    async fn recompute_metric(&self, req: RecomputeMetricRequest) -> Result<(), ()> {
        let metrics = self
            .metrics_repository
            .get_definitions(req.user())
            .await
            .unwrap();
        let activities = self
            .activity_repository
            .lock()
            .await
            .list_activities_with_timeseries(req.user())
            .await
            .unwrap();

        for metric in metrics {
            let values = metric.compute_values(&activities);
            tracing::info!("New value {:?} for metric {:?}", values, metric);
            for (key, value) in values.iter() {
                let _ = self
                    .metrics_repository
                    .update_metric_values(metric.id(), (key, *value))
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

            async fn recompute_metric(
                &self,
                req: RecomputeMetricRequest,
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
            mock.expect_recompute_metric().returning(|_| Ok(()));
            mock.expect_get_training_metrics().returning(|_| vec![]);
            mock.expect_delete_metric().returning(|_| Ok(()));

            mock
        }
    }
}

#[cfg(test)]
mod tests_training_metrics_service {
    use std::{collections::HashMap, mem, ops::DerefMut, sync::Arc};

    use anyhow::anyhow;
    use tokio::sync::Mutex;

    use crate::domain::{
        models::{
            activity::{ActivityId, ActivityStatistic, TimeseriesMetric},
            training_metrics::{
                TrainingMetricAggregate, TrainingMetricDefinition, TrainingMetricGranularity,
                TrainingMetricId, TrainingMetricSource, TrainingMetricValues,
            },
        },
        ports::{
            DeleteMetricError, GetDefinitionError, GetTrainingMetricValueError,
            GetTrainingMetricsDefinitionsError, SaveTrainingMetricError, UpdateMetricError,
        },
        services::activity::test_utils::MockActivityRepository,
    };

    use super::*;

    #[derive(Clone)]
    struct MockTrainingMetricsRepository {
        save_definitins_result: Arc<Mutex<Result<(), SaveTrainingMetricError>>>,
        get_definitions_result:
            Arc<Mutex<Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError>>>,
        update_metric_values_result: Arc<Mutex<Result<(), UpdateMetricError>>>,
        get_metric_values_result:
            Arc<Mutex<Result<TrainingMetricValues, GetTrainingMetricValueError>>>,
        get_definition_result:
            Arc<Mutex<Result<Option<TrainingMetricDefinition>, GetDefinitionError>>>,
        delete_definition_result: Arc<Mutex<Result<(), DeleteMetricError>>>,
        delete_definition_call_list: Arc<Mutex<Vec<TrainingMetricId>>>,
    }

    impl TrainingMetricsRepository for MockTrainingMetricsRepository {
        async fn save_definitions(
            &self,
            _definition: TrainingMetricDefinition,
        ) -> Result<(), crate::domain::ports::SaveTrainingMetricError> {
            let mut guard = self.save_definitins_result.lock().await;
            let mut result = Err(SaveTrainingMetricError::Unknown(anyhow!(
                "substitute error"
            )));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }

        async fn get_definitions(
            &self,
            _user: &UserId,
        ) -> Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError> {
            let mut guard = self.get_definitions_result.lock().await;
            let mut result = Err(GetTrainingMetricsDefinitionsError::Unknown(anyhow!(
                "substitute error"
            )));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }

        async fn update_metric_values(
            &self,
            _id: &TrainingMetricId,
            _values: (&str, f64),
        ) -> Result<(), UpdateMetricError> {
            let mut guard = self.update_metric_values_result.lock().await;
            let mut result = Err(UpdateMetricError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }

        async fn get_metric_values(
            &self,
            _id: &TrainingMetricId,
        ) -> Result<TrainingMetricValues, GetTrainingMetricValueError> {
            let mut guard = self.get_metric_values_result.lock().await;
            let mut result = Err(GetTrainingMetricValueError::Unknown(anyhow!(
                "substitute error"
            )));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }

        async fn get_definition(
            &self,
            _metric: &TrainingMetricId,
        ) -> Result<Option<TrainingMetricDefinition>, GetDefinitionError> {
            let mut guard = self.get_definition_result.lock().await;
            let mut result = Err(GetDefinitionError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }

        async fn delete_definition(
            &self,
            metric: &TrainingMetricId,
        ) -> Result<(), DeleteMetricError> {
            let mut guard = self.delete_definition_result.lock().await;
            let mut result = Err(DeleteMetricError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.deref_mut(), &mut result);
            self.delete_definition_call_list
                .lock()
                .await
                .push(metric.clone());
            result
        }
    }

    impl Default for MockTrainingMetricsRepository {
        fn default() -> Self {
            Self {
                save_definitins_result: Arc::new(Mutex::new(Ok(()))),
                get_definitions_result: Arc::new(Mutex::new(Ok(vec![
                    TrainingMetricDefinition::new(
                        TrainingMetricId::default(),
                        UserId::test_default(),
                        TrainingMetricSource::Timeseries((
                            TimeseriesMetric::Power,
                            TrainingMetricAggregate::Average,
                        )),
                        TrainingMetricGranularity::Weekly,
                        TrainingMetricAggregate::Max,
                    ),
                ]))),
                update_metric_values_result: Arc::new(Mutex::new(Ok(()))),
                get_metric_values_result: Arc::new(Mutex::new(Ok(TrainingMetricValues::default()))),
                get_definition_result: Arc::new(Mutex::new(Ok(Some(
                    TrainingMetricDefinition::new(
                        TrainingMetricId::default(),
                        UserId::test_default(),
                        TrainingMetricSource::Timeseries((
                            TimeseriesMetric::Power,
                            TrainingMetricAggregate::Average,
                        )),
                        TrainingMetricGranularity::Weekly,
                        TrainingMetricAggregate::Max,
                    ),
                )))),
                delete_definition_result: Arc::new(Mutex::new(Ok(()))),
                delete_definition_call_list: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[tokio::test]
    async fn test_training_metric_service() {
        let repository = MockTrainingMetricsRepository::default();
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_list_activities_with_timeseries()
            .returning(|_| Ok(vec![]));

        let service =
            TrainingMetricService::new(repository, Arc::new(Mutex::new(activity_repository)));
        let req = RecomputeMetricRequest::new(UserId::test_default(), Some(ActivityId::default()));

        let res = service.recompute_metric(req).await;

        assert!(res.is_ok())
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_when_get_definitions_err() {
        let repository = MockTrainingMetricsRepository {
            get_definitions_result: Arc::new(Mutex::new(Err(
                GetTrainingMetricsDefinitionsError::Unknown(anyhow!("an error")),
            ))),
            ..Default::default()
        };
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

        let res = service.get_training_metrics(&UserId::test_default()).await;
        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_def_without_values() {
        let repository = MockTrainingMetricsRepository {
            get_definitions_result: Arc::new(Mutex::new(Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )]))),
            ..Default::default()
        };
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
        let repository = MockTrainingMetricsRepository {
            get_definitions_result: Arc::new(Mutex::new(Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::test_default(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )]))),
            get_metric_values_result: Arc::new(Mutex::new(Ok(TrainingMetricValues::new(
                HashMap::from([("toto".to_string(), 0.3)]),
            )))),
            ..Default::default()
        };
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
        let repository = MockTrainingMetricsRepository {
            get_definition_result: Arc::new(Mutex::new(Ok(None))),
            ..Default::default()
        };
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
        let repository = MockTrainingMetricsRepository {
            get_definition_result: Arc::new(Mutex::new(Ok(Some(TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                "other_user".to_string().into(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            ))))),
            ..Default::default()
        };
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
        let repository = MockTrainingMetricsRepository {
            get_definition_result: Arc::new(Mutex::new(Ok(Some(TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                "user".to_string().into(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            ))))),
            ..Default::default()
        };
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository.clone(), activity_repository);

        let req = DeleteTrainingMetricRequest::new(
            "user".to_string().into(),
            TrainingMetricId::from("test"),
        );

        let res = service.delete_metric(req).await;

        assert!(res.is_ok());
        let call_list = repository.delete_definition_call_list.lock().await.clone();
        assert_eq!(call_list, vec![TrainingMetricId::from("test")]);
    }
}
