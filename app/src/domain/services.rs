use std::sync::Arc;

use anyhow::anyhow;
use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::domain::{
    models::{
        UserId,
        activity::{Activity, ActivityId},
        training_metrics::{TrainingMetricDefinition, TrainingMetricId, TrainingMetricValues},
    },
    ports::{
        ActivityRepository, CreateActivityError, CreateActivityRequest, CreateTrainingMetricError,
        CreateTrainingMetricRequest, GetActivityError, IActivityService, ITrainingMetricService,
        ListActivitiesError, RawDataRepository, RecomputeMetricRequest, TrainingMetricsRepository,
    },
};

///////////////////////////////////////////////////////////////////
/// ACTIVITY SERVICE
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct ActivityService<AR, RDR, TMS>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
    TMS: ITrainingMetricService,
{
    activity_repository: Arc<Mutex<AR>>,
    raw_data_repository: RDR,
    training_metrics_service: Arc<TMS>,
}

impl<AR, RDR, TMS> ActivityService<AR, RDR, TMS>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
    TMS: ITrainingMetricService,
{
    pub fn new(
        activity_repository: Arc<Mutex<AR>>,
        raw_data_repository: RDR,
        training_metrics_service: Arc<TMS>,
    ) -> Self {
        Self {
            activity_repository,
            raw_data_repository,
            training_metrics_service,
        }
    }
}

