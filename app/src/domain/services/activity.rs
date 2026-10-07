use std::{sync::Arc, time::Instant};

use anyhow::anyhow;
use log::warn;

use crate::domain::{
    models::{
        UserId,
        activity::{
            Activity, ActivityDurationCurves, ActivityId, ActivityMetric, ActivityMetrics,
            ActivityWithParsedData, DEFAULT_METRICS,
        },
        search::{SearchDocument, SearchDocumentType},
    },
    ports::{
        activity::{
            ActivityRepository, CreateActivityError, CreateActivityRequest, DeleteActivityError,
            DeleteActivityRequest, DurationCurveNotification, GetActivityError,
            GetAllActivitiesError, GetAllActivitiesRequest, GetRawActivityError,
            GetRawActivityRequest, IActivityService, ListActivitiesError, ListActivitiesFilters,
            NotificationsRemaining, PatchActivityError, PatchActivityRequest, RawActivity,
            RawDataRepository,
        },
        search::{IDocumentsForSearch, RemainingDocuments},
    },
};

const BATCH_SIZE: u32 = 50;

#[derive(Debug, Clone)]
pub struct ActivityService<AR, RDR>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
{
    activity_repository: AR,
    raw_data_repository: RDR,
    notify_new_document: Arc<tokio::sync::Notify>,
    notify_duration_curve: Arc<tokio::sync::Notify>,
}

impl<AR, RDR> ActivityService<AR, RDR>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
{
    pub fn new(
        activity_repository: AR,
        raw_data_repository: RDR,
        notify_new_document: Arc<tokio::sync::Notify>,
        notify_duration_curve: Arc<tokio::sync::Notify>,
    ) -> Self {
        Self {
            activity_repository,
            raw_data_repository,
            notify_new_document,
            notify_duration_curve,
        }
    }

