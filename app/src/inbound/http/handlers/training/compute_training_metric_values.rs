use std::collections::HashMap;

use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::training::{TrainingMetricDefinition, TrainingMetricFilters},
        ports::{
            ComputeTrainingMetricValuesError, DateRange, IActivityService, IPreferencesService,
            ITrainingService,
        },
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
            handlers::training::types::{
                APITrainingMetricAggregate, APITrainingMetricFilters, APITrainingMetricGranularity,
                APITrainingMetricGroupBy, APITrainingMetricSource,
            },
        },
        parser::ParseFile,
    },
};

/// Request body for computing training metric values
#[derive(Debug, Deserialize)]
pub struct ComputeMetricValuesRequest {
    source: APITrainingMetricSource,
    granularity: APITrainingMetricGranularity,
    aggregate: APITrainingMetricAggregate,
    #[serde(default)]
    filters: Option<APITrainingMetricFilters>,
    #[serde(default)]
    group_by: Option<APITrainingMetricGroupBy>,
    start: DateTime<FixedOffset>,
    end: Option<DateTime<FixedOffset>>,
}

impl From<&ComputeMetricValuesRequest> for DateRange {
    fn from(value: &ComputeMetricValuesRequest) -> Self {
        let start_date = value.start.date_naive();
        let end_date = value
            .end
            .map(|e| e.date_naive())
            .unwrap_or_else(|| Local::now().date_naive());
        Self::new(start_date, end_date)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody {
    values: HashMap<String, MetricValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricValue {
    value: f64,
    group: Option<String>,
}

impl From<ComputeTrainingMetricValuesError> for StatusCode {
    fn from(value: ComputeTrainingMetricValuesError) -> Self {
        match value {
            ComputeTrainingMetricValuesError::Unknown(_) => Self::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn compute_training_metric_values<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR, PS>>,
    Json(request): Json<ComputeMetricValuesRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let date_range = DateRange::from(&request);

    // Build the training metric definition from the request
    let filters = request
        .filters
        .map(TrainingMetricFilters::from)
        .unwrap_or_else(TrainingMetricFilters::empty);

    let group_by = request.group_by.map(|gb| gb.into());

    let definition = TrainingMetricDefinition::new(
        user.user().clone(),
        request.source.into(),
        request.granularity.into(),
        request.aggregate.into(),
        filters,
        group_by,
    );

    let values = state
        .training_metrics_service
        .compute_training_metric_values(&definition, &date_range)
        .await
        .map_err(StatusCode::from)?;

    let response_values: HashMap<String, MetricValue> = values
        .into_iter()
        .map(|(bin, value)| {
            (
                bin.granule().to_string(),
                MetricValue {
                    value: value.value(),
                    group: bin.group().clone(),
                },
            )
        })
        .collect();

    Ok(Json(ResponseBody {
        values: response_values,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_request_deserialize_minimal() {
        // Test with only required fields
        // Demonstrates basic JSON format for the request
        let json = r#"{
            "source": {"Statistic": "Calories"},
            "granularity": "Daily",
            "aggregate": "Sum",
            "start": "2024-01-01T00:00:00+00:00"
        }"#;
        let result: Result<ComputeMetricValuesRequest, _> = serde_json::from_str(json);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert!(request.filters.is_none());
        assert!(request.group_by.is_none());
        assert!(request.end.is_none());
    }

    #[test]
    fn test_request_deserialize_all_fields() {
        // Test with all fields provided
        // Demonstrates complete JSON format including:
        // - Statistic source (alternative: {"Timeseries": ["Speed", "Average"]})
        // - Optional end date
        // - Optional group_by (values: Sport, SportCategory, WorkoutType, RpeRange, Bonked)
        // - Optional filters with sports (Sport or SportCategory)
        let json = r#"{
            "source": {"Statistic": "Calories"},
            "granularity": "Daily",
            "aggregate": "Sum",
            "start": "2024-01-01T00:00:00+00:00",
            "end": "2024-12-31T23:59:59+00:00",
            "group_by": "Sport",
            "filters": {"sports": [{"Sport": "Running"}]}
        }"#;
        let result: Result<ComputeMetricValuesRequest, _> = serde_json::from_str(json);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert!(request.filters.is_some());
        assert!(request.group_by.is_some());
        assert!(request.end.is_some());
    }
}
