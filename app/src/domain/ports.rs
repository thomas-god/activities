use derive_more::Constructor;
use thiserror::Error;

use crate::domain::models::UserId;
use crate::domain::models::activity::{
    Activity, ActivityId, ActivityName, ActivityNaturalKey, ActivityStartTime, ActivityStatistics,
    ActivityTimeseries, ActivityWithTimeseries, Sport,
};
use crate::domain::models::training_metrics::{
    TrainingMetricAggregate, TrainingMetricDefinition, TrainingMetricGranularity, TrainingMetricId,
    TrainingMetricSource, TrainingMetricValues,
};

///////////////////////////////////////////////////////////////////
/// ACTIVITY SERVICE
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct CreateActivityRequest {
    user: UserId,
    sport: Sport,
    start_time: ActivityStartTime,
    statistics: ActivityStatistics,
    timeseries: ActivityTimeseries,
    raw_content: Vec<u8>,
}

impl CreateActivityRequest {
    pub fn new(
        user: UserId,
        sport: Sport,
        start_time: ActivityStartTime,
        statistics: ActivityStatistics,
        timeseries: ActivityTimeseries,
        raw_content: Vec<u8>,
    ) -> Self {
        Self {
            user,
            sport,
            start_time,
            statistics,
            timeseries,
            raw_content,
        }
    }

    pub fn user(&self) -> &UserId {
        &self.user
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
    #[error("User {0} does not exists")]
    UserDoesNotExist(UserId),
}

#[derive(Debug, Clone, Constructor, Default)]
pub struct ModifyActivityRequest {
    user: UserId,
    activity: ActivityId,
    name: Option<ActivityName>,
}

impl ModifyActivityRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn activity(&self) -> &ActivityId {
        &self.activity
    }

    pub fn name(&self) -> Option<&ActivityName> {
        self.name.as_ref()
    }
}

#[derive(Debug, Error)]
pub enum ModifyActivityError {
    #[error("Activity {0} does not exists")]
    ActivityDoesNotExist(ActivityId),
    #[error("User {0} does not own activity {1}")]
    UserDoesNotOwnActivity(UserId, ActivityId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Constructor)]
pub struct DeleteActivityRequest {
    user: UserId,
    activity: ActivityId,
}

impl DeleteActivityRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn activity(&self) -> &ActivityId {
        &self.activity
    }
}

#[derive(Debug, Error)]
pub enum DeleteActivityError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
    #[error("Activity {0} does not exists")]
    ActivityDoesNotExist(ActivityId),
    #[error("User {0} does not own activity {1}")]
    UserDoesNotOwnActivity(UserId, ActivityId),
}

