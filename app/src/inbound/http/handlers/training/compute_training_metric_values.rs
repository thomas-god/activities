use std::collections::HashMap;

use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::{
            activity::{ActivityMetricSource, ActivityMetricV2, Unit},
            training::{
                TrainingMetricAggregate, TrainingMetricDefinition, TrainingMetricFilters,
                TrainingMetricGranularity, TrainingMetricSummary, TrainingMetricSummaryAverage,
                TrainingMetricTarget, TrainingMetricWindow,
            },
        },
        ports::{
            DateRange,
            activity::IActivityService,
            preferences::IPreferencesService,
            training::{
                ComputeTrainingMetricValuesError, GetTrainingMetricValuesError,
                GetTrainingMetricValuesRequest, ITrainingService,
            },
        },
    },
    inbound::{
        auth::AuthenticatedUser,
        http::{
            AppState,
            handlers::training::{
                types::{
                    APITimeseriesWindow, APITrainingMetricAggregate, APITrainingMetricFilters,
                    APITrainingMetricGranularity, APITrainingMetricGroupBy, APITrainingMetricScope,
                    APITrainingMetricSource, APITrainingMetricSummary, APITrainingMetricTarget,
                    SportsResponse, TrainingMetricBody, format_source_metric,
                },
                utils::{
                    GranuleValues, MetricsDateRange, convert_metric_target_unit,
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
    #[serde(default)]
    target: Option<APITrainingMetricTarget>,
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

impl From<ComputeTrainingMetricValuesError> for StatusCode {
    fn from(value: ComputeTrainingMetricValuesError) -> Self {
        match value {
            ComputeTrainingMetricValuesError::Unknown(_) => Self::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(skip_all, err)]
pub async fn compute_training_metric_values<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    PS: IPreferencesService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, PS>>,
    Json(request): Json<ComputeMetricValuesRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let date_range = DateRange::from(&request);

    let filters = request
        .filters
        .as_ref()
        .map(TrainingMetricFilters::try_from)
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .unwrap_or_else(TrainingMetricFilters::empty);

    let target = request
        .target
        .as_ref()
        .map(TrainingMetricTarget::try_from)
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let window: Option<TrainingMetricWindow> = request.window.as_ref().map(|w| w.into());
    let range = MetricsDateRange {
        start: request.start,
        end: request.end,
    };

    let req = GetTrainingMetricValuesRequest::ByDefinition {
        user: user.user().clone(),
        metric: request.metric,
        window: window.clone(),
        filters,
        summary: TrainingMetricSummary::from(&request.summary),
        target,
    };

    let values = match state
        .training_metrics_service
        .get_training_metric_values(req, &date_range)
        .await
    {
        Ok(values) => values,
        Err(err) => {
            if matches!(&err, GetTrainingMetricValuesError::Unknown(_)) {
                tracing::error!(
                    "Error computing training metric values: {}",
                    err.to_string()
                );
            }
            return Err(StatusCode::from(err));
        }
    };

    let values = convert_metric_values_unit(group_metric_values(values));
    let values = match window.as_ref() {
        Some(window) => fill_missing_granules(values, window, &range),
        None => values,
    };
    let unit = values.unit();
    let (values, summary) = values.values_and_summary();
    // Convert the target to the values' display unit so it can be drawn on the chart
    let target = target
        .as_ref()
        .and_then(|t| convert_metric_target_unit(t, unit));

    Ok(Json(to_body(&request, values, unit, summary, target)))
}

fn to_body(
    request: &ComputeMetricValuesRequest,
    values: HashMap<String, HashMap<String, f64>>,
    unit: Unit,
    summary: HashMap<String, f64>,
    target: Option<TrainingMetricTarget>,
) -> TrainingMetricBody {
    TrainingMetricBody {
        // ID not relevant for temporary metric values
        id: "temporary-metric".to_string(),
        name: None,
        metric: request.metric.to_string(),
        metric_formated: format_source_metric(&request.metric.source()),
        unit: unit.to_string(),
        granularity: request.window.as_ref().map(|w| w.granularity().to_string()),
        aggregate: request.window.as_ref().map(|w| w.aggregate().to_string()),
        sports: request
            .filters
            .as_ref()
            .map(|f| SportsResponse::from(&f.sports))
            .unwrap_or_default(),
        workout_types: request
            .filters
            .as_ref()
            .map(|f| {
                f.workout_types
                    .as_ref()
                    .map(|wt| wt.iter().map(|t| t.to_string()).collect())
            })
            .flatten(),
        bonked: request
            .filters
            .as_ref()
            .map(|f| f.bonked.as_ref().map(|b| b.to_string()))
            .flatten(),
        rpes: request
            .filters
            .as_ref()
            .map(|f| f.rpes.as_ref().map(|rs| rs.iter().map(|r| *r).collect()))
            .flatten(),
        show_average: request
            .summary
            .average
            .as_ref()
            .map(|avg| TrainingMetricSummaryAverage::from(avg)),
        target,
        values,
        group_by: request
            .window
            .as_ref()
            .map(|w| w.group_by().as_ref().map(|g| g.to_string()))
            .flatten(),
        // Default to Global as not relevant for temporary metric
        scope: APITrainingMetricScope::Global,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    use crate::domain::models::activity::{BonkStatus, Sport, SportCategory, Unit, WorkoutType};
    use crate::domain::models::training::{
        SportFilter, TrainingMetricSummaryAverage, TrainingMetricTarget,
    };
    use crate::inbound::http::handlers::training::types::APITrainingMetricSummaryAverage;

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

    #[test]
    fn test_request_deserialize_with_target() {
        let json = r#"{
            "metric": "Calories",
            "target": {"value": 100.0, "unit": "km"},
            "start": "2024-01-01T00:00:00+00:00"
        }"#;
        let result: Result<ComputeMetricValuesRequest, _> = serde_json::from_str(json);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(
            request.target,
            Some(APITrainingMetricTarget::new(100.0, "km".to_string()))
        );
    }

    #[test]
    fn test_to_body_minimal_request_uses_defaults() {
        let request = ComputeMetricValuesRequest {
            metric: ActivityMetricV2::Calories,
            window: None,
            filters: None,
            summary: APITrainingMetricSummary::default(),
            target: None,
            start: DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00").unwrap(),
            end: None,
        };

        let values = HashMap::from([(
            "Other".to_string(),
            HashMap::from([("2024-01-01".to_string(), 100.0)]),
        )]);
        let summary = HashMap::new();

        let body = to_body(
            &request,
            values.clone(),
            Unit::KiloCalorie,
            summary.clone(),
            None,
        );

        assert_eq!(body.id, "temporary-metric");
        assert_eq!(body.name, None);
        assert_eq!(body.metric, "Calories");
        assert_eq!(body.metric_formated, "Calories");
        assert_eq!(body.unit, "kcal");
        assert_eq!(body.granularity, None);
        assert_eq!(body.aggregate, None);
        assert!(body.sports.sports.is_empty());
        assert!(body.sports.categories.is_empty());
        assert_eq!(body.workout_types, None);
        assert_eq!(body.bonked, None);
        assert_eq!(body.rpes, None);
        assert_eq!(body.show_average, None);
        assert_eq!(body.target, None);
        assert_eq!(body.values, values);
        assert_eq!(body.group_by, None);
        assert_eq!(body.scope, APITrainingMetricScope::Global);
        assert_eq!(body.summary, summary);
    }

    #[test]
    fn test_to_body_full_request_maps_all_fields() {
        let request = ComputeMetricValuesRequest {
            metric: ActivityMetricV2::Calories,
            window: Some(APITimeseriesWindow::new(
                APITrainingMetricGranularity::Weekly,
                APITrainingMetricAggregate::Sum,
                Some(APITrainingMetricGroupBy::Sport),
            )),
            filters: Some(APITrainingMetricFilters {
                sports: Some(vec![SportFilter::Sport(Sport::Running)]),
                workout_types: Some(vec![WorkoutType::Easy, WorkoutType::Intervals]),
                bonked: Some(BonkStatus::Bonked),
                rpes: Some(vec![6, 7]),
            }),
            summary: APITrainingMetricSummary::new(Some(APITrainingMetricSummaryAverage::new(
                true,
            ))),
            target: Some(APITrainingMetricTarget::new(100.0, "km".to_string())),
            start: DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00").unwrap(),
            end: Some(DateTime::parse_from_rfc3339("2024-12-31T23:59:59+00:00").unwrap()),
        };

        let values = HashMap::from([
            (
                "Running".to_string(),
                HashMap::from([
                    ("2024-W01".to_string(), 10.0),
                    ("2024-W02".to_string(), 20.0),
                ]),
            ),
            (
                "Cycling".to_string(),
                HashMap::from([("2024-W01".to_string(), 30.0)]),
            ),
        ]);
        let summary = HashMap::from([("average".to_string(), 15.0)]);
        let target = Some(TrainingMetricTarget::new(100.0, Unit::Kilometer));

        let body = to_body(
            &request,
            values.clone(),
            Unit::KiloCalorie,
            summary.clone(),
            target.clone(),
        );

        assert_eq!(body.id, "temporary-metric");
        assert_eq!(body.name, None);
        assert_eq!(body.metric, "Calories");
        assert_eq!(body.metric_formated, "Calories");
        assert_eq!(body.unit, "kcal");
        assert_eq!(body.granularity, Some("Weekly".to_string()));
        assert_eq!(body.aggregate, Some("Sum".to_string()));
        assert_eq!(body.sports.sports, vec!["Running".to_string()]);
        assert!(body.sports.categories.is_empty());
        assert_eq!(
            body.workout_types,
            Some(vec!["easy".to_string(), "intervals".to_string()])
        );
        assert_eq!(body.bonked, Some("bonked".to_string()));
        assert_eq!(body.rpes, Some(vec![6, 7]));
        assert_eq!(
            body.show_average,
            Some(TrainingMetricSummaryAverage::new(true))
        );
        assert_eq!(body.target, target);
        assert_eq!(body.values, values);
        assert_eq!(body.group_by, Some("Sport".to_string()));
        assert_eq!(body.scope, APITrainingMetricScope::Global);
        assert_eq!(body.summary, summary);
    }

    #[test]
    fn test_to_body_timeseries_metric_and_sport_category_filter() {
        let request = ComputeMetricValuesRequest {
            metric: ActivityMetricV2::AvgSpeed,
            window: Some(APITimeseriesWindow::new(
                APITrainingMetricGranularity::Daily,
                APITrainingMetricAggregate::Average,
                None,
            )),
            filters: Some(APITrainingMetricFilters {
                sports: Some(vec![SportFilter::SportCategory(SportCategory::Running)]),
                workout_types: None,
                bonked: None,
                rpes: None,
            }),
            summary: APITrainingMetricSummary::default(),
            target: None,
            start: DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00").unwrap(),
            end: None,
        };

        let body = to_body(
            &request,
            HashMap::new(),
            Unit::MeterPerSecond,
            HashMap::new(),
            None,
        );

        assert_eq!(body.metric, "AvgSpeed");
        assert_eq!(body.metric_formated, "Activity Average Speed");
        assert_eq!(body.unit, "m/s");
        assert_eq!(body.granularity, Some("Daily".to_string()));
        assert_eq!(body.aggregate, Some("Average".to_string()));
        assert_eq!(body.group_by, None);
        assert_eq!(body.sports.categories, vec!["Running".to_string()]);
        assert!(body.sports.sports.is_empty());
        assert_eq!(body.workout_types, None);
        assert_eq!(body.bonked, None);
        assert_eq!(body.rpes, None);
        assert_eq!(body.show_average, None);
        assert_eq!(body.target, None);
    }

    #[test]
    fn test_to_body_window_without_filters_keeps_sports_default() {
        let request = ComputeMetricValuesRequest {
            metric: ActivityMetricV2::Distance,
            window: Some(APITimeseriesWindow::new(
                APITrainingMetricGranularity::Monthly,
                APITrainingMetricAggregate::Sum,
                Some(APITrainingMetricGroupBy::WorkoutType),
            )),
            filters: None,
            summary: APITrainingMetricSummary::default(),
            target: None,
            start: DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00").unwrap(),
            end: None,
        };

        let body = to_body(&request, HashMap::new(), Unit::Meter, HashMap::new(), None);

        assert_eq!(body.metric, "Distance");
        assert_eq!(body.metric_formated, "Distance");
        assert_eq!(body.unit, "m");
        assert_eq!(body.granularity, Some("Monthly".to_string()));
        assert_eq!(body.aggregate, Some("Sum".to_string()));
        assert_eq!(body.group_by, Some("WorkoutType".to_string()));
        // No filters provided -> sports response falls back to default (empty)
        assert!(body.sports.sports.is_empty());
        assert!(body.sports.categories.is_empty());
        assert_eq!(body.workout_types, None);
        assert_eq!(body.bonked, None);
        assert_eq!(body.rpes, None);
        assert_eq!(body.show_average, None);
        assert_eq!(body.target, None);
        assert_eq!(body.scope, APITrainingMetricScope::Global);
    }

    #[test]
    fn test_to_body_empty_filter_lists_are_preserved() {
        let request = ComputeMetricValuesRequest {
            metric: ActivityMetricV2::Calories,
            window: None,
            filters: Some(APITrainingMetricFilters {
                sports: Some(vec![]),
                workout_types: Some(vec![]),
                bonked: None,
                rpes: Some(vec![]),
            }),
            summary: APITrainingMetricSummary::default(),
            target: None,
            start: DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00").unwrap(),
            end: None,
        };

        let body = to_body(
            &request,
            HashMap::new(),
            Unit::KiloCalorie,
            HashMap::new(),
            None,
        );

        assert!(body.sports.sports.is_empty());
        assert!(body.sports.categories.is_empty());
        assert_eq!(body.workout_types, Some(vec![]));
        assert_eq!(body.bonked, None);
        assert_eq!(body.rpes, Some(vec![]));
        assert_eq!(body.granularity, None);
        assert_eq!(body.aggregate, None);
        assert_eq!(body.group_by, None);
    }
}
