use std::collections::{HashMap, HashSet};

use chrono::{DateTime, FixedOffset, Local};
use serde::Deserialize;

use crate::domain::{
    models::{
        activity::{ActivityStatistic, TimeseriesMetric, ToUnit, Unit},
        training::{
            ActivityMetricSource, TrainingMetricAggregate, TrainingMetricBin,
            TrainingMetricGranularity, TrainingMetricValues,
        },
    },
    ports::DateRange,
};

/// Constant for representing the "no group" case (when TrainingMetricBin has group = None)
pub const NO_GROUP: &str = "Other";

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
pub type GranuleValues = HashMap<String, f64>;

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
pub type GroupedMetricValues = HashMap<String, GranuleValues>;

#[derive(Debug, Deserialize)]
pub struct MetricsDateRange {
    pub start: DateTime<FixedOffset>,
    pub end: Option<DateTime<FixedOffset>>,
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

/// Fills metric values with zeros for all expected granules in the date range.
///
/// This function takes sparse metric values (which may only contain data for some dates/weeks/months)
/// and produces a complete dataset with zero values for any missing granules. This is useful for
/// creating consistent time series data for visualization.
///
/// # Arguments
///
/// * `granularity` - The time granularity (Daily, Weekly, or Monthly)
/// * `values` - The sparse training metric values to fill
/// * `range` - The date range to fill values for
///
/// # Returns
///
/// A HashMap organized by group, where each group contains a complete set of granule -> value mappings.
/// Missing values are filled with 0.0.
///
/// # Example
///
/// ```ignore
/// // Input: values for 2025-09-24 and 2025-09-25, range covers 2025-09-24 to 2025-09-26
/// // Output: values for all three days, with 2025-09-26 filled as 0.0
/// ```
pub fn fill_metric_values(
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
                .map(|metric_value| metric_value.value())
                .unwrap_or(0.0);

            granule_values.insert(granule.clone(), value);
        }

        result.insert(group, granule_values);
    }

    result
}

