use thiserror::Error;

use crate::domain::models::{
    Activity, ActivityDuration, ActivityId, ActivityNaturalKey, ActivityStartTime, Sport,
};

#[derive(Debug, Clone)]
pub struct CreateActivityRequest {
    sport: Sport,
    duration: ActivityDuration,
    start_time: ActivityStartTime,
    raw_content: Vec<u8>,
}

impl CreateActivityRequest {
    pub fn new(
        sport: Sport,
        duration: ActivityDuration,
        start_time: ActivityStartTime,
        raw_content: Vec<u8>,
    ) -> Self {
        Self {
            sport,
            duration,
            start_time,
            raw_content,
        }
    }

    pub fn start_time(&self) -> &ActivityStartTime {
        &self.start_time
    }

    pub fn raw_content(&self) -> &[u8] {
        &self.raw_content
    }

    pub fn sport(&self) -> &Sport {
        &self.sport
    }

    pub fn duration(&self) -> &ActivityDuration {
        &self.duration
    }
}

#[derive(Debug, Error)]
pub enum CreateActivityError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait ActivityService: Clone + Send + Sync + 'static {
    fn create_activity(
        &self,
        req: CreateActivityRequest,
    ) -> impl Future<Output = Result<Activity, CreateActivityError>> + Send;

    fn list_activities(
        &self,
    ) -> impl Future<Output = Result<Vec<Activity>, ListActivitiesError>> + Send;
}
#[derive(Debug, Error)]
pub enum SimilarActivityError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum SaveActivityError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum ListActivitiesError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait ActivityRepository: Clone + Send + Sync + 'static {
    fn similar_activity_exists(
        &self,
        natural_key: &ActivityNaturalKey,
    ) -> impl Future<Output = Result<bool, SimilarActivityError>> + Send;

    fn save_activity(
        &self,
        activity: &Activity,
    ) -> impl Future<Output = Result<(), SaveActivityError>> + Send;

    fn list_activities(
        &self,
    ) -> impl Future<Output = Result<Vec<Activity>, ListActivitiesError>> + Send;
}

#[derive(Debug, Error)]
pub enum SaveRawDataError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait RawDataRepository: Clone + Send + Sync + 'static {
    fn save_raw_data(
        &self,
        activity_id: &ActivityId,
        content: &[u8],
    ) -> impl Future<Output = Result<(), SaveRawDataError>> + Send;
}