impl<AR, RDR, TMS> IActivityService for ActivityService<AR, RDR, TMS>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
    TMS: ITrainingMetricService,
{
    async fn create_activity(
        &self,
        req: CreateActivityRequest,
    ) -> Result<Activity, CreateActivityError> {
        // Check candidate timeseries metrics have the same lenghts
        let time_len = req.timeseries().time().len();
        if req
            .timeseries()
            .metrics()
            .iter()
            .any(|metric| metric.values().len() != time_len)
        {
            return Err(CreateActivityError::TimeseriesMetricsNotSameLength);
        }

        // Create activity from request
        let id = ActivityId::new();
        let activity = Activity::new(
            id.clone(),
            UserId::default(),
            *req.start_time(),
            *req.duration(),
            *req.sport(),
            req.statistics().clone(),
            req.timeseries().clone(),
        );

        let activity_repository = self.activity_repository.lock().await;
        if activity_repository
            .similar_activity_exists(&activity.natural_key())
            .await
            .map_err(|err| {
                anyhow!(err).context(format!("A similar activity already exists {:?}", activity))
            })?
        {
            return Err(CreateActivityError::SimilarActivityExistsError);
        }

        // Persist activity
        activity_repository
            .save_activity(&activity)
            .await
            .map_err(|err| anyhow!(err).context(format!("Failed to persist activity {}", id)))?;

        // Persist raw data
        self.raw_data_repository
            .save_raw_data(&id, req.raw_content())
            .await
            .map_err(|err| {
                anyhow!(err).context(format!("Failed to persist raw data for activity {}", id))
            })?;

        // Dispatch metrics update
        let metric_service = self.training_metrics_service.clone();
        let activity_id = activity.id().clone();
        tokio::spawn(async move {
            let req = RecomputeMetricRequest::new(activity_id);
            metric_service.recompute_metric(req).await
        });

        Ok(activity)
    }

    async fn list_activities(&self, user: &UserId) -> Result<Vec<Activity>, ListActivitiesError> {
        let repository = self.activity_repository.lock().await;
        repository.list_activities(user).await
    }

    async fn get_activity(&self, activity_id: &ActivityId) -> Result<Activity, GetActivityError> {
        let repository = self.activity_repository.lock().await;
        match repository.get_activity(activity_id).await {
            Ok(Some(activity)) => Ok(activity),
            Ok(None) => Err(GetActivityError::ActivityDoesNotExist(activity_id.clone())),
            Err(err) => Err(err),
        }
    }
}

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
            req.source().clone(),
            req.granularity().clone(),
            req.aggregate().clone(),
        );
        self.metrics_repository
            .save_definitions(metric_definition)
            .await?;

        Ok(id)
    }

    async fn recompute_metric(&self, _req: RecomputeMetricRequest) -> Result<(), ()> {
        let metrics = self.metrics_repository.get_definitions().await.unwrap();
        let activities = self
            .activity_repository
            .lock()
            .await
            .list_activities(&UserId::default())
            .await
            .unwrap();

        for metric in metrics {
            let values = metric.compute_values(&activities);
            tracing::info!("New value {:?} for metric {:?}", values, metric);
            for (key, value) in values.iter() {
                let _ = self
                    .metrics_repository
                    .save_metric_values(metric.id(), (key, *value))
                    .await;
            }
        }
        Ok(())
    }

    async fn get_training_metrics(&self) -> Vec<(TrainingMetricDefinition, TrainingMetricValues)> {
        let Ok(definitions) = self.metrics_repository.get_definitions().await else {
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
}

///////////////////////////////////////////////////////////////////
// MOCK IMPLEMENTATIONS FOR TESTING
///////////////////////////////////////////////////////////////////

#[cfg(test)]
pub mod test_utils {
    use std::mem;
    use std::sync::{Arc, Mutex};

    use super::*;

    use crate::domain::models::activity::{
        ActivityDuration, ActivityNaturalKey, ActivityStartTime, ActivityStatistics,
        ActivityTimeseries, Sport,
    };
    use crate::domain::ports::{ListActivitiesError, SaveActivityError, SimilarActivityError};

    #[derive(Clone)]
    pub struct MockActivityService {
        pub create_activity_result: Arc<Mutex<Result<Activity, CreateActivityError>>>,
        pub list_activities_result: Arc<Mutex<Result<Vec<Activity>, ListActivitiesError>>>,
        pub get_activity_result: Arc<Mutex<Result<Activity, GetActivityError>>>,
    }

    impl Default for MockActivityService {
        fn default() -> Self {
            Self {
                create_activity_result: Arc::new(Mutex::new(Ok(Activity::new(
                    ActivityId::new(),
                    UserId::default(),
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    ActivityDuration::from(3600),
                    Sport::Running,
                    ActivityStatistics::default(),
                    ActivityTimeseries::default(),
                )))),
                list_activities_result: Arc::new(Mutex::new(Ok(vec![]))),
                get_activity_result: Arc::new(Mutex::new(Ok(Activity::new(
                    ActivityId::new(),
                    UserId::default(),
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    ActivityDuration::from(3600),
                    Sport::Running,
                    ActivityStatistics::default(),
                    ActivityTimeseries::default(),
                )))),
            }
        }
    }

    impl IActivityService for MockActivityService {
        async fn create_activity(
            &self,
            _req: CreateActivityRequest,
        ) -> Result<Activity, CreateActivityError> {
            let mut guard = self.create_activity_result.lock();
            let mut result = Err(CreateActivityError::Unknown(anyhow!("Substitute errror")));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }

        async fn list_activities(
            &self,
            _user: &UserId,
        ) -> Result<Vec<Activity>, ListActivitiesError> {
            let mut guard = self.list_activities_result.lock();
            let mut result = Err(ListActivitiesError::Unknown(anyhow!("Substitute errror")));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }

        async fn get_activity(
            &self,
            _activity_id: &ActivityId,
        ) -> Result<Activity, GetActivityError> {
            let mut guard = self.get_activity_result.lock();
            let mut result = Err(GetActivityError::Unknown(anyhow!("Substitute errror")));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }
    }

    #[derive(Clone)]
    pub struct MockActivityRepository {
        pub similar_activity_result: Arc<Mutex<Result<bool, SimilarActivityError>>>,
        pub save_activity_result: Arc<Mutex<Result<(), SaveActivityError>>>,
        pub list_activities_result: Arc<Mutex<Result<Vec<Activity>, ListActivitiesError>>>,
        pub get_activity_result: Arc<Mutex<Result<Option<Activity>, GetActivityError>>>,
    }

    impl ActivityRepository for MockActivityRepository {
        async fn similar_activity_exists(
            &self,
            _natural_key: &ActivityNaturalKey,
        ) -> Result<bool, SimilarActivityError> {
            let mut guard = self.similar_activity_result.lock();
            let mut result = Err(SimilarActivityError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }

        async fn save_activity(&self, _activity: &Activity) -> Result<(), SaveActivityError> {
            let mut guard = self.save_activity_result.lock();
            let mut result = Err(SaveActivityError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }

        async fn list_activities(
            &self,
            _user: &UserId,
        ) -> Result<Vec<Activity>, ListActivitiesError> {
            let mut guard = self.list_activities_result.lock();
            let mut result = Err(ListActivitiesError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }

        async fn get_activity(
            &self,
            _id: &ActivityId,
        ) -> Result<Option<Activity>, GetActivityError> {
            let mut guard = self.get_activity_result.lock();
            let mut result = Err(GetActivityError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }
    }

    impl Default for MockActivityRepository {
        fn default() -> Self {
            Self {
                similar_activity_result: Arc::new(Mutex::new(Ok(false))),
                save_activity_result: Arc::new(Mutex::new(Ok(()))),
                list_activities_result: Arc::new(Mutex::new(Ok(vec![]))),
                get_activity_result: Arc::new(Mutex::new(Ok(None))),
            }
        }
    }

    #[derive(Clone)]
    pub struct MockTrainingMetricsService {
        pub create_metric_result: Arc<Mutex<Result<TrainingMetricId, CreateTrainingMetricError>>>,
        pub recompute_metrics_result: Arc<Mutex<Result<(), ()>>>,
        pub get_training_metrics_result:
            Arc<Mutex<Vec<(TrainingMetricDefinition, TrainingMetricValues)>>>,
    }

    impl Default for MockTrainingMetricsService {
        fn default() -> Self {
            Self {
                create_metric_result: Arc::new(Mutex::new(Ok(TrainingMetricId::default()))),
                recompute_metrics_result: Arc::new(Mutex::new(Ok(()))),
                get_training_metrics_result: Arc::new(Mutex::new(vec![])),
            }
        }
    }

    impl ITrainingMetricService for MockTrainingMetricsService {
        async fn create_metric(
            &self,
            _req: crate::domain::ports::CreateTrainingMetricRequest,
        ) -> Result<TrainingMetricId, CreateTrainingMetricError> {
            let mut guard = self.create_metric_result.lock();
            let mut result = Err(CreateTrainingMetricError::Unknown(anyhow!(
                "substitute error"
            )));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }
        async fn recompute_metric(&self, _req: RecomputeMetricRequest) -> Result<(), ()> {
            let mut guard = self.recompute_metrics_result.lock();
            let mut result = Err(());
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }

        async fn get_training_metrics(
            &self,
        ) -> Vec<(TrainingMetricDefinition, TrainingMetricValues)> {
            let mut guard = self.get_training_metrics_result.lock();
            let mut result = Vec::new();
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }
    }
}

#[cfg(test)]
mod tests_activity_service {
    use std::{mem, ops::DerefMut, sync::Arc};

    use anyhow::anyhow;
    use tokio::sync::Mutex;

    use crate::domain::{
        models::{
            UserId,
            activity::{
                ActivityDuration, ActivityStartTime, ActivityStatistics, ActivityTimeseries, Sport,
                Timeseries, TimeseriesMetric, TimeseriesTime, TimeseriesValue,
            },
        },
        ports::{SaveActivityError, SaveRawDataError},
        services::test_utils::{MockActivityRepository, MockTrainingMetricsService},
    };

    use super::*;

    #[derive(Clone)]
    struct MockRawDataRepository {
        save_raw_data: Arc<Mutex<Result<(), SaveRawDataError>>>,
    }

    impl RawDataRepository for MockRawDataRepository {
        async fn save_raw_data(
            &self,
            _activity_id: &ActivityId,
            _content: &[u8],
        ) -> Result<(), SaveRawDataError> {
            let mut guard = self.save_raw_data.lock().await;
            let mut result = Err(SaveRawDataError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }
    }

    fn default_activity_request() -> CreateActivityRequest {
        let sport = Sport::Running;
        let start_time = ActivityStartTime::from_timestamp(3600).unwrap();
        let duration = ActivityDuration(1200);
        let content = vec![1, 2, 3];
        let statistics = ActivityStatistics::default();
        let timeseries = ActivityTimeseries::default();
        CreateActivityRequest::new(
            UserId::default(),
            sport,
            duration,
            start_time,
            statistics,
            timeseries,
            content,
        )
    }

    #[tokio::test]
    async fn test_service_create_activity_err_if_similar_activity_exists() {
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository {
            similar_activity_result: Arc::new(std::sync::Mutex::new(Ok(true))),
            ..Default::default()
        }));
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Ok(()))),
        };
        let metrics_service = Arc::new(MockTrainingMetricsService::default());
        let service =
            ActivityService::new(activity_repository, raw_data_repository, metrics_service);

        let req = default_activity_request();

        let res = service.create_activity(req).await;

        assert!(res.is_err());
        let Err(CreateActivityError::SimilarActivityExistsError) = res else {
            unreachable!(
                "Should have returned a Err(CreateActivityError::SimilarActivityExistsError)"
            )
        };
    }

    #[tokio::test]
    async fn test_service_create_activity() {
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Ok(()))),
        };
        let metrics_service = Arc::new(MockTrainingMetricsService::default());
        let service =
            ActivityService::new(activity_repository, raw_data_repository, metrics_service);

        let req = default_activity_request();

        let res = service.create_activity(req).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_service_create_activity_save_activity_error() {
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository {
            save_activity_result: Arc::new(std::sync::Mutex::new(Err(SaveActivityError::Unknown(
                anyhow!("an error occured"),
            )))),
            ..Default::default()
        }));
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Ok(()))),
        };
        let metrics_service = Arc::new(MockTrainingMetricsService::default());
        let service =
            ActivityService::new(activity_repository, raw_data_repository, metrics_service);

        let req = default_activity_request();

        let res = service.create_activity(req).await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_service_create_activity_raw_data_error() {
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Err(SaveRawDataError::Unknown(anyhow!(
                "an error occured"
            ))))),
        };
        let metrics_service = Arc::new(MockTrainingMetricsService::default());
        let service =
            ActivityService::new(activity_repository, raw_data_repository, metrics_service);

        let req = default_activity_request();

        let res = service.create_activity(req).await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_create_timeseries_returns_err_when_timeseries_have_different_lenghts_than_time_vec()
     {
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Err(SaveRawDataError::Unknown(anyhow!(
                "an error occured"
            ))))),
        };
        let metrics_service = Arc::new(MockTrainingMetricsService::default());
        let service =
            ActivityService::new(activity_repository, raw_data_repository, metrics_service);

        let sport = Sport::Running;
        let start_time = ActivityStartTime::from_timestamp(3600).unwrap();
        let duration = ActivityDuration(1200);
        let content = vec![1, 2, 3];
        let statistics = ActivityStatistics::default();
        let timeseries = ActivityTimeseries::new(
            TimeseriesTime::new(vec![0, 1, 2]),
            vec![
                Timeseries::new(
                    TimeseriesMetric::Power,
                    vec![
                        Some(TimeseriesValue::Int(0)),
                        Some(TimeseriesValue::Int(100)),
                    ],
                ),
                Timeseries::new(
                    TimeseriesMetric::Speed,
                    vec![
                        Some(TimeseriesValue::Float(0.)),
                        Some(TimeseriesValue::Float(100.)),
                        None,
                    ],
                ),
            ],
        );

        let req = CreateActivityRequest::new(
            UserId::default(),
            sport,
            duration,
            start_time,
            statistics,
            timeseries,
            content,
        );

        let res = service.create_activity(req).await;

        match res {
            Err(CreateActivityError::TimeseriesMetricsNotSameLength) => {}
            _ => unreachable!(
                "Should have returned an Err(CreateActivityError::TimeseriesNotSameLength) "
            ),
        }
    }
}

