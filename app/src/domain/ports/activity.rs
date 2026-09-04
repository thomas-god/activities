use derive_more::Constructor;
use thiserror::Error;

///////////////////////////////////////////////////////////////////
/// ACTIVITY SERVICE
///////////////////////////////////////////////////////////////////
use crate::domain::{
    models::{
        UserId,
        activity::{
            Activity, ActivityDuration, ActivityId, ActivityMetricV2, ActivityMetricsV2,
            ActivityNaturalKey, ActivityPatch, ActivityStartTime, ActivityStatistics,
            ActivityTimeseries, ActivityWithParsedData, Sport,
        },
        search::SearchDocument,
    },
    ports::{DateRange, DateTimeRange, search::DocumentsRemaining},
};

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

    fn list_activities_with_parsed_data(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> impl Future<Output = Result<Vec<ActivityWithParsedData>, ListActivitiesError>> + Send;

    fn list_activities_with_metrics(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
        metrics: &[ActivityMetricV2],
    ) -> impl Future<Output = Result<Vec<(Activity, ActivityMetricsV2)>, ListActivitiesError>> + Send;

    fn list_activities_with_metrics_and_parsed_data(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
        metrics: &[ActivityMetricV2],
    ) -> impl Future<
        Output = Result<Vec<(ActivityWithParsedData, ActivityMetricsV2)>, ListActivitiesError>,
    > + Send;

    fn get_activity_with_parsed_data(
        &self,
        activity_id: &ActivityId,
    ) -> impl Future<Output = Result<ActivityWithParsedData, GetActivityError>> + Send;

    fn get_activity_with_metrics_and_parsed_data(
        &self,
        activity_id: &ActivityId,
        metrics: &[ActivityMetricV2],
    ) -> impl Future<Output = Result<(ActivityWithParsedData, ActivityMetricsV2), GetActivityError>> + Send;

    fn patch_activity(
        &self,
        req: PatchActivityRequest,
    ) -> impl Future<Output = Result<(), PatchActivityError>> + Send;

    fn delete_activity(
        &self,
        req: DeleteActivityRequest,
    ) -> impl Future<Output = Result<(), DeleteActivityError>> + Send;

    fn get_raw_activity(
        &self,
        req: GetRawActivityRequest,
    ) -> impl Future<Output = Result<RawActivity, GetRawActivityError>> + Send;

    fn get_all_raw_activities(
        &self,
        req: GetAllActivitiesRequest,
    ) -> impl Future<Output = Result<Vec<RawActivity>, GetAllActivitiesError>> + Send;
}

#[derive(Debug, Clone)]
pub struct CreateActivityRequest {
    user: UserId,
    sport: Sport,
    start_time: ActivityStartTime,
    duration: ActivityDuration,
    statistics: ActivityStatistics,
    timeseries: ActivityTimeseries,
    raw_content: RawContent,
}

impl CreateActivityRequest {
    pub fn new(
        user: UserId,
        sport: Sport,
        start_time: ActivityStartTime,
        duration: ActivityDuration,
        statistics: ActivityStatistics,
        timeseries: ActivityTimeseries,
        raw_content: RawContent,
    ) -> Self {
        Self {
            user,
            sport,
            start_time,
            duration,
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

    pub fn duration(&self) -> &ActivityDuration {
        &self.duration
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
pub struct PatchActivityRequest {
    activity: ActivityId,
    user: UserId,
    patch: ActivityPatch,
}

impl PatchActivityRequest {
    pub fn activity(&self) -> &ActivityId {
        &self.activity
    }
    pub fn user(&self) -> &UserId {
        &self.user
    }
    pub fn patch(&self) -> &ActivityPatch {
        &self.patch
    }
    pub fn as_patch(self) -> ActivityPatch {
        self.patch
    }
}

#[derive(Debug, Error)]
pub enum PatchActivityError {
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

#[derive(Debug, Clone, Constructor)]
pub struct GetRawActivityRequest {
    activity: ActivityId,
    user: UserId,
}

impl GetRawActivityRequest {
    pub fn activity(&self) -> &ActivityId {
        &self.activity
    }
    pub fn user(&self) -> &UserId {
        &self.user
    }
}

#[derive(Debug, Error)]
pub enum GetRawActivityError {
    #[error("Activity {0} does not exists")]
    ActivityDoesNotExist(ActivityId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Constructor)]
pub struct GetAllActivitiesRequest {
    user: UserId,
}

impl GetAllActivitiesRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct RawActivity {
    name: String,
    content: Vec<u8>,
}

impl RawActivity {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }

    pub fn as_vec(self) -> Vec<u8> {
        self.content
    }
}

#[derive(Debug, Error)]
pub enum GetAllActivitiesError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
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
pub enum UpdateActivityMetricError {
    #[error("Activity {0} does not exist")]
    ActivityDoesNotExist(ActivityId),
    #[error("User {0} does not own activity {1}")]
    UserDoesNotOwnActivity(UserId, ActivityId),
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

    fn list_activity_documents(
        &self,
        batch_size: i64,
        offset: i64,
    ) -> impl Future<Output = Result<(Vec<SearchDocument>, DocumentsRemaining), anyhow::Error>> + Send;

    fn list_user_activities(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> impl Future<Output = Result<Vec<Activity>, ListActivitiesError>> + Send;

    fn get_raw_activity(
        &self,
        user: &UserId,
        activity: &ActivityId,
    ) -> impl Future<Output = Result<RawActivity, GetRawActivityError>> + Send;

    fn list_all_raw_activities(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Vec<RawActivity>, ListActivitiesError>> + Send;

    fn list_activities_with_parsed_data(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
    ) -> impl Future<Output = Result<Vec<ActivityWithParsedData>, ListActivitiesError>> + Send;

    fn update_activity_metric(
        &self,
        activity: &ActivityId,
        metric: &ActivityMetricV2,
        value: &Option<f64>,
    ) -> impl Future<Output = Result<(), UpdateActivityMetricError>> + Send;

    fn get_activities_with_metrics(
        &self,
        user: &UserId,
        filters: &ListActivitiesFilters,
        metrics: &[ActivityMetricV2],
    ) -> impl Future<Output = Result<Vec<(Activity, ActivityMetricsV2)>, ListActivitiesError>> + Send;

    fn get_activity(
        &self,
        id: &ActivityId,
    ) -> impl Future<Output = Result<Option<Activity>, GetActivityError>> + Send;

    fn get_activity_with_metrics(
        &self,
        id: &ActivityId,
        metrics: &[ActivityMetricV2],
    ) -> impl Future<Output = Result<Option<(Activity, ActivityMetricsV2)>, GetActivityError>> + Send;

    fn get_activity_with_parsed_data(
        &self,
        id: &ActivityId,
    ) -> impl Future<Output = Result<Option<ActivityWithParsedData>, GetActivityError>> + Send;

    fn delete_activity(
        &self,
        user: &UserId,
        activity: &ActivityId,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send;

    fn get_user_history_date_range(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<Option<DateTimeRange>, anyhow::Error>> + Send;

    fn get_outbox_documents_to_process(
        &self,
    ) -> impl Future<Output = Result<Vec<SearchDocument>, anyhow::Error>> + Send;

    fn mark_outbox_document_as_processed(
        &self,
        document: &SearchDocument,
        processed_at: chrono::DateTime<chrono::Utc>,
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
        content: RawContent,
    ) -> impl Future<Output = Result<(), SaveRawDataError>> + Send;

    fn get_raw_data(
        &self,
        activity_id: &ActivityId,
    ) -> impl Future<Output = Result<RawContent, GetRawDataError>> + Send;
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
