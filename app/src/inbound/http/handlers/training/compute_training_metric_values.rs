use std::collections::HashMap;

use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::{
            activity::{ActivityMetric, ActivityMetricSource, Unit},
            training::{
                TrainingMetricActivityFilters, TrainingMetricAggregate, TrainingMetricDefinition,
                TrainingMetricGranularity, TrainingMetricSource, TrainingMetricSummary,
                TrainingMetricSummaryAverage, TrainingMetricTarget, TrainingMetricWindow,
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
                    APIActivityMetricSource, APITimeseriesWindow, APITrainingMetricAggregate,
                    APITrainingMetricFilters, APITrainingMetricGranularity, APITrainingMetricScope,
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
    source: APITrainingMetricSource,
    window: Option<APITimeseriesWindow>,
    #[serde(default)]
    summary: APITrainingMetricSummary,
    #[serde(default)]
    target: Option<APITrainingMetricTarget>,
    start: chrono::NaiveDate,
    end: Option<chrono::NaiveDate>,
}

impl From<&ComputeMetricValuesRequest> for DateRange {
    fn from(value: &ComputeMetricValuesRequest) -> Self {
        let end_date = value.end.unwrap_or_else(|| Local::now().date_naive());
        Self::new(value.start, end_date)
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

    let source =
        TrainingMetricSource::try_from(&request.source).map_err(|_| StatusCode::BAD_REQUEST)?;

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
        source,
        window: window.clone(),
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
        source: request.source.clone(),
        metric_formated: request.source.format_source_metric(),
        unit: unit.to_string(),
        granularity: request.window.as_ref().map(|w| w.granularity().to_string()),
        aggregate: request.window.as_ref().map(|w| w.aggregate().to_string()),
        show_average: request
            .summary
            .average
            .as_ref()
            .map(TrainingMetricSummaryAverage::from),
        target,
        values,
        // Default to Global as not relevant for temporary metric
        scope: APITrainingMetricScope::Global,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    use crate::domain::models::activity::{
        ActivityRpe, BonkStatus, Sport, SportCategory, Unit, WorkoutType,
    };
    use crate::domain::models::training::{
        ActivitySource, HooperIndexSource, SportFilter, TrainingMetricSummaryAverage,
        TrainingMetricTarget, WeightAndNutritionSource,
    };
    use crate::inbound::http::handlers::training::types::{
        APIActivitySource, APITrainingMetricActivityGroupBy, APITrainingMetricSummaryAverage,
    };

    fn extract_group_by(
        request: &ComputeMetricValuesRequest,
    ) -> Option<APITrainingMetricActivityGroupBy> {
        match &request.source {
            APITrainingMetricSource::Activity(source) => source.group_by.clone(),
            _ => None,
        }
    }

    fn extract_filters(request: &ComputeMetricValuesRequest) -> Option<APITrainingMetricFilters> {
        match &request.source {
            APITrainingMetricSource::Activity(source) => Some(source.filters.clone()),
            _ => None,
        }
    }

    #[test]
    fn test_request_deserialize_minimal() {
        // Test with only required fields
        // Demonstrates basic JSON format for the request
        let json = r#"{
            "source": {"type": "activity", "metric": {"metric": "Calories"}},
            "window": {
                "granularity": "Daily",
                "aggregate": "Sum"
            },
            "start": "2024-01-01"
        }"#;
        let result: Result<ComputeMetricValuesRequest, _> = serde_json::from_str(json);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert!(extract_group_by(&request).is_none());
        // Filters now live in the source and default to an empty set
        assert_eq!(
            extract_filters(&request),
            Some(APITrainingMetricFilters::default())
        );
        assert_eq!(
            request.window.unwrap(),
            APITimeseriesWindow::new(
                APITrainingMetricGranularity::Daily,
                APITrainingMetricAggregate::Sum,
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
        // - Optional filters with sports (Sport or SportCategory)
        let json = r#"{
            "source": {
                "type": "activity",
                "metric": {
                    "metric": "AvgSpeed",
                    "group_by": "Sport",
                    "filters": {"sports": [{"Sport": "Running"}]}
                }
            },
            "window": {
                "granularity": "Daily",
                "aggregate": "Sum"
            },
            "start": "2024-01-01",
            "end": "2024-12-31"
        }"#;
        let result: Result<ComputeMetricValuesRequest, _> = serde_json::from_str(json);

        assert!(result.is_ok());
        let request = result.unwrap();

        assert_eq!(
            extract_group_by(&request),
            Some(APITrainingMetricActivityGroupBy::Sport)
        );
        assert_eq!(
            extract_filters(&request),
            Some(APITrainingMetricFilters {
                sports: Some(vec![SportFilter::Sport(Sport::Running)]),
                workout_types: None,
                bonked: None,
                rpes: None
            })
        );
        assert_eq!(
            request.window.unwrap(),
            APITimeseriesWindow::new(
                APITrainingMetricGranularity::Daily,
                APITrainingMetricAggregate::Sum,
            )
        );
        assert!(request.end.is_some());
    }

    #[test]
    fn test_request_deserialize_with_target() {
        let json = r#"{
            "source": {"type": "activity", "metric": {"metric": "Calories"}},
            "target": {"value": 100.0, "unit": "km"},
            "start": "2024-01-01"
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
    fn test_request_deserialize_full_json_layout() {
        // Reference example of the request JSON layout with every field populated.
        // Optional fields absent from the payload fall back to defaults, see
        // test_request_deserialize_minimal.
        let json = r#"{
            "source": {
                "type": "activity",
                "metric": {
                    "metric": "AvgHeartRate",
                    "group_by": "SportCategory",
                    "filters": {
                        "sports": [{"Sport": "Running"}, {"SportCategory": "Cycling"}],
                        "workout_types": ["Easy", "Intervals"],
                        "bonked": "Bonked",
                        "rpes": [5, 6, 7]
                    }
                }
            },
            "window": {
                "granularity": "Weekly",
                "aggregate": "Average"
            },
            "summary": {"average": {"include_zeros": true}},
            "target": {"value": 150.5, "unit": "bpm"},
            "start": "2024-01-01",
            "end": "2024-12-31"
        }"#;
        let result: Result<ComputeMetricValuesRequest, _> = serde_json::from_str(json);

        assert!(result.is_ok());
        let request = result.unwrap();

        assert_eq!(
            request.source,
            APITrainingMetricSource::Activity(APIActivitySource::new(
                ActivityMetric::AvgHeartRate,
                Some(APITrainingMetricActivityGroupBy::SportCategory),
                APITrainingMetricFilters {
                    sports: Some(vec![
                        SportFilter::Sport(Sport::Running),
                        SportFilter::SportCategory(SportCategory::Cycling),
                    ]),
                    workout_types: Some(vec![WorkoutType::Easy, WorkoutType::Intervals]),
                    bonked: Some(BonkStatus::Bonked),
                    rpes: Some(vec![5, 6, 7]),
                },
            ))
        );
        assert_eq!(
            request.window,
            Some(APITimeseriesWindow::new(
                APITrainingMetricGranularity::Weekly,
                APITrainingMetricAggregate::Average,
            ))
        );
        assert_eq!(
            request.summary,
            APITrainingMetricSummary::new(Some(APITrainingMetricSummaryAverage::new(true)))
        );
        assert_eq!(
            request.target,
            Some(APITrainingMetricTarget::new(150.5, "bpm".to_string()))
        );
        assert_eq!(request.start, "2024-01-01".parse().unwrap());
        assert_eq!(request.end, Some("2024-12-31".parse().unwrap()));
    }

    #[test]
    fn test_request_deserialize_non_activity_sources_json_layout() {
        // Hooper index and weight & nutrition sources take a plain source name and
        // carry no group_by/filters.
        let json = r#"{
            "source": {"type": "hooperIndex", "metric": "Sleep"},
            "window": {
                "granularity": "Monthly",
                "aggregate": "Sum"
            },
            "start": "2024-01-01"
        }"#;
        let result: Result<ComputeMetricValuesRequest, _> = serde_json::from_str(json);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(
            request.source,
            APITrainingMetricSource::HooperIndex(HooperIndexSource::Sleep)
        );

        let json = r#"{
            "source": {"type": "weightAndNutrition", "metric": "Weight"},
            "start": "2024-01-01"
        }"#;
        let result: Result<ComputeMetricValuesRequest, _> = serde_json::from_str(json);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(
            request.source,
            APITrainingMetricSource::WeightAndNutrition(WeightAndNutritionSource::Weight)
        );
    }

    #[test]
    fn test_to_body_minimal_request_uses_defaults() {
        let request = ComputeMetricValuesRequest {
            source: APITrainingMetricSource::Activity(APIActivitySource::new(
                ActivityMetric::Calories,
                APITrainingMetricActivityGroupBy::none(),
                APITrainingMetricFilters::default(),
            )),
            window: None,
            summary: APITrainingMetricSummary::default(),
            target: None,
            start: "2024-01-01".parse::<chrono::NaiveDate>().unwrap(),
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
        assert_eq!(body.metric_formated, "Calories");
        assert_eq!(body.unit, "kcal");
        assert_eq!(body.granularity, None);
        assert_eq!(body.aggregate, None);
        assert_eq!(body.show_average, None);
        assert_eq!(body.target, None);
        assert_eq!(body.values, values);
        assert_eq!(body.scope, APITrainingMetricScope::Global);
        assert_eq!(body.summary, summary);
    }

    #[test]
    fn test_to_body_full_request_maps_all_fields() {
        let request = ComputeMetricValuesRequest {
            source: APITrainingMetricSource::Activity(APIActivitySource::new(
                ActivityMetric::Calories,
                Some(APITrainingMetricActivityGroupBy::Sport),
                APITrainingMetricFilters {
                    sports: Some(vec![SportFilter::Sport(Sport::Running)]),
                    workout_types: Some(vec![WorkoutType::Easy, WorkoutType::Intervals]),
                    bonked: Some(BonkStatus::Bonked),
                    rpes: Some(vec![6, 7]),
                },
            )),
            window: Some(APITimeseriesWindow::new(
                APITrainingMetricGranularity::Weekly,
                APITrainingMetricAggregate::Sum,
            )),
            summary: APITrainingMetricSummary::new(Some(APITrainingMetricSummaryAverage::new(
                true,
            ))),
            target: Some(APITrainingMetricTarget::new(100.0, "km".to_string())),
            start: "2024-01-01".parse::<chrono::NaiveDate>().unwrap(),
            end: Some("2024-12-31".parse::<chrono::NaiveDate>().unwrap()),
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
        assert_eq!(
            body.source,
            APITrainingMetricSource::Activity(APIActivitySource::new(
                ActivityMetric::Calories,
                Some(APITrainingMetricActivityGroupBy::Sport),
                APITrainingMetricFilters {
                    sports: Some(vec![SportFilter::Sport(Sport::Running)]),
                    workout_types: Some(vec![WorkoutType::Easy, WorkoutType::Intervals]),
                    bonked: Some(BonkStatus::Bonked),
                    rpes: Some(vec![6, 7]),
                }
            ))
        );
        assert_eq!(body.metric_formated, "Calories");
        assert_eq!(body.unit, "kcal");
        assert_eq!(body.granularity, Some("Weekly".to_string()));
        assert_eq!(body.aggregate, Some("Sum".to_string()));
        assert_eq!(
            body.show_average,
            Some(TrainingMetricSummaryAverage::new(true))
        );
        assert_eq!(body.target, target);
        assert_eq!(body.values, values);
        assert_eq!(body.scope, APITrainingMetricScope::Global);
        assert_eq!(body.summary, summary);
    }

    #[test]
    fn test_to_body_timeseries_metric_and_sport_category_filter() {
        let request = ComputeMetricValuesRequest {
            source: APITrainingMetricSource::Activity(APIActivitySource::new(
                ActivityMetric::AvgSpeed,
                APITrainingMetricActivityGroupBy::none(),
                APITrainingMetricFilters {
                    sports: Some(vec![SportFilter::SportCategory(SportCategory::Running)]),
                    workout_types: None,
                    bonked: None,
                    rpes: None,
                },
            )),
            window: Some(APITimeseriesWindow::new(
                APITrainingMetricGranularity::Daily,
                APITrainingMetricAggregate::Average,
            )),
            summary: APITrainingMetricSummary::default(),
            target: None,
            start: "2024-01-01".parse::<chrono::NaiveDate>().unwrap(),
            end: None,
        };

        let body = to_body(
            &request,
            HashMap::new(),
            Unit::MeterPerSecond,
            HashMap::new(),
            None,
        );

        assert_eq!(
            body.source,
            APITrainingMetricSource::Activity(APIActivitySource::new(
                ActivityMetric::AvgSpeed,
                APITrainingMetricActivityGroupBy::none(),
                APITrainingMetricFilters {
                    sports: Some(vec![SportFilter::SportCategory(SportCategory::Running)]),
                    workout_types: None,
                    bonked: None,
                    rpes: None,
                },
            ))
        );
        assert_eq!(body.metric_formated, "Activity Average Speed");
        assert_eq!(body.unit, "m/s");
        assert_eq!(body.granularity, Some("Daily".to_string()));
        assert_eq!(body.aggregate, Some("Average".to_string()));
        assert_eq!(body.show_average, None);
        assert_eq!(body.target, None);
    }

    #[test]
    fn test_to_body_window_without_filters_keeps_sports_default() {
        let request = ComputeMetricValuesRequest {
            source: APITrainingMetricSource::Activity(APIActivitySource::new(
                ActivityMetric::Distance,
                Some(APITrainingMetricActivityGroupBy::WorkoutType),
                APITrainingMetricFilters::default(),
            )),
            window: Some(APITimeseriesWindow::new(
                APITrainingMetricGranularity::Monthly,
                APITrainingMetricAggregate::Sum,
            )),
            summary: APITrainingMetricSummary::default(),
            target: None,
            start: "2024-01-01".parse::<chrono::NaiveDate>().unwrap(),
            end: None,
        };

        let body = to_body(&request, HashMap::new(), Unit::Meter, HashMap::new(), None);

        assert_eq!(
            body.source,
            APITrainingMetricSource::Activity(APIActivitySource::new(
                ActivityMetric::Distance,
                Some(APITrainingMetricActivityGroupBy::WorkoutType),
                APITrainingMetricFilters::default(),
            ))
        );
        assert_eq!(body.metric_formated, "Distance");
        assert_eq!(body.unit, "m");
        assert_eq!(body.granularity, Some("Monthly".to_string()));
        assert_eq!(body.aggregate, Some("Sum".to_string()));
        assert_eq!(body.show_average, None);
        assert_eq!(body.target, None);
        assert_eq!(body.scope, APITrainingMetricScope::Global);
    }

    #[test]
    fn test_to_body_empty_filter_lists_are_preserved() {
        let request = ComputeMetricValuesRequest {
            source: APITrainingMetricSource::Activity(APIActivitySource::new(
                ActivityMetric::Calories,
                APITrainingMetricActivityGroupBy::none(),
                APITrainingMetricFilters {
                    sports: Some(vec![]),
                    workout_types: Some(vec![]),
                    bonked: None,
                    rpes: Some(vec![]),
                },
            )),
            window: None,
            summary: APITrainingMetricSummary::default(),
            target: None,
            start: "2024-01-01".parse::<chrono::NaiveDate>().unwrap(),
            end: None,
        };

        let body = to_body(
            &request,
            HashMap::new(),
            Unit::KiloCalorie,
            HashMap::new(),
            None,
        );

        assert_eq!(
            body.source,
            APITrainingMetricSource::Activity(APIActivitySource::new(
                ActivityMetric::Calories,
                APITrainingMetricActivityGroupBy::none(),
                APITrainingMetricFilters {
                    sports: Some(vec![]),
                    workout_types: Some(vec![]),
                    bonked: None,
                    rpes: Some(vec![]),
                }
            ))
        );
        assert_eq!(body.granularity, None);
        assert_eq!(body.aggregate, None);
    }
}
