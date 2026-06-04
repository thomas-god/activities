use anyhow::anyhow;

use crate::domain::{
    models::{
        UserId,
        activity::{
            Activity, ActivityId, ActivityMetricV2, ActivityMetricsV2, ActivityWithParsedData,
        },
    },
    ports::activity::{
        ActivityRepository, CreateActivityError, CreateActivityRequest, DeleteActivityError,
        DeleteActivityRequest, GetActivityError, GetAllActivitiesError, GetAllActivitiesRequest,
        GetRawActivityError, GetRawActivityRequest, IActivityService, ListActivitiesError,
        ListActivitiesFilters, ModifyActivityError, ModifyActivityRequest, RawActivity,
        RawDataRepository, UpdateActivityFeedbackError, UpdateActivityFeedbackRequest,
        UpdateActivityNutritionError, UpdateActivityNutritionRequest, UpdateActivityRpeError,
        UpdateActivityRpeRequest, UpdateActivityWorkoutTypeError, UpdateActivityWorkoutTypeRequest,
    },
};

#[derive(Debug, Clone)]
pub struct ActivityService<AR, RDR>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
{
    activity_repository: AR,
    raw_data_repository: RDR,
}

impl<AR, RDR> ActivityService<AR, RDR>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
{
    pub fn new(activity_repository: AR, raw_data_repository: RDR) -> Self {
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
        let activity = Activity::new_empty(
            id.clone(),
            req.user().clone(),
            *req.start_time(),
            *req.duration(),
            *req.sport(),
            req.statistics().clone(),
        );

        if self
            .activity_repository
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
        self.activity_repository
            .save_activity(&activity)
            .await
            .map_err(|err| anyhow!(err).context(format!("Failed to persist activity {}", id)))?;

        Ok(activity)
    }

