use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::{
            activity::ActivityMetricV2,
            preferences::{ActivityListSummary, ActivityListSummaryItem, Preference},
            training::{TrainingMetricId, TrainingMetricScope},
        },
        ports::preferences::{DeletePreferenceError, GetPreferenceError, SetPreferenceError},
    },
    inbound::http::handlers::training::types::APITrainingMetricScope,
};

#[derive(Debug, Serialize)]
#[serde(tag = "key", content = "value")]
pub enum PreferenceResponse {
    #[serde(rename = "favorite_metric")]
    FavoriteMetric(String),
    #[serde(rename = "favorite_metric")]
    ActivityListSummary(String),
}

impl TryFrom<Preference> for PreferenceResponse {
    type Error = String;

    fn try_from(value: Preference) -> Result<Self, Self::Error> {
        match value {
            Preference::FavoriteMetric(id) => {
                Ok(PreferenceResponse::FavoriteMetric(id.to_string()))
            }
            Preference::ActivityListSummary(summary) => {
                Ok(PreferenceResponse::ActivityListSummary(
                    serde_json::to_string(&summary)
                        .map_err(|err| format!("Cannot serialize {:?}: {}", &summary, err))?,
                ))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum APIActivityListSummaryItem {
    Metric(ActivityMetricV2),
    #[allow(clippy::upper_case_acronyms)]
    RPE,
    WorkoutType,
}

impl From<APIActivityListSummaryItem> for ActivityListSummaryItem {
    fn from(value: APIActivityListSummaryItem) -> Self {
        match value {
            APIActivityListSummaryItem::Metric(metric) => Self::Metric(metric),
            APIActivityListSummaryItem::RPE => Self::RPE,
            APIActivityListSummaryItem::WorkoutType => Self::WorkoutType,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct APIActivityListSummary {
    scope: APITrainingMetricScope,
    items: Vec<APIActivityListSummaryItem>,
}

impl From<APIActivityListSummary> for ActivityListSummary {
    fn from(value: APIActivityListSummary) -> Self {
        Self::new(
            value.scope.into(),
            value
                .items
                .into_iter()
                .map(ActivityListSummaryItem::from)
                .collect(),
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "key", content = "value")]
pub enum SetPreferenceRequest {
    #[serde(rename = "favorite_metric")]
    FavoriteMetric(String),
    #[serde(rename = "activity_list_summary")]
    ActivityListSummary(APIActivityListSummary),
}

impl From<SetPreferenceRequest> for Preference {
    fn from(req: SetPreferenceRequest) -> Self {
        match req {
            SetPreferenceRequest::FavoriteMetric(id) => {
                Preference::FavoriteMetric(TrainingMetricId::from(id.as_str()))
            }
            SetPreferenceRequest::ActivityListSummary(summary) => {
                Preference::ActivityListSummary(ActivityListSummary::from(summary))
            }
        }
    }
}

impl From<GetPreferenceError> for StatusCode {
    fn from(_value: GetPreferenceError) -> Self {
        Self::INTERNAL_SERVER_ERROR
    }
}

impl From<SetPreferenceError> for StatusCode {
    fn from(_value: SetPreferenceError) -> Self {
        Self::INTERNAL_SERVER_ERROR
    }
}

impl From<DeletePreferenceError> for StatusCode {
    fn from(_value: DeletePreferenceError) -> Self {
        Self::INTERNAL_SERVER_ERROR
    }
}
