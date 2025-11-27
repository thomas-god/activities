use chrono::{DateTime, FixedOffset, NaiveDate};
use derive_more::Constructor;
use serde::Deserialize;
use thiserror::Error;

use crate::domain::models::UserId;
use crate::domain::models::activity::{
    Activity, ActivityFeedback, ActivityId, ActivityName, ActivityNaturalKey, ActivityNutrition,
    ActivityRpe, ActivityStartTime, ActivityStatistics, ActivityTimeseries, ActivityWithTimeseries,
    Sport, WorkoutType,
};
use crate::domain::models::training::{
    ActivityMetricSource, TrainingMetric, TrainingMetricAggregate, TrainingMetricDefinition,
    TrainingMetricFilters, TrainingMetricGranularity, TrainingMetricGroupBy, TrainingMetricId,
    TrainingMetricName, TrainingMetricScope, TrainingMetricValues, TrainingNote,
    TrainingNoteContent, TrainingNoteDate, TrainingNoteId, TrainingNoteTitle, TrainingPeriod,
    TrainingPeriodCreationError, TrainingPeriodId, TrainingPeriodSports,
    TrainingPeriodWithActivities,
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
pub struct UpdateActivityRpeRequest {
    user: UserId,
    activity: ActivityId,
    rpe: Option<ActivityRpe>,
}

impl UpdateActivityRpeRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn activity(&self) -> &ActivityId {
        &self.activity
    }

    pub fn rpe(&self) -> Option<&ActivityRpe> {
        self.rpe.as_ref()
    }
}

#[derive(Debug, Error)]
pub enum UpdateActivityRpeError {
    #[error("Activity {0} does not exists")]
    ActivityDoesNotExist(ActivityId),
    #[error("User {0} does not own activity {1}")]
    UserDoesNotOwnActivity(UserId, ActivityId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Constructor)]
pub struct UpdateActivityWorkoutTypeRequest {
    user: UserId,
    activity: ActivityId,
    workout_type: Option<WorkoutType>,
}

impl UpdateActivityWorkoutTypeRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn activity(&self) -> &ActivityId {
        &self.activity
    }

    pub fn workout_type(&self) -> Option<&WorkoutType> {
        self.workout_type.as_ref()
    }
}

#[derive(Debug, Error)]
pub enum UpdateActivityWorkoutTypeError {
    #[error("Activity {0} does not exists")]
    ActivityDoesNotExist(ActivityId),
    #[error("User {0} does not own activity {1}")]
    UserDoesNotOwnActivity(UserId, ActivityId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Constructor)]
pub struct UpdateActivityNutritionRequest {
    user: UserId,
    activity: ActivityId,
    nutrition: Option<ActivityNutrition>,
}

impl UpdateActivityNutritionRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn activity(&self) -> &ActivityId {
        &self.activity
    }

    pub fn nutrition(&self) -> &Option<ActivityNutrition> {
        &self.nutrition
    }
}

#[derive(Debug, Error)]
pub enum UpdateActivityNutritionError {
    #[error("Activity {0} does not exists")]
    ActivityDoesNotExist(ActivityId),
    #[error("User {0} does not own activity {1}")]
    UserDoesNotOwnActivity(UserId, ActivityId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Constructor)]
pub struct UpdateActivityFeedbackRequest {
    user: UserId,
    activity: ActivityId,
    feedback: Option<ActivityFeedback>,
}

impl UpdateActivityFeedbackRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn activity(&self) -> &ActivityId {
        &self.activity
    }

    pub fn feedback(&self) -> &Option<ActivityFeedback> {
        &self.feedback
    }
}

#[derive(Debug, Error)]
pub enum UpdateActivityFeedbackError {
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

    fn update_activity_rpe(
        &self,
        req: UpdateActivityRpeRequest,
    ) -> impl Future<Output = Result<(), UpdateActivityRpeError>> + Send;

    fn update_activity_workout_type(
        &self,
        req: UpdateActivityWorkoutTypeRequest,
    ) -> impl Future<Output = Result<(), UpdateActivityWorkoutTypeError>> + Send;

    fn update_activity_nutrition(
        &self,
        req: UpdateActivityNutritionRequest,
    ) -> impl Future<Output = Result<(), UpdateActivityNutritionError>> + Send;

    fn update_activity_feedback(
        &self,
        req: UpdateActivityFeedbackRequest,
    ) -> impl Future<Output = Result<(), UpdateActivityFeedbackError>> + Send;

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

