use anyhow::anyhow;

use crate::domain::{
    models::{Activity, ActivityId},
    ports::{
        ActivityRepository, ActivityService, CreateActivityError, CreateActivityRequest,
        GetActivityError, ListActivitiesError, RawDataRepository,
    },
};

#[derive(Debug, Clone)]
pub struct Service<AR, RDR>
where
    AR: ActivityRepository,
    RDR: RawDataRepository,
{
    activity_repository: AR,
    raw_data_repository: RDR,
}

impl<AR, RDR> Service<AR, RDR>
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

impl<AR, RDR> ActivityService for Service<AR, RDR>
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
            *req.start_time(),
            *req.duration(),
            *req.sport(),
            req.timeseries().clone(),
        );

        tracing::info!("Parsed new activity {:?}", &activity);

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

        // Persist activity
        self.activity_repository
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
        self.activity_repository.list_activities().await
    }

    async fn get_activity(&self, activity_id: &ActivityId) -> Result<Activity, GetActivityError> {
        match self.activity_repository.get_activity(activity_id).await {
            Ok(Some(activity)) => Ok(activity),
            Ok(None) => Err(GetActivityError::ActivityDoesNotExist(activity_id.clone())),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
pub mod test_utils {
    use std::mem;
    use std::sync::{Arc, Mutex};

    use super::*;

    use crate::domain::models::{ActivityDuration, ActivityStartTime, Sport, Timeseries};
    use crate::domain::ports::ListActivitiesError;

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
                    Timeseries::new(vec![]),
                )))),
                list_activities_result: Arc::new(Mutex::new(Ok(vec![]))),
                get_activity_result: Arc::new(Mutex::new(Ok(Activity::new(
                    ActivityId::new(),
                    ActivityStartTime::from_timestamp(1000).unwrap(),
                    ActivityDuration::from(3600),
                    Sport::Running,
                    Timeseries::new(vec![]),
                )))),
            }
        }
    }

    impl ActivityService for MockActivityService {
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
}

#[cfg(test)]
mod tests {
    use std::{mem, ops::DerefMut, sync::Arc};

    use anyhow::anyhow;
    use tokio::sync::Mutex;

    use crate::domain::{
        models::{ActivityDuration, ActivityNaturalKey, ActivityStartTime, Sport, Timeseries},
        ports::{
            GetActivityError, ListActivitiesError, SaveActivityError, SaveRawDataError,
            SimilarActivityError,
        },
    };

    use super::*;

    #[derive(Clone)]
    struct MockActivityRepository {
        similar_activity_result: Arc<Mutex<Result<bool, SimilarActivityError>>>,
        save_activity_result: Arc<Mutex<Result<(), SaveActivityError>>>,
        list_activities_result: Arc<Mutex<Result<Vec<Activity>, ListActivitiesError>>>,
        get_activity_result: Arc<Mutex<Result<Option<Activity>, GetActivityError>>>,
    }

    impl ActivityRepository for MockActivityRepository {
        async fn similar_activity_exists(
            &self,
            _natural_key: &ActivityNaturalKey,
        ) -> Result<bool, SimilarActivityError> {
            let mut guard = self.similar_activity_result.lock().await;
            let mut result = Err(SimilarActivityError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }

        async fn save_activity(&self, _activity: &Activity) -> Result<(), SaveActivityError> {
            let mut guard = self.save_activity_result.lock().await;
            let mut result = Err(SaveActivityError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }

        async fn list_activities(&self) -> Result<Vec<Activity>, ListActivitiesError> {
            let mut guard = self.list_activities_result.lock().await;
            let mut result = Err(ListActivitiesError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }

        async fn get_activity(
            &self,
            _id: &ActivityId,
        ) -> Result<Option<Activity>, GetActivityError> {
            let mut guard = self.get_activity_result.lock().await;
            let mut result = Err(GetActivityError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }
    }

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
        let timeseries = Timeseries::new(vec![]);
        CreateActivityRequest::new(sport, duration, start_time, timeseries, content)
    }

    #[tokio::test]
    async fn test_service_create_activity_err_if_similar_activity_exists() {
        let activity_repository = MockActivityRepository {
            similar_activity_result: Arc::new(Mutex::new(Ok(true))),
            save_activity_result: Arc::new(Mutex::new(Ok(()))),
            list_activities_result: Arc::new(Mutex::new(Ok(vec![]))),
            get_activity_result: Arc::new(Mutex::new(Ok(None))),
        };
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Ok(()))),
        };
        let service = Service::new(activity_repository, raw_data_repository);

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
        let activity_repository = MockActivityRepository {
            similar_activity_result: Arc::new(Mutex::new(Ok(false))),
            save_activity_result: Arc::new(Mutex::new(Ok(()))),
            list_activities_result: Arc::new(Mutex::new(Ok(vec![]))),
            get_activity_result: Arc::new(Mutex::new(Ok(None))),
        };
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Ok(()))),
        };
        let service = Service::new(activity_repository, raw_data_repository);

        let req = default_activity_request();

        let res = service.create_activity(req).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_service_create_activity_save_activity_error() {
        let activity_repository = MockActivityRepository {
            similar_activity_result: Arc::new(Mutex::new(Ok(false))),
            save_activity_result: Arc::new(Mutex::new(Err(SaveActivityError::Unknown(anyhow!(
                "an error occured"
            ))))),
            list_activities_result: Arc::new(Mutex::new(Ok(vec![]))),
            get_activity_result: Arc::new(Mutex::new(Ok(None))),
        };
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Ok(()))),
        };
        let service = Service::new(activity_repository, raw_data_repository);

        let req = default_activity_request();

        let res = service.create_activity(req).await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_service_create_activity_raw_data_error() {
        let activity_repository = MockActivityRepository {
            similar_activity_result: Arc::new(Mutex::new(Ok(false))),
            save_activity_result: Arc::new(Mutex::new(Ok(()))),
            list_activities_result: Arc::new(Mutex::new(Ok(vec![]))),
            get_activity_result: Arc::new(Mutex::new(Ok(None))),
        };
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Err(SaveRawDataError::Unknown(anyhow!(
                "an error occured"
            ))))),
        };
        let service = Service::new(activity_repository, raw_data_repository);

        let req = default_activity_request();

        let res = service.create_activity(req).await;

        assert!(res.is_err())
    }
}
