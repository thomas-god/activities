use anyhow::anyhow;

use crate::domain::{
    models::{Activity, ActivityId},
    ports::{
        ActivityRepository, ActivityService, CreateActivityError, CreateActivityRequest,
        RawDataRepository,
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
        let activity = Activity::new(id.clone(), *req.calories(), *req.duration(), *req.sport());

        tracing::info!("Parsed new activity {:?}", &activity);

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

    async fn list_activities(&self) -> Result<Vec<Activity>, super::ports::ListActivitiesError> {
        self.activity_repository.list_activities().await
    }
}

#[cfg(test)]
pub mod test_utils {
    use std::mem;
    use std::sync::{Arc, Mutex};

    use super::*;

    use crate::domain::ports::ListActivitiesError;

    #[derive(Clone)]
    pub struct MockActivityService {
        pub create_activity_result: Arc<Mutex<Result<Activity, CreateActivityError>>>,
        pub list_activities_result: Arc<Mutex<Result<Vec<Activity>, ListActivitiesError>>>,
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
    }
}

#[cfg(test)]
mod tests {
    use std::{mem, ops::DerefMut, sync::Arc};

    use anyhow::anyhow;
    use tokio::sync::Mutex;

    use crate::domain::{
        models::Sport,
        ports::{ListActivitiesError, SaveActivityError, SaveRawDataError},
    };

    use super::*;

    #[derive(Clone)]
    struct MockActivityRepository {
        save_activity_result: Arc<Mutex<Result<(), SaveActivityError>>>,
        list_activity_result: Arc<Mutex<Result<Vec<Activity>, ListActivitiesError>>>,
    }

    impl ActivityRepository for MockActivityRepository {
        async fn save_activity(&self, _activity: &Activity) -> Result<(), SaveActivityError> {
            let mut guard = self.save_activity_result.lock().await;
            let mut result = Err(SaveActivityError::Unknown(anyhow!("substitute error")));
            mem::swap(guard.deref_mut(), &mut result);
            result
        }

        async fn list_activities(&self) -> Result<Vec<Activity>, ListActivitiesError> {
            let mut guard = self.list_activity_result.lock().await;
            let mut result = Err(ListActivitiesError::Unknown(anyhow!("substitute error")));
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

    #[tokio::test]
    async fn test_service_create_activity() {
        let activity_repository = MockActivityRepository {
            save_activity_result: Arc::new(Mutex::new(Ok(()))),
            list_activity_result: Arc::new(Mutex::new(Ok(vec![]))),
        };
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Ok(()))),
        };
        let service = Service::new(activity_repository, raw_data_repository);

        let sport = Some(Sport::Running);
        let duration = Some(1200);
        let calories = Some(12);
        let content = vec![1, 2, 3];
        let req = CreateActivityRequest::new(sport, duration, calories, content);

        let res = service.create_activity(req).await;

        assert!(res.is_ok());
        let res = res.unwrap();
        assert_eq!(res.calories(), &Some(12))
    }

    #[tokio::test]
    async fn test_service_create_activity_save_activity_error() {
        let activity_repository = MockActivityRepository {
            save_activity_result: Arc::new(Mutex::new(Err(SaveActivityError::Unknown(anyhow!(
                "an error occured"
            ))))),
            list_activity_result: Arc::new(Mutex::new(Ok(vec![]))),
        };
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Ok(()))),
        };
        let service = Service::new(activity_repository, raw_data_repository);

        let sport = Some(Sport::Running);
        let duration = Some(1200);
        let calories = Some(12);
        let content = vec![1, 2, 3];
        let req = CreateActivityRequest::new(sport, duration, calories, content);

        let res = service.create_activity(req).await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_service_create_activity_raw_data_error() {
        let activity_repository = MockActivityRepository {
            save_activity_result: Arc::new(Mutex::new(Ok(()))),
            list_activity_result: Arc::new(Mutex::new(Ok(vec![]))),
        };
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Err(SaveRawDataError::Unknown(anyhow!(
                "an error occured"
            ))))),
        };
        let service = Service::new(activity_repository, raw_data_repository);

        let sport = Some(Sport::Running);
        let duration = Some(1200);
        let calories = Some(12);
        let content = vec![1, 2, 3];
        let req = CreateActivityRequest::new(sport, duration, calories, content);

        let res = service.create_activity(req).await;

        assert!(res.is_err())
    }
}
