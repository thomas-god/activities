use std::sync::Arc;

use anyhow::anyhow;
use tokio::sync::Mutex;

use crate::domain::{
    models::{
        UserId,
        activity::{
            Activity, ActivityId, ActivityStatistic, ActivityWithTimeseries, TimeseriesAggregate,
            TimeseriesMetric,
        },
    },
    ports::{
        ActivityRepository, CreateActivityError, CreateActivityRequest, DeleteActivityError,
        DeleteActivityRequest, GetActivityError, GetActivityMetricError, GetAllActivitiesError,
        GetAllActivitiesRequest, IActivityService, ListActivitiesError, ListActivitiesFilters,
        ModifyActivityError, ModifyActivityRequest, RawActivity, RawDataRepository,
        UpdateActivityFeedbackError, UpdateActivityFeedbackRequest, UpdateActivityNutritionError,
        UpdateActivityNutritionRequest, UpdateActivityRpeError, UpdateActivityRpeRequest,
        UpdateActivityWorkoutTypeError, UpdateActivityWorkoutTypeRequest,
    },
};

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
        // Create activity from request
        let id = ActivityId::new();
        let activity = Activity::new(
            id.clone(),
            req.user().clone(),
            None,
            *req.start_time(),
            *req.sport(),
            req.statistics().clone(),
            // RPE, WorkoutType, Nutrition and Feedback are set to None for new activities
            None,
            None,
            None,
            None,
        );
        let activity_with_timeseries =
            ActivityWithTimeseries::new(activity.clone(), req.timeseries().clone());

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

        // Persist raw data
        self.raw_data_repository
            .save_raw_data(&id, req.raw_content())
            .await
            .map_err(|err| {
                anyhow!(err).context(format!("Failed to persist raw data for activity {}", id))
            })?;

        // Persist activity
        activity_repository
            .save_activity(&activity_with_timeseries)
            .await
            .map_err(|err| anyhow!(err).context(format!("Failed to persist activity {}", id)))?;

        Ok(activity)
    }

    async fn list_activities(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> Result<Vec<Activity>, ListActivitiesError> {
        let repository = self.activity_repository.lock().await;
        repository.list_activities(user, filters).await
    }

    async fn list_activities_with_timeseries(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> Result<Vec<ActivityWithTimeseries>, ListActivitiesError> {
        let repository = self.activity_repository.lock().await;
        repository
            .list_activities_with_timeseries(user, filters)
            .await
    }

    async fn get_activity(&self, activity_id: &ActivityId) -> Result<Activity, GetActivityError> {
        let repository = self.activity_repository.lock().await;
        match repository.get_activity(activity_id).await {
            Ok(Some(activity)) => Ok(activity),
            Ok(None) => Err(GetActivityError::ActivityDoesNotExist(activity_id.clone())),
            Err(err) => Err(GetActivityError::Unknown(err)),
        }
    }

    async fn get_activity_with_timeseries(
        &self,
        activity_id: &ActivityId,
    ) -> Result<ActivityWithTimeseries, GetActivityError> {
        let repository = self.activity_repository.lock().await;
        match repository.get_activity_with_timeseries(activity_id).await {
            Ok(Some(activity)) => Ok(activity),
            Ok(None) => Err(GetActivityError::ActivityDoesNotExist(activity_id.clone())),
            Err(err) => Err(GetActivityError::Unknown(err)),
        }
    }

    async fn get_activity_metric(
        &self,
        activity_id: &ActivityId,
        metric: &TimeseriesMetric,
        aggregate: &TimeseriesAggregate,
    ) -> Result<Option<f64>, GetActivityMetricError> {
        let repository = self.activity_repository.lock().await;
        let Some(activity) = repository
            .get_activity_with_timeseries(activity_id)
            .await
            .map_err(|err| anyhow!(err))?
        else {
            return Err(GetActivityMetricError::ActivityDoesNotExist(
                activity_id.clone(),
            ));
        };

        Ok(aggregate.value_from_timeseries(metric, &activity))
    }

    async fn get_activity_statistic(
        &self,
        activity_id: &ActivityId,
        statistic: &ActivityStatistic,
    ) -> Result<Option<f64>, GetActivityMetricError> {
        let repository = self.activity_repository.lock().await;
        let Some(activity) = repository
            .get_activity(activity_id)
            .await
            .map_err(|err| anyhow!(err))?
        else {
            return Err(GetActivityMetricError::ActivityDoesNotExist(
                activity_id.clone(),
            ));
        };

        Ok(activity.statistics().get(statistic).cloned())
    }

    async fn modify_activity(&self, req: ModifyActivityRequest) -> Result<(), ModifyActivityError> {
        let Ok(Some(activity)) = self
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

        if activity.user() != req.user() {
            return Err(ModifyActivityError::UserDoesNotOwnActivity(
                req.user().clone(),
                req.activity().clone(),
            ));
        }

        let _ = self
            .activity_repository
            .lock()
            .await
            .modify_activity_name(req.activity(), req.name().cloned())
            .await;

        Ok(())
    }

    async fn update_activity_rpe(
        &self,
        req: UpdateActivityRpeRequest,
    ) -> Result<(), UpdateActivityRpeError> {
        let Ok(Some(activity)) = self
            .activity_repository
            .lock()
            .await
            .get_activity(req.activity())
            .await
        else {
            return Err(UpdateActivityRpeError::ActivityDoesNotExist(
                req.activity().clone(),
            ));
        };

        if activity.user() != req.user() {
            return Err(UpdateActivityRpeError::UserDoesNotOwnActivity(
                req.user().clone(),
                req.activity().clone(),
            ));
        }

        let _ = self
            .activity_repository
            .lock()
            .await
            .update_activity_rpe(req.activity(), req.rpe().cloned())
            .await;

        Ok(())
    }

    async fn update_activity_workout_type(
        &self,
        req: UpdateActivityWorkoutTypeRequest,
    ) -> Result<(), UpdateActivityWorkoutTypeError> {
        let Ok(Some(activity)) = self
            .activity_repository
            .lock()
            .await
            .get_activity(req.activity())
            .await
        else {
            return Err(UpdateActivityWorkoutTypeError::ActivityDoesNotExist(
                req.activity().clone(),
            ));
        };

        if activity.user() != req.user() {
            return Err(UpdateActivityWorkoutTypeError::UserDoesNotOwnActivity(
                req.user().clone(),
                req.activity().clone(),
            ));
        }

        let _ = self
            .activity_repository
            .lock()
            .await
            .update_activity_workout_type(req.activity(), req.workout_type().cloned())
            .await;

        Ok(())
    }

    async fn update_activity_nutrition(
        &self,
        req: UpdateActivityNutritionRequest,
    ) -> Result<(), UpdateActivityNutritionError> {
        let Ok(Some(activity)) = self
            .activity_repository
            .lock()
            .await
            .get_activity(req.activity())
            .await
        else {
            return Err(UpdateActivityNutritionError::ActivityDoesNotExist(
                req.activity().clone(),
            ));
        };

        if activity.user() != req.user() {
            return Err(UpdateActivityNutritionError::UserDoesNotOwnActivity(
                req.user().clone(),
                req.activity().clone(),
            ));
        }

        let _ = self
            .activity_repository
            .lock()
            .await
            .update_activity_nutrition(req.activity(), req.nutrition().clone())
            .await;

        Ok(())
    }

    async fn update_activity_feedback(
        &self,
        req: UpdateActivityFeedbackRequest,
    ) -> Result<(), UpdateActivityFeedbackError> {
        let Ok(Some(activity)) = self
            .activity_repository
            .lock()
            .await
            .get_activity(req.activity())
            .await
        else {
            return Err(UpdateActivityFeedbackError::ActivityDoesNotExist(
                req.activity().clone(),
            ));
        };

        if activity.user() != req.user() {
            return Err(UpdateActivityFeedbackError::UserDoesNotOwnActivity(
                req.user().clone(),
                req.activity().clone(),
            ));
        }

        let _ = self
            .activity_repository
            .lock()
            .await
            .update_activity_feedback(req.activity(), req.feedback().as_ref().cloned())
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

    async fn get_all_activities(
        &self,
        req: GetAllActivitiesRequest,
    ) -> Result<Vec<RawActivity>, GetAllActivitiesError> {
        self.activity_repository
            .lock()
            .await
            .list_all_raw_activities(req.user())
            .await
            .map_err(|err| GetAllActivitiesError::Unknown(anyhow!(err)))
    }
}

///////////////////////////////////////////////////////////////////
// MOCK IMPLEMENTATIONS FOR TESTING
///////////////////////////////////////////////////////////////////

#[cfg(test)]
pub mod test_utils {

    use mockall::mock;

    use super::*;

    use crate::domain::models::activity::{
        ActivityName, ActivityNaturalKey, ActivityStartTime, ActivityStatistics, Sport,
        TimeseriesMetric,
    };
    use crate::domain::ports::{
        DeleteActivityError, GetActivityMetricError, GetAllActivitiesError,
        GetAllActivitiesRequest, ListActivitiesError, ModifyActivityError, RawActivity,
        SaveActivityError, SimilarActivityError,
    };

    mock! {
        pub ActivityService {}

        impl Clone for  ActivityService {
            fn clone(&self) -> Self;
        }

        impl IActivityService for ActivityService {
            async fn create_activity(
                &self,
                req: CreateActivityRequest,
            ) -> Result<Activity, CreateActivityError>;

            async fn list_activities(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters
            ) -> Result<Vec<Activity>, ListActivitiesError>;

            async fn list_activities_with_timeseries(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters
            ) -> Result<Vec<ActivityWithTimeseries>, ListActivitiesError>;

            async fn get_activity(
                &self,
                activity_id: &ActivityId,
            ) -> Result<Activity, GetActivityError>;

            async fn get_activity_with_timeseries(
                &self,
                activity_id: &ActivityId,
            ) -> Result<ActivityWithTimeseries, GetActivityError>;

            async fn get_activity_metric(
                &self,
                activity_id: &ActivityId,
                metric: &TimeseriesMetric,
                aggregate: &TimeseriesAggregate,
            ) -> Result<Option<f64>, GetActivityMetricError>;

            async fn get_activity_statistic(
                &self,
                activity_id: &ActivityId,
                statistic: &ActivityStatistic,
            ) -> Result<Option<f64>, GetActivityMetricError>;

            async fn modify_activity(
                &self,
                req: ModifyActivityRequest,
            ) -> Result<(), ModifyActivityError>;

            async fn update_activity_rpe(
                &self,
                req: UpdateActivityRpeRequest,
            ) -> Result<(), UpdateActivityRpeError>;

            async fn update_activity_workout_type(
                &self,
                _req: UpdateActivityWorkoutTypeRequest,
            ) -> Result<(), UpdateActivityWorkoutTypeError>;

            async fn update_activity_nutrition(
                &self,
                _req: UpdateActivityNutritionRequest,
            ) -> Result<(), UpdateActivityNutritionError>;

            async fn update_activity_feedback(
                &self,
                _req: UpdateActivityFeedbackRequest,
            ) -> Result<(), UpdateActivityFeedbackError>;

            async fn delete_activity(
                &self,
                req: DeleteActivityRequest,
            ) -> Result<(), DeleteActivityError>;

            async fn get_all_activities(
                &self,
                req: GetAllActivitiesRequest,
            ) -> Result<Vec<RawActivity>, GetAllActivitiesError>;
        }
    }

    impl MockActivityService {
        pub fn test_default() -> Self {
            let mut mock = Self::new();
            mock.default_create_activity();
            mock.default_list_activities();
            mock.default_get_activity();
            mock.default_modify_activity();
            mock.default_update_activity_rpe();
            mock.default_delete_activity();

            mock
        }

        pub fn default_create_activity(&mut self) {
            self.expect_create_activity().returning(|_| {
                Ok(Activity::new(
                    ActivityId::new(),
                    UserId::test_default(),
                    None,
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    Sport::Running,
                    ActivityStatistics::default(),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }
        pub fn default_list_activities(&mut self) {
            self.expect_list_activities().returning(|_, _| Ok(vec![]));
        }

        pub fn default_get_activity(&mut self) {
            self.expect_get_activity().returning(|_| {
                Ok(Activity::new(
                    ActivityId::new(),
                    UserId::test_default(),
                    None,
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    Sport::Running,
                    ActivityStatistics::default(),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }

        pub fn default_modify_activity(&mut self) {
            self.expect_modify_activity().returning(|_| Ok(()));
        }

        pub fn default_update_activity_rpe(&mut self) {
            self.expect_update_activity_rpe().returning(|_| Ok(()));
        }

        pub fn default_update_activity_feedback(&mut self) {
            self.expect_update_activity_feedback().returning(|_| Ok(()));
        }

        pub fn default_delete_activity(&mut self) {
            self.expect_delete_activity().returning(|_| Ok(()));
        }
    }

    mock! {
        pub ActivityRepository {}

        impl Clone for ActivityRepository {
            fn clone(&self) -> Self;
        }

        impl ActivityRepository for ActivityRepository {
            async fn similar_activity_exists(
                &self,
                natural_key: &ActivityNaturalKey,
            ) -> Result<bool, SimilarActivityError>;

            async fn save_activity(
                &self,
                activity: &ActivityWithTimeseries,
            ) -> Result<(), SaveActivityError>;

            async fn list_activities(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters
            ) -> Result<Vec<Activity>, ListActivitiesError>;

            async fn list_all_raw_activities(
                &self,
                user: &UserId,
            ) -> Result<Vec<RawActivity>, ListActivitiesError>;

            async fn list_activities_with_timeseries(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters
            ) -> Result<Vec<ActivityWithTimeseries>, ListActivitiesError>;

            async fn get_activity(
                &self,
                id: &ActivityId,
            ) -> Result<Option<Activity>, anyhow::Error>;

            async fn get_activity_with_timeseries(
                &self,
                id: &ActivityId,
            ) -> Result<Option<ActivityWithTimeseries>, anyhow::Error>;

            async fn modify_activity_name(
                &self,
                id: &ActivityId,
                name: Option<ActivityName>,
            ) -> Result<(), anyhow::Error>;

            async fn update_activity_rpe(
                &self,
                id: &ActivityId,
                rpe: Option<crate::domain::models::activity::ActivityRpe>,
            ) -> Result<(), anyhow::Error>;

            async fn update_activity_workout_type(
                &self,
                id: &ActivityId,
                workout_type: Option<crate::domain::models::activity::WorkoutType>,
            ) ->Result<(), anyhow::Error>;

            async fn update_activity_nutrition(
                &self,
                id: &ActivityId,
                nutrition: Option<crate::domain::models::activity::ActivityNutrition>,
            ) -> Result<(), anyhow::Error>;

            async fn update_activity_feedback(
                &self,
                id: &ActivityId,
                feedback: Option<crate::domain::models::activity::ActivityFeedback>,
            ) -> Result<(), anyhow::Error>;

            async fn delete_activity(
                &self,
                activity: &ActivityId,
            ) -> Result<(), anyhow::Error>;

            async fn get_user_history_date_range(
                &self,
                user: &UserId,
            ) -> Result<Option<crate::domain::ports::DateTimeRange>, anyhow::Error>;
        }

    }
}

#[cfg(test)]
mod tests_activity_service {
    use std::{collections::HashMap, sync::Arc};

    use anyhow::anyhow;
    use mockall::mock;
    use tokio::sync::Mutex;

    use crate::domain::{
        models::{
            UserId,
            activity::{
                ActivityName, ActivityStartTime, ActivityStatistics, ActivityTimeseries, Sport,
            },
        },
        ports::{
            DeleteActivityError, DeleteActivityRequest, GetRawDataError, ModifyActivityError,
            ModifyActivityRequest, RawContent, SaveActivityError, SaveRawDataError,
        },
        services::activity::test_utils::MockActivityRepository,
    };

    use super::*;

    mock! {
        pub RawDataRepository {}

        impl Clone for RawDataRepository {
            fn clone(&self) -> Self;
        }

        impl RawDataRepository for RawDataRepository {
            async fn save_raw_data(
                &self,
                _activity_id: &ActivityId,
                _content: RawContent,
            ) -> Result<(), SaveRawDataError>;

            async fn get_raw_data(
                &self,
                _activity_id: &ActivityId,
            ) -> Result<RawContent, GetRawDataError>;
        }
    }

    fn default_activity_request() -> CreateActivityRequest {
        let sport = Sport::Running;
        let start_time = ActivityStartTime::from_timestamp(3600).unwrap();
        let content = RawContent::new("fit".to_string(), vec![1, 2, 3]);
        let statistics = ActivityStatistics::default();
        let timeseries = ActivityTimeseries::default();
        CreateActivityRequest::new(
            UserId::test_default(),
            sport,
            start_time,
            statistics,
            timeseries,
            content,
        )
    }

    #[tokio::test]
    async fn test_service_create_activity_err_if_similar_activity_exists() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_similar_activity_exists()
            .returning(|_| Ok(true));

        let raw_data_repository = MockRawDataRepository::new();

        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

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
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_similar_activity_exists()
            .returning(|_| Ok(false));
        activity_repository
            .expect_save_activity()
            .times(1)
            .returning(|_| Ok(()));
        let mut raw_data_repository = MockRawDataRepository::new();
        raw_data_repository
            .expect_save_raw_data()
            .returning(|_, __| Ok(()));

        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = default_activity_request();

        let res = service.create_activity(req).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_service_create_activity_save_activity_error() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_similar_activity_exists()
            .returning(|_| Ok(false));
        activity_repository
            .expect_save_activity()
            .returning(|_| Err(SaveActivityError::Unknown(anyhow!("an error occured"))));

        let mut raw_data_repository = MockRawDataRepository::new();
        raw_data_repository
            .expect_save_raw_data()
            .returning(|_, _| Ok(()));
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = default_activity_request();

        let res = service.create_activity(req).await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_service_create_activity_raw_data_error_do_not_save_activity() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_similar_activity_exists()
            .returning(|_| Ok(false));
        activity_repository.expect_save_activity().times(0);

        let mut raw_data_repository = MockRawDataRepository::new();
        raw_data_repository
            .expect_save_raw_data()
            .returning(|_, _| Err(SaveRawDataError::Unknown));

        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = default_activity_request();

        let res = service.create_activity(req).await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_activity_service_modify_activity_not_found() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activity()
            .returning(|_| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req =
            ModifyActivityRequest::new(UserId::test_default(), ActivityId::from("test"), None);

        let Err(ModifyActivityError::ActivityDoesNotExist(activity)) =
            service.modify_activity(req).await
        else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(activity, ActivityId::from("test"));
    }

    #[tokio::test]
    async fn test_activity_service_modify_activity_not_owned_by_user() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::from("another_user".to_string()),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req =
            ModifyActivityRequest::new("test_user".into(), ActivityId::from("test_activity"), None);

        let Err(ModifyActivityError::UserDoesNotOwnActivity(user, activity)) =
            service.modify_activity(req).await
        else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(user, "test_user".to_string().into());
        assert_eq!(activity, ActivityId::from("test_activity"));
    }

    #[tokio::test]
    async fn test_activity_service_modify_activity_ok() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });
        activity_repository
            .expect_modify_activity_name()
            .withf(|id, name| {
                *id == ActivityId::from("test")
                    && *name == Some(ActivityName::new("new name".to_string()))
            })
            .returning(|_, __| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = ModifyActivityRequest::new(
            UserId::test_default(),
            ActivityId::from("test"),
            Some(ActivityName::new("new name".to_string())),
        );

        let res = service.modify_activity(req).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_rpe_ok() {
        use crate::domain::models::activity::ActivityRpe;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });
        activity_repository
            .expect_update_activity_rpe()
            .withf(|id, rpe| *id == ActivityId::from("test") && *rpe == Some(ActivityRpe::Five))
            .returning(|_, _| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = UpdateActivityRpeRequest::new(
            UserId::test_default(),
            ActivityId::from("test"),
            Some(ActivityRpe::Five),
        );

        let res = service.update_activity_rpe(req).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_rpe_not_found() {
        use crate::domain::models::activity::ActivityRpe;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activity()
            .return_once(|_| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = UpdateActivityRpeRequest::new(
            UserId::test_default(),
            ActivityId::from("test"),
            Some(ActivityRpe::Five),
        );

        let Err(UpdateActivityRpeError::ActivityDoesNotExist(activity_id)) =
            service.update_activity_rpe(req).await
        else {
            unreachable!("Should have returned an error")
        };
        assert_eq!(activity_id, ActivityId::from("test"));
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_rpe_wrong_user() {
        use crate::domain::models::activity::ActivityRpe;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                "other_user".into(),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = UpdateActivityRpeRequest::new(
            UserId::test_default(),
            ActivityId::from("test_activity"),
            Some(ActivityRpe::Five),
        );

        let Err(UpdateActivityRpeError::UserDoesNotOwnActivity(user, activity)) =
            service.update_activity_rpe(req).await
        else {
            unreachable!("Should have returned an error")
        };
        assert_eq!(user, UserId::test_default());
        assert_eq!(activity, ActivityId::from("test_activity"));
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_feedback_ok() {
        use crate::domain::models::activity::ActivityFeedback;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });
        activity_repository
            .expect_update_activity_feedback()
            .withf(|id, feedback| {
                *id == ActivityId::from("test")
                    && *feedback == Some(ActivityFeedback::from("Great ride!"))
            })
            .returning(|_, _| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = UpdateActivityFeedbackRequest::new(
            UserId::test_default(),
            ActivityId::from("test"),
            Some(ActivityFeedback::from("Great ride!")),
        );

        let res = service.update_activity_feedback(req).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_feedback_not_found() {
        use crate::domain::models::activity::ActivityFeedback;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activity()
            .return_once(|_| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = UpdateActivityFeedbackRequest::new(
            UserId::test_default(),
            ActivityId::from("test"),
            Some(ActivityFeedback::from("Great ride!")),
        );

        let Err(UpdateActivityFeedbackError::ActivityDoesNotExist(activity_id)) =
            service.update_activity_feedback(req).await
        else {
            unreachable!("Should have returned an error")
        };
        assert_eq!(activity_id, ActivityId::from("test"));
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_feedback_wrong_user() {
        use crate::domain::models::activity::ActivityFeedback;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                "other_user".into(),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = UpdateActivityFeedbackRequest::new(
            UserId::test_default(),
            ActivityId::from("test_activity"),
            Some(ActivityFeedback::from("Great ride!")),
        );

        let Err(UpdateActivityFeedbackError::UserDoesNotOwnActivity(user, activity)) =
            service.update_activity_feedback(req).await
        else {
            unreachable!("Should have returned an error")
        };
        assert_eq!(user, UserId::test_default());
        assert_eq!(activity, ActivityId::from("test_activity"));
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_workout_type_ok() {
        use crate::domain::models::activity::WorkoutType;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Running,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });
        activity_repository
            .expect_update_activity_workout_type()
            .withf(|id, wt| *id == ActivityId::from("test") && *wt == Some(WorkoutType::Intervals))
            .returning(|_, _| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = UpdateActivityWorkoutTypeRequest::new(
            UserId::test_default(),
            ActivityId::from("test"),
            Some(WorkoutType::Intervals),
        );

        let res = service.update_activity_workout_type(req).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_workout_type_not_found() {
        use crate::domain::models::activity::WorkoutType;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activity()
            .return_once(|_| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = UpdateActivityWorkoutTypeRequest::new(
            UserId::test_default(),
            ActivityId::from("test"),
            Some(WorkoutType::Tempo),
        );

        let Err(UpdateActivityWorkoutTypeError::ActivityDoesNotExist(activity_id)) =
            service.update_activity_workout_type(req).await
        else {
            unreachable!("Should have returned an error")
        };
        assert_eq!(activity_id, ActivityId::from("test"));
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_workout_type_wrong_user() {
        use crate::domain::models::activity::WorkoutType;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                "other_user".into(),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Running,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = UpdateActivityWorkoutTypeRequest::new(
            UserId::test_default(),
            ActivityId::from("test_activity"),
            Some(WorkoutType::LongRun),
        );

        let Err(UpdateActivityWorkoutTypeError::UserDoesNotOwnActivity(user, activity)) =
            service.update_activity_workout_type(req).await
        else {
            unreachable!("Should have returned an error")
        };
        assert_eq!(user, UserId::test_default());
        assert_eq!(activity, ActivityId::from("test_activity"));
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_nutrition_ok() {
        use crate::domain::models::activity::{ActivityNutrition, BonkStatus};

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Running,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });
        activity_repository
            .expect_update_activity_nutrition()
            .returning(|_, _| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let nutrition =
            ActivityNutrition::new(BonkStatus::Bonked, Some("2 gels, 500ml water".to_string()));
        let req = UpdateActivityNutritionRequest::new(
            UserId::test_default(),
            ActivityId::from("test_activity"),
            Some(nutrition),
        );

        let res = service.update_activity_nutrition(req).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_nutrition_not_found() {
        use crate::domain::models::activity::{ActivityNutrition, BonkStatus};

        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activity()
            .return_once(|_| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let nutrition = ActivityNutrition::new(BonkStatus::None, None);
        let req = UpdateActivityNutritionRequest::new(
            UserId::test_default(),
            ActivityId::from("test_activity"),
            Some(nutrition),
        );

        let Err(UpdateActivityNutritionError::ActivityDoesNotExist(activity)) =
            service.update_activity_nutrition(req).await
        else {
            unreachable!("Should have returned an error")
        };
        assert_eq!(activity, ActivityId::from("test_activity"));
    }

    #[tokio::test]
    async fn test_activity_service_update_activity_nutrition_wrong_user() {
        use crate::domain::models::activity::{ActivityNutrition, BonkStatus};

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                "other_user".into(),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Running,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let nutrition =
            ActivityNutrition::new(BonkStatus::Bonked, Some("Forgot to eat".to_string()));
        let req = UpdateActivityNutritionRequest::new(
            UserId::test_default(),
            ActivityId::from("test_activity"),
            Some(nutrition),
        );

        let Err(UpdateActivityNutritionError::UserDoesNotOwnActivity(user, activity)) =
            service.update_activity_nutrition(req).await
        else {
            unreachable!("Should have returned an error")
        };
        assert_eq!(user, UserId::test_default());
        assert_eq!(activity, ActivityId::from("test_activity"));
    }

    #[tokio::test]
    async fn test_activity_service_delete_activity_not_found() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activity()
            .return_once(|_| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = DeleteActivityRequest::new(UserId::test_default(), ActivityId::from("test"));

        let Err(DeleteActivityError::ActivityDoesNotExist(activity)) =
            service.delete_activity(req).await
        else {
            unreachable!("Should have returned an err")
        };
        assert_eq!(activity, ActivityId::from("test"));
    }

    #[tokio::test]
    async fn test_activity_service_delete_activity_not_owned_by_user() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().return_once(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::from("another_user".to_string()),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

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
        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::from("test_user".to_string()),
                None,
                ActivityStartTime::from_timestamp(0).unwrap(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
                None,
                None,
                None,
                None,
            )))
        });
        activity_repository
            .expect_delete_activity()
            .withf(|id| *id == ActivityId::from("test_activity"))
            .returning(|_| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();

        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = DeleteActivityRequest::new(
            "test_user".to_string().into(),
            ActivityId::from("test_activity"),
        );

        let res = service.delete_activity(req).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_activity_service_delete_activity_does_not_propagate_on_error() {
        let user_id = UserId::from("test_user".to_string());
        let activity_id = ActivityId::from("test_activity");

        let mut activity_repository = MockActivityRepository::new();
        // Activity doesn't exist
        activity_repository
            .expect_get_activity()
            .return_once(move |_| Ok(None));
        let raw_data_repository = MockRawDataRepository::default();

        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let req = DeleteActivityRequest::new(user_id.clone(), activity_id.clone());

        let res = service.delete_activity(req).await;
        assert!(res.is_err());

        // Give any potential spawned task a chance to run (there shouldn't be any)
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_get_all_activities_ok() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_list_all_raw_activities()
            .returning(|_| Ok(Vec::new()));
        let raw_data_repository = MockRawDataRepository::default();

        let service = ActivityService::new(
            Arc::new(Mutex::new(activity_repository)),
            raw_data_repository,
        );

        let user = UserId::test_default();

        let res = service
            .get_all_activities(GetAllActivitiesRequest::new(user))
            .await
            .unwrap();

        assert!(res.is_empty());
    }

    mod test_activity_service_get_activity_metric {
        use crate::domain::models::activity::{
            ActiveTime, ActivityStatistic, Timeseries, TimeseriesActiveTime, TimeseriesTime,
            TimeseriesValue,
        };

        use super::*;

        fn default_activity() -> ActivityWithTimeseries {
            ActivityWithTimeseries::new(
                Activity::new(
                    ActivityId::from("test_activity"),
                    UserId::from("test_user".to_string()),
                    None,
                    ActivityStartTime::from_timestamp(0).unwrap(),
                    Sport::Cycling,
                    ActivityStatistics::new(HashMap::from([(ActivityStatistic::Duration, 1200.)])),
                    None,
                    None,
                    None,
                    None,
                ),
                ActivityTimeseries::new(
                    TimeseriesTime::new(vec![0, 1, 2]),
                    TimeseriesActiveTime::new(vec![
                        ActiveTime::Running(0),
                        ActiveTime::Running(1),
                        ActiveTime::Running(2),
                    ]),
                    vec![],
                    vec![Timeseries::new(
                        TimeseriesMetric::Cadence,
                        vec![
                            Some(TimeseriesValue::Int(10)),
                            Some(TimeseriesValue::Int(20)),
                            Some(TimeseriesValue::Int(30)),
                        ],
                    )],
                )
                .unwrap(),
            )
        }

        #[tokio::test]
        async fn test_activity_does_not_have_timeseries_for_this_metric() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activity_with_timeseries()
                .returning(|_| Ok(Some(default_activity())));

            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(
                Arc::new(Mutex::new(activity_repository)),
                raw_data_repository,
            );

            assert!(
                service
                    .get_activity_metric(
                        &ActivityId::from("test_activity"),
                        &TimeseriesMetric::Power,
                        &TimeseriesAggregate::Average
                    )
                    .await
                    .unwrap()
                    .is_none()
            )
        }

        #[tokio::test]
        async fn test_activity_has_timeseries_for_this_metric() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activity_with_timeseries()
                .returning(|_| Ok(Some(default_activity())));

            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(
                Arc::new(Mutex::new(activity_repository)),
                raw_data_repository,
            );

            assert!(
                service
                    .get_activity_metric(
                        &ActivityId::from("test_activity"),
                        &TimeseriesMetric::Cadence,
                        &TimeseriesAggregate::Average
                    )
                    .await
                    .unwrap()
                    .is_some()
            )
        }

        #[tokio::test]
        async fn test_activity_does_not_have_this_statistics() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activity()
                .returning(|_| Ok(Some(default_activity().activity().clone())));

            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(
                Arc::new(Mutex::new(activity_repository)),
                raw_data_repository,
            );

            assert!(
                service
                    .get_activity_statistic(
                        &ActivityId::from("test_activity"),
                        &ActivityStatistic::Calories,
                    )
                    .await
                    .unwrap()
                    .is_none()
            )
        }

        #[tokio::test]
        async fn test_activity_has_this_statistics() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activity()
                .returning(|_| Ok(Some(default_activity().activity().clone())));

            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(
                Arc::new(Mutex::new(activity_repository)),
                raw_data_repository,
            );

            assert!(
                service
                    .get_activity_statistic(
                        &ActivityId::from("test_activity"),
                        &ActivityStatistic::Duration,
                    )
                    .await
                    .unwrap()
                    .is_some()
            )
        }
    }
}