/// Converts metric values from their raw units to display units.
///
/// This function handles unit conversions for different metric types:
/// - Distance: meters → kilometers (÷ 1000)
/// - Speed: m/s → km/h (× 3.6)
/// - Pace: s/m → s/km (× 1000)
/// - Other metrics: no conversion
///
/// Special case: When aggregate is NumberOfActivities, always returns Activity unit
/// without any value conversion, regardless of the source metric type.
///
/// # Arguments
///
/// * `values` - The grouped metric values to convert
/// * `source` - The metric source (statistic or timeseries)
/// * `aggregate` - The aggregation type
///
/// # Returns
///
/// A tuple of (Unit, GroupedMetricValues) with converted values and the appropriate unit
pub fn convert_metric_values(
    values: GroupedMetricValues,
    source: &ActivityMetricSource,
    aggregate: &TrainingMetricAggregate,
) -> (Unit, GroupedMetricValues) {
    if matches!(aggregate, TrainingMetricAggregate::NumberOfActivities) {
        return (Unit::NumberOfActivities, values);
    }

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
            TimeseriesMetric::Pace => (
                Unit::SecondPerKilometer,
                values
                    .iter()
                    .map(|(group, group_values)| {
                        (
                            group.clone(),
                            group_values
                                .iter()
                                .map(|(k, val)| (k.clone(), *val * 1000.))
                                .collect(),
                        )
                    })
                    .collect(),
            ),
            _ => (metric.unit(), values),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::DateTime;

    use crate::domain::models::training::{
        TrainingMetricBin, TrainingMetricGranularity, TrainingMetricValue, TrainingMetricValues,
    };

    use super::*;

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

    #[test]
    fn test_convert_metric_values_number_of_activities_returns_activity_unit() {
        // When aggregate is NumberOfActivities, unit should always be Activity
        let values = HashMap::from([(
            "Other".to_string(),
            HashMap::from([
                ("2025-09-24".to_string(), 5.0),
                ("2025-09-25".to_string(), 3.0),
            ]),
        )]);

        use crate::domain::models::training::{TimeseriesAggregate, TrainingMetricAggregate};

        // Test with Distance source (which normally converts to Kilometer)
        let (unit, converted_values) = convert_metric_values(
            values.clone(),
            &ActivityMetricSource::Statistic(ActivityStatistic::Distance),
            &TrainingMetricAggregate::NumberOfActivities,
        );

        assert_eq!(unit, Unit::NumberOfActivities);
        // Values should not be converted (no division by 1000)
        assert_eq!(
            converted_values.get("Other").unwrap().get("2025-09-24"),
            Some(&5.0)
        );
        assert_eq!(
            converted_values.get("Other").unwrap().get("2025-09-25"),
            Some(&3.0)
        );

        // Test with Duration source
        let (unit, converted_values) = convert_metric_values(
            values,
            &ActivityMetricSource::Statistic(ActivityStatistic::Duration),
            &TrainingMetricAggregate::NumberOfActivities,
        );

        assert_eq!(unit, Unit::NumberOfActivities);
        assert_eq!(
            converted_values.get("Other").unwrap().get("2025-09-24"),
            Some(&5.0)
        );
    }

    #[test]
    fn test_convert_metric_values_distance_statistic_converts_to_kilometers() {
        use crate::domain::models::training::TrainingMetricAggregate;

        // Distance statistic should be converted from meters to kilometers
        let values = HashMap::from([(
            "Cycling".to_string(),
            HashMap::from([
                ("2025-09-24".to_string(), 10000.0), // 10km in meters
                ("2025-09-25".to_string(), 25000.0), // 25km in meters
            ]),
        )]);

        let (unit, converted_values) = convert_metric_values(
            values,
            &ActivityMetricSource::Statistic(ActivityStatistic::Distance),
            &TrainingMetricAggregate::Sum,
        );

        assert_eq!(unit, Unit::Kilometer);
        assert_eq!(
            converted_values.get("Cycling").unwrap().get("2025-09-24"),
            Some(&10.0)
        );
        assert_eq!(
            converted_values.get("Cycling").unwrap().get("2025-09-25"),
            Some(&25.0)
        );
    }

    #[test]
    fn test_convert_metric_values_distance_timeseries_converts_to_kilometers() {
        use crate::domain::models::training::{TimeseriesAggregate, TrainingMetricAggregate};

        // Distance timeseries should also be converted to kilometers
        let values = HashMap::from([(
            "Running".to_string(),
            HashMap::from([("2025-09-24".to_string(), 5000.0)]), // 5km in meters
        )]);

        let (unit, converted_values) = convert_metric_values(
            values,
            &ActivityMetricSource::Timeseries((
                TimeseriesMetric::Distance,
                TimeseriesAggregate::Max,
            )),
            &TrainingMetricAggregate::Max,
        );

        assert_eq!(unit, Unit::Kilometer);
        assert_eq!(
            converted_values.get("Running").unwrap().get("2025-09-24"),
            Some(&5.0)
        );
    }

    #[test]
    fn test_convert_metric_values_speed_timeseries_converts_to_kmh() {
        use crate::domain::models::training::{TimeseriesAggregate, TrainingMetricAggregate};

        // Speed should be converted from m/s to km/h
        let values = HashMap::from([(
            "Cycling".to_string(),
            HashMap::from([
                ("2025-09-24".to_string(), 10.0), // 10 m/s = 36 km/h
                ("2025-09-25".to_string(), 5.55), // 5.55 m/s ≈ 20 km/h
            ]),
        )]);

        let (unit, converted_values) = convert_metric_values(
            values,
            &ActivityMetricSource::Timeseries((
                TimeseriesMetric::Speed,
                TimeseriesAggregate::Average,
            )),
            &TrainingMetricAggregate::Average,
        );

        assert_eq!(unit, Unit::KilometerPerHour);
        assert_eq!(
            converted_values.get("Cycling").unwrap().get("2025-09-24"),
            Some(&36.0)
        );
        assert!(
            (converted_values
                .get("Cycling")
                .unwrap()
                .get("2025-09-25")
                .unwrap()
                - 19.98)
                .abs()
                < 0.01
        );
    }

    #[test]
    fn test_convert_metric_values_duration_statistic_no_conversion() {
        use crate::domain::models::training::TrainingMetricAggregate;

        // Duration should keep its unit (seconds) and values unchanged
        let values = HashMap::from([(
            "Other".to_string(),
            HashMap::from([
                ("2025-09-24".to_string(), 3600.0), // 1 hour in seconds
                ("2025-09-25".to_string(), 7200.0), // 2 hours in seconds
            ]),
        )]);

        let (unit, converted_values) = convert_metric_values(
            values.clone(),
            &ActivityMetricSource::Statistic(ActivityStatistic::Duration),
            &TrainingMetricAggregate::Sum,
        );

        assert_eq!(unit, Unit::Second);
        assert_eq!(
            converted_values.get("Other").unwrap().get("2025-09-24"),
            Some(&3600.0)
        );
        assert_eq!(
            converted_values.get("Other").unwrap().get("2025-09-25"),
            Some(&7200.0)
        );
    }

    #[test]
    fn test_convert_metric_values_power_timeseries_no_conversion() {
        use crate::domain::models::training::{TimeseriesAggregate, TrainingMetricAggregate};

        // Power should keep its unit (watts) and values unchanged
        let values = HashMap::from([(
            "Cycling".to_string(),
            HashMap::from([("2025-09-24".to_string(), 250.0)]),
        )]);

        let (unit, converted_values) = convert_metric_values(
            values,
            &ActivityMetricSource::Timeseries((
                TimeseriesMetric::Power,
                TimeseriesAggregate::Average,
            )),
            &TrainingMetricAggregate::Average,
        );

        assert_eq!(unit, Unit::Watt);
        assert_eq!(
            converted_values.get("Cycling").unwrap().get("2025-09-24"),
            Some(&250.0)
        );
    }

    #[test]
    fn test_convert_metric_values_with_multiple_groups() {
        use crate::domain::models::training::TrainingMetricAggregate;

        // Test that conversion works correctly with multiple groups
        let values = HashMap::from([
            (
                "Cycling".to_string(),
                HashMap::from([("2025-09-24".to_string(), 20000.0)]),
            ),
            (
                "Running".to_string(),
                HashMap::from([("2025-09-24".to_string(), 10000.0)]),
            ),
        ]);

        let (unit, converted_values) = convert_metric_values(
            values,
            &ActivityMetricSource::Statistic(ActivityStatistic::Distance),
            &TrainingMetricAggregate::Sum,
        );

        assert_eq!(unit, Unit::Kilometer);
        assert_eq!(
            converted_values.get("Cycling").unwrap().get("2025-09-24"),
            Some(&20.0)
        );
        assert_eq!(
            converted_values.get("Running").unwrap().get("2025-09-24"),
            Some(&10.0)
        );
    }

    #[test]
    fn test_convert_metric_values_pace_timeseries_converts_to_s_per_km() {
        use crate::domain::models::training::{TimeseriesAggregate, TrainingMetricAggregate};

        // Pace should be converted from s/m to s/km (multiply by 1000)
        let values = HashMap::from([(
            "Running".to_string(),
            HashMap::from([
                ("2025-09-24".to_string(), 0.2),  // 0.2 s/m = 200 s/km = 3:20 min/km
                ("2025-09-25".to_string(), 0.25), // 0.25 s/m = 250 s/km = 4:10 min/km
            ]),
        )]);

        let (unit, converted_values) = convert_metric_values(
            values,
            &ActivityMetricSource::Timeseries((
                TimeseriesMetric::Pace,
                TimeseriesAggregate::Average,
            )),
            &TrainingMetricAggregate::Average,
        );

        assert_eq!(unit, Unit::SecondPerKilometer);
        assert_eq!(
            converted_values.get("Running").unwrap().get("2025-09-24"),
            Some(&200.0)
        );
        assert_eq!(
            converted_values.get("Running").unwrap().get("2025-09-25"),
            Some(&250.0)
        );
    }

    #[test]
    fn test_convert_metric_values_pace_with_multiple_groups() {
        use crate::domain::models::training::{TimeseriesAggregate, TrainingMetricAggregate};

        // Test pace conversion with multiple sports
        let values = HashMap::from([
            (
                "Running".to_string(),
                HashMap::from([("2025-09-24".to_string(), 0.2)]), // 200 s/km
            ),
            (
                "Cycling".to_string(),
                HashMap::from([("2025-09-24".to_string(), 0.1)]), // 100 s/km
            ),
        ]);

        let (unit, converted_values) = convert_metric_values(
            values,
            &ActivityMetricSource::Timeseries((
                TimeseriesMetric::Pace,
                TimeseriesAggregate::Average,
            )),
            &TrainingMetricAggregate::Average,
        );

        assert_eq!(unit, Unit::SecondPerKilometer);
        assert_eq!(
            converted_values.get("Running").unwrap().get("2025-09-24"),
            Some(&200.0)
        );
        assert_eq!(
            converted_values.get("Cycling").unwrap().get("2025-09-24"),
            Some(&100.0)
        );
    }

    #[test]
    fn test_convert_metric_values_pace_zero_value() {
        use crate::domain::models::training::{TimeseriesAggregate, TrainingMetricAggregate};

        // Test that zero pace values are handled correctly (0 * 1000 = 0)
        let values = HashMap::from([(
            "Running".to_string(),
            HashMap::from([("2025-09-24".to_string(), 0.0)]),
        )]);

        let (unit, converted_values) = convert_metric_values(
            values,
            &ActivityMetricSource::Timeseries((
                TimeseriesMetric::Pace,
                TimeseriesAggregate::Average,
            )),
            &TrainingMetricAggregate::Average,
        );

        assert_eq!(unit, Unit::SecondPerKilometer);
        assert_eq!(
            converted_values.get("Running").unwrap().get("2025-09-24"),
            Some(&0.0)
        );
    }
}
