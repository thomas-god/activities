use derive_more::Constructor;
use thiserror::Error;

use crate::domain::models::activity::{
    Activity, ActivityDuration, ActivityId, ActivityNaturalKey, ActivityStartTime,
    ActivityStatistics, ActivityTimeseries, Sport,
};
use crate::domain::models::training_metrics::{
    TrainingMetricDefinition, TrainingMetricId, TrainingMetricValues,
};

///////////////////////////////////////////////////////////////////
/// ACTIVITY SERVICE
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct CreateActivityRequest {
    sport: Sport,
    duration: ActivityDuration,
    start_time: ActivityStartTime,
    statistics: ActivityStatistics,
    timeseries: ActivityTimeseries,
    raw_content: Vec<u8>,
}

impl CreateActivityRequest {
    pub fn new(
        sport: Sport,
        duration: ActivityDuration,
        start_time: ActivityStartTime,
        statistics: ActivityStatistics,
        timeseries: ActivityTimeseries,
        raw_content: Vec<u8>,
    ) -> Self {
        Self {
            sport,
            duration,
            start_time,
            statistics,
            timeseries,
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

    pub fn statistics(&self) -> &ActivityStatistics {
        &self.statistics
    }

    pub fn timeseries(&self) -> &ActivityTimeseries {
        &self.timeseries
    }
}

#[derive(Debug, Error)]
pub enum CreateActivityError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
    #[error("An activity with similar data already exists")]
    SimilarActivityExistsError,
    #[error("Timeseries metrics do not have the same length")]
    TimeseriesMetricsNotSameLength,
}

pub trait IActivityService: Clone + Send + Sync + 'static {
    fn create_activity(
        &self,
        req: CreateActivityRequest,
    ) -> impl Future<Output = Result<Activity, CreateActivityError>> + Send;

    fn list_activities(
        &self,
    ) -> impl Future<Output = Result<Vec<Activity>, ListActivitiesError>> + Send;

    fn get_activity(
        &self,
        activity_id: &ActivityId,
    ) -> impl Future<Output = Result<Activity, GetActivityError>> + Send;
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

#[derive(Debug, Error)]
pub enum GetActivityError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
    #[error("Activity {0} does not exist")]
    ActivityDoesNotExist(ActivityId),
}

///////////////////////////////////////////////////////////////////
// ACTIVITY AND RAW DATA REPOSITORIES
///////////////////////////////////////////////////////////////////

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

    fn get_activity(
        &self,
        id: &ActivityId,
    ) -> impl Future<Output = Result<Option<Activity>, GetActivityError>> + Send;
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

///////////////////////////////////////////////////////////////////
/// TRAINING METRICS SERVICE
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct RecomputeMetricRequest {
    new_activity: ActivityId,
}

impl RecomputeMetricRequest {
    pub fn new_activity(&self) -> &ActivityId {
        &self.new_activity
    }
}

pub trait ITrainingMetricService: Clone + Send + Sync + 'static {
    fn recompute_metric(
        &self,
        req: RecomputeMetricRequest,
    ) -> impl Future<Output = Result<(), ()>> + Send;

    fn get_training_metrics(
        &self,
    ) -> impl Future<Output = Vec<(TrainingMetricDefinition, TrainingMetricValues)>> + Send;
}

#[derive(Debug, Error)]
pub enum GetTrainingMetricsDefinitionsError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum UpdateMetricError {
    #[error("Training metric {0:?} does not exist")]
    TrainingMetricDoesNotExists(TrainingMetricId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum GetTrainingMetricValueError {
    #[error("Training metric {0:?} does not exist")]
    TrainingMetricDoesNotExists(TrainingMetricId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait TrainingMetricsRepository: Clone + Send + Sync + 'static {
    fn get_definitions(
        &self,
    ) -> impl Future<
        Output = Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError>,
    > + Send;

    fn save_metric_values(
        &self,
        id: &TrainingMetricId,
        values: (&str, f64),
    ) -> impl Future<Output = Result<(), UpdateMetricError>> + Send;

    fn get_metric_values(
        &self,
        id: &TrainingMetricId,
    ) -> impl Future<Output = Result<TrainingMetricValues, GetTrainingMetricValueError>> + Send;
}
