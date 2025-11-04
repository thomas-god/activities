use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::domain::{
    models::{preferences::Preference, training::TrainingMetricId},
    ports::{DeletePreferenceError, GetPreferenceError, SetPreferenceError},
};

#[derive(Debug, Serialize)]
#[serde(tag = "key", content = "value")]
pub enum PreferenceResponse {
    #[serde(rename = "favorite_metric")]
    FavoriteMetric(String),
}

impl From<Preference> for PreferenceResponse {
    fn from(pref: Preference) -> Self {
        match pref {
            Preference::FavoriteMetric(id) => PreferenceResponse::FavoriteMetric(id.to_string()),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "key", content = "value")]
pub enum SetPreferenceRequest {
    #[serde(rename = "favorite_metric")]
    FavoriteMetric(String),
}

impl From<SetPreferenceRequest> for Preference {
    fn from(req: SetPreferenceRequest) -> Self {
        match req {
            SetPreferenceRequest::FavoriteMetric(id) => {
                Preference::FavoriteMetric(TrainingMetricId::from(id.as_str()))
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
