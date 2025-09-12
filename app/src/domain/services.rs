use std::sync::Arc;

use anyhow::anyhow;
use derive_more::Constructor;
use tokio::sync::Mutex;

use crate::domain::{
    models::activity::{Activity, ActivityId},
    ports::{
        ActivityRepository, CreateActivityError, CreateActivityRequest, GetActivityError,
        IActivityService, ITrainingMetricService, ListActivitiesError, RawDataRepository,
        RecomputeMetricRequest, TrainingMetricsRepository,
    },
};

///////////////////////////////////////////////////////////////////
/// ACTIVITY SERVICE
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct ActivityService<AR, RDR>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
{
    activity_repository: Arc<Mutex<AR>>,
    raw_data_repository: RDR,
}

impl<AR, RDR> ActivityService<AR, RDR>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
{
    pub fn new(activity_repository: Arc<Mutex<AR>>, raw_data_repository: RDR) -> Self {
        Self {
            activity_repository,
            raw_data_repository,
        }
    }
}

impl<AR, RDR> IActivityService for ActivityService<AR, RDR>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
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
            *req.start_time(),
            *req.duration(),
            *req.sport(),
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

        Ok(activity)
    }

    async fn list_activities(&self) -> Result<Vec<Activity>, ListActivitiesError> {
        let repository = self.activity_repository.lock().await;
        repository.list_activities().await
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
    async fn recompute_metric(&self, _req: RecomputeMetricRequest) -> Result<(), ()> {
        let metrics = self.metrics_repository.get_definitions().await.unwrap();
        let activities = self
            .activity_repository
            .lock()
            .await
            .list_activities()
            .await
            .unwrap();

        for metric in metrics {
            let a = metric.compute_values(&activities);
            println!("New metrics: {:?}", a);
        }
        Ok(())
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
        ActivityDuration, ActivityNaturalKey, ActivityStartTime, Sport, Timeseries,
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
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    ActivityDuration::from(3600),
                    Sport::Running,
                    Timeseries::default(),
                )))),
                list_activities_result: Arc::new(Mutex::new(Ok(vec![]))),
                get_activity_result: Arc::new(Mutex::new(Ok(Activity::new(
                    ActivityId::new(),
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    ActivityDuration::from(3600),
                    Sport::Running,
                    Timeseries::default(),
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

        async fn list_activities(&self) -> Result<Vec<Activity>, ListActivitiesError> {
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

        async fn list_activities(&self) -> Result<Vec<Activity>, ListActivitiesError> {
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
        pub recompute_metrics_result: Arc<Mutex<Result<(), ()>>>,
    }

    impl Default for MockTrainingMetricsService {
        fn default() -> Self {
            Self {
                recompute_metrics_result: Arc::new(Mutex::new(Ok(()))),
            }
        }
    }

    impl ITrainingMetricService for MockTrainingMetricsService {
        async fn recompute_metric(&self, _req: RecomputeMetricRequest) -> Result<(), ()> {
            let mut guard = self.recompute_metrics_result.lock();
            let mut result = Err(());
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
        models::activity::{
            ActivityDuration, ActivityStartTime, Metric, Sport, Timeseries, TimeseriesMetric,
            TimeseriesTime, TimeseriesValue,
        },
        ports::{SaveActivityError, SaveRawDataError},
        services::test_utils::MockActivityRepository,
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
        let timeseries = Timeseries::default();
        CreateActivityRequest::new(sport, duration, start_time, timeseries, content)
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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
        let service = ActivityService::new(activity_repository, raw_data_repository);

        let sport = Sport::Running;
        let start_time = ActivityStartTime::from_timestamp(3600).unwrap();
        let duration = ActivityDuration(1200);
        let content = vec![1, 2, 3];
        let timeseries = Timeseries::new(
            TimeseriesTime::new(vec![0, 1, 2]),
            vec![
                TimeseriesMetric::new(
                    Metric::Power,
                    vec![
                        Some(TimeseriesValue::Int(0)),
                        Some(TimeseriesValue::Int(100)),
                    ],
                ),
                TimeseriesMetric::new(
                    Metric::Speed,
                    vec![
                        Some(TimeseriesValue::Float(0.)),
                        Some(TimeseriesValue::Float(100.)),
                        None,
                    ],
                ),
            ],
        );

        let req = CreateActivityRequest::new(sport, duration, start_time, timeseries, content);

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
    use std::{mem, ops::DerefMut, sync::Arc};

    use tokio::sync::Mutex;

    use crate::domain::{
        models::{
            activity::Metric,
            training_metrics::{
                TrainingMetricAggregate, TrainingMetricDefinition, TrainingMetricGranularity,
                TrainingMetricId, TrainingMetricValues,
            },
        },
        ports::{
            GetTrainingMetricValueError, GetTrainingMetricsDefinitionsError, UpdateMetricError,
        },
        services::test_utils::MockActivityRepository,
    };

    use super::*;

    #[derive(Clone)]
    struct MockTrainingMetricsRepository {
        get_definitions_result:
            Arc<Mutex<Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError>>>,
        update_metric_values_result: Arc<Mutex<Result<(), UpdateMetricError>>>,
        get_metric_values_result:
            Arc<Mutex<Result<TrainingMetricValues, GetTrainingMetricValueError>>>,
    }

    impl TrainingMetricsRepository for MockTrainingMetricsRepository {
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

        async fn update_metric_values(
            &self,
            _id: &TrainingMetricId,
            _new_value: (String, f64),
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
                get_definitions_result: Arc::new(Mutex::new(Ok(vec![
                    TrainingMetricDefinition::new(
                        TrainingMetricId::default(),
                        Metric::Power,
                        TrainingMetricAggregate::Average,
                        TrainingMetricGranularity::Weekly,
                        TrainingMetricAggregate::Max,
                    ),
                ]))),
                update_metric_values_result: Arc::new(Mutex::new(Ok(()))),
                get_metric_values_result: Arc::new(Mutex::new(Ok(TrainingMetricValues::new(
                    vec![],
                )))),
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
}