pub trait IActivityService: Clone + Send + Sync + 'static {
    fn create_activity(
        &self,
        req: CreateActivityRequest,
    ) -> impl Future<Output = Result<Activity, CreateActivityError>> + Send;

    fn list_activities(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Activity>, ListActivitiesError>> + Send;

    fn list_activities_with_timeseries(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<ActivityWithTimeseries>, ListActivitiesError>> + Send;

    fn get_activity(
        &self,
        activity_id: &ActivityId,
    ) -> impl Future<Output = Result<Activity, GetActivityError>> + Send;

    fn get_activity_with_timeseries(
        &self,
        activity_id: &ActivityId,
    ) -> impl Future<Output = Result<ActivityWithTimeseries, GetActivityError>> + Send;

    fn modify_activity(
        &self,
        req: ModifyActivityRequest,
    ) -> impl Future<Output = Result<(), ModifyActivityError>> + Send;

    fn delete_activity(
        &self,
        req: DeleteActivityRequest,
    ) -> impl Future<Output = Result<(), DeleteActivityError>> + Send;
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
        activity: &ActivityWithTimeseries,
    ) -> impl Future<Output = Result<(), SaveActivityError>> + Send;

    fn list_activities(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Activity>, ListActivitiesError>> + Send;

    fn list_activities_with_timeseries(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<ActivityWithTimeseries>, ListActivitiesError>> + Send;

    fn get_activity(
        &self,
        id: &ActivityId,
    ) -> impl Future<Output = Result<Option<Activity>, GetActivityError>> + Send;

    fn get_activity_with_timeseries(
        &self,
        id: &ActivityId,
    ) -> impl Future<Output = Result<Option<ActivityWithTimeseries>, GetActivityError>> + Send;

    fn modify_activity_name(
        &self,
        id: &ActivityId,
        name: Option<ActivityName>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn delete_activity(
        &self,
        activity: &ActivityId,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;
}

#[derive(Debug, Error)]
pub enum SaveRawDataError {
    #[error("Raw data already exist for activity {0}")]
    ActivityRawDataExist(ActivityId),
    #[error("Unknown error")]
    Unknown,
}

#[derive(Debug, Error)]
pub enum GetRawDataError {
    #[error("No raw data found for activity {0}")]
    NoRawDataFound(ActivityId),
    #[error("Unknown error")]
    Unknown,
}

pub trait RawDataRepository: Clone + Send + Sync + 'static {
    fn save_raw_data(
        &self,
        activity_id: &ActivityId,
        content: &[u8],
    ) -> impl Future<Output = Result<(), SaveRawDataError>> + Send;

    fn get_raw_data(
        &self,
        activity_id: &ActivityId,
    ) -> impl Future<Output = Result<Vec<u8>, GetRawDataError>> + Send;
}

///////////////////////////////////////////////////////////////////
/// TRAINING METRICS SERVICE
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct CreateTrainingMetricRequest {
    user: UserId,
    source: TrainingMetricSource,
    granularity: TrainingMetricGranularity,
    aggregate: TrainingMetricAggregate,
}

impl CreateTrainingMetricRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn source(&self) -> &TrainingMetricSource {
        &self.source
    }

    pub fn granularity(&self) -> &TrainingMetricGranularity {
        &self.granularity
    }

    pub fn aggregate(&self) -> &TrainingMetricAggregate {
        &self.aggregate
    }
}

#[derive(Debug, Error)]
pub enum CreateTrainingMetricError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
    #[error("Error when saving training metric definition")]
    SaveMetricError(#[from] SaveTrainingMetricError),
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct RecomputeMetricRequest {
    user: UserId,
    new_activity: Option<ActivityId>,
}

impl RecomputeMetricRequest {
    pub fn new_activity(&self) -> &Option<ActivityId> {
        &self.new_activity
    }

    pub fn user(&self) -> &UserId {
        &self.user
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct DeleteTrainingMetricRequest {
    user: UserId,
    metric: TrainingMetricId,
}

impl DeleteTrainingMetricRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn metric(&self) -> &TrainingMetricId {
        &self.metric
    }
}

#[derive(Debug, Error)]
pub enum DeleteTrainingMetricError {
    #[error("Training metric with id {0} does not exists")]
    MetricDoesNotExist(TrainingMetricId),
    #[error("User {0} does not own training metric {1}")]
    UserDoesNotOwnTrainingMetric(UserId, TrainingMetricId),
    #[error("An infratstructure error occured when getting defintion")]
    GetDefinitionError(#[from] GetDefinitionError),
    #[error("An infratstructure error occured when trying to delete defintion")]
    DeleteMetricError(#[from] DeleteMetricError),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait ITrainingMetricService: Clone + Send + Sync + 'static {
    fn create_metric(
        &self,
        req: CreateTrainingMetricRequest,
    ) -> impl Future<Output = Result<TrainingMetricId, CreateTrainingMetricError>> + Send;

    fn recompute_metric(
        &self,
        req: RecomputeMetricRequest,
    ) -> impl Future<Output = Result<(), ()>> + Send;

    fn get_training_metrics(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Vec<(TrainingMetricDefinition, TrainingMetricValues)>> + Send;

    fn delete_metric(
        &self,
        req: DeleteTrainingMetricRequest,
    ) -> impl Future<Output = Result<(), DeleteTrainingMetricError>> + Send;
}

#[derive(Debug, Error)]
pub enum SaveTrainingMetricError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
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

#[derive(Debug, Error)]
pub enum DeleteMetricError {
    #[error("Training metric {0:?} does not exist")]
    TrainingMetricDoesNotExists(TrainingMetricId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum GetDefinitionError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait TrainingMetricsRepository: Clone + Send + Sync + 'static {
    fn save_definitions(
        &self,
        definition: TrainingMetricDefinition,
    ) -> impl Future<Output = Result<(), SaveTrainingMetricError>> + Send;

    fn get_definitions(
        &self,
        user: &UserId,
    ) -> impl Future<
        Output = Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError>,
    > + Send;

    fn update_metric_values(
        &self,
        id: &TrainingMetricId,
        values: (&str, f64),
    ) -> impl Future<Output = Result<(), UpdateMetricError>> + Send;

    fn get_metric_values(
        &self,
        id: &TrainingMetricId,
    ) -> impl Future<Output = Result<TrainingMetricValues, GetTrainingMetricValueError>> + Send;

    fn get_definition(
        &self,
        metric: &TrainingMetricId,
    ) -> impl Future<Output = Result<Option<TrainingMetricDefinition>, GetDefinitionError>> + Send;

    fn delete_definition(
        &self,
        metric: &TrainingMetricId,
    ) -> impl Future<Output = Result<(), DeleteMetricError>> + Send;
}