    async fn list_activities(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> Result<Vec<Activity>, ListActivitiesError> {
        self.activity_repository
            .list_activities(user, filters)
            .await
    }

    async fn list_activities_with_parsed_data(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> Result<Vec<ActivityWithParsedData>, ListActivitiesError> {
        self.activity_repository
            .list_activities_with_parsed_data(user, filters)
            .await
    }

    async fn list_activities_with_metrics(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
        metrics: &[ActivityMetricV2],
    ) -> Result<Vec<(Activity, ActivityMetricsV2)>, ListActivitiesError> {
        let mut activities = self
            .activity_repository
            .get_activities_with_metrics(user, filters, metrics)
            .await?;

        for (activity, activity_metrics) in activities.iter_mut() {
            let missing_metrics = metrics
                .iter()
                .filter(|metric| !activity_metrics.contains_key(metric))
                .collect::<Vec<_>>();

            if missing_metrics.is_empty() {
                continue;
            }

            let Some(activity_with_parsed_data) = self
                .activity_repository
                .get_activity_with_parsed_data(activity.id())
                .await
                .map_err(|err| ListActivitiesError::Unknown(anyhow!(err)))?
            else {
                continue;
            };

            for metric in missing_metrics {
                let value = metric.compute_value(&activity_with_parsed_data);
                activity_metrics.insert(*metric, value);

                self.activity_repository
                    .update_activity_metric(activity.id(), metric, &value)
                    .await
                    .unwrap();
            }
        }

        Ok(activities)
    }

    async fn list_activities_with_metrics_and_parsed_data(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
        metrics: &[ActivityMetricV2],
    ) -> Result<Vec<(ActivityWithParsedData, ActivityMetricsV2)>, ListActivitiesError> {
        let activities = self
            .list_activities_with_metrics(user, filters, metrics)
            .await?;

        let mut res = Vec::new();
        for (activity, metrics) in activities {
            let Ok(activity_with_parsed_data) =
                self.get_activity_with_parsed_data(activity.id()).await
            else {
                continue;
            };
            res.push((activity_with_parsed_data, metrics));
        }

        Ok(res)
    }

    async fn get_activity_with_parsed_data(
        &self,
        activity_id: &ActivityId,
    ) -> Result<ActivityWithParsedData, GetActivityError> {
        match self
            .activity_repository
            .get_activity_with_parsed_data(activity_id)
            .await
        {
            Ok(Some(activity)) => Ok(activity),
            Ok(None) => Err(GetActivityError::ActivityDoesNotExist(activity_id.clone())),
            Err(err) => Err(err),
        }
    }

    async fn get_activity_with_metrics_and_parsed_data(
        &self,
        activity_id: &ActivityId,
        metrics: &[ActivityMetricV2],
    ) -> Result<(ActivityWithParsedData, ActivityMetricsV2), GetActivityError> {
        let (activity, metrics) = match self
            .activity_repository
            .get_activity_with_metrics(activity_id, metrics)
            .await
        {
            Ok(Some(res)) => res,
            Ok(None) => return Err(GetActivityError::ActivityDoesNotExist(activity_id.clone())),
            Err(err) => return Err(err),
        };

        let activity = match self
            .activity_repository
            .get_activity_with_parsed_data(activity.id())
            .await
        {
            Ok(Some(activity)) => activity,
            Ok(None) => return Err(GetActivityError::ActivityDoesNotExist(activity_id.clone())),
            Err(err) => return Err(err),
        };

        Ok((activity, metrics))
    }

    async fn modify_activity(&self, req: ModifyActivityRequest) -> Result<(), ModifyActivityError> {
        let Ok(Some(activity)) = self.activity_repository.get_activity(req.activity()).await else {
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
            .modify_activity_name(req.activity(), req.name().cloned())
            .await;

        Ok(())
    }

    async fn update_activity_rpe(
        &self,
        req: UpdateActivityRpeRequest,
    ) -> Result<(), UpdateActivityRpeError> {
        let Ok(Some(activity)) = self.activity_repository.get_activity(req.activity()).await else {
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
            .update_activity_rpe(req.activity(), req.rpe().cloned())
            .await;

        Ok(())
    }

    async fn update_activity_workout_type(
        &self,
        req: UpdateActivityWorkoutTypeRequest,
    ) -> Result<(), UpdateActivityWorkoutTypeError> {
        let Ok(Some(activity)) = self.activity_repository.get_activity(req.activity()).await else {
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
            .update_activity_workout_type(req.activity(), req.workout_type().cloned())
            .await;

        Ok(())
    }

    async fn update_activity_nutrition(
        &self,
        req: UpdateActivityNutritionRequest,
    ) -> Result<(), UpdateActivityNutritionError> {
        let Ok(Some(activity)) = self.activity_repository.get_activity(req.activity()).await else {
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
            .update_activity_nutrition(req.activity(), req.nutrition().clone())
            .await;

        Ok(())
    }

    async fn update_activity_feedback(
        &self,
        req: UpdateActivityFeedbackRequest,
    ) -> Result<(), UpdateActivityFeedbackError> {
        let Ok(Some(activity)) = self.activity_repository.get_activity(req.activity()).await else {
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
            .update_activity_feedback(req.activity(), req.feedback().as_ref().cloned())
            .await;

        Ok(())
    }

    async fn delete_activity(&self, req: DeleteActivityRequest) -> Result<(), DeleteActivityError> {
        let Ok(Some(activity)) = self.activity_repository.get_activity(req.activity()).await else {
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
            .delete_activity(req.activity())
            .await?;

        Ok(())
    }

    async fn get_raw_activity(
        &self,
        req: GetRawActivityRequest,
    ) -> Result<RawActivity, GetRawActivityError> {
        self.activity_repository
            .get_raw_activity(req.user(), req.activity())
            .await
    }

    async fn get_all_raw_activities(
        &self,
        req: GetAllActivitiesRequest,
    ) -> Result<Vec<RawActivity>, GetAllActivitiesError> {
        self.activity_repository
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
        ActivityDuration, ActivityName, ActivityNaturalKey, ActivityStartTime, ActivityStatistics,
        Sport,
    };
    use crate::domain::ports::activity::{
        DeleteActivityError, GetAllActivitiesError, GetAllActivitiesRequest, GetRawActivityError,
        GetRawActivityRequest, ListActivitiesError, ModifyActivityError, RawActivity,
        SaveActivityError, SimilarActivityError, UpdateActivityMetricError,
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

            async fn list_activities_with_parsed_data(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters
            ) -> Result<Vec<ActivityWithParsedData>, ListActivitiesError>;

            async fn list_activities_with_metrics(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters,
                metrics: &[ActivityMetricV2],
            ) -> Result<Vec<(Activity, ActivityMetricsV2)>, ListActivitiesError>;

            async fn list_activities_with_metrics_and_parsed_data(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters,
                metrics: &[ActivityMetricV2],
            ) -> Result<Vec<(ActivityWithParsedData, ActivityMetricsV2)>, ListActivitiesError>;

            async fn get_activity_with_parsed_data(
                &self,
                activity_id: &ActivityId,
            ) -> Result<ActivityWithParsedData, GetActivityError>;

            async fn get_activity_with_metrics_and_parsed_data(
                &self,
                activity_id: &ActivityId,
                metrics: &[ActivityMetricV2],
            ) -> Result<(ActivityWithParsedData, ActivityMetricsV2), GetActivityError>;

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

            async fn get_raw_activity(
                &self,
                req: GetRawActivityRequest,
            ) -> Result<RawActivity, GetRawActivityError>;

            async fn get_all_raw_activities(
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
            mock.default_modify_activity();
            mock.default_update_activity_rpe();
            mock.default_delete_activity();

            mock
        }

        pub fn default_create_activity(&mut self) {
            self.expect_create_activity().returning(|_| {
                Ok(Activity::new_empty(
                    ActivityId::new(),
                    UserId::test_default(),
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    ActivityDuration::default(),
                    Sport::Running,
                    ActivityStatistics::default(),
                ))
            });
        }
        pub fn default_list_activities(&mut self) {
            self.expect_list_activities().returning(|_, _| Ok(vec![]));
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
                activity: &Activity,
            ) -> Result<(), SaveActivityError>;

            async fn list_activities(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters
            ) -> Result<Vec<Activity>, ListActivitiesError>;

            async fn get_raw_activity(
                &self,
                user: &UserId,
                activity: &ActivityId,
            ) -> Result<RawActivity, GetRawActivityError>;

            async fn list_all_raw_activities(
                &self,
                user: &UserId,
            ) -> Result<Vec<RawActivity>, ListActivitiesError>;

            async fn list_activities_with_parsed_data(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters
            ) -> Result<Vec<ActivityWithParsedData>, ListActivitiesError>;

            async fn update_activity_metric(
                &self,
                activity: &ActivityId,
                metric: &ActivityMetricV2,
                value: &Option<f64>,
            ) -> Result<(), UpdateActivityMetricError>;

            async fn get_activities_with_metrics(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters,
                metrics: &[ActivityMetricV2],
            ) -> Result<Vec<(Activity, ActivityMetricsV2)>, ListActivitiesError>;

            async fn get_activity(
                &self,
                id: &ActivityId,
            ) -> Result<Option<Activity>, GetActivityError>;

            async fn get_activity_with_metrics(
                &self,
                id: &ActivityId,
                metrics: &[ActivityMetricV2],
            ) -> Result<Option<(Activity, ActivityMetricsV2)>, GetActivityError>;

            async fn get_activity_with_parsed_data(
                &self,
                id: &ActivityId,
            ) -> Result<Option<ActivityWithParsedData>, GetActivityError>;

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
    use std::collections::HashMap;

    use anyhow::anyhow;
    use mockall::mock;

    use crate::domain::{
        models::{
            UserId,
            activity::{
                ActivityDuration, ActivityName, ActivityStartTime, ActivityStatistics,
                ActivityTimeseries, Sport,
            },
        },
        ports::activity::{
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
        let duration = ActivityDuration::default();
        let content = RawContent::new("fit".to_string(), vec![1, 2, 3]);
        let statistics = ActivityStatistics::default();
        let timeseries = ActivityTimeseries::default();
        CreateActivityRequest::new(
            UserId::test_default(),
            sport,
            start_time,
            duration,
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

        let service = ActivityService::new(activity_repository, raw_data_repository);

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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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

        let service = ActivityService::new(activity_repository, raw_data_repository);

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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                UserId::from("another_user".to_string()),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
            )))
        });
        activity_repository
            .expect_update_activity_rpe()
            .withf(|id, rpe| *id == ActivityId::from("test") && *rpe == Some(ActivityRpe::Five))
            .returning(|_, _| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                "other_user".into(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                "other_user".into(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Running,
                ActivityStatistics::new(HashMap::new()),
            )))
        });
        activity_repository
            .expect_update_activity_workout_type()
            .withf(|id, wt| *id == ActivityId::from("test") && *wt == Some(WorkoutType::Intervals))
            .returning(|_, _| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                "other_user".into(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Running,
                ActivityStatistics::new(HashMap::new()),
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Running,
                ActivityStatistics::new(HashMap::new()),
            )))
        });
        activity_repository
            .expect_update_activity_nutrition()
            .returning(|_, _| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                "other_user".into(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Running,
                ActivityStatistics::new(HashMap::new()),
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                UserId::from("another_user".to_string()),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

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
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                UserId::from("test_user".to_string()),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
                ActivityStatistics::new(HashMap::new()),
            )))
        });
        activity_repository
            .expect_delete_activity()
            .withf(|id| *id == ActivityId::from("test_activity"))
            .returning(|_| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();

        let service = ActivityService::new(activity_repository, raw_data_repository);

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

        let service = ActivityService::new(activity_repository, raw_data_repository);

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

        let service = ActivityService::new(activity_repository, raw_data_repository);

        let user = UserId::test_default();

        let res = service
            .get_all_raw_activities(GetAllActivitiesRequest::new(user))
            .await
            .unwrap();

        assert!(res.is_empty());
    }

    mod test_activity_service_list_activities_with_metrics_v2 {
        use mockall::predicate::eq;

        use crate::domain::models::activity::{
            ActiveTime, ActivityId, ActivityStatistic, Timeseries, TimeseriesActiveTime,
            TimeseriesMetric, TimeseriesTime, TimeseriesValue,
        };

        use super::*;

        fn default_activity() -> ActivityWithParsedData {
            ActivityWithParsedData::new(
                Activity::new_empty(
                    ActivityId::from("test_activity"),
                    UserId::from("test_user".to_string()),
                    ActivityStartTime::from_timestamp(0).unwrap(),
                    ActivityDuration::from(1200.),
                    Sport::Cycling,
                    ActivityStatistics::new(HashMap::from([(ActivityStatistic::Duration, 1200.)])),
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
                ActivityStatistics::new(HashMap::from([(ActivityStatistic::Duration, 1200.)])),
            )
        }

        #[tokio::test]
        async fn test_no_activities() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| Ok(vec![]));
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(activity_repository, raw_data_repository);
            let metrics = vec![ActivityMetricV2::Calories, ActivityMetricV2::AvgHeartRate];
            let res = service
                .list_activities_with_metrics(
                    &UserId::test_default(),
                    &ListActivitiesFilters::empty(),
                    &metrics,
                )
                .await
                .unwrap();

            assert!(res.is_empty());
        }

        #[tokio::test]
        async fn test_activity_with_requested_metrics_values() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| {
                    Ok(vec![(
                        default_activity().activity().clone(),
                        ActivityMetricsV2::new(HashMap::from([
                            (ActivityMetricV2::Calories, Some(1.)),
                            (ActivityMetricV2::AvgHeartRate, Some(12.3)),
                        ])),
                    )])
                });
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(activity_repository, raw_data_repository);
            let metrics = vec![ActivityMetricV2::Calories, ActivityMetricV2::AvgHeartRate];
            let res = service
                .list_activities_with_metrics(
                    &UserId::test_default(),
                    &ListActivitiesFilters::empty(),
                    &metrics,
                )
                .await
                .unwrap();

            assert_eq!(res.len(), 1);
            let (_activity, metrics) = res.first().unwrap();
            assert_eq!(metrics.get(&ActivityMetricV2::Calories).unwrap(), &Some(1.));
            assert_eq!(
                metrics.get(&ActivityMetricV2::AvgHeartRate).unwrap(),
                &Some(12.3)
            );
        }

        #[tokio::test]
        async fn test_activity_with_requested_metrics_values_some_are_none() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| {
                    Ok(vec![(
                        default_activity().activity().clone(),
                        ActivityMetricsV2::new(HashMap::from([
                            (ActivityMetricV2::Calories, Some(1.)),
                            (ActivityMetricV2::AvgHeartRate, None),
                        ])),
                    )])
                });
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(activity_repository, raw_data_repository);
            let metrics = vec![ActivityMetricV2::Calories, ActivityMetricV2::AvgHeartRate];
            let res = service
                .list_activities_with_metrics(
                    &UserId::test_default(),
                    &ListActivitiesFilters::empty(),
                    &metrics,
                )
                .await
                .unwrap();

            assert_eq!(res.len(), 1);
            let (_activity, metrics) = res.first().unwrap();
            assert_eq!(metrics.get(&ActivityMetricV2::Calories).unwrap(), &Some(1.));
            assert_eq!(metrics.get(&ActivityMetricV2::AvgHeartRate).unwrap(), &None);
        }

        #[tokio::test]
        async fn test_activity_with_missing_requested_metrics_values_and_missing_in_timeseries() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| {
                    Ok(vec![(
                        default_activity().activity().clone(),
                        ActivityMetricsV2::new(HashMap::from([
                            (ActivityMetricV2::Calories, Some(1.)),
                            // ActivityMetricV2::AvgHeartRate is missing and no HR values in timeseries
                        ])),
                    )])
                });
            activity_repository
                .expect_get_activity_with_parsed_data()
                .times(1)
                .with(eq(ActivityId::from("test_activity")))
                .returning(|_| Ok(Some(default_activity())));
            activity_repository
                .expect_update_activity_metric()
                .times(1)
                .with(
                    eq(ActivityId::from("test_activity")),
                    eq(ActivityMetricV2::AvgHeartRate),
                    eq(None),
                )
                .returning(|_, _, _| Ok(()));
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(activity_repository, raw_data_repository);
            let metrics = vec![ActivityMetricV2::Calories, ActivityMetricV2::AvgHeartRate];
            let res = service
                .list_activities_with_metrics(
                    &UserId::test_default(),
                    &ListActivitiesFilters::empty(),
                    &metrics,
                )
                .await
                .unwrap();

            assert_eq!(res.len(), 1);
            let (_activity, metrics) = res.first().unwrap();
            assert_eq!(metrics.get(&ActivityMetricV2::Calories).unwrap(), &Some(1.));
            assert_eq!(metrics.get(&ActivityMetricV2::AvgHeartRate).unwrap(), &None);
        }

        #[tokio::test]
        async fn test_activity_with_missing_requested_metrics_values_and_present_in_timeseries() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| {
                    Ok(vec![(
                        default_activity().activity().clone(),
                        ActivityMetricsV2::new(HashMap::from([
                            (ActivityMetricV2::Calories, Some(1.)),
                            // ActivityMetricV2::MaxCadence is missing with cadence values in timeseries
                        ])),
                    )])
                });
            activity_repository
                .expect_get_activity_with_parsed_data()
                .times(1)
                .with(eq(ActivityId::from("test_activity")))
                .returning(|_| Ok(Some(default_activity())));
            activity_repository
                .expect_update_activity_metric()
                .times(1)
                .with(
                    eq(ActivityId::from("test_activity")),
                    eq(ActivityMetricV2::MaxCadence),
                    eq(Some(30.)),
                )
                .returning(|_, _, _| Ok(()));
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(activity_repository, raw_data_repository);
            let metrics = vec![ActivityMetricV2::Calories, ActivityMetricV2::MaxCadence];
            let res = service
                .list_activities_with_metrics(
                    &UserId::test_default(),
                    &ListActivitiesFilters::empty(),
                    &metrics,
                )
                .await
                .unwrap();

            assert_eq!(res.len(), 1);
            let (_activity, metrics) = res.first().unwrap();
            assert_eq!(metrics.get(&ActivityMetricV2::Calories).unwrap(), &Some(1.));
            assert_eq!(
                metrics.get(&ActivityMetricV2::MaxCadence).unwrap(),
                &Some(30.)
            );
        }

        #[tokio::test]
        async fn test_activity_with_parsed_data_missing() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| {
                    Ok(vec![(
                        default_activity().activity().clone(),
                        ActivityMetricsV2::new(HashMap::from([
                            (ActivityMetricV2::Calories, Some(1.)),
                            // ActivityMetricV2::MaxCadence is missing and we can't find the activity's timeseries
                        ])),
                    )])
                });
            activity_repository
                .expect_get_activity_with_parsed_data()
                .times(1)
                .with(eq(ActivityId::from("test_activity")))
                .returning(|_| Ok(None));
            activity_repository.expect_update_activity_metric().times(0);
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(activity_repository, raw_data_repository);
            let metrics = vec![ActivityMetricV2::Calories, ActivityMetricV2::MaxCadence];
            let res = service
                .list_activities_with_metrics(
                    &UserId::test_default(),
                    &ListActivitiesFilters::empty(),
                    &metrics,
                )
                .await
                .unwrap();

            assert_eq!(res.len(), 1);
            let (_activity, metrics) = res.first().unwrap();
            assert_eq!(metrics.get(&ActivityMetricV2::Calories).unwrap(), &Some(1.));
            assert!(metrics.get(&ActivityMetricV2::MaxCadence).is_none(),);
        }

        #[tokio::test]
        async fn test_repo_error_when_getting_timeseries() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| {
                    Ok(vec![(
                        default_activity().activity().clone(),
                        ActivityMetricsV2::new(HashMap::from([
                            (ActivityMetricV2::Calories, Some(1.)),
                            // ActivityMetricV2::MaxCadence is missing
                        ])),
                    )])
                });
            activity_repository
                .expect_get_activity_with_parsed_data()
                .times(1)
                .with(eq(ActivityId::from("test_activity")))
                .returning(|_| Err(GetActivityError::Unknown(anyhow!("error"))));
            activity_repository.expect_update_activity_metric().times(0);
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(activity_repository, raw_data_repository);
            let metrics = vec![ActivityMetricV2::Calories, ActivityMetricV2::MaxCadence];
            let res = service
                .list_activities_with_metrics(
                    &UserId::test_default(),
                    &ListActivitiesFilters::empty(),
                    &metrics,
                )
                .await;
            assert!(res.is_err());
        }

        #[tokio::test]
        async fn test_repo_error_when_getting_metrics_v2() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| Err(ListActivitiesError::Unknown(anyhow!("error"))));

            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(activity_repository, raw_data_repository);
            let metrics = vec![ActivityMetricV2::Calories, ActivityMetricV2::MaxCadence];
            let res = service
                .list_activities_with_metrics(
                    &UserId::test_default(),
                    &ListActivitiesFilters::empty(),
                    &metrics,
                )
                .await;
            assert!(res.is_err());
        }
    }
}
