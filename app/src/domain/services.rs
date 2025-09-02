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
        let activity = Activity::new(id.clone(), req.calories().clone());

        // Persist activity
        self.activity_repository
            .save_activity(&activity)
            .await
            .map_err(|err| anyhow!(err).context(format!("Failed to persist activity {}", id)))?;

        // Persist raw data
        self.raw_data_repository
            .save_raw_data(&id, &req.raw_content())
            .await
            .map_err(|err| {
                anyhow!(err).context(format!("Failed to persist raw data for activity {}", id))
            })?;

        Ok(activity)
    }
}

#[cfg(test)]
mod tests {
    use std::{mem, ops::DerefMut, sync::Arc};

    use anyhow::anyhow;
    use tokio::sync::Mutex;

    use crate::domain::ports::{SaveActivityError, SaveRawDataError};

    use super::*;

    #[derive(Clone)]
    struct MockActivityRepository {
        save_activity_result: Arc<Mutex<Result<(), SaveActivityError>>>,
    }

    impl ActivityRepository for MockActivityRepository {
        async fn save_activity(&self, _activity: &Activity) -> Result<(), SaveActivityError> {
            let mut guard = self.save_activity_result.lock().await;
            let mut result = Err(SaveActivityError::Unknown(anyhow!("substitute error")));
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
        };
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Ok(()))),
        };
        let service = Service::new(activity_repository, raw_data_repository);

        let calories = Some(12);
        let content = vec![1, 2, 3];
        let req = CreateActivityRequest::new(calories, content);

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
        };
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Ok(()))),
        };
        let service = Service::new(activity_repository, raw_data_repository);

        let calories = Some(12);
        let content = vec![1, 2, 3];
        let req = CreateActivityRequest::new(calories, content);

        let res = service.create_activity(req).await;

        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_service_create_activity_raw_data_error() {
        let activity_repository = MockActivityRepository {
            save_activity_result: Arc::new(Mutex::new(Ok(()))),
        };
        let raw_data_repository = MockRawDataRepository {
            save_raw_data: Arc::new(Mutex::new(Err(SaveRawDataError::Unknown(anyhow!(
                "an error occured"
            ))))),
        };
        let service = Service::new(activity_repository, raw_data_repository);

        let calories = Some(12);
        let content = vec![1, 2, 3];
        let req = CreateActivityRequest::new(calories, content);

        let res = service.create_activity(req).await;

        assert!(res.is_err())
    }
}
