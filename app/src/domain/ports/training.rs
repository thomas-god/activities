use chrono::NaiveDate;
use derive_more::Constructor;
use thiserror::Error;

use crate::domain::{
    models::{
        UserId,
        activity::{Activity, ActivityId, ActivityMetricV2, ActivityWithParsedData},
        training::{
            TrainingMetric, TrainingMetricAggregate, TrainingMetricDefinition,
            TrainingMetricFilters, TrainingMetricGranularity, TrainingMetricGroupBy,
            TrainingMetricId, TrainingMetricName, TrainingMetricScope, TrainingMetricValues,
            TrainingMetricsOrdering, TrainingNote, TrainingNoteContent, TrainingNoteDate,
            TrainingNoteId, TrainingNoteTitle, TrainingPeriod, TrainingPeriodCreationError,
            TrainingPeriodId, TrainingPeriodSports, TrainingPeriodWithActivities,
        },
    },
    ports::DateRange,
};

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct CreateTrainingMetricRequest {
    user: UserId,
    name: TrainingMetricName,
    metric: ActivityMetricV2,
    granularity: TrainingMetricGranularity,
    aggregate: TrainingMetricAggregate,
    filters: TrainingMetricFilters,
    group_by: Option<TrainingMetricGroupBy>,
    scope: TrainingMetricScope,
}

impl CreateTrainingMetricRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn name(&self) -> &TrainingMetricName {
        &self.name
    }

    pub fn metric(&self) -> &ActivityMetricV2 {
        &self.metric
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

    pub fn group_by(&self) -> &Option<TrainingMetricGroupBy> {
        &self.group_by
    }

    pub fn scope(&self) -> &TrainingMetricScope {
        &self.scope
    }
}

/// Copy a training metric definition (global or not) to a training period.
#[derive(Debug, Clone, Constructor)]
pub struct CopyTrainingMetricRequest {
    user: UserId,
    source_metric: TrainingMetricId,
    target_period: TrainingPeriodId,
}

impl CopyTrainingMetricRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn source_metric(&self) -> &TrainingMetricId {
        &self.source_metric
    }
    pub fn target_period(&self) -> &TrainingPeriodId {
        &self.target_period
    }
}

