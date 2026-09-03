use anyhow::anyhow;
use log::warn;

use crate::domain::{
    models::{
        UserId,
        activity::{
            Activity, ActivityId, ActivityMetricV2, ActivityMetricsV2, ActivityWithParsedData,
            DEFAULT_METRICS,
        },
        search::SearchDocument,
    },
    ports::{
        activity::{
            ActivityRepository, CreateActivityError, CreateActivityRequest, DeleteActivityError,
            DeleteActivityRequest, GetActivityError, GetAllActivitiesError,
            GetAllActivitiesRequest, GetRawActivityError, GetRawActivityRequest, IActivityService,
            ListActivitiesError, ListActivitiesFilters, PatchActivityError, PatchActivityRequest,
            RawActivity, RawDataRepository,
        },
        search::DocumentsForSearch,
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
    #[tracing::instrument(skip_all, err)]
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
        );

        let activity_with_parsed_data = ActivityWithParsedData::new(
            activity.clone(),
            req.timeseries().clone(),
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

        // Pre-compute base metrics for the new activity
        for ref metric in DEFAULT_METRICS {
            let value = metric.compute_value(&activity_with_parsed_data);
            let _ = self
                .activity_repository
                .update_activity_metric(activity.id(), metric, &value)
                .await;
        }

        Ok(activity)
    }

    #[tracing::instrument(skip_all, err)]
    async fn list_activities(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> Result<Vec<Activity>, ListActivitiesError> {
        self.activity_repository
            .list_activities(user, filters)
            .await
    }

    #[tracing::instrument(skip_all, err)]
    async fn list_activities_with_parsed_data(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> Result<Vec<ActivityWithParsedData>, ListActivitiesError> {
        self.activity_repository
            .list_activities_with_parsed_data(user, filters)
            .await
    }

    #[tracing::instrument(skip_all, err)]
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

    #[tracing::instrument(skip_all, err)]
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

    #[tracing::instrument(skip_all, err)]
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

    #[tracing::instrument(skip_all, err)]
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

    #[tracing::instrument(skip_all, err)]
    async fn patch_activity(&self, req: PatchActivityRequest) -> Result<(), PatchActivityError> {
        let Ok(Some(activity)) = self.activity_repository.get_activity(req.activity()).await else {
            return Err(PatchActivityError::ActivityDoesNotExist(
                req.activity().clone(),
            ));
        };

        if activity.user() != req.user() {
            warn!(
                "User {} is trying to modify activity {} without owning it",
                req.user(),
                req.activity()
            );
            return Err(PatchActivityError::UserDoesNotOwnActivity(
                req.user().clone(),
                req.activity().clone(),
            ));
        }

        let new_activity = activity.apply_patch(req.as_patch());

        self.activity_repository
            .save_activity(&new_activity)
            .await
            .map_err(|err| {
                anyhow!(err).context(format!("Failed to persist activity {}", new_activity.id()))
            })?;

        Ok(())
    }

    #[tracing::instrument(skip_all, err)]
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

    #[tracing::instrument(skip_all, err)]
    async fn get_raw_activity(
        &self,
        req: GetRawActivityRequest,
    ) -> Result<RawActivity, GetRawActivityError> {
        self.activity_repository
            .get_raw_activity(req.user(), req.activity())
            .await
    }

    #[tracing::instrument(skip_all, err)]
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

impl<AR, RDR> DocumentsForSearch for ActivityService<AR, RDR>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
{
    async fn get_documents_to_process(&self) -> Result<Vec<SearchDocument>, anyhow::Error> {
        self.activity_repository
            .get_outbox_documents_to_process()
            .await
    }

    async fn mark_document_as_processed(
        &self,
        document: &SearchDocument,
        processed_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), anyhow::Error> {
        self.activity_repository
            .mark_outbox_document_as_processed(document, processed_at)
            .await
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
        ActivityDuration, ActivityNaturalKey, ActivityStartTime, Sport,
    };
    use crate::domain::models::search::SearchDocument;
    use crate::domain::ports::activity::{
        DeleteActivityError, GetAllActivitiesError, GetAllActivitiesRequest, GetRawActivityError,
        GetRawActivityRequest, ListActivitiesError, PatchActivityError, PatchActivityRequest,
        RawActivity, SaveActivityError, SimilarActivityError, UpdateActivityMetricError,
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

            async fn patch_activity(
                &self,
                req: PatchActivityRequest
            ) -> Result<(), PatchActivityError>;

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
                ))
            });
        }
        pub fn default_list_activities(&mut self) {
            self.expect_list_activities().returning(|_, _| Ok(vec![]));
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

            async fn delete_activity(
                &self,
                activity: &ActivityId,
            ) -> Result<(), anyhow::Error>;

            async fn get_user_history_date_range(
                &self,
                user: &UserId,
            ) -> Result<Option<crate::domain::ports::DateTimeRange>, anyhow::Error>;

            async fn get_outbox_documents_to_process(
                &self,
            ) -> Result<Vec<SearchDocument>, anyhow::Error>;

            async fn mark_outbox_document_as_processed(
                &self,
                document: &SearchDocument,
                processed_at: chrono::DateTime<chrono::Utc>,
            ) -> Result<(), anyhow::Error>;
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
            DeleteActivityError, DeleteActivityRequest, GetRawDataError, RawContent,
            SaveActivityError, SaveRawDataError,
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
        activity_repository
            .expect_update_activity_metric()
            .times(DEFAULT_METRICS.len())
            .returning(|_, _, _| Ok(()));
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
    async fn test_activity_service_patch_activity_ok() {
        use crate::domain::models::activity::{ActivityFeedback, ActivityPatch, ActivityRpe};

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                Some(ActivityName::from("Long ride")),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
                None,
                None,
                None,
                None,
            )))
        });
        // Only the patched fields should change, the others must be preserved as-is.
        activity_repository
            .expect_save_activity()
            .withf(|activity| {
                activity.id() == &ActivityId::from("test_activity")
                    && activity.rpe().as_ref() == Some(&ActivityRpe::Five)
                    && activity.feedback().as_ref().map(|f| f.as_str()) == Some("Great ride!")
                    && activity.name().map(|n| n.to_string()) == Some("Long ride".to_string())
                    && activity.nutrition().is_none()
            })
            .returning(|_| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

        let req = PatchActivityRequest::new(
            ActivityId::from("test_activity"),
            UserId::test_default(),
            ActivityPatch::new(
                None,
                Some(Some(ActivityRpe::Five)),
                None,
                None,
                Some(Some(ActivityFeedback::from("Great ride!"))),
            ),
        );

        let res = service.patch_activity(req).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_activity_service_patch_activity_not_found() {
        use crate::domain::models::activity::ActivityPatch;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activity()
            .return_once(|_| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

        let req = PatchActivityRequest::new(
            ActivityId::from("test"),
            UserId::test_default(),
            ActivityPatch::default(),
        );

        let Err(PatchActivityError::ActivityDoesNotExist(activity_id)) =
            service.patch_activity(req).await
        else {
            unreachable!("Should have returned an error")
        };
        assert_eq!(activity_id, ActivityId::from("test"));
    }

    #[tokio::test]
    async fn test_activity_service_patch_activity_not_owned_by_user() {
        use crate::domain::models::activity::ActivityPatch;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                "another_user".into(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

        let req = PatchActivityRequest::new(
            ActivityId::from("test_activity"),
            UserId::test_default(),
            ActivityPatch::default(),
        );

        let Err(PatchActivityError::UserDoesNotOwnActivity(user, activity_id)) =
            service.patch_activity(req).await
        else {
            unreachable!("Should have returned an error")
        };
        assert_eq!(user, UserId::test_default());
        assert_eq!(activity_id, ActivityId::from("test_activity"));
    }

    #[tokio::test]
    async fn test_activity_service_patch_activity_save_error() {
        use crate::domain::models::activity::ActivityPatch;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_| {
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
            )))
        });
        activity_repository
            .expect_save_activity()
            .returning(|_| Err(SaveActivityError::Unknown(anyhow!("an error occured"))));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(activity_repository, raw_data_repository);

        let req = PatchActivityRequest::new(
            ActivityId::from("test_activity"),
            UserId::test_default(),
            ActivityPatch::default(),
        );

        let Err(PatchActivityError::Unknown(_)) = service.patch_activity(req).await else {
            unreachable!("Should have returned an error")
        };
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
