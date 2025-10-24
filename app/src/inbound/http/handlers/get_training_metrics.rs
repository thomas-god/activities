use std::collections::{HashMap, HashSet};

use axum::{
    Extension,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, FixedOffset, Local};
use derive_more::Constructor;
use serde::{Deserialize, Serialize, de};
use serde_json::json;

use crate::{
    domain::{
        models::{
            activity::{ActivityStatistic, TimeseriesMetric, ToUnit, Unit},
            training::{
                ActivityMetricSource, TrainingMetricBin, TrainingMetricDefinition,
                TrainingMetricGranularity, TrainingMetricValues,
            },
        },
        ports::{DateRange, IActivityService, ITrainingService},
    },
    inbound::{
        http::{
            AppState,
            auth::{AuthenticatedUser, IUserService},
        },
        parser::ParseFile,
    },
};

/// Constant for representing the "no group" case (when TrainingMetricBin has group = None)
const NO_GROUP: &str = "no_group";

/// Type alias for metric values grouped by granule (e.g., date or week)
/// Maps granule string (like "2025-09-24" or "2025-W39") to the metric value
///
/// Example:
/// ```ignore
/// {
///   "2025-09-24": 100.0,
///   "2025-09-25": 150.0,
///   "2025-09-26": 0.0
/// }
/// ```
type GranuleValues = HashMap<String, f64>;

/// Type alias for metric values organized by group
/// Maps group name (sport, activity type, etc.) to its granule values
/// Groups with None are represented by the NO_GROUP constant
///
/// Example:
/// ```ignore
/// {
///   "Cycling": {
///     "2025-09-24": 100.0,
///     "2025-09-25": 150.0
///   },
///   "Running": {
///     "2025-09-24": 50.0,
///     "2025-09-25": 75.0
///   },
///   "no_group": {  // NO_GROUP constant for activities with no group
///     "2025-09-24": 200.0,
///     "2025-09-25": 0.0
///   }
/// }
/// ```
type GroupedMetricValues = HashMap<String, GranuleValues>;

#[derive(Debug, Deserialize)]
pub struct MetricsDateRange {
    start: DateTime<FixedOffset>,
    end: Option<DateTime<FixedOffset>>,
}