#[derive(Debug, Error)]
pub enum CopyTrainingMetricError {
    #[error("Training metric does not exist")]
    MetricDoesNotExist(TrainingMetricId),
    #[error("Training period does not exist")]
    PeriodDoesNotExist(TrainingMetricId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
    #[error("Error when saving training metric definition")]
    SaveMetricError(#[from] SaveTrainingMetricError),
}

pub enum GetTrainingMetricValuesRequest {
    ByTrainingMetricId(UserId, TrainingMetricId),
    ByDefinition {
        user: UserId,
        metric: ActivityMetricV2,
        granularity: TrainingMetricGranularity,
        aggregate: TrainingMetricAggregate,
        filters: TrainingMetricFilters,
        group_by: Option<TrainingMetricGroupBy>,
    },
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
    new_activities: Vec<ActivityWithParsedData>,
}

impl UpdateMetricsValuesRequest {
    pub fn new_activities(&self) -> &[ActivityWithParsedData] {
        &self.new_activities
    }

    pub fn user(&self) -> &UserId {
        &self.user
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct RemoveActivityFromMetricsRequest {
    user: UserId,
    deleted_activity: Activity,
}

impl RemoveActivityFromMetricsRequest {
    pub fn deleted_activity(&self) -> &Activity {
        &self.deleted_activity
    }

    pub fn user(&self) -> &UserId {
        &self.user
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct UpdateMetricsForActivityRequest {
    user: UserId,
    activity_id: ActivityId,
}

impl UpdateMetricsForActivityRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn activity_id(&self) -> &ActivityId {
        &self.activity_id
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
    #[error("An infratstructure error occured when getting defintion")]
    GetDefinitionError(#[from] GetDefinitionError),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct UpdateTrainingMetricNameRequest {
    user: UserId,
    metric_id: TrainingMetricId,
    name: TrainingMetricName,
}

impl UpdateTrainingMetricNameRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn metric_id(&self) -> &TrainingMetricId {
        &self.metric_id
    }

    pub fn name(&self) -> &TrainingMetricName {
        &self.name
    }
}

#[derive(Debug, Error)]
pub enum UpdateTrainingMetricNameError {
    #[error("Training metric {0} does not exist")]
    MetricDoesNotExist(TrainingMetricId),
    #[error("An infrastructure error occured when getting definition")]
    GetDefinitionError(#[from] GetDefinitionError),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

///////////////////////////////////////////////////////////////////
/// TRAINING SERVICE
///////////////////////////////////////////////////////////////////
pub trait ITrainingService: Clone + Send + Sync + 'static {
    fn create_metric(
        &self,
        req: CreateTrainingMetricRequest,
    ) -> impl Future<Output = Result<TrainingMetricId, CreateTrainingMetricError>> + Send;

    fn delete_metric(
        &self,
        req: DeleteTrainingMetricRequest,
    ) -> impl Future<Output = Result<(), DeleteTrainingMetricError>> + Send;

    fn copy_training_metric(
        &self,
        req: CopyTrainingMetricRequest,
    ) -> impl Future<Output = Result<(), CopyTrainingMetricError>> + Send;

    fn get_training_metrics_values(
        &self,
        user: &UserId,
        date_range: &DateRange,
        scope: &TrainingMetricScope,
    ) -> impl Future<Output = Vec<(TrainingMetric, TrainingMetricValues)>> + Send;

    fn get_training_metric_values(
        &self,
        req: GetTrainingMetricValuesRequest,
        date_range: &DateRange,
    ) -> impl Future<Output = Result<TrainingMetricValues, GetTrainingMetricValuesError>> + Send;

    fn update_training_metric_name(
        &self,
        req: UpdateTrainingMetricNameRequest,
    ) -> impl Future<Output = Result<(), UpdateTrainingMetricNameError>> + Send;

    fn create_training_period(
        &self,
        req: CreateTrainingPeriodRequest,
    ) -> impl Future<Output = Result<TrainingPeriodId, CreateTrainingPeriodError>> + Send;

    fn get_training_periods(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Vec<TrainingPeriod>> + Send;

    fn get_active_training_periods(
        &self,
        user: &UserId,
        ref_date: &NaiveDate,
    ) -> impl Future<Output = Vec<TrainingPeriod>> + Send;

    fn get_training_period(
        &self,
        user: &UserId,
        period: &TrainingPeriodId,
    ) -> impl Future<Output = Option<TrainingPeriod>> + Send;

    fn get_training_period_with_activities_with_metrics(
        &self,
        user: &UserId,
        period: &TrainingPeriodId,
        metrics: &[ActivityMetricV2],
    ) -> impl Future<Output = Option<TrainingPeriodWithActivities>> + Send;

    fn delete_training_period(
        &self,
        req: DeleteTrainingPeriodRequest,
    ) -> impl Future<Output = Result<(), DeleteTrainingPeriodError>> + Send;

    fn update_training_period_name(
        &self,
        req: UpdateTrainingPeriodNameRequest,
    ) -> impl Future<Output = Result<(), UpdateTrainingPeriodNameError>> + Send;

    fn update_training_period_note(
        &self,
        req: UpdateTrainingPeriodNoteRequest,
    ) -> impl Future<Output = Result<(), UpdateTrainingPeriodNoteError>> + Send;

    fn update_training_period_dates(
        &self,
        req: UpdateTrainingPeriodDatesRequest,
    ) -> impl Future<Output = Result<(), UpdateTrainingPeriodDatesError>> + Send;

    fn create_training_note(
        &self,
        req: CreateTrainingNoteRequest,
    ) -> impl Future<Output = Result<TrainingNoteId, CreateTrainingNoteError>> + Send;

    fn get_training_note(
        &self,
        user: &UserId,
        note_id: &TrainingNoteId,
    ) -> impl Future<Output = Result<Option<TrainingNote>, GetTrainingNoteError>> + Send;

    fn get_training_notes(
        &self,
        user: &UserId,
        date_range: &Option<DateRange>,
    ) -> impl Future<Output = Result<Vec<TrainingNote>, GetTrainingNoteError>> + Send;

    fn update_training_note(
        &self,
        user: &UserId,
        note_id: &TrainingNoteId,
        title: Option<TrainingNoteTitle>,
        content: TrainingNoteContent,
        date: TrainingNoteDate,
    ) -> impl Future<Output = Result<(), UpdateTrainingNoteError>> + Send;

    fn delete_training_note(
        &self,
        user: &UserId,
        note_id: &TrainingNoteId,
    ) -> impl Future<Output = Result<(), DeleteTrainingNoteError>> + Send;

    fn get_training_period_notes(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
    ) -> impl Future<Output = Result<Vec<TrainingNote>, GetTrainingNoteError>> + Send;

    fn get_training_period_metrics_values(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
    ) -> impl Future<Output = Vec<(TrainingMetric, TrainingMetricValues)>> + Send;

    fn get_training_metrics_ordering(
        &self,
        user: &UserId,
        scope: &TrainingMetricScope,
    ) -> impl Future<Output = Result<TrainingMetricsOrdering, GetTrainingMetricsOrderingError>> + Send;

    fn set_training_metrics_ordering(
        &self,
        user: &UserId,
        scope: &TrainingMetricScope,
        ordering: TrainingMetricsOrdering,
    ) -> impl Future<Output = Result<(), SetTrainingMetricsOrderingError>> + Send;
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
pub enum GetTrainingMetricsOrderingError {
    #[error("Training period {0} does not exist")]
    TrainingPeriodDoesNotExist(TrainingPeriodId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum SetTrainingMetricsOrderingError {
    #[error("Training period {0} does not exist")]
    TrainingPeriodDoesNotExist(TrainingPeriodId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum GetDefinitionError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum GetTrainingMetricValuesError {
    #[error("Training metric {0:?} does not exist")]
    TrainingMetricDoesNotExists(TrainingMetricId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum ComputeTrainingMetricValuesError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

impl From<ComputeTrainingMetricValuesError> for GetTrainingMetricValuesError {
    fn from(value: ComputeTrainingMetricValuesError) -> Self {
        match value {
            ComputeTrainingMetricValuesError::Unknown(err) => Self::Unknown(err),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct CreateTrainingPeriodRequest {
    user: UserId,
    start: NaiveDate,
    end: Option<NaiveDate>,
    name: String,
    sports: TrainingPeriodSports,
    note: Option<String>,
}

impl CreateTrainingPeriodRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn start(&self) -> &NaiveDate {
        &self.start
    }

    pub fn end(&self) -> &Option<NaiveDate> {
        &self.end
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn sports(&self) -> &TrainingPeriodSports {
        &self.sports
    }

    pub fn note(&self) -> &Option<String> {
        &self.note
    }

    pub fn to_period(
        self,
        id: &TrainingPeriodId,
    ) -> Result<TrainingPeriod, TrainingPeriodCreationError> {
        TrainingPeriod::new(
            id.clone(),
            self.user,
            self.start,
            self.end,
            self.name,
            self.sports,
            self.note,
        )
    }
}

#[derive(Debug, Error)]
pub enum SaveTrainingPeriodError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum CreateTrainingPeriodError {
    #[error("Invalid period")]
    InvalidPeriod(#[from] TrainingPeriodCreationError),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct DeleteTrainingPeriodRequest {
    user: UserId,
    period_id: TrainingPeriodId,
}

impl DeleteTrainingPeriodRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn period_id(&self) -> &TrainingPeriodId {
        &self.period_id
    }
}

#[derive(Debug, Error)]
pub enum DeleteTrainingPeriodError {
    #[error("Training period {0} does not exist")]
    PeriodDoesNotExist(TrainingPeriodId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct UpdateTrainingPeriodNameRequest {
    user: UserId,
    period_id: TrainingPeriodId,
    name: String,
}

impl UpdateTrainingPeriodNameRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn period_id(&self) -> &TrainingPeriodId {
        &self.period_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Error)]
pub enum UpdateTrainingPeriodNameError {
    #[error("Training period {0} does not exist")]
    PeriodDoesNotExist(TrainingPeriodId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct UpdateTrainingPeriodNoteRequest {
    user: UserId,
    period_id: TrainingPeriodId,
    note: Option<String>,
}

impl UpdateTrainingPeriodNoteRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn period_id(&self) -> &TrainingPeriodId {
        &self.period_id
    }

    pub fn note(&self) -> &Option<String> {
        &self.note
    }
}

#[derive(Debug, Error)]
pub enum UpdateTrainingPeriodNoteError {
    #[error("Training period {0} does not exist")]
    PeriodDoesNotExist(TrainingPeriodId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct UpdateTrainingPeriodDatesRequest {
    user: UserId,
    period_id: TrainingPeriodId,
    start: NaiveDate,
    end: Option<NaiveDate>,
}

impl UpdateTrainingPeriodDatesRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn period_id(&self) -> &TrainingPeriodId {
        &self.period_id
    }

    pub fn start(&self) -> &NaiveDate {
        &self.start
    }

    pub fn end(&self) -> &Option<NaiveDate> {
        &self.end
    }
}

#[derive(Debug, Error)]
pub enum UpdateTrainingPeriodDatesError {
    #[error("Training period {0} does not exist")]
    PeriodDoesNotExist(TrainingPeriodId),
    #[error("End date must be None or after start date")]
    EndDateBeforeStartDate,
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

///////////////////////////////////////////////////////////////////
/// TRAINING NOTE TYPES
///////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct CreateTrainingNoteRequest {
    user: UserId,
    title: Option<TrainingNoteTitle>,
    content: TrainingNoteContent,
    date: TrainingNoteDate,
}

impl CreateTrainingNoteRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn title(&self) -> &Option<TrainingNoteTitle> {
        &self.title
    }

    pub fn content(&self) -> &TrainingNoteContent {
        &self.content
    }

    pub fn date(&self) -> &TrainingNoteDate {
        &self.date
    }
}

#[derive(Debug, Error)]
pub enum CreateTrainingNoteError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum SaveTrainingNoteError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum GetTrainingNoteError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum UpdateTrainingNoteError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum DeleteTrainingNoteError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

///////////////////////////////////////////////////////////////////
/// TRAINING REPOSITORY
///////////////////////////////////////////////////////////////////
pub trait TrainingRepository: Clone + Send + Sync + 'static {
    fn save_training_metric_definition(
        &self,
        metric: TrainingMetric,
    ) -> impl Future<Output = Result<(), SaveTrainingMetricError>> + Send;

    fn get_definition(
        &self,
        user: &UserId,
        metric: &TrainingMetricId,
    ) -> impl Future<Output = Result<Option<TrainingMetricDefinition>, GetDefinitionError>> + Send;

    fn delete_definition(
        &self,
        user: &UserId,
        metric: &TrainingMetricId,
    ) -> impl Future<Output = Result<(), DeleteTrainingMetricError>> + Send;

    fn update_training_metric_name(
        &self,
        user: &UserId,
        metric_id: &TrainingMetricId,
        name: TrainingMetricName,
    ) -> impl Future<Output = Result<(), UpdateTrainingMetricNameError>> + Send;

    fn get_global_metrics(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<TrainingMetric>, GetTrainingMetricsDefinitionsError>> + Send;

    fn get_period_metrics(
        &self,
        user: &UserId,
        period: &TrainingPeriodId,
    ) -> impl Future<Output = Result<Vec<TrainingMetric>, GetTrainingMetricsDefinitionsError>> + Send;

    fn save_training_period(
        &self,
        period: TrainingPeriod,
    ) -> impl Future<Output = Result<(), SaveTrainingPeriodError>> + Send;

    fn get_training_periods(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Vec<TrainingPeriod>> + Send;

    fn get_active_training_periods(
        &self,
        user: &UserId,
        ref_date: &NaiveDate,
    ) -> impl Future<Output = Vec<TrainingPeriod>> + Send;

    fn get_training_period(
        &self,
        user: &UserId,
        period: &TrainingPeriodId,
    ) -> impl Future<Output = Option<TrainingPeriod>> + Send;

    fn delete_training_period(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
    ) -> impl Future<Output = Result<(), DeleteTrainingPeriodError>> + Send;

    fn update_training_period_name(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
        name: String,
    ) -> impl Future<Output = Result<(), UpdateTrainingPeriodNameError>> + Send;

    fn update_training_period_note(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
        note: Option<String>,
    ) -> impl Future<Output = Result<(), UpdateTrainingPeriodNoteError>> + Send;

    fn update_training_period_dates(
        &self,
        user: &UserId,
        period_id: &TrainingPeriodId,
        start: NaiveDate,
        end: Option<NaiveDate>,
    ) -> impl Future<Output = Result<(), UpdateTrainingPeriodDatesError>> + Send;

    fn save_training_note(
        &self,
        note: TrainingNote,
    ) -> impl Future<Output = Result<(), SaveTrainingNoteError>> + Send;

    fn get_training_note(
        &self,
        user: &UserId,
        note_id: &TrainingNoteId,
    ) -> impl Future<Output = Result<Option<TrainingNote>, GetTrainingNoteError>> + Send;

    fn get_training_notes(
        &self,
        user: &UserId,
        date_range: &Option<DateRange>,
    ) -> impl Future<Output = Result<Vec<TrainingNote>, GetTrainingNoteError>> + Send;

    fn update_training_note(
        &self,
        user: &UserId,
        note_id: &TrainingNoteId,
        title: Option<TrainingNoteTitle>,
        content: TrainingNoteContent,
        date: TrainingNoteDate,
    ) -> impl Future<Output = Result<(), UpdateTrainingNoteError>> + Send;

    fn delete_training_note(
        &self,
        user: &UserId,
        note_id: &TrainingNoteId,
    ) -> impl Future<Output = Result<(), DeleteTrainingNoteError>> + Send;

    fn get_training_metrics_ordering(
        &self,
        user: &UserId,
        scope: &TrainingMetricScope,
    ) -> impl Future<Output = Result<TrainingMetricsOrdering, GetTrainingMetricsOrderingError>> + Send;

    fn set_training_metrics_ordering(
        &self,
        user: &UserId,
        scope: &TrainingMetricScope,
        ordering: TrainingMetricsOrdering,
    ) -> impl Future<Output = Result<(), SetTrainingMetricsOrderingError>> + Send;
}
