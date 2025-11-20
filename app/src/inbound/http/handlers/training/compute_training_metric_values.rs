use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::training::{
            ActivityMetricSource, TrainingMetricAggregate, TrainingMetricDefinition,
            TrainingMetricFilters, TrainingMetricGranularity,
        },
        ports::{
            ComputeTrainingMetricValuesError, DateRange, IActivityService, IPreferencesService,
            ITrainingService,
        },
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
            handlers::training::{
                types::{
                    APITrainingMetricAggregate, APITrainingMetricFilters,
                    APITrainingMetricGranularity, APITrainingMetricGroupBy,
                    APITrainingMetricSource,
                },
                utils::{
                    GroupedMetricValues, MetricsDateRange, convert_metric_values,
                    fill_metric_values,
                },
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
    values: GroupedMetricValues,
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

    // Convert request types before they are moved
    let source: ActivityMetricSource = request.source.into();
    let granularity: TrainingMetricGranularity = request.granularity.into();
    let aggregate: TrainingMetricAggregate = request.aggregate.into();
    let range = MetricsDateRange {
        start: request.start,
        end: request.end,
    };

    let definition = TrainingMetricDefinition::new(
        user.user().clone(),
        source.clone(),
        granularity.clone(),
        aggregate.clone(),
        filters,
        group_by,
    );

    let values = state
        .training_metrics_service
        .compute_training_metric_values(&definition, &date_range)
        .await
        .map_err(StatusCode::from)?;

    let values = fill_metric_values(&granularity, values, &range);
    let (_, values) = convert_metric_values(values, &source, &aggregate);

    Ok(Json(ResponseBody { values }))
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
