use thiserror::Error;

use crate::domain::models::{Activity, ActivityId, Sport};

#[derive(Debug, Clone)]
pub struct CreateActivityRequest {
    sport: Option<Sport>,
    duration: Option<usize>,
    calories: Option<usize>,
    raw_content: Vec<u8>,
}

impl CreateActivityRequest {
    pub fn new(
        sport: Option<Sport>,
        duration: Option<usize>,
        calories: Option<usize>,
        raw_content: Vec<u8>,
    ) -> Self {
        Self {
            sport,
            duration,
            calories,
            raw_content,
        }
    }

    pub fn calories(&self) -> &Option<usize> {
        &self.calories
    }

    pub fn raw_content(&self) -> &[u8] {
        &self.raw_content
    }

    pub fn sport(&self) -> &Option<Sport> {
        &self.sport
    }

    pub fn duration(&self) -> &Option<usize> {
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
}

#[derive(Debug, Error)]
pub enum SaveActivityError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait ActivityRepository: Clone + Send + Sync + 'static {
    fn save_activity(
        &self,
        activity: &Activity,
    ) -> impl Future<Output = Result<(), SaveActivityError>> + Send;
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