#[cfg(test)]
mod tests_training_metrics_service {
    use std::{collections::HashMap, mem, ops::DerefMut, sync::Arc};

    use tokio::sync::Mutex;

    use crate::domain::{
        models::{
            activity::{ActivityStatistic, TimeseriesMetric},
            training_metrics::{
                TrainingMetricAggregate, TrainingMetricDefinition, TrainingMetricGranularity,
                TrainingMetricId, TrainingMetricSource, TrainingMetricValues,
            },
        },
        ports::{
            GetTrainingMetricValueError, GetTrainingMetricsDefinitionsError,
            SaveTrainingMetricError, UpdateMetricError,
        },
        services::test_utils::MockActivityRepository,
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
        ) -> Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError> {
            let mut guard = self.get_definitions_result.lock().await;
            let mut result = Err(GetTrainingMetricsDefinitionsError::Unknown(anyhow!(
                "substitute error"
            )));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }

        async fn save_metric_values(
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
    }

    impl Default for MockTrainingMetricsRepository {
        fn default() -> Self {
            Self {
                save_definitins_result: Arc::new(Mutex::new(Ok(()))),
                get_definitions_result: Arc::new(Mutex::new(Ok(vec![
                    TrainingMetricDefinition::new(
                        TrainingMetricId::default(),
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
            }
        }
    }

    #[tokio::test]
    async fn test_training_metric_service() {
        let repository = MockTrainingMetricsRepository::default();
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);
        let req = RecomputeMetricRequest::new(ActivityId::default());

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

        let res = service.get_training_metrics().await;
        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_def_without_values() {
        let repository = MockTrainingMetricsRepository {
            get_definitions_result: Arc::new(Mutex::new(Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )]))),
            ..Default::default()
        };
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

        let res = service.get_training_metrics().await;

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
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

        let res = service.get_training_metrics().await;

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )
        );
        assert_eq!(*value.get("toto").unwrap(), 0.3);
    }
}
