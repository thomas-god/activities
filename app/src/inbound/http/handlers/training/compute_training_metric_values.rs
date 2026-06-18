use std::collections::HashMap;

use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::{
            activity::{ActivityMetricSource, ActivityMetricV2},
            training::{
                TrainingMetricAggregate, TrainingMetricDefinition, TrainingMetricFilters,
                TrainingMetricGranularity, TrainingMetricWindow,
            },
        },
        ports::{
            DateRange,
            activity::IActivityService,
            preferences::IPreferencesService,
            training::{
                ComputeTrainingMetricValuesError, GetTrainingMetricValuesRequest, ITrainingService,
            },
        },
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
            handlers::training::{
                types::{
                    APITimeseriesWindow, APITrainingMetricAggregate, APITrainingMetricFilters,
                    APITrainingMetricGranularity, APITrainingMetricGroupBy,
                    APITrainingMetricSource, APITrainingMetricSummary,
                },
                utils::{
                    GranuleValues, GroupedMetricValues, MetricsDateRange,
                    convert_metric_values_unit, fill_missing_granules, group_metric_values,
                },
            },
        },
        parser::ParseFile,
    },
};

/// Request body for computing training metric values
#[derive(Debug, Deserialize)]
pub struct ComputeMetricValuesRequest {
    metric: ActivityMetricV2,
    window: Option<APITimeseriesWindow>,
    #[serde(default)]
    filters: Option<APITrainingMetricFilters>,
    #[serde(default)]
    summary: APITrainingMetricSummary,
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
    values: HashMap<String, GranuleValues>,
    summary: HashMap<String, f64>,
    unit: String,
}

impl From<GroupedMetricValues> for ResponseBody {
    fn from(value: GroupedMetricValues) -> Self {
        let unit = value.unit();
        let (values, summary) = value.values_and_summary();
        ResponseBody {
            unit: unit.to_string(),
            values: values,
            summary: summary,
        }
    }
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

    let filters = request
        .filters
        .map(TrainingMetricFilters::try_from)
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .unwrap_or_else(TrainingMetricFilters::empty);

    let window: Option<TrainingMetricWindow> = request.window.map(|w| w.into());
    let range = MetricsDateRange {
        start: request.start,
        end: request.end,
    };

    let req = GetTrainingMetricValuesRequest::ByDefinition {
        user: user.user().clone(),
        metric: request.metric,
        window: window.clone(),
        filters,
        summary: request.summary.into(),
    };

    let values = state
        .training_metrics_service
        .get_training_metric_values(req, &date_range)
        .await
        .map_err(StatusCode::from)?;

    let values = convert_metric_values_unit(group_metric_values(values));
    let values = match window.as_ref() {
        Some(window) => fill_missing_granules(values, window, &range),
        None => values,
    };

    Ok(Json(ResponseBody::from(values)))
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
            "metric": "Calories",
            "window": {
                "granularity": "Daily",
                "aggregate": "Sum"
            },
            "start": "2024-01-01T00:00:00+00:00"
        }"#;
        let result: Result<ComputeMetricValuesRequest, _> = serde_json::from_str(json);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert!(request.filters.is_none());
        assert_eq!(
            request.window.unwrap(),
            APITimeseriesWindow::new(
                APITrainingMetricGranularity::Daily,
                APITrainingMetricAggregate::Sum,
                None
            )
        );
        assert!(request.end.is_none());
    }

    #[test]
    fn test_request_deserialize_all_fields() {
        // Test with all fields provided
        // Demonstrates complete JSON format including:
        // - Metric: ActivityMetricV2::AvgSpeed
        // - Optional end date
        // - Optional group_by (values: Sport, SportCategory, WorkoutType, RpeRange, Bonked)
        // - Optional filters with sports (Sport or SportCategory)
        let json = r#"{
            "metric": "AvgSpeed",
            "window": {
                "granularity": "Daily",
                "aggregate": "Sum",
                "group_by": "Sport"
            },
            "start": "2024-01-01T00:00:00+00:00",
            "end": "2024-12-31T23:59:59+00:00",
            "filters": {"sports": [{"Sport": "Running"}]}
        }"#;
        let result: Result<ComputeMetricValuesRequest, _> = serde_json::from_str(json);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert!(request.filters.is_some());
        assert_eq!(
            request.window.unwrap(),
            APITimeseriesWindow::new(
                APITrainingMetricGranularity::Daily,
                APITrainingMetricAggregate::Sum,
                Some(APITrainingMetricGroupBy::Sport)
            )
        );
        assert!(request.end.is_some());
    }
}
