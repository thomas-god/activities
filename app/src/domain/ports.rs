use chrono::{DateTime, FixedOffset, NaiveDate};
use derive_more::Constructor;
use serde::Deserialize;
use thiserror::Error;

use crate::domain::models::UserId;
use crate::domain::models::activity::{
    Activity, ActivityId, ActivityName, ActivityNaturalKey, ActivityStartTime, ActivityStatistics,
    ActivityTimeseries, ActivityWithTimeseries, Sport,
};
use crate::domain::models::training::{
    ActivityMetricSource, TrainingMetricAggregate, TrainingMetricDefinition, TrainingMetricFilters,
    TrainingMetricGranularity, TrainingMetricId, TrainingMetricValue, TrainingMetricValues,
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
    raw_content: RawContent,
}

impl CreateActivityRequest {
    pub fn new(
        user: UserId,
        sport: Sport,
        start_time: ActivityStartTime,
        statistics: ActivityStatistics,
        timeseries: ActivityTimeseries,
        raw_content: RawContent,
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

    pub fn raw_content(self) -> RawContent {
        self.raw_content
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
    #[error("User {0} does not exists")]
    UserDoesNotExist(UserId),
}

/// Represents the content of the initial activity file needed for later reuse/reparsing, namely
/// the file's bytes and its extension (fit, tcx, etc.).
#[derive(Debug, Clone, Constructor, PartialEq)]
pub struct RawContent {
    extension: String,
    content: Vec<u8>,
}

impl RawContent {
    pub fn extension(&self) -> &str {
        &self.extension
    }

    pub fn raw_content(self) -> Vec<u8> {
        self.content
    }
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

#[derive(Debug, Clone, Constructor)]
pub struct ListActivitiesFilters {
    limit: Option<usize>,
    date_range: Option<DateRange>,
}

impl ListActivitiesFilters {
    pub fn empty() -> Self {
        Self {
            limit: None,
            date_range: None,
        }
    }

    pub fn limit(&self) -> &Option<usize> {
        &self.limit
    }

    pub fn set_limit(self, limit: Option<usize>) -> Self {
        Self { limit, ..self }
    }

    pub fn date_range(&self) -> &Option<DateRange> {
        &self.date_range
    }

    pub fn set_date_range(self, date_range: Option<DateRange>) -> Self {
        Self { date_range, ..self }
    }
}

pub trait IActivityService: Clone + Send + Sync + 'static {
    fn create_activity(
        &self,
        req: CreateActivityRequest,
    ) -> impl Future<Output = Result<Activity, CreateActivityError>> + Send;

    fn list_activities(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> impl Future<Output = Result<Vec<Activity>, ListActivitiesError>> + Send;

    fn list_activities_with_timeseries(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
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

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct DateTimeRange {
    start: DateTime<FixedOffset>,
    end: Option<DateTime<FixedOffset>>,
}

impl DateTimeRange {
    pub fn start(&self) -> &DateTime<FixedOffset> {
        &self.start
    }

    pub fn end(&self) -> &Option<DateTime<FixedOffset>> {
        &self.end
    }
}

#[derive(Debug, Clone, PartialEq, Constructor, Deserialize)]
pub struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl DateRange {
    pub fn start(&self) -> &NaiveDate {
        &self.start
    }

    pub fn end(&self) -> &NaiveDate {
        &self.end
    }
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
        filters: &ListActivitiesFilters,
    ) -> impl Future<Output = Result<Vec<Activity>, ListActivitiesError>> + Send;

    fn list_activities_with_timeseries(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> impl Future<Output = Result<Vec<ActivityWithTimeseries>, ListActivitiesError>> + Send;

    fn get_activity(
        &self,
        id: &ActivityId,
    ) -> impl Future<Output = Result<Option<Activity>, anyhow::Error>> + Send;

    fn get_activity_with_timeseries(
        &self,
        id: &ActivityId,
    ) -> impl Future<Output = Result<Option<ActivityWithTimeseries>, anyhow::Error>> + Send;

    fn modify_activity_name(
        &self,
        id: &ActivityId,
        name: Option<ActivityName>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn delete_activity(
        &self,
        activity: &ActivityId,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn get_user_history_date_range(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Option<DateTimeRange>, anyhow::Error>> + Send;
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
        content: RawContent,
    ) -> impl Future<Output = Result<(), SaveRawDataError>> + Send;

    fn get_raw_data(
        &self,
        activity_id: &ActivityId,
    ) -> impl Future<Output = Result<RawContent, GetRawDataError>> + Send;
}

///////////////////////////////////////////////////////////////////
/// TRAINING SERVICE
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct CreateTrainingMetricRequest {
    user: UserId,
    source: ActivityMetricSource,
    granularity: TrainingMetricGranularity,
    aggregate: TrainingMetricAggregate,
    filters: TrainingMetricFilters,
    initial_date_range: Option<DateRange>,
}

impl CreateTrainingMetricRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn source(&self) -> &ActivityMetricSource {
        &self.source
    }

    pub fn granularity(&self) -> &TrainingMetricGranularity {
        &self.granularity
    }

    pub fn aggregate(&self) -> &TrainingMetricAggregate {
        &self.aggregate
    }

    pub fn filters(&self) -> &TrainingMetricFilters {
        &self.filters
    }

    pub fn initial_date_range(&self) -> &Option<DateRange> {
        &self.initial_date_range
    }
}

#[derive(Debug, Error)]
pub enum CreateTrainingMetricError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
    #[error("Error when saving training metric definition")]
    SaveMetricError(#[from] SaveTrainingMetricError),
}

#[derive(Debug, Clone, Constructor)]
pub struct UpdateMetricsValuesRequest {
    user: UserId,
    new_activities: Vec<ActivityWithTimeseries>,
}

impl UpdateMetricsValuesRequest {
    pub fn new_activities(&self) -> &[ActivityWithTimeseries] {
        &self.new_activities
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

pub trait ITrainingService: Clone + Send + Sync + 'static {
    fn create_metric(
        &self,
        req: CreateTrainingMetricRequest,
    ) -> impl Future<Output = Result<TrainingMetricId, CreateTrainingMetricError>> + Send;

    fn update_metrics_values(
        &self,
        req: UpdateMetricsValuesRequest,
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

pub trait TrainingRepository: Clone + Send + Sync + 'static {
    fn save_definition(
        &self,
        definition: TrainingMetricDefinition,
    ) -> impl Future<Output = Result<(), SaveTrainingMetricError>> + Send;

    fn get_definition(
        &self,
        metric: &TrainingMetricId,
    ) -> impl Future<Output = Result<Option<TrainingMetricDefinition>, GetDefinitionError>> + Send;

    fn delete_definition(
        &self,
        metric: &TrainingMetricId,
    ) -> impl Future<Output = Result<(), DeleteMetricError>> + Send;

    fn get_definitions(
        &self,
        user: &UserId,
    ) -> impl Future<
        Output = Result<Vec<TrainingMetricDefinition>, GetTrainingMetricsDefinitionsError>,
    > + Send;

    fn update_metric_values(
        &self,
        id: &TrainingMetricId,
        values: (String, TrainingMetricValue),
    ) -> impl Future<Output = Result<(), UpdateMetricError>> + Send;

    fn get_metric_value(
        &self,
        id: &TrainingMetricId,
        bin_key: &str,
    ) -> impl Future<Output = Result<Option<TrainingMetricValue>, GetTrainingMetricValueError>> + Send;

    fn get_metric_values(
        &self,
        id: &TrainingMetricId,
    ) -> impl Future<Output = Result<TrainingMetricValues, GetTrainingMetricValueError>> + Send;
}

#[cfg(test)]
pub mod test_utils {
    use mockall::mock;

    use super::*;

    mock! {
        pub RawDataRepository {}

        impl Clone for RawDataRepository {
            fn clone(&self) -> Self;
        }

        impl RawDataRepository for RawDataRepository {
            async fn save_raw_data(
                &self,
                activity_id: &ActivityId,
                content: RawContent,
            ) -> Result<(), SaveRawDataError>;

            async fn get_raw_data(
                &self,
                activity_id: &ActivityId,
            ) -> Result<RawContent, GetRawDataError>;
        }
    }
}
