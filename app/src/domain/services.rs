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
        CreateTrainingMetricRequest, DeleteActivityError, DeleteActivityRequest,
        DeleteTrainingMetricError, DeleteTrainingMetricRequest, GetActivityError, IActivityService,
        ITrainingMetricService, ListActivitiesError, ModifyActivityError, ModifyActivityRequest,
        RawDataRepository, RecomputeMetricRequest, TrainingMetricsRepository,
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
            None,
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
            let req = RecomputeMetricRequest::new(UserId::default(), Some(activity_id));
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

    async fn modify_activity(&self, req: ModifyActivityRequest) -> Result<(), ModifyActivityError> {
        let Ok(Some(_activity)) = self
            .activity_repository
            .lock()
            .await
            .get_activity(req.activity())
            .await
        else {
            return Err(ModifyActivityError::ActivityDoesNotExist(
                req.activity().clone(),
            ));
        };

        let _ = self
            .activity_repository
            .lock()
            .await
            .modify_activity_name(req.activity(), req.name().cloned())
            .await;

        Ok(())
    }

    async fn delete_activity(&self, req: DeleteActivityRequest) -> Result<(), DeleteActivityError> {
        let Ok(Some(activity)) = self
            .activity_repository
            .lock()
            .await
            .get_activity(req.activity())
            .await
        else {
            return Err(DeleteActivityError::ActivityDoesNotExist(
                req.activity().clone(),
            ));
        };

        if activity.user() != req.user() {
            return Err(DeleteActivityError::UserDoesNotOwnActivity(
                req.user().clone(),
                req.activity().clone(),
            ));
        }

        self.activity_repository
            .lock()
            .await
            .delete_activity(req.activity())
            .await?;

        Ok(())
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
            .list_activities(req.user())
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
    use std::mem;
    use std::sync::{Arc, Mutex};

    use super::*;

    use crate::domain::models::activity::{
        ActivityDuration, ActivityName, ActivityNaturalKey, ActivityStartTime, ActivityStatistics,
        ActivityTimeseries, Sport,
    };
    use crate::domain::ports::{
        DeleteActivityError, ListActivitiesError, ModifyActivityError, SaveActivityError,
        SimilarActivityError,
    };

    #[derive(Clone)]
    pub struct MockActivityService {
        pub create_activity_result: Arc<Mutex<Result<Activity, CreateActivityError>>>,
        pub list_activities_result: Arc<Mutex<Result<Vec<Activity>, ListActivitiesError>>>,
        pub get_activity_result: Arc<Mutex<Result<Activity, GetActivityError>>>,
        pub modify_activity_result: Arc<Mutex<Result<(), ModifyActivityError>>>,
        pub delete_activity_result: Arc<Mutex<Result<(), DeleteActivityError>>>,
    }

    impl Default for MockActivityService {
        fn default() -> Self {
            Self {
                create_activity_result: Arc::new(Mutex::new(Ok(Activity::new(
                    ActivityId::new(),
                    UserId::default(),
                    None,
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
                    None,
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    ActivityDuration::from(3600),
                    Sport::Running,
                    ActivityStatistics::default(),
                    ActivityTimeseries::default(),
                )))),
                modify_activity_result: Arc::new(Mutex::new(Ok(()))),
                delete_activity_result: Arc::new(Mutex::new(Ok(()))),
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

        async fn modify_activity(
            &self,
            _req: crate::domain::ports::ModifyActivityRequest,
        ) -> Result<(), ModifyActivityError> {
            let mut guard = self.modify_activity_result.lock();
            let mut result = Err(ModifyActivityError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }

        async fn delete_activity(
            &self,
            _req: crate::domain::ports::DeleteActivityRequest,
        ) -> Result<(), crate::domain::ports::DeleteActivityError> {
            let mut guard = self.delete_activity_result.lock();
            let mut result = Err(DeleteActivityError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }
    }

    #[allow(clippy::type_complexity)]
    #[derive(Clone)]
    pub struct MockActivityRepository {
        pub similar_activity_result: Arc<Mutex<Result<bool, SimilarActivityError>>>,
        pub save_activity_result: Arc<Mutex<Result<(), SaveActivityError>>>,
        pub list_activities_result: Arc<Mutex<Result<Vec<Activity>, ListActivitiesError>>>,
        pub get_activity_result: Arc<Mutex<Result<Option<Activity>, GetActivityError>>>,
        pub modify_activity_name_result: Arc<Mutex<Result<(), anyhow::Error>>>,
        pub modify_activity_name_call_list: Arc<Mutex<Vec<(ActivityId, Option<ActivityName>)>>>,
        pub delete_activity_result: Arc<Mutex<Result<(), anyhow::Error>>>,
        pub delete_activity_call_list: Arc<Mutex<Vec<ActivityId>>>,
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

        async fn modify_activity_name(
            &self,
            id: &ActivityId,
            name: Option<ActivityName>,
        ) -> Result<(), anyhow::Error> {
            let mut guard = self.modify_activity_name_result.lock();
            let mut result = Err(anyhow!("substitute error"));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            self.modify_activity_name_call_list
                .lock()
                .unwrap()
                .push((id.clone(), name.clone()));
            result
        }

        async fn delete_activity(&self, activity: &ActivityId) -> Result<(), anyhow::Error> {
            let mut guard = self.delete_activity_result.lock();
            let mut result = Err(anyhow!("substitute error"));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            self.delete_activity_call_list
                .lock()
                .unwrap()
                .push(activity.clone());
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
                modify_activity_name_result: Arc::new(Mutex::new(Ok(()))),
                modify_activity_name_call_list: Arc::new(Mutex::new(Vec::new())),
                delete_activity_result: Arc::new(Mutex::new(Ok(()))),
                delete_activity_call_list: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[derive(Clone)]
    pub struct MockTrainingMetricsService {
        pub create_metric_result: Arc<Mutex<Result<TrainingMetricId, CreateTrainingMetricError>>>,
        pub recompute_metrics_result: Arc<Mutex<Result<(), ()>>>,
        pub get_training_metrics_result:
            Arc<Mutex<Vec<(TrainingMetricDefinition, TrainingMetricValues)>>>,
        pub delete_metric_result: Arc<Mutex<Result<(), DeleteTrainingMetricError>>>,
    }

    impl Default for MockTrainingMetricsService {
        fn default() -> Self {
            Self {
                create_metric_result: Arc::new(Mutex::new(Ok(TrainingMetricId::default()))),
                recompute_metrics_result: Arc::new(Mutex::new(Ok(()))),
                get_training_metrics_result: Arc::new(Mutex::new(vec![])),
                delete_metric_result: Arc::new(Mutex::new(Ok(()))),
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
            _user: &UserId,
        ) -> Vec<(TrainingMetricDefinition, TrainingMetricValues)> {
            let mut guard = self.get_training_metrics_result.lock();
            let mut result = Vec::new();
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }

        async fn delete_metric(
            &self,
            _req: DeleteTrainingMetricRequest,
        ) -> Result<(), DeleteTrainingMetricError> {
            let mut guard = self.delete_metric_result.lock();
            let mut result = Err(DeleteTrainingMetricError::Unknown(anyhow!(
                "substitute error"
            )));
            mem::swap(guard.as_deref_mut().unwrap(), &mut result);
            result
        }
    }
}

#[cfg(test)]
mod tests_activity_service {
    use std::{collections::HashMap, mem, ops::DerefMut, sync::Arc};

    use anyhow::anyhow;
    use tokio::sync::Mutex;

    use crate::domain::{
        models::{
            UserId,
            activity::{
                ActivityDuration, ActivityName, ActivityStartTime, ActivityStatistics,
                ActivityTimeseries, Sport, Timeseries, TimeseriesMetric, TimeseriesTime,
                TimeseriesValue,
            },
        },
        ports::{
            DeleteActivityError, DeleteActivityRequest, ModifyActivityError, ModifyActivityRequest,
            SaveActivityError, SaveRawDataError,
        },
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

    impl Default for MockRawDataRepository {
        fn default() -> Self {
            Self {
                save_raw_data: Arc::new(Mutex::new(Ok(()))),
            }
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

    #[tokio::test]
    async fn test_activity_service_modify_activity_not_found() {
        let activity_repository = Arc::new(tokio::sync::Mutex::new(MockActivityRepository {
            get_activity_result: Arc::new(std::sync::Mutex::new(Ok(None))),
            ..Default::default()
        }));
        let raw_data_repository = MockRawDataRepository::default();
        let metrics_service = Arc::new(MockTrainingMetricsService::default());
        let service =
            ActivityService::new(activity_repository, raw_data_repository, metrics_service);

        let req = ModifyActivityRequest::new(ActivityId::from("test"), None);

        let Err(ModifyActivityError::ActivityDoesNotExist(activity)) =
            service.modify_activity(req).await
        else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(activity, ActivityId::from("test"));
    }

    #[tokio::test]
    async fn test_activity_service_modify_activity_ok() {
        let activity_repository = Arc::new(tokio::sync::Mutex::new(MockActivityRepository {
            get_activity_result: Arc::new(std::sync::Mutex::new(Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::from("another_user".to_string()),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration(0),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
                ActivityTimeseries::new(TimeseriesTime::new(Vec::new()), Vec::new()),
            ))))),
            ..Default::default()
        }));
        let raw_data_repository = MockRawDataRepository::default();
        let metrics_service = Arc::new(MockTrainingMetricsService::default());
        let service = ActivityService::new(
            activity_repository.clone(),
            raw_data_repository,
            metrics_service,
        );

        let req = ModifyActivityRequest::new(
            ActivityId::from("test"),
            Some(ActivityName::new("new name".to_string())),
        );

        let res = service.modify_activity(req).await;
        assert!(res.is_ok());

        let call_list = activity_repository
            .lock()
            .await
            .modify_activity_name_call_list
            .lock()
            .unwrap()
            .clone();
        assert_eq!(
            call_list,
            vec![(
                ActivityId::from("test"),
                Some(ActivityName::new("new name".to_string()))
            )]
        );
    }

    #[tokio::test]
    async fn test_activity_service_delete_activity_not_found() {
        let activity_repository = Arc::new(tokio::sync::Mutex::new(MockActivityRepository {
            get_activity_result: Arc::new(std::sync::Mutex::new(Ok(None))),
            ..Default::default()
        }));
        let raw_data_repository = MockRawDataRepository::default();
        let metrics_service = Arc::new(MockTrainingMetricsService::default());
        let service =
            ActivityService::new(activity_repository, raw_data_repository, metrics_service);

        let req = DeleteActivityRequest::new(UserId::default(), ActivityId::from("test"));

        let Err(DeleteActivityError::ActivityDoesNotExist(activity)) =
            service.delete_activity(req).await
        else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(activity, ActivityId::from("test"));
    }

    #[tokio::test]
    async fn test_activity_service_delete_activity_not_owned_by_user() {
        let activity_repository = Arc::new(tokio::sync::Mutex::new(MockActivityRepository {
            get_activity_result: Arc::new(std::sync::Mutex::new(Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::from("another_user".to_string()),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration(0),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
                ActivityTimeseries::new(TimeseriesTime::new(Vec::new()), Vec::new()),
            ))))),
            ..Default::default()
        }));
        let raw_data_repository = MockRawDataRepository::default();
        let metrics_service = Arc::new(MockTrainingMetricsService::default());
        let service =
            ActivityService::new(activity_repository, raw_data_repository, metrics_service);

        let req = DeleteActivityRequest::new(
            "test_user".to_string().into(),
            ActivityId::from("test_activity"),
        );

        let Err(DeleteActivityError::UserDoesNotOwnActivity(user, activity)) =
            service.delete_activity(req).await
        else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(user, "test_user".to_string().into());
        assert_eq!(activity, ActivityId::from("test_activity"));
    }

    #[tokio::test]
    async fn test_activity_service_delete_activity_ok() {
        let activity_repository = Arc::new(tokio::sync::Mutex::new(MockActivityRepository {
            get_activity_result: Arc::new(std::sync::Mutex::new(Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::from("test_user".to_string()),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration(0),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
                ActivityTimeseries::new(TimeseriesTime::new(Vec::new()), Vec::new()),
            ))))),
            ..Default::default()
        }));
        let raw_data_repository = MockRawDataRepository::default();
        let metrics_service = Arc::new(MockTrainingMetricsService::default());
        let service = ActivityService::new(
            activity_repository.clone(),
            raw_data_repository,
            metrics_service,
        );

        let req = DeleteActivityRequest::new(
            "test_user".to_string().into(),
            ActivityId::from("test_activity"),
        );

        let res = service.delete_activity(req).await;
        assert!(res.is_ok());
        let call_list = activity_repository
            .lock()
            .await
            .delete_activity_call_list
            .lock()
            .unwrap()
            .clone();
        assert_eq!(call_list, vec![ActivityId::from("test_activity")]);
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
            DeleteMetricError, GetDefinitionError, GetTrainingMetricValueError,
            GetTrainingMetricsDefinitionsError, SaveTrainingMetricError, UpdateMetricError,
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
                        UserId::default(),
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
                        UserId::default(),
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
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);
        let req = RecomputeMetricRequest::new(UserId::default(), Some(ActivityId::default()));

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

        let res = service.get_training_metrics(&UserId::default()).await;
        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_training_metrics_service_get_metrics_def_without_values() {
        let repository = MockTrainingMetricsRepository {
            get_definitions_result: Arc::new(Mutex::new(Ok(vec![TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::default(),
                TrainingMetricSource::Statistic(ActivityStatistic::Calories),
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            )]))),
            ..Default::default()
        };
        let activity_repository = Arc::new(Mutex::new(MockActivityRepository::default()));
        let service = TrainingMetricService::new(repository, activity_repository);

        let res = service.get_training_metrics(&UserId::default()).await;

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::default(),
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
                UserId::default(),
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

        let res = service.get_training_metrics(&UserId::default()).await;

        assert_eq!(res.len(), 1);
        let (def, value) = res.first().unwrap();
        assert_eq!(
            def,
            &TrainingMetricDefinition::new(
                TrainingMetricId::from("test"),
                UserId::default(),
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