    fn update_activity_rpe(
        &self,
        id: &ActivityId,
        rpe: Option<ActivityRpe>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn update_activity_workout_type(
        &self,
        id: &ActivityId,
        workout_type: Option<WorkoutType>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn update_activity_nutrition(
        &self,
        id: &ActivityId,
        nutrition: Option<ActivityNutrition>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn update_activity_feedback(
        &self,
        id: &ActivityId,
        feedback: Option<ActivityFeedback>,
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
    name: TrainingMetricName,
    source: ActivityMetricSource,
    granularity: TrainingMetricGranularity,
    aggregate: TrainingMetricAggregate,
    filters: TrainingMetricFilters,
    group_by: Option<TrainingMetricGroupBy>,
    training_period: Option<TrainingPeriodId>,
    initial_date_range: Option<DateRange>,
}

impl CreateTrainingMetricRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn name(&self) -> &TrainingMetricName {
        &self.name
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

    pub fn group_by(&self) -> &Option<TrainingMetricGroupBy> {
        &self.group_by
    }

    pub fn training_period(&self) -> &Option<TrainingPeriodId> {
        &self.training_period
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
    #[error("User {0} does not own training metric {1}")]
    UserDoesNotOwnTrainingMetric(UserId, TrainingMetricId),
    #[error("An infratstructure error occured when getting defintion")]
    GetDefinitionError(#[from] GetDefinitionError),
    #[error("An infratstructure error occured when trying to delete defintion")]
    DeleteMetricError(#[from] DeleteMetricError),
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
    #[error("User {0} does not own training metric {1}")]
    UserDoesNotOwnTrainingMetric(UserId, TrainingMetricId),
    #[error("An infrastructure error occured when getting definition")]
    GetDefinitionError(#[from] GetDefinitionError),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct UpdateTrainingMetricScopeRequest {
    user: UserId,
    metric_id: TrainingMetricId,
    scope: TrainingMetricScope,
}

impl UpdateTrainingMetricScopeRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn metric_id(&self) -> &TrainingMetricId {
        &self.metric_id
    }

    pub fn scope(&self) -> &TrainingMetricScope {
        &self.scope
    }
}

#[derive(Debug, Error)]
pub enum UpdateTrainingMetricScopeError {
    #[error("Training metric {0} does not exist")]
    MetricDoesNotExist(TrainingMetricId),
    #[error("User {0} does not own training metric {1}")]
    UserDoesNotOwnTrainingMetric(UserId, TrainingMetricId),
    #[error("Training period {0} does not exist")]
    TrainingPeriodDoesNotExist(TrainingPeriodId),
    #[error("An infrastructure error occured when getting definition")]
    GetDefinitionError(#[from] GetDefinitionError),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait ITrainingService: Clone + Send + Sync + 'static {
    fn create_metric(
        &self,
        req: CreateTrainingMetricRequest,
    ) -> impl Future<Output = Result<TrainingMetricId, CreateTrainingMetricError>> + Send;

    fn delete_metric(
        &self,
        req: DeleteTrainingMetricRequest,
    ) -> impl Future<Output = Result<(), DeleteTrainingMetricError>> + Send;

    fn update_metric_scope(
        &self,
        req: UpdateTrainingMetricScopeRequest,
    ) -> impl Future<Output = Result<(), UpdateTrainingMetricScopeError>> + Send;

    fn get_training_metrics_values(
        &self,
        user: &UserId,
        date_range: &Option<DateRange>,
        scope: &TrainingMetricScope,
    ) -> impl Future<Output = Vec<(TrainingMetric, TrainingMetricValues)>> + Send;

    fn get_training_metric_values(
        &self,
        user: &UserId,
        metric_id: &TrainingMetricId,
        date_range: &DateRange,
    ) -> impl Future<Output = Result<TrainingMetricValues, GetTrainingMetricValuesError>> + Send;

    fn compute_training_metric_values(
        &self,
        definition: &TrainingMetricDefinition,
        date_range: &DateRange,
    ) -> impl Future<Output = Result<TrainingMetricValues, ComputeTrainingMetricValuesError>> + Send;

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

    fn get_training_period(
        &self,
        user: &UserId,
        period: &TrainingPeriodId,
    ) -> impl Future<Output = Option<TrainingPeriod>> + Send;

    fn get_training_period_with_activities(
        &self,
        user: &UserId,
        period: &TrainingPeriodId,
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
}

#[derive(Debug, Error)]
pub enum SaveTrainingMetricError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum UpdateTrainingMetricScopeRepositoryError {
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
    #[error("User {0} does not own training period {1}")]
    UserDoesNotOwnPeriod(UserId, TrainingPeriodId),
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
    #[error("User {0} does not own training period {1}")]
    UserDoesNotOwnPeriod(UserId, TrainingPeriodId),
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
    #[error("User {0} does not own training period {1}")]
    UserDoesNotOwnPeriod(UserId, TrainingPeriodId),
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
    #[error("User {0} does not own training period {1}")]
    UserDoesNotOwnPeriod(UserId, TrainingPeriodId),
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

pub trait TrainingRepository: Clone + Send + Sync + 'static {
    fn save_training_metric_definition(
        &self,
        metric: TrainingMetric,
    ) -> impl Future<Output = Result<(), SaveTrainingMetricError>> + Send;

    fn update_training_metric_scope(
        &self,
        metric: &TrainingMetricId,
        scope: &TrainingMetricScope,
    ) -> impl Future<Output = Result<(), UpdateTrainingMetricScopeRepositoryError>> + Send;

    fn get_definition(
        &self,
        metric: &TrainingMetricId,
    ) -> impl Future<Output = Result<Option<TrainingMetricDefinition>, GetDefinitionError>> + Send;

    fn delete_definition(
        &self,
        metric: &TrainingMetricId,
    ) -> impl Future<Output = Result<(), DeleteMetricError>> + Send;

    fn update_training_metric_name(
        &self,
        metric_id: &TrainingMetricId,
        name: TrainingMetricName,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

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

    fn get_training_period(
        &self,
        user: &UserId,
        period: &TrainingPeriodId,
    ) -> impl Future<Output = Option<TrainingPeriod>> + Send;

    fn delete_training_period(
        &self,
        period_id: &TrainingPeriodId,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn update_training_period_name(
        &self,
        period_id: &TrainingPeriodId,
        name: String,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn update_training_period_note(
        &self,
        period_id: &TrainingPeriodId,
        note: Option<String>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn update_training_period_dates(
        &self,
        period_id: &TrainingPeriodId,
        start: NaiveDate,
        end: Option<NaiveDate>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn save_training_note(
        &self,
        note: TrainingNote,
    ) -> impl Future<Output = Result<(), SaveTrainingNoteError>> + Send;

    fn get_training_note(
        &self,
        note_id: &TrainingNoteId,
    ) -> impl Future<Output = Result<Option<TrainingNote>, GetTrainingNoteError>> + Send;

    fn get_training_notes(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<TrainingNote>, GetTrainingNoteError>> + Send;

    fn update_training_note(
        &self,
        note_id: &TrainingNoteId,
        title: Option<TrainingNoteTitle>,
        content: TrainingNoteContent,
        date: TrainingNoteDate,
    ) -> impl Future<Output = Result<(), UpdateTrainingNoteError>> + Send;

    fn delete_training_note(
        &self,
        note_id: &TrainingNoteId,
    ) -> impl Future<Output = Result<(), DeleteTrainingNoteError>> + Send;
}

///////////////////////////////////////////////////////////////////
/// PREFERENCES SERVICE
///////////////////////////////////////////////////////////////////
use crate::domain::models::preferences::{Preference, PreferenceKey};

#[derive(Debug, Error)]
pub enum GetPreferenceError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum SetPreferenceError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum DeletePreferenceError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait IPreferencesService: Clone + Send + Sync + 'static {
    /// Get a specific preference for a user
    fn get_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> impl Future<Output = Result<Option<Preference>, GetPreferenceError>> + Send;

    /// Get all preferences for a user
    fn get_all_preferences(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Preference>, GetPreferenceError>> + Send;

    /// Set (add or modify) a preference for a user
    fn set_preference(
        &self,
        user: &UserId,
        preference: Preference,
    ) -> impl Future<Output = Result<(), SetPreferenceError>> + Send;

    /// Delete a specific preference for a user
    fn delete_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> impl Future<Output = Result<(), DeletePreferenceError>> + Send;
}

///////////////////////////////////////////////////////////////////
/// PREFERENCES REPOSITORY
///////////////////////////////////////////////////////////////////

#[derive(Debug, Error)]
pub enum SavePreferenceError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

pub trait PreferencesRepository: Clone + Send + Sync + 'static {
    fn get_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> impl Future<Output = Result<Option<Preference>, anyhow::Error>> + Send;

    fn get_all_preferences(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<Preference>, anyhow::Error>> + Send;

    fn save_preference(
        &self,
        user: &UserId,
        preference: &Preference,
    ) -> impl Future<Output = Result<(), SavePreferenceError>> + Send;

    fn delete_preference(
        &self,
        user: &UserId,
        key: &PreferenceKey,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;
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
