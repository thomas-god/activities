// In memory implemenations of repository ports

use std::{collections::HashMap, ops::DerefMut, sync::Arc};

use tokio::sync::Mutex;

use crate::domain::{
    models::{Activity, ActivityId, ActivityNaturalKey},
    ports::{
        ActivityRepository, RawDataRepository, SaveActivityError, SaveRawDataError,
        SimilarActivityError,
    },
};

#[derive(Clone)]
pub struct InMemoryActivityRepository {
    activities: Arc<Mutex<Vec<Activity>>>,
}

impl InMemoryActivityRepository {
    pub fn new(activities: Vec<Activity>) -> Self {
        Self {
            activities: Arc::new(Mutex::new(activities)),
        }
    }
}

impl ActivityRepository for InMemoryActivityRepository {
    async fn similar_activity_exists(
        &self,
        natural_key: &ActivityNaturalKey,
    ) -> Result<bool, SimilarActivityError> {
        let guard = self.activities.lock().await;
        Ok(guard
            .iter()
            .any(|activity| activity.natural_key() == *natural_key))
    }

    async fn save_activity(&self, activity: &Activity) -> Result<(), SaveActivityError> {
        let mut guard = self.activities.lock().await;
        guard.deref_mut().push(activity.clone());
        Ok(())
    }

    async fn list_activities(
        &self,
    ) -> Result<Vec<Activity>, crate::domain::ports::ListActivitiesError> {
        let guard = self.activities.lock().await;
        Ok(guard.clone())
    }
}

#[derive(Clone)]
pub struct InMemoryRawDataRepository {
    raw_data: Arc<Mutex<HashMap<ActivityId, Vec<u8>>>>,
}

impl InMemoryRawDataRepository {
    pub fn new(raw_data: HashMap<ActivityId, Vec<u8>>) -> Self {
        Self {
            raw_data: Arc::new(Mutex::new(raw_data)),
        }
    }
}

impl RawDataRepository for InMemoryRawDataRepository {
    async fn save_raw_data(
        &self,
        activity_id: &ActivityId,
        content: &[u8],
    ) -> Result<(), SaveRawDataError> {
        let mut guard = self.raw_data.lock().await;
        guard
            .deref_mut()
            .insert(activity_id.clone(), content.into());
        Ok(())
    }
}