impl From<&MetricsDateRange> for DateRange {
    fn from(value: &MetricsDateRange) -> Self {
        let start_date = value.start.date_naive();
        let end_date = value
            .end
            .map(|e| e.date_naive())
            .unwrap_or_else(|| Local::now().date_naive());
        Self::new(start_date, end_date)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody(Vec<ResponseBodyItem>);

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBodyItem {
    id: String,
    metric: String,
    unit: String,
    granularity: String,
    aggregate: String,
    sports: Vec<String>,
    values: GroupedMetricValues,
}

fn to_response_body_item(
    metric: (TrainingMetricDefinition, TrainingMetricValues),
    range: &MetricsDateRange,
) -> ResponseBodyItem {
    let (def, metric_values) = metric;
    let values = fill_metric_values(def.granularity(), metric_values, range);
    let (unit, values) = convert_metric_values(values, def.source());

    ResponseBodyItem {
        id: def.id().to_string(),
        metric: format_source(def.source()),
        unit: unit.to_string(),
        granularity: def.granularity().to_string(),
        aggregate: def.aggregate().to_string(),
        sports: def
            .filters()
            .sports()
            .as_ref()
            .map(|sports| sports.iter().map(|sport| sport.to_string()).collect())
            .unwrap_or_default(),
        values,
    }
}

fn fill_metric_values(
    granularity: &TrainingMetricGranularity,
    values: TrainingMetricValues,
    range: &MetricsDateRange,
) -> GroupedMetricValues {
    // 1. Get all expected granules (bins) from the date range
    let expected_granules: Vec<String> = granularity
        .bins_keys(
            &range.start,
            &range.end.unwrap_or(Local::now().fixed_offset()),
        )
        .iter()
        .map(|granule| granule.to_string())
        .collect();

    // 2. Extract unique groups from the values
    let groups: HashSet<String> = values
        .iter()
        .map(|(bin, _)| bin.group().clone().unwrap_or_else(|| NO_GROUP.to_string()))
        .collect();

    // 3. For each group, create a map of granule -> value
    let mut result: GroupedMetricValues = HashMap::new();

    for group in groups {
        let mut granule_values: GranuleValues = HashMap::new();

        for granule in &expected_granules {
            // Try to find a value for this granule and group combination
            let bin = if group == NO_GROUP {
                TrainingMetricBin::new(granule.clone(), None)
            } else {
                TrainingMetricBin::new(granule.clone(), Some(group.clone()))
            };

            let value = values
                .get(&bin)
                .map(|metric_value| *metric_value.value())
                .unwrap_or(0.0);

            granule_values.insert(granule.clone(), value);
        }

        result.insert(group, granule_values);
    }

    result
}

fn convert_metric_values(
    values: GroupedMetricValues,
    source: &ActivityMetricSource,
) -> (Unit, GroupedMetricValues) {
    match source {
        ActivityMetricSource::Statistic(stat) => match stat {
            ActivityStatistic::Distance => (
                Unit::Kilometer,
                values
                    .iter()
                    .map(|(group, group_values)| {
                        (
                            group.clone(),
                            group_values
                                .iter()
                                .map(|(k, val)| (k.clone(), *val / 1000.))
                                .collect(),
                        )
                    })
                    .collect(),
            ),
            _ => (stat.unit(), values),
        },
        ActivityMetricSource::Timeseries((metric, _)) => match metric {
            TimeseriesMetric::Distance => (
                Unit::Kilometer,
                values
                    .iter()
                    .map(|(group, group_values)| {
                        (
                            group.clone(),
                            group_values
                                .iter()
                                .map(|(k, val)| (k.clone(), *val / 1000.))
                                .collect(),
                        )
                    })
                    .collect(),
            ),
            TimeseriesMetric::Speed => (
                Unit::KilometerPerHour,
                values
                    .iter()
                    .map(|(group, group_values)| {
                        (
                            group.clone(),
                            group_values
                                .iter()
                                .map(|(k, val)| (k.clone(), *val * 3.6))
                                .collect(),
                        )
                    })
                    .collect(),
            ),
            _ => (metric.unit(), values),
        },
    }
}

fn format_source(source: &ActivityMetricSource) -> String {
    match source {
        ActivityMetricSource::Statistic(stat) => stat.to_string(),
        ActivityMetricSource::Timeseries((metric, aggregate)) => {
            format!("{aggregate:?} {metric:?}")
        }
    }
}

pub async fn get_training_metrics<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
>(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<AppState<AS, PF, TMS, UR>>,
    Query(date_range): Query<MetricsDateRange>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = state
        .training_metrics_service
        .get_training_metrics(user.user(), &Some(DateRange::from(&date_range)))
        .await;

    let body = ResponseBody(
        res.into_iter()
            .map(|metric| to_response_body_item(metric, &date_range))
            .collect(),
    );

    Ok(json!(body).to_string())
}

#[cfg(test)]
mod tests {
    use crate::domain::models::{
        activity::{ActivityStatistic, TimeseriesMetric},
        training::{TimeseriesAggregate, TrainingMetricAggregate, TrainingMetricValue},
    };

    use super::*;

    #[test]
    fn test_format_definition_source() {
        assert_eq!(
            format_source(&ActivityMetricSource::Statistic(
                ActivityStatistic::Calories
            )),
            "Calories".to_string()
        );
        assert_eq!(
            format_source(&ActivityMetricSource::Timeseries((
                TimeseriesMetric::Distance,
                TimeseriesAggregate::Max
            ))),
            "Max Distance".to_string()
        );
    }

    #[test]
    fn test_fill_metric_values_without_groups() {
        // Test with values that have no group (group = None)
        let range = MetricsDateRange {
            start: DateTime::parse_from_rfc3339("2025-09-24T00:00:00Z").unwrap(),
            end: Some(DateTime::parse_from_rfc3339("2025-09-26T00:00:00Z").unwrap()),
        };

        let values = TrainingMetricValues::new(HashMap::from([
            (
                TrainingMetricBin::new("2025-09-24".to_string(), None),
                TrainingMetricValue::Max(10.0),
            ),
            (
                TrainingMetricBin::new("2025-09-25".to_string(), None),
                TrainingMetricValue::Max(15.0),
            ),
        ]));

        let result = fill_metric_values(&TrainingMetricGranularity::Daily, values, &range);

        // Should have one group (the "no group" case, represented by NO_GROUP constant)
        assert_eq!(result.len(), 1);

        // The group key for "no group" should be NO_GROUP constant
        let group_values = result
            .get(NO_GROUP)
            .expect("Should have NO_GROUP key for no group");

        // Should have 3 days: 2025-09-24, 2025-09-25, 2025-09-26
        assert_eq!(group_values.len(), 3);
        assert_eq!(group_values.get("2025-09-24"), Some(&10.0));
        assert_eq!(group_values.get("2025-09-25"), Some(&15.0));
        assert_eq!(group_values.get("2025-09-26"), Some(&0.0)); // Missing value, filled to be 0
    }

    #[test]
    fn test_fill_metric_values_with_single_group() {
        // Test with values that all belong to the same group
        let range = MetricsDateRange {
            start: DateTime::parse_from_rfc3339("2025-09-24T00:00:00Z").unwrap(),
            end: Some(DateTime::parse_from_rfc3339("2025-09-26T00:00:00Z").unwrap()),
        };

        let values = TrainingMetricValues::new(HashMap::from([
            (
                TrainingMetricBin::new("2025-09-24".to_string(), Some("Cycling".to_string())),
                TrainingMetricValue::Max(10.0),
            ),
            (
                TrainingMetricBin::new("2025-09-25".to_string(), Some("Cycling".to_string())),
                TrainingMetricValue::Max(15.0),
            ),
        ]));

        let result = fill_metric_values(&TrainingMetricGranularity::Daily, values, &range);

        // Should have one group: "Cycling"
        assert_eq!(result.len(), 1);

        let cycling_values = result.get("Cycling").expect("Should have Cycling group");

        // Should have 3 days
        assert_eq!(cycling_values.len(), 3);
        assert_eq!(cycling_values.get("2025-09-24"), Some(&10.0));
        assert_eq!(cycling_values.get("2025-09-25"), Some(&15.0));
        assert_eq!(cycling_values.get("2025-09-26"), Some(&0.0)); // Missing value, filled to be 0
    }

    #[test]
    fn test_fill_metric_values_with_multiple_groups() {
        // Test with values from multiple groups
        let range = MetricsDateRange {
            start: DateTime::parse_from_rfc3339("2025-09-24T00:00:00Z").unwrap(),
            end: Some(DateTime::parse_from_rfc3339("2025-09-26T00:00:00Z").unwrap()),
        };

        let values = TrainingMetricValues::new(HashMap::from([
            // Cycling values
            (
                TrainingMetricBin::new("2025-09-24".to_string(), Some("Cycling".to_string())),
                TrainingMetricValue::Max(10.0),
            ),
            (
                TrainingMetricBin::new("2025-09-25".to_string(), Some("Cycling".to_string())),
                TrainingMetricValue::Max(15.0),
            ),
            // Running values
            (
                TrainingMetricBin::new("2025-09-24".to_string(), Some("Running".to_string())),
                TrainingMetricValue::Max(5.0),
            ),
            (
                TrainingMetricBin::new("2025-09-26".to_string(), Some("Running".to_string())),
                TrainingMetricValue::Max(8.0),
            ),
        ]));

        let result = fill_metric_values(&TrainingMetricGranularity::Daily, values, &range);

        // Should have two groups: "Cycling" and "Running"
        assert_eq!(result.len(), 2);

        // Check Cycling group
        let cycling_values = result.get("Cycling").expect("Should have Cycling group");
        assert_eq!(cycling_values.len(), 3);
        assert_eq!(cycling_values.get("2025-09-24"), Some(&10.0));
        assert_eq!(cycling_values.get("2025-09-25"), Some(&15.0));
        assert_eq!(cycling_values.get("2025-09-26"), Some(&0.0)); // Missing for cycling

        // Check Running group
        let running_values = result.get("Running").expect("Should have Running group");
        assert_eq!(running_values.len(), 3);
        assert_eq!(running_values.get("2025-09-24"), Some(&5.0));
        assert_eq!(running_values.get("2025-09-25"), Some(&0.0)); // Missing for running
        assert_eq!(running_values.get("2025-09-26"), Some(&8.0));
    }

    #[test]
    fn test_fill_metric_values_with_mixed_groups() {
        // Test with some values having groups and some without
        let range = MetricsDateRange {
            start: DateTime::parse_from_rfc3339("2025-09-24T00:00:00Z").unwrap(),
            end: Some(DateTime::parse_from_rfc3339("2025-09-25T00:00:00Z").unwrap()),
        };

        let values = TrainingMetricValues::new(HashMap::from([
            // No group
            (
                TrainingMetricBin::new("2025-09-24".to_string(), None),
                TrainingMetricValue::Max(20.0),
            ),
            // Cycling group
            (
                TrainingMetricBin::new("2025-09-24".to_string(), Some("Cycling".to_string())),
                TrainingMetricValue::Max(10.0),
            ),
            (
                TrainingMetricBin::new("2025-09-25".to_string(), Some("Cycling".to_string())),
                TrainingMetricValue::Max(15.0),
            ),
        ]));

        let result = fill_metric_values(&TrainingMetricGranularity::Daily, values, &range);

        // Should have two groups: NO_GROUP for None and "Cycling"
        assert_eq!(result.len(), 2);

        // Check no-group (NO_GROUP constant)
        let no_group_values = result
            .get(NO_GROUP)
            .expect("Should have NO_GROUP key for no group");
        assert_eq!(no_group_values.len(), 2);
        assert_eq!(no_group_values.get("2025-09-24"), Some(&20.0));
        assert_eq!(no_group_values.get("2025-09-25"), Some(&0.0)); // Missing value

        // Check Cycling group
        let cycling_values = result.get("Cycling").expect("Should have Cycling group");
        assert_eq!(cycling_values.len(), 2);
        assert_eq!(cycling_values.get("2025-09-24"), Some(&10.0));
        assert_eq!(cycling_values.get("2025-09-25"), Some(&15.0));
    }

    #[test]
    fn test_fill_metric_values_empty_values() {
        // Test with no values - should return empty result
        let range = MetricsDateRange {
            start: DateTime::parse_from_rfc3339("2025-09-24T00:00:00Z").unwrap(),
            end: Some(DateTime::parse_from_rfc3339("2025-09-25T00:00:00Z").unwrap()),
        };

        let values = TrainingMetricValues::new(HashMap::new());

        let result = fill_metric_values(&TrainingMetricGranularity::Daily, values, &range);

        // With no values, there are no groups, so result should be empty
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_fill_metric_values_weekly_granularity() {
        // Test with weekly granularity to ensure granularity handling works
        let range = MetricsDateRange {
            start: DateTime::parse_from_rfc3339("2025-09-22T00:00:00Z").unwrap(), // Week starting Sep 22
            end: Some(DateTime::parse_from_rfc3339("2025-10-05T00:00:00Z").unwrap()), // Week starting Oct 5
        };

        // Weekly granularity uses the Monday date as the key, not "2025-W39" format
        let values = TrainingMetricValues::new(HashMap::from([(
            TrainingMetricBin::new("2025-09-22".to_string(), Some("Cycling".to_string())),
            TrainingMetricValue::Max(100.0),
        )]));

        let result = fill_metric_values(&TrainingMetricGranularity::Weekly, values, &range);

        // Should have one group: "Cycling"
        assert_eq!(result.len(), 1);

        let cycling_values = result.get("Cycling").expect("Should have Cycling group");

        // Should have at least 2 weeks
        assert!(cycling_values.len() >= 2);
        assert_eq!(cycling_values.get("2025-09-22"), Some(&100.0));
        // Other weeks should have 0
        assert!(cycling_values.contains_key("2025-09-29"));
    }
}