    pub async fn compute_missing_duration_curves(&self) {
        tracing::info!("Starting to compute missing duration curves");
        let start = Instant::now();
        let mut processed_activities = 0;
        let mut number_of_errors = 0;

        loop {
            let activities = match self
                .activity_repository
                .get_activities_without_duration_curves(BATCH_SIZE)
                .await
            {
                Ok(activities) => activities,
                Err(err) => {
                    tracing::error!("Unable to process missing duration curves: {}", err);
                    return;
                }
            };

            if activities.is_empty() {
                break;
            }

            if number_of_errors > BATCH_SIZE {
                // To avoid being stuck in an infinite loop due to unprocessable activities
                // coming back and not allowing activities.is_empty() to break
                tracing::error!(
                    "Too many errors while processing missing durations curves, aborting"
                );
                break;
            }

            for (activity_id, user_id) in activities.iter() {
                let activity = match self
                    .activity_repository
                    .get_activity_with_parsed_data(user_id, activity_id)
                    .await
                {
                    Ok(Some(activity)) => activity,
                    Ok(None) => {
                        tracing::error!(
                            "Result is None when trying to get activity {}",
                            activity_id
                        );
                        number_of_errors += 1;
                        continue;
                    }
                    Err(err) => {
                        tracing::error!("Unable to get activity {}: {}", activity_id, err);
                        number_of_errors += 1;
                        continue;
                    }
                };

                let updated_activity = activity.recompute_duration_curves();

                if let Err(err) = self
                    .activity_repository
                    .save_activity(&updated_activity)
                    .await
                {
                    tracing::error!(
                        "Error while trying to persist updated activity {}: {}",
                        activity_id,
                        err
                    );
                    number_of_errors += 1;
                    continue;
                }

                processed_activities += 1;
            }
        }

        if processed_activities > 0 {
            self.notify_duration_curve.notify_one();
        }

        tracing::info!(
            "Finished processing missing duration curves: {} activities processed in {}s",
            processed_activities,
            start.elapsed().as_secs()
        );
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

        let duration_curves = ActivityDurationCurves::from(&activity, req.timeseries());

        let activity_with_parsed_data = ActivityWithParsedData::new(
            activity.clone(),
            req.timeseries().clone(),
            req.statistics().clone(),
            duration_curves,
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
            .save_activity(&activity_with_parsed_data)
            .await
            .map_err(|err| anyhow!(err).context(format!("Failed to persist activity {}", id)))?;
        self.notify_new_document.notify_one();
        if !activity_with_parsed_data.duration_curves().is_empty() {
            self.notify_duration_curve.notify_one();
        }

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
    async fn list_activities_with_metrics(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
        metrics: &[ActivityMetric],
    ) -> Result<Vec<(Activity, ActivityMetrics)>, ListActivitiesError> {
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
                .get_activity_with_parsed_data(user, activity.id())
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
    async fn get_activity_with_parsed_data_and_metrics(
        &self,
        user: &UserId,
        activity_id: &ActivityId,
        metrics: &[ActivityMetric],
    ) -> Result<(ActivityWithParsedData, ActivityMetrics), GetActivityError> {
        let (activity, metrics) = match self
            .activity_repository
            .get_activity_with_metrics(user, activity_id, metrics)
            .await
        {
            Ok(Some(res)) => res,
            Ok(None) => return Err(GetActivityError::ActivityDoesNotExist(activity_id.clone())),
            Err(err) => return Err(err),
        };

        let activity = match self
            .activity_repository
            .get_activity_with_parsed_data(user, activity.id())
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
        let Ok(Some(activity)) = self
            .activity_repository
            .get_activity(req.user(), req.activity())
            .await
        else {
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

        let new_activity = activity.patch(req.as_patch());

        self.activity_repository
            .update_activity(&new_activity)
            .await
            .map_err(|err| {
                anyhow!(err).context(format!("Failed to persist activity {}", new_activity.id()))
            })?;

        self.notify_new_document.notify_one();

        Ok(())
    }

    #[tracing::instrument(skip_all, err)]
    async fn delete_activity(&self, req: DeleteActivityRequest) -> Result<(), DeleteActivityError> {
        let Ok(Some(activity)) = self
            .activity_repository
            .get_activity(req.user(), req.activity())
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
            .delete_activity(req.user(), req.activity())
            .await?;

        self.notify_new_document.notify_one();
        self.notify_duration_curve.notify_one();

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

    #[tracing::instrument(skip_all, err)]
    async fn list_pending_duration_curve_notifications(
        &self,
        batch_size: i64,
        page: i64,
    ) -> Result<(Vec<DurationCurveNotification>, NotificationsRemaining), anyhow::Error> {
        self.activity_repository
            .list_pending_duration_curve_notifications(batch_size, page)
            .await
    }

    #[tracing::instrument(skip_all, err)]
    async fn mark_duration_curve_notifications_as_processed(
        &self,
        activity: &ActivityId,
        user: &UserId,
        processed_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), anyhow::Error> {
        self.activity_repository
            .mark_duration_curve_notifications_as_processed(activity, user, processed_at)
            .await
    }
}

impl<AR, RDR> IDocumentsForSearch for ActivityService<AR, RDR>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
{
    #[tracing::instrument(skip_all, err)]
    async fn snapshot_documents(
        &self,
        batch_size: i64,
        page: i64,
    ) -> Result<(Vec<SearchDocument>, RemainingDocuments), anyhow::Error> {
        self.activity_repository
            .list_activity_documents(batch_size, page)
            .await
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_pending_documents_to_process(&self) -> Result<Vec<SearchDocument>, anyhow::Error> {
        self.activity_repository
            .get_outbox_documents_to_process()
            .await
    }

    #[tracing::instrument(skip_all, err)]
    async fn mark_document_as_processed(
        &self,
        document: &SearchDocument,
        processed_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), anyhow::Error> {
        self.activity_repository
            .mark_outbox_document_as_processed(document, processed_at)
            .await
    }

    fn service_kind(&self) -> SearchDocumentType {
        SearchDocumentType::Activity
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
        DeleteActivityError, DurationCurveNotification, GetAllActivitiesError,
        GetAllActivitiesRequest, GetRawActivityError, GetRawActivityRequest, ListActivitiesError,
        NotificationsRemaining, PatchActivityError, PatchActivityRequest, RawActivity,
        SaveActivityError, SimilarActivityError, UpdateActivityMetricError,
    };
    use crate::domain::ports::search::RemainingDocuments;

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

            async fn list_activities_with_metrics(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters,
                metrics: &[ActivityMetric],
            ) -> Result<Vec<(Activity, ActivityMetrics)>, ListActivitiesError>;

            async fn get_activity_with_parsed_data_and_metrics(
                &self,
                user: &UserId,
                activity_id: &ActivityId,
                metrics: &[ActivityMetric],
            ) -> Result<(ActivityWithParsedData, ActivityMetrics), GetActivityError>;

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

            async fn list_pending_duration_curve_notifications(
                &self,
                batch_size: i64,
                page: i64,
             ) -> Result<(Vec<DurationCurveNotification>, NotificationsRemaining), anyhow::Error>;

            async fn mark_duration_curve_notifications_as_processed(
                &self,
                activity: &ActivityId,
                user: &UserId,
                processed_at: chrono::DateTime<chrono::Utc>,
            ) -> Result<(), anyhow::Error>;
        }
    }

    impl MockActivityService {
        pub fn test_default() -> Self {
            let mut mock = Self::new();
            mock.default_create_activity();
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
                activity: &ActivityWithParsedData,
            ) -> Result<(), SaveActivityError>;

            async fn update_activity(
                &self,
                activity: &Activity,
            ) -> Result<(), SaveActivityError>;

            async fn list_activity_documents(
                &self,
                batch_size: i64,
                page: i64,
            ) -> Result<(Vec<SearchDocument>, RemainingDocuments), anyhow::Error>;

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
                metric: &ActivityMetric,
                value: &Option<f64>,
            ) -> Result<(), UpdateActivityMetricError>;

            async fn get_activities_with_metrics(
                &self,
                user: &UserId,
                filters: &ListActivitiesFilters,
                metrics: &[ActivityMetric],
            ) -> Result<Vec<(Activity, ActivityMetrics)>, ListActivitiesError>;

            async fn get_activity(
                &self,
                user: &UserId,
                id: &ActivityId,
            ) -> Result<Option<Activity>, GetActivityError>;

            async fn get_activity_with_metrics(
                &self,
                user: &UserId,
                id: &ActivityId,
                metrics: &[ActivityMetric],
            ) -> Result<Option<(Activity, ActivityMetrics)>, GetActivityError>;

            async fn get_activity_with_parsed_data(
                &self,
                user: &UserId,
                id: &ActivityId,
            ) -> Result<Option<ActivityWithParsedData>, GetActivityError>;

            async fn delete_activity(
                &self,
                user: &UserId,
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

            async fn list_pending_duration_curve_notifications(
                &self,
                batch_size: i64,
                page: i64,
            ) -> Result<(Vec<DurationCurveNotification>, NotificationsRemaining), anyhow::Error>;

            async fn mark_duration_curve_notifications_as_processed(
                &self,
                activity: &ActivityId,
                user: &UserId,
                processed_at: chrono::DateTime<chrono::Utc>,
            ) -> Result<(), anyhow::Error>;

            async fn get_activities_without_duration_curves(
                &self,
                limit: u32,
            ) -> Result<Vec<(ActivityId, UserId)>, anyhow::Error>;
        }

    }
}

#[cfg(test)]
mod tests_activity_service {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};

    use anyhow::anyhow;
    use mockall::mock;

    use crate::domain::{
        models::{
            UserId,
            activity::{
                ActiveTime, ActivityDuration, ActivityName, ActivityStartTime, ActivityStatistics,
                ActivityTimeseries, Sport, Timeseries, TimeseriesActiveTime, TimeseriesMetric,
                TimeseriesTime, TimeseriesValue,
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

    /// A running activity request whose timeseries contains distance values, producing a
    /// pace duration curve.
    fn activity_request_with_duration_curves() -> CreateActivityRequest {
        let timeseries = ActivityTimeseries::new(
            TimeseriesTime::new((0..11).collect()),
            TimeseriesActiveTime::new(vec![ActiveTime::Running(1); 11]),
            vec![],
            vec![Timeseries::new(
                TimeseriesMetric::Distance,
                (0..=10)
                    .map(|value| Some(TimeseriesValue::Float(value as f64 * 5.)))
                    .collect(),
            )],
        )
        .unwrap();

        let sport = Sport::Running;
        let start_time = ActivityStartTime::from_timestamp(3600).unwrap();
        let duration = ActivityDuration::default();
        let content = RawContent::new("fit".to_string(), vec![1, 2, 3]);
        let statistics = ActivityStatistics::default();
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

    ///////////////////////////////////////////////////////////////////
    // Helpers to observe the `notify_new_document` notifications
    ///////////////////////////////////////////////////////////////////

    /// Registers a waiter on the given notify and asserts that the service
    /// called `notify_one` (fails if no notification arrives in time).
    async fn expect_notified(notify: &Arc<tokio::sync::Notify>) {
        let notify = Arc::clone(notify);
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            notify.notified().await;
            let _ = tx.send(());
        });

        tokio::time::timeout(tokio::time::Duration::from_millis(500), rx)
            .await
            .expect("expected the activity service to trigger notify_one")
            .expect("the notify waiter task failed");
    }

    /// Asserts that the given notify has NOT been triggered, leaving the
    /// waiter enough time to register and catch any spurious notification.
    async fn expect_not_notified(notify: &Arc<tokio::sync::Notify>) {
        let was_notified = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&was_notified);
        let notify = Arc::clone(notify);
        tokio::spawn(async move {
            notify.notified().await;
            flag.store(true, Ordering::SeqCst);
        });

        // Let the waiter register and any (unexpected) notify fire.
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        assert!(
            !was_notified.load(Ordering::SeqCst),
            "expected the activity service NOT to trigger notify_one"
        );
    }

    #[tokio::test]
    async fn test_service_create_activity_err_if_similar_activity_exists() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_similar_activity_exists()
            .returning(|_| Ok(true));

        let raw_data_repository = MockRawDataRepository::new();

        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
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
        activity_repository
            .expect_update_activity_metric()
            .times(DEFAULT_METRICS.len())
            .returning(|_, _, _| Ok(()));
        let mut raw_data_repository = MockRawDataRepository::new();
        raw_data_repository
            .expect_save_raw_data()
            .returning(|_, __| Ok(()));

        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
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
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
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
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
        );

        let req = default_activity_request();

        let res = service.create_activity(req).await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_service_create_activity_triggers_notify() {
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
            .returning(|_, _| Ok(()));

        let notify = Arc::new(tokio::sync::Notify::new());
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::clone(&notify),
            Arc::new(tokio::sync::Notify::new()),
        );

        let res = service.create_activity(default_activity_request()).await;
        assert!(res.is_ok());

        expect_notified(&notify).await;
    }

    #[tokio::test]
    async fn test_service_create_activity_similar_exists_does_not_trigger_notify() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_similar_activity_exists()
            .returning(|_| Ok(true));

        let raw_data_repository = MockRawDataRepository::new();

        let notify = Arc::new(tokio::sync::Notify::new());
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::clone(&notify),
            Arc::new(tokio::sync::Notify::new()),
        );

