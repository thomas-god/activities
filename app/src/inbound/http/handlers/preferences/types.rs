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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "key", content = "value")]
pub enum PreferencePayload {
    #[serde(rename = "activity_list_summary")]
    ActivityListSummary(APIActivityListSummary),
}

impl From<PreferencePayload> for Preference {
    fn from(req: PreferencePayload) -> Self {
        match req {
            PreferencePayload::ActivityListSummary(summary) => {
                Preference::ActivityListSummary(ActivityListSummary::from(summary))
            }
        }
    }
}

impl TryFrom<Preference> for PreferencePayload {
    type Error = String;

    fn try_from(value: Preference) -> Result<Self, Self::Error> {
        match value {
            Preference::ActivityListSummary(summary) => Ok(PreferencePayload::ActivityListSummary(
                APIActivityListSummary::from(summary),
            )),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", content = "value")]
pub enum APIActivityListSummaryItem {
    Metric(ActivityMetricV2),
    #[serde(rename = "rpe")]
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

impl From<&ActivityListSummaryItem> for APIActivityListSummaryItem {
    fn from(value: &ActivityListSummaryItem) -> Self {
        match value {
            ActivityListSummaryItem::Metric(metric) => Self::Metric(*metric),
            ActivityListSummaryItem::RPE => Self::RPE,
            ActivityListSummaryItem::WorkoutType => Self::WorkoutType,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

impl From<ActivityListSummary> for APIActivityListSummary {
    fn from(value: ActivityListSummary) -> Self {
        Self {
            scope: value.scope().into(),
            items: value
                .items()
                .iter()
                .map(APIActivityListSummaryItem::from)
                .collect(),
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

#[cfg(test)]
mod test_http_preferences {
    use super::*;

    #[test]
    fn test_serialize_activity_list_summary() {
        let json = r#"{
            "scope": {
                "type": "trainingPeriod",
                "trainingPeriodId": "test-id"
            },
            "items": [
                {"type": "metric", "value": "Distance"},
                {"type": "rpe"},
                {"type": "workoutType"}
            ]
        }"#;
        let result: Result<APIActivityListSummary, _> = serde_json::from_str(json);

        assert_eq!(
            APIActivityListSummary {
                scope: APITrainingMetricScope::TrainingPeriod {
                    training_period_id: "test-id".to_string()
                },
                items: vec![
                    APIActivityListSummaryItem::Metric(ActivityMetricV2::Distance),
                    APIActivityListSummaryItem::RPE,
                    APIActivityListSummaryItem::WorkoutType,
                ]
            },
            result.unwrap()
        );
    }

    #[test]
    fn test_serialize_preference_payload_activity_list_summary() {
        let json = r#"{
         "key": "activity_list_summary",
         "value": {
            "scope": {
                "type": "trainingPeriod",
                "trainingPeriodId": "test-id"
            },
            "items": [
                {"type": "metric", "value": "Distance"},
                {"type": "rpe"},
                {"type": "workoutType"}
            ]
        }}"#;
        let result: Result<PreferencePayload, _> = serde_json::from_str(json);

        assert_eq!(
            PreferencePayload::ActivityListSummary(APIActivityListSummary {
                scope: APITrainingMetricScope::TrainingPeriod {
                    training_period_id: "test-id".to_string()
                },
                items: vec![
                    APIActivityListSummaryItem::Metric(ActivityMetricV2::Distance),
                    APIActivityListSummaryItem::RPE,
                    APIActivityListSummaryItem::WorkoutType,
                ]
            }),
            result.unwrap()
        );
    }
}