        let res = service.create_activity(default_activity_request()).await;
        assert!(res.is_err());

        expect_not_notified(&notify).await;
    }

    #[tokio::test]
    async fn test_service_create_activity_with_duration_curves_triggers_notify() {
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
            .returning(|_, _| Ok(()));

        let notify = Arc::new(tokio::sync::Notify::new());
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::clone(&notify),
        );

        let res = service
            .create_activity(activity_request_with_duration_curves())
            .await;
        assert!(res.is_ok());

        expect_notified(&notify).await;
    }

    #[tokio::test]
    async fn test_service_create_activity_without_duration_curves_does_not_trigger_notify() {
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
            .returning(|_, _| Ok(()));

        let notify = Arc::new(tokio::sync::Notify::new());
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::clone(&notify),
        );

        // The default request has an empty timeseries, so the activity gets no duration
        // curve.
        let res = service.create_activity(default_activity_request()).await;
        assert!(res.is_ok());

        expect_not_notified(&notify).await;
    }

    #[tokio::test]
    async fn test_service_create_activity_error_does_not_trigger_duration_curve_notify() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_similar_activity_exists()
            .returning(|_| Ok(true));

        let raw_data_repository = MockRawDataRepository::new();

        let notify = Arc::new(tokio::sync::Notify::new());
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::clone(&notify),
        );

        let res = service.create_activity(default_activity_request()).await;
        assert!(res.is_err());

        expect_not_notified(&notify).await;
    }

    #[tokio::test]
    async fn test_activity_service_patch_activity_ok() {
        use crate::domain::models::activity::{ActivityFeedback, ActivityPatch, ActivityRpe};

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_, _| {
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
            .expect_update_activity()
            .withf(|activity| {
                activity.id() == &ActivityId::from("test_activity")
                    && activity.rpe().as_ref() == Some(&ActivityRpe::Five)
                    && activity.feedback().as_ref().map(|f| f.as_str()) == Some("Great ride!")
                    && activity.name().map(|n| n.to_string()) == Some("Long ride".to_string())
                    && activity.nutrition().is_none()
            })
            .returning(|_| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
        );

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
            .return_once(|_, _| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
        );

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
    async fn test_activity_service_patch_activity_triggers_notify() {
        use crate::domain::models::activity::ActivityPatch;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_, _| {
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
            )))
        });
        activity_repository
            .expect_update_activity()
            .times(1)
            .returning(|_| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();

        let notify = Arc::new(tokio::sync::Notify::new());
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::clone(&notify),
            Arc::new(tokio::sync::Notify::new()),
        );

        let req = PatchActivityRequest::new(
            ActivityId::from("test_activity"),
            UserId::test_default(),
            ActivityPatch::default(),
        );

        let res = service.patch_activity(req).await;
        assert!(res.is_ok());

        expect_notified(&notify).await;
    }

    #[tokio::test]
    async fn test_activity_service_patch_activity_not_found_does_not_trigger_notify() {
        use crate::domain::models::activity::ActivityPatch;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activity()
            .return_once(|_, _| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();

        let notify = Arc::new(tokio::sync::Notify::new());
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::clone(&notify),
            Arc::new(tokio::sync::Notify::new()),
        );

        let req = PatchActivityRequest::new(
            ActivityId::from("test"),
            UserId::test_default(),
            ActivityPatch::default(),
        );

        let res = service.patch_activity(req).await;
        assert!(res.is_err());

        expect_not_notified(&notify).await;
    }

    #[tokio::test]
    async fn test_activity_service_patch_activity_not_owned_by_user() {
        use crate::domain::models::activity::ActivityPatch;

        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_, _| {
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                "another_user".into(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
            )))
        });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
        );

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
        activity_repository.expect_get_activity().returning(|_, _| {
            Ok(Some(Activity::new_empty(
                ActivityId::from("test_activity"),
                UserId::test_default(),
                ActivityStartTime::from_timestamp(0).unwrap(),
                ActivityDuration::default(),
                Sport::Cycling,
            )))
        });
        activity_repository
            .expect_update_activity()
            .returning(|_| Err(SaveActivityError::Unknown(anyhow!("an error occured"))));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
        );

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
            .return_once(|_, _| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
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
    async fn test_activity_service_delete_activity_triggers_notify() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_, _| {
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
            .times(1)
            .returning(|_, _| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();

        let notify = Arc::new(tokio::sync::Notify::new());
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::clone(&notify),
            Arc::new(tokio::sync::Notify::new()),
        );

        let req = DeleteActivityRequest::new(
            "test_user".to_string().into(),
            ActivityId::from("test_activity"),
        );

        let res = service.delete_activity(req).await;
        assert!(res.is_ok());

        expect_notified(&notify).await;
    }

    #[tokio::test]
    async fn test_activity_service_delete_activity_not_found_does_not_trigger_notify() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activity()
            .return_once(|_, _| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();

        let notify = Arc::new(tokio::sync::Notify::new());
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::clone(&notify),
            Arc::new(tokio::sync::Notify::new()),
        );

        let req = DeleteActivityRequest::new(UserId::test_default(), ActivityId::from("test"));

        let res = service.delete_activity(req).await;
        assert!(res.is_err());

        expect_not_notified(&notify).await;
    }

    #[tokio::test]
    async fn test_service_delete_activity_triggers_duration_curve_notify() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository.expect_get_activity().returning(|_, _| {
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
            .times(1)
            .returning(|_, _| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();

        let notify = Arc::new(tokio::sync::Notify::new());
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::clone(&notify),
        );

        let req = DeleteActivityRequest::new(
            "test_user".to_string().into(),
            ActivityId::from("test_activity"),
        );

        let res = service.delete_activity(req).await;
        assert!(res.is_ok());

        expect_notified(&notify).await;
    }

    #[tokio::test]
    async fn test_service_delete_activity_error_does_not_trigger_duration_curve_notify() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activity()
            .return_once(|_, _| Ok(None));

        let raw_data_repository = MockRawDataRepository::default();

        let notify = Arc::new(tokio::sync::Notify::new());
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::clone(&notify),
        );

        let req = DeleteActivityRequest::new(UserId::test_default(), ActivityId::from("test"));

        let res = service.delete_activity(req).await;
        assert!(res.is_err());

        expect_not_notified(&notify).await;
    }

    #[tokio::test]
    async fn test_activity_service_delete_activity_not_owned_by_user() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activity()
            .return_once(|_, _| {
                Ok(Some(Activity::new_empty(
                    ActivityId::from("test_activity"),
                    UserId::from("another_user".to_string()),
                    ActivityStartTime::from_timestamp(0).unwrap(),
                    ActivityDuration::default(),
                    Sport::Cycling,
                )))
            });

        let raw_data_repository = MockRawDataRepository::default();
        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
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
        activity_repository.expect_get_activity().returning(|_, _| {
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
            .withf(|user, id| {
                *user == UserId::from("test_user") && *id == ActivityId::from("test_activity")
            })
            .returning(|_, _| Ok(()));

        let raw_data_repository = MockRawDataRepository::default();

        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
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
            .return_once(move |_, _| Ok(None));
        let raw_data_repository = MockRawDataRepository::default();

        let service = ActivityService::new(
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
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
            activity_repository,
            raw_data_repository,
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(tokio::sync::Notify::new()),
        );

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
                ActivityDurationCurves::default(),
            )
        }

        #[tokio::test]
        async fn test_no_activities() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| Ok(vec![]));
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(
                activity_repository,
                raw_data_repository,
                Arc::new(tokio::sync::Notify::new()),
                Arc::new(tokio::sync::Notify::new()),
            );
            let metrics = vec![ActivityMetric::Calories, ActivityMetric::AvgHeartRate];
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
                        ActivityMetrics::new(HashMap::from([
                            (ActivityMetric::Calories, Some(1.)),
                            (ActivityMetric::AvgHeartRate, Some(12.3)),
                        ])),
                    )])
                });
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(
                activity_repository,
                raw_data_repository,
                Arc::new(tokio::sync::Notify::new()),
                Arc::new(tokio::sync::Notify::new()),
            );
            let metrics = vec![ActivityMetric::Calories, ActivityMetric::AvgHeartRate];
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
            assert_eq!(metrics.get(&ActivityMetric::Calories).unwrap(), &Some(1.));
            assert_eq!(
                metrics.get(&ActivityMetric::AvgHeartRate).unwrap(),
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
                        ActivityMetrics::new(HashMap::from([
                            (ActivityMetric::Calories, Some(1.)),
                            (ActivityMetric::AvgHeartRate, None),
                        ])),
                    )])
                });
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(
                activity_repository,
                raw_data_repository,
                Arc::new(tokio::sync::Notify::new()),
                Arc::new(tokio::sync::Notify::new()),
            );
            let metrics = vec![ActivityMetric::Calories, ActivityMetric::AvgHeartRate];
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
            assert_eq!(metrics.get(&ActivityMetric::Calories).unwrap(), &Some(1.));
            assert_eq!(metrics.get(&ActivityMetric::AvgHeartRate).unwrap(), &None);
        }

        #[tokio::test]
        async fn test_activity_with_missing_requested_metrics_values_and_missing_in_timeseries() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| {
                    Ok(vec![(
                        default_activity().activity().clone(),
                        ActivityMetrics::new(HashMap::from([
                            (ActivityMetric::Calories, Some(1.)),
                            // ActivityMetricV2::AvgHeartRate is missing and no HR values in timeseries
                        ])),
                    )])
                });
            activity_repository
                .expect_get_activity_with_parsed_data()
                .times(1)
                .with(
                    eq(UserId::test_default()),
                    eq(ActivityId::from("test_activity")),
                )
                .returning(|_, _| Ok(Some(default_activity())));
            activity_repository
                .expect_update_activity_metric()
                .times(1)
                .with(
                    eq(ActivityId::from("test_activity")),
                    eq(ActivityMetric::AvgHeartRate),
                    eq(None),
                )
                .returning(|_, _, _| Ok(()));
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(
                activity_repository,
                raw_data_repository,
                Arc::new(tokio::sync::Notify::new()),
                Arc::new(tokio::sync::Notify::new()),
            );
            let metrics = vec![ActivityMetric::Calories, ActivityMetric::AvgHeartRate];
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
            assert_eq!(metrics.get(&ActivityMetric::Calories).unwrap(), &Some(1.));
            assert_eq!(metrics.get(&ActivityMetric::AvgHeartRate).unwrap(), &None);
        }

        #[tokio::test]
        async fn test_activity_with_missing_requested_metrics_values_and_present_in_timeseries() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| {
                    Ok(vec![(
                        default_activity().activity().clone(),
                        ActivityMetrics::new(HashMap::from([
                            (ActivityMetric::Calories, Some(1.)),
                            // ActivityMetricV2::MaxCadence is missing with cadence values in timeseries
                        ])),
                    )])
                });
            activity_repository
                .expect_get_activity_with_parsed_data()
                .times(1)
                .with(
                    eq(UserId::test_default()),
                    eq(ActivityId::from("test_activity")),
                )
                .returning(|_, _| Ok(Some(default_activity())));
            activity_repository
                .expect_update_activity_metric()
                .times(1)
                .with(
                    eq(ActivityId::from("test_activity")),
                    eq(ActivityMetric::MaxCadence),
                    eq(Some(30.)),
                )
                .returning(|_, _, _| Ok(()));
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(
                activity_repository,
                raw_data_repository,
                Arc::new(tokio::sync::Notify::new()),
                Arc::new(tokio::sync::Notify::new()),
            );
            let metrics = vec![ActivityMetric::Calories, ActivityMetric::MaxCadence];
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
            assert_eq!(metrics.get(&ActivityMetric::Calories).unwrap(), &Some(1.));
            assert_eq!(
                metrics.get(&ActivityMetric::MaxCadence).unwrap(),
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
                        ActivityMetrics::new(HashMap::from([
                            (ActivityMetric::Calories, Some(1.)),
                            // ActivityMetricV2::MaxCadence is missing and we can't find the activity's timeseries
                        ])),
                    )])
                });
            activity_repository
                .expect_get_activity_with_parsed_data()
                .times(1)
                .with(
                    eq(UserId::test_default()),
                    eq(ActivityId::from("test_activity")),
                )
                .returning(|_, _| Ok(None));
            activity_repository.expect_update_activity_metric().times(0);
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(
                activity_repository,
                raw_data_repository,
                Arc::new(tokio::sync::Notify::new()),
                Arc::new(tokio::sync::Notify::new()),
            );
            let metrics = vec![ActivityMetric::Calories, ActivityMetric::MaxCadence];
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
            assert_eq!(metrics.get(&ActivityMetric::Calories).unwrap(), &Some(1.));
            assert!(metrics.get(&ActivityMetric::MaxCadence).is_none(),);
        }

        #[tokio::test]
        async fn test_repo_error_when_getting_timeseries() {
            let mut activity_repository = MockActivityRepository::new();
            activity_repository
                .expect_get_activities_with_metrics()
                .returning(|_, _, _| {
                    Ok(vec![(
                        default_activity().activity().clone(),
                        ActivityMetrics::new(HashMap::from([
                            (ActivityMetric::Calories, Some(1.)),
                            // ActivityMetricV2::MaxCadence is missing
                        ])),
                    )])
                });
            activity_repository
                .expect_get_activity_with_parsed_data()
                .times(1)
                .with(
                    eq(UserId::test_default()),
                    eq(ActivityId::from("test_activity")),
                )
                .returning(|_, _| Err(GetActivityError::Unknown(anyhow!("error"))));
            activity_repository.expect_update_activity_metric().times(0);
            let raw_data_repository = MockRawDataRepository::default();

            let service = ActivityService::new(
                activity_repository,
                raw_data_repository,
                Arc::new(tokio::sync::Notify::new()),
                Arc::new(tokio::sync::Notify::new()),
            );
            let metrics = vec![ActivityMetric::Calories, ActivityMetric::MaxCadence];
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

            let service = ActivityService::new(
                activity_repository,
                raw_data_repository,
                Arc::new(tokio::sync::Notify::new()),
                Arc::new(tokio::sync::Notify::new()),
            );
            let metrics = vec![ActivityMetric::Calories, ActivityMetric::MaxCadence];
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

    ///////////////////////////////////////////////////////////////////
    // compute_missing_duration_curves
    ///////////////////////////////////////////////////////////////////

    use mockall::predicate::eq;

    use crate::domain::models::activity::DurationCurveType;

    fn running_activity(id: &ActivityId) -> Activity {
        Activity::new_empty(
            id.clone(),
            UserId::test_default(),
            ActivityStartTime::from_timestamp(3600).unwrap(),
            ActivityDuration::default(),
            Sport::Running,
        )
    }

    /// A running activity timeseries with a monotonically increasing distance metric,
    /// from which a pace duration curve can be computed.
    fn running_timeseries_with_distance() -> ActivityTimeseries {
        ActivityTimeseries::new(
            TimeseriesTime::new((0..11).collect()),
            TimeseriesActiveTime::new(vec![ActiveTime::Running(1); 11]),
            vec![],
            vec![Timeseries::new(
                TimeseriesMetric::Distance,
                (0..=10)
                    .map(|value| Some(TimeseriesValue::Float(value as f64 * 5.)))
                    .collect(),
            )],
        )
        .unwrap()
    }

    /// A running activity whose stored duration curves are missing (empty), while its
    /// timeseries would produce a pace curve when recomputed.
    fn activity_with_missing_duration_curves(id: &ActivityId) -> ActivityWithParsedData {
        let activity = running_activity(id);
        let timeseries = running_timeseries_with_distance();
        ActivityWithParsedData::new(
            activity,
            timeseries,
            ActivityStatistics::default(),
            ActivityDurationCurves::default(),
        )
    }

    fn duration_curve_service(
        activity_repository: MockActivityRepository,
        notify_duration_curve: Arc<tokio::sync::Notify>,
    ) -> ActivityService<MockActivityRepository, MockRawDataRepository> {
        ActivityService::new(
            activity_repository,
            MockRawDataRepository::default(),
            Arc::new(tokio::sync::Notify::new()),
            notify_duration_curve,
        )
    }

    /// Registers a `Notified` waiter on the notify channel and returns it, so tests can
    /// observe whether the service called `notify_one()`. The waiter is enabled so it is
    /// registered before the service runs; afterwards, `enable()` on the returned future
    /// returns true if and only if the service sent a notification.
    fn registered_notified(
        notify: &Arc<tokio::sync::Notify>,
    ) -> std::pin::Pin<Box<tokio::sync::futures::Notified<'_>>> {
        let mut future = Box::pin(notify.notified());
        assert!(!future.as_mut().enable());
        future
    }

    #[tokio::test]
    async fn test_compute_missing_duration_curves_recomputes_and_saves() {
        let activity_id = ActivityId::from("activity_1");
        let batch_activity_id = activity_id.clone();

        let mut activity_repository = MockActivityRepository::new();
        // First batch contains the activity, the second batch is empty (loop termination).
        activity_repository
            .expect_get_activities_without_duration_curves()
            .times(1)
            .with(eq(50_u32))
            .returning(move |_| Ok(vec![(batch_activity_id.clone(), UserId::test_default())]));
        activity_repository
            .expect_get_activities_without_duration_curves()
            .times(1)
            .with(eq(50_u32))
            .returning(|_| Ok(vec![]));
        activity_repository
            .expect_get_activity_with_parsed_data()
            .times(1)
            .with(
                eq(UserId::test_default()),
                eq(ActivityId::from("activity_1")),
            )
            .returning({
                let activity_id = activity_id.clone();
                move |_, _| Ok(Some(activity_with_missing_duration_curves(&activity_id)))
            });
        let saved = Arc::new(std::sync::Mutex::new(Vec::new()));
        let saved_sink = Arc::clone(&saved);
        activity_repository
            .expect_save_activity()
            .times(1)
            .returning(move |activity| {
                saved_sink.lock().unwrap().push(activity.clone());
                Ok(())
            });

        let notify = Arc::new(tokio::sync::Notify::new());
        let mut notified = registered_notified(&notify);
        let service = duration_curve_service(activity_repository, notify.clone());
        service.compute_missing_duration_curves().await;

        // One activity was successfully processed, so a notification must be sent.
        assert!(notified.as_mut().enable());

        let saved = saved.lock().unwrap();
        assert_eq!(1, saved.len());
        assert_eq!(&activity_id, saved[0].id());

        // The recomputed duration curves of a running activity with a distance metric
        // should be a pace curve with the expected best average speeds.
        let expected = ActivityDurationCurves::from(
            &running_activity(&activity_id),
            &running_timeseries_with_distance(),
        );
        let expected: Vec<_> = expected.iter().collect();
        let actual: Vec<_> = saved[0].duration_curves().iter().collect();
        assert_eq!(expected, actual);

        let pace_curve = actual[0];
        assert_eq!(DurationCurveType::Pace, pace_curve.curve_type());
        // 11 samples from 0 to 50 meters: best average speed over 5s and 10s is 5 m/s.
        assert_eq!(Some(5.), pace_curve.values()[0]);
        assert_eq!(Some(5.), pace_curve.values()[1]);
        assert!(pace_curve.values()[2..].iter().all(Option::is_none));
    }

    #[tokio::test]
    async fn test_compute_missing_duration_curves_repo_error_is_not_fatal() {
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activities_without_duration_curves()
            .times(1)
            .returning(|_| Err(anyhow!("repo error")));
        activity_repository
            .expect_get_activity_with_parsed_data()
            .times(0);
        activity_repository.expect_save_activity().times(0);

        let notify = Arc::new(tokio::sync::Notify::new());
        let mut notified = registered_notified(&notify);
        let service = duration_curve_service(activity_repository, notify.clone());
        service.compute_missing_duration_curves().await;

        // Nothing was processed, so no notification must be sent.
        assert!(!notified.as_mut().enable());
    }

    #[tokio::test]
    async fn test_compute_missing_duration_curves_skips_missing_activity() {
        let activity_id = ActivityId::from("activity_1");

        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activities_without_duration_curves()
            .times(1)
            .with(eq(50_u32))
            .returning(move |_| Ok(vec![(activity_id.clone(), UserId::test_default())]));
        activity_repository
            .expect_get_activities_without_duration_curves()
            .times(1)
            .with(eq(50_u32))
            .returning(|_| Ok(vec![]));
        activity_repository
            .expect_get_activity_with_parsed_data()
            .times(1)
            .with(
                eq(UserId::test_default()),
                eq(ActivityId::from("activity_1")),
            )
            .returning(|_, _| Ok(None));
        activity_repository.expect_save_activity().times(0);

        let notify = Arc::new(tokio::sync::Notify::new());
        let mut notified = registered_notified(&notify);
        let service = duration_curve_service(activity_repository, notify.clone());
        service.compute_missing_duration_curves().await;

        // No activity could be processed, so no notification must be sent.
        assert!(!notified.as_mut().enable());
    }

    #[tokio::test]
    async fn test_compute_missing_duration_curves_save_error_is_not_fatal() {
        let activity_id = ActivityId::from("activity_1");
        let batch_activity_id = activity_id.clone();

        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activities_without_duration_curves()
            .times(1)
            .with(eq(50_u32))
            .returning(move |_| Ok(vec![(batch_activity_id.clone(), UserId::test_default())]));
        activity_repository
            .expect_get_activities_without_duration_curves()
            .times(1)
            .with(eq(50_u32))
            .returning(|_| Ok(vec![]));
        activity_repository
            .expect_get_activity_with_parsed_data()
            .times(1)
            .with(
                eq(UserId::test_default()),
                eq(ActivityId::from("activity_1")),
            )
            .returning({
                let activity_id = activity_id.clone();
                move |_, _| Ok(Some(activity_with_missing_duration_curves(&activity_id)))
            });
        activity_repository
            .expect_save_activity()
            .times(1)
            .returning(|_| Err(SaveActivityError::Unknown(anyhow!("save failed"))));

        let notify = Arc::new(tokio::sync::Notify::new());
        let mut notified = registered_notified(&notify);
        let service = duration_curve_service(activity_repository, notify.clone());
        service.compute_missing_duration_curves().await;

        // The only activity failed to be persisted, so no notification must be sent.
        assert!(!notified.as_mut().enable());
    }

    #[tokio::test]
    async fn test_compute_missing_duration_curves_processes_all_activities() {
        let id_1 = ActivityId::from("activity_1");
        let id_2 = ActivityId::from("activity_2");
        let batch_id_1 = id_1.clone();
        let batch_id_2 = id_2.clone();

        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activities_without_duration_curves()
            .times(1)
            .with(eq(50_u32))
            .returning(move |_| {
                Ok(vec![
                    (batch_id_1.clone(), UserId::test_default()),
                    (batch_id_2.clone(), UserId::test_default()),
                ])
            });
        activity_repository
            .expect_get_activities_without_duration_curves()
            .times(1)
            .with(eq(50_u32))
            .returning(|_| Ok(vec![]));
        activity_repository
            .expect_get_activity_with_parsed_data()
            .times(1)
            .with(
                eq(UserId::test_default()),
                eq(ActivityId::from("activity_1")),
            )
            .returning({
                let activity_id = id_1.clone();
                move |_, _| Ok(Some(activity_with_missing_duration_curves(&activity_id)))
            });
        activity_repository
            .expect_get_activity_with_parsed_data()
            .times(1)
            .with(
                eq(UserId::test_default()),
                eq(ActivityId::from("activity_2")),
            )
            .returning({
                let activity_id = id_2.clone();
                move |_, _| Ok(Some(activity_with_missing_duration_curves(&activity_id)))
            });
        let saved = Arc::new(std::sync::Mutex::new(Vec::new()));
        let saved_sink = Arc::clone(&saved);
        activity_repository
            .expect_save_activity()
            .times(2)
            .returning(move |activity| {
                saved_sink.lock().unwrap().push(activity.clone());
                Ok(())
            });

        let notify = Arc::new(tokio::sync::Notify::new());
        let mut notified = registered_notified(&notify);
        let service = duration_curve_service(activity_repository, notify.clone());
        service.compute_missing_duration_curves().await;

        // Both activities were successfully processed, so a notification must be sent.
        assert!(notified.as_mut().enable());

        let saved = saved.lock().unwrap();
        assert_eq!(2, saved.len());
        let saved_ids: Vec<_> = saved.iter().map(|a| a.id().clone()).collect();
        assert_eq!(
            vec![
                ActivityId::from("activity_1"),
                ActivityId::from("activity_2")
            ],
            saved_ids
        );
    }

    #[tokio::test]
    async fn test_compute_missing_duration_curves_aborts_when_too_many_errors() {
        // A first batch of 51 activities, all of which fail to be persisted: the error
        // counter rises above BATCH_SIZE (50) and the loop must abort instead of spinning
        // forever on the same unprocessable activities coming back batch after batch.
        let mut activity_repository = MockActivityRepository::new();
        activity_repository
            .expect_get_activities_without_duration_curves()
            .times(1)
            .with(eq(50_u32))
            .returning(|_| {
                Ok((0..51)
                    .map(|i| {
                        (
                            ActivityId::from(format!("activity_{i}").as_str()),
                            UserId::test_default(),
                        )
                    })
                    .collect())
            });
        // A second batch is fetched before the abort check happens: if the service did
        // not abort, this activity would be processed successfully and the loop would
        // keep going (making the mock expectations below panic).
        activity_repository
            .expect_get_activities_without_duration_curves()
            .times(1)
            .with(eq(50_u32))
            .returning(|_| {
                Ok(vec![(
                    ActivityId::from("activity_51"),
                    UserId::test_default(),
                )])
            });
        activity_repository
            .expect_get_activity_with_parsed_data()
            .times(51)
            .returning(|_, id| Ok(Some(activity_with_missing_duration_curves(id))));
        activity_repository
            .expect_save_activity()
            .times(51)
            .returning(|_| Err(SaveActivityError::Unknown(anyhow!("save failed"))));

        let notify = Arc::new(tokio::sync::Notify::new());
        let mut notified = registered_notified(&notify);
        let service = duration_curve_service(activity_repository, notify.clone());
        service.compute_missing_duration_curves().await;

        // No activity was successfully persisted, so no notification must be sent.
        assert!(!notified.as_mut().enable());
    }
}
