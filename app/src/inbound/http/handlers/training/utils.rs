use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use chrono::{DateTime, FixedOffset, Local};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::domain::{
    models::{
        activity::{ActivityMetricSource, ActivityStatistic, TimeseriesMetric, ToUnit, Unit},
        training::{
            TrainingMetricAggregate, TrainingMetricBin, TrainingMetricGranularity,
            TrainingMetricValues, TrainingMetricWindow,
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
#[derive(Debug, Clone, Constructor, PartialEq)]
pub struct GroupedMetricValues {
    values: HashMap<String, GranuleValues>,
    summary_values: HashMap<String, f64>,
    unit: Unit,
}

impl GroupedMetricValues {
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, HashMap<String, f64>> {
        self.values.iter()
    }

    pub fn iter_mut(
        &mut self,
    ) -> std::collections::hash_map::IterMut<'_, String, HashMap<String, f64>> {
        self.values.iter_mut()
    }

    pub fn unit(&self) -> Unit {
        self.unit
    }

    pub fn values_and_summary(self) -> (HashMap<String, GranuleValues>, HashMap<String, f64>) {
        (self.values, self.summary_values)
    }
}

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

pub fn group_metric_values(values: TrainingMetricValues) -> GroupedMetricValues {
    let mut grouped_values: HashMap<String, GranuleValues> = HashMap::new();
    for (bin, value) in values.iter() {
        grouped_values
            .entry(bin.group().clone().unwrap_or_else(|| NO_GROUP.to_string()))
            .or_default()
            .insert(bin.granule().to_string(), value.value());
    }

    GroupedMetricValues::new(
        grouped_values,
        values.summary_values().as_hash_map(),
        values.unit(),
    )
}

pub fn fill_missing_granules(
    mut values: GroupedMetricValues,
    window: &TrainingMetricWindow,
    range: &MetricsDateRange,
) -> GroupedMetricValues {
    let expected_granules: Vec<String> = window
        .granularity()
        .bins_keys(
            &range.start,
            &range.end.unwrap_or(Local::now().fixed_offset()),
        )
        .iter()
        .map(|granule| granule.to_string())
        .collect();

    for (_group, values) in values.iter_mut() {
        for granule in expected_granules.iter() {
            values.entry(granule.to_owned()).or_insert(0.);
        }
    }

    values
}

/// Converts metric values from their raw units to display units.
///
/// This function handles unit conversions for different metric types:
/// - Distance: meters → kilometers (÷ 1000)
/// - Speed: m/s → km/h (× 3.6)
/// - Pace: s:m → s:km (× 1000)
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
pub fn convert_metric_values_unit(values: GroupedMetricValues) -> GroupedMetricValues {
    match values.unit {
        Unit::Meter => {
            let converted_values = values
                .iter()
                .map(|(group, group_values)| {
                    (
                        group.clone(),
                        group_values
                            .iter()
                            .map(|(k, val)| (k.clone(), *val / 1000.))
                            .collect::<GranuleValues>(),
                    )
                })
                .collect::<HashMap<_, _>>();
            let converted_summary = values
                .summary_values
                .iter()
                .map(|(name, value)| (name.clone(), *value / 1000.))
                .collect::<HashMap<_, _>>();
            GroupedMetricValues::new(converted_values, converted_summary, Unit::Kilometer)
        }
        Unit::MeterPerSecond => {
            let converted = values
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
                .collect();
            let converted_summary = values
                .summary_values
                .iter()
                .map(|(name, value)| (name.clone(), *value * 3.6))
                .collect::<HashMap<_, _>>();
            GroupedMetricValues::new(converted, converted_summary, Unit::KilometerPerHour)
        }
        Unit::SecondPerMeter => {
            let converted = values
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
                .collect();
            let converted_summary = values
                .summary_values
                .iter()
                .map(|(name, value)| (name.clone(), *value * 1000.))
                .collect::<HashMap<_, _>>();
            GroupedMetricValues::new(converted, converted_summary, Unit::SecondPerKilometer)
        }
        _ => values,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::DateTime;

    use crate::domain::models::activity::TimeseriesAggregate;
    use crate::domain::models::training::{
        TrainingMetricAggregate, TrainingMetricBin, TrainingMetricGranularity, TrainingMetricValue,
        TrainingMetricValues,
    };

    use super::*;

    #[test]
    fn test_convert_metric_values_distance_statistic_converts_to_kilometers() {
        use crate::domain::models::training::TrainingMetricAggregate;

        // Distance statistic should be converted from meters to kilometers
        let values = GroupedMetricValues::new(
            HashMap::from([(
                "Cycling".to_string(),
                HashMap::from([
                    ("2025-09-24".to_string(), 10000.0), // 10km in meters
                    ("2025-09-25".to_string(), 25000.0), // 25km in meters
                ]),
            )]),
            HashMap::from([("average".to_string(), 12000.)]),
            Unit::Meter,
        );

        let converted_values = convert_metric_values_unit(values);

        assert_eq!(converted_values.unit(), Unit::Kilometer);
        assert_eq!(
            converted_values
                .values
                .get("Cycling")
                .unwrap()
                .get("2025-09-24"),
            Some(&10.0)
        );
        assert_eq!(
            converted_values
                .values
                .get("Cycling")
                .unwrap()
                .get("2025-09-25"),
            Some(&25.0)
        );
        assert_eq!(
            *converted_values.summary_values.get("average").unwrap(),
            12.
        );
    }

    #[test]
    fn test_convert_metric_values_distance_timeseries_converts_to_kilometers() {
        // Distance timeseries should also be converted to kilometers
        let values = GroupedMetricValues::new(
            HashMap::from([(
                "Running".to_string(),
                HashMap::from([("2025-09-24".to_string(), 5000.0)]), // 5km in meters
            )]),
            HashMap::from([("average".to_string(), 12000.)]),
            Unit::Meter,
        );

        let converted_values = convert_metric_values_unit(values);

        assert_eq!(converted_values.unit(), Unit::Kilometer);
        assert_eq!(
            converted_values
                .values
                .get("Running")
                .unwrap()
                .get("2025-09-24"),
            Some(&5.0)
        );
        assert_eq!(
            *converted_values.summary_values.get("average").unwrap(),
            12.
        );
    }

    #[test]
    fn test_convert_metric_values_speed_timeseries_converts_to_kmh() {
        // Speed should be converted from m/s to km/h
        let values = GroupedMetricValues::new(
            HashMap::from([(
                "Cycling".to_string(),
                HashMap::from([
                    ("2025-09-24".to_string(), 10.0), // 10 m/s = 36 km/h
                    ("2025-09-25".to_string(), 5.55), // 5.55 m/s ≈ 20 km/h
                ]),
            )]),
            HashMap::from([("average".to_string(), 10.)]),
            Unit::MeterPerSecond,
        );

        let converted_values = convert_metric_values_unit(values);

        assert_eq!(converted_values.unit(), Unit::KilometerPerHour);
        assert_eq!(
            converted_values
                .values
                .get("Cycling")
                .unwrap()
                .get("2025-09-24"),
            Some(&36.0)
        );
        assert!(
            (converted_values
                .values
                .get("Cycling")
                .unwrap()
                .get("2025-09-25")
                .unwrap()
                - 19.98)
                .abs()
                < 0.01
        );
        assert_eq!(
            *converted_values.summary_values.get("average").unwrap(),
            36.
        );
    }

    #[test]
    fn test_convert_metric_values_duration_statistic_no_conversion() {
        use crate::domain::models::training::TrainingMetricAggregate;

        // Duration should keep its unit (seconds) and values unchanged
        let values = GroupedMetricValues::new(
            HashMap::from([(
                "Other".to_string(),
                HashMap::from([
                    ("2025-09-24".to_string(), 3600.0), // 1 hour in seconds
                    ("2025-09-25".to_string(), 7200.0), // 2 hours in seconds
                ]),
            )]),
            HashMap::from([("average".to_string(), 1000.)]),
            Unit::Second,
        );

        let converted_values = convert_metric_values_unit(values.clone());

        assert_eq!(converted_values.unit(), Unit::Second);
        assert_eq!(
            converted_values
                .values
                .get("Other")
                .unwrap()
                .get("2025-09-24"),
            Some(&3600.0)
        );
        assert_eq!(
            converted_values
                .values
                .get("Other")
                .unwrap()
                .get("2025-09-25"),
            Some(&7200.0)
        );
        assert_eq!(
            *converted_values.summary_values.get("average").unwrap(),
            1000.
        );
    }

    #[test]
    fn test_convert_metric_values_power_timeseries_no_conversion() {
        // Power should keep its unit (watts) and values unchanged
        let values = GroupedMetricValues::new(
            HashMap::from([(
                "Cycling".to_string(),
                HashMap::from([("2025-09-24".to_string(), 250.0)]),
            )]),
            HashMap::from([("average".to_string(), 100.)]),
            Unit::Watt,
        );

        let converted_values = convert_metric_values_unit(values);

        assert_eq!(converted_values.unit(), Unit::Watt);
        assert_eq!(
            converted_values
                .values
                .get("Cycling")
                .unwrap()
                .get("2025-09-24"),
            Some(&250.0)
        );
        assert_eq!(
            *converted_values.summary_values.get("average").unwrap(),
            100.
        );
    }

    #[test]
    fn test_convert_metric_values_with_multiple_groups() {
        use crate::domain::models::training::TrainingMetricAggregate;

        // Test that conversion works correctly with multiple groups
        let values = GroupedMetricValues::new(
            HashMap::from([
                (
                    "Cycling".to_string(),
                    HashMap::from([("2025-09-24".to_string(), 20000.0)]),
                ),
                (
                    "Running".to_string(),
                    HashMap::from([("2025-09-24".to_string(), 10000.0)]),
                ),
            ]),
            HashMap::from([("average".to_string(), 1000.)]),
            Unit::Meter,
        );

        let converted_values = convert_metric_values_unit(values);

        assert_eq!(converted_values.unit(), Unit::Kilometer);
        assert_eq!(
            converted_values
                .values
                .get("Cycling")
                .unwrap()
                .get("2025-09-24"),
            Some(&20.0)
        );
        assert_eq!(
            converted_values
                .values
                .get("Running")
                .unwrap()
                .get("2025-09-24"),
            Some(&10.0)
        );
        assert_eq!(*converted_values.summary_values.get("average").unwrap(), 1.);
    }

    #[test]
    fn test_convert_metric_values_pace_timeseries_converts_to_s_per_km() {
        // Pace should be converted from s:m to s:km (multiply by 1000)
        let values = GroupedMetricValues::new(
            HashMap::from([(
                "Running".to_string(),
                HashMap::from([
                    ("2025-09-24".to_string(), 0.2),  // 0.2 s:m = 200 s/km = 3:20 min/km
                    ("2025-09-25".to_string(), 0.25), // 0.25 s:m = 250 s/km = 4:10 min/km
                ]),
            )]),
            HashMap::from([("average".to_string(), 0.2)]),
            Unit::SecondPerMeter,
        );

        let converted_values = convert_metric_values_unit(values);

        assert_eq!(converted_values.unit(), Unit::SecondPerKilometer);
        assert_eq!(
            converted_values
                .values
                .get("Running")
                .unwrap()
                .get("2025-09-24"),
            Some(&200.0)
        );
        assert_eq!(
            converted_values
                .values
                .get("Running")
                .unwrap()
                .get("2025-09-25"),
            Some(&250.0)
        );
        assert_eq!(
            *converted_values.summary_values.get("average").unwrap(),
            200.
        );
    }

    #[test]
    fn test_convert_metric_values_pace_with_multiple_groups() {
        use crate::domain::models::training::TrainingMetricAggregate;

        // Test pace conversion with multiple sports
        let values = GroupedMetricValues::new(
            HashMap::from([
                (
                    "Running".to_string(),
                    HashMap::from([("2025-09-24".to_string(), 0.2)]), // 200 s/km
                ),
                (
                    "Cycling".to_string(),
                    HashMap::from([("2025-09-24".to_string(), 0.1)]), // 100 s/km
                ),
            ]),
            HashMap::from([("average".to_string(), 0.2)]),
            Unit::SecondPerMeter,
        );

        let converted_values = convert_metric_values_unit(values);

        assert_eq!(converted_values.unit(), Unit::SecondPerKilometer);
        assert_eq!(
            converted_values
                .values
                .get("Running")
                .unwrap()
                .get("2025-09-24"),
            Some(&200.0)
        );
        assert_eq!(
            converted_values
                .values
                .get("Cycling")
                .unwrap()
                .get("2025-09-24"),
            Some(&100.0)
        );
        assert_eq!(
            *converted_values.summary_values.get("average").unwrap(),
            200.
        );
    }

    #[test]
    fn test_convert_metric_values_pace_zero_value() {
        // Test that zero pace values are handled correctly (0 * 1000 = 0)
        let values = GroupedMetricValues::new(
            HashMap::from([(
                "Running".to_string(),
                HashMap::from([("2025-09-24".to_string(), 0.0)]),
            )]),
            HashMap::from([("average".to_string(), 0.0)]),
            Unit::SecondPerMeter,
        );

        let converted_values = convert_metric_values_unit(values);

        assert_eq!(converted_values.unit(), Unit::SecondPerKilometer);
        assert_eq!(
            converted_values
                .values
                .get("Running")
                .unwrap()
                .get("2025-09-24"),
            Some(&0.0)
        );
        assert_eq!(
            *converted_values.summary_values.get("average").unwrap(),
            0.0
        );
    }
}

#[cfg(test)]
mod test_grouping_metric_values {
    use crate::domain::models::training::{TrainingMetricSummaryValues, TrainingMetricValue};

    use super::*;

    #[test]
    fn test_no_group_multiple_values() {
        let values = TrainingMetricValues::new(
            HashMap::from([
                (
                    TrainingMetricBin::new("2025-09-24".to_string(), None),
                    TrainingMetricValue::Max(10.0),
                ),
                (
                    TrainingMetricBin::new("2025-09-25".to_string(), None),
                    TrainingMetricValue::Max(15.0),
                ),
            ]),
            TrainingMetricSummaryValues::default(),
            Unit::Kilometer,
        );

        let grouped_values = group_metric_values(values);

        assert_eq!(
            grouped_values,
            GroupedMetricValues::new(
                HashMap::from([(
                    NO_GROUP.to_string(),
                    HashMap::from([
                        ("2025-09-24".to_string(), 10.0,),
                        ("2025-09-25".to_string(), 15.0),
                    ])
                )]),
                HashMap::new(),
                Unit::Kilometer
            )
        );
    }

    #[test]
    fn test_no_group_and_one_group_with_multiple_values() {
        let values = TrainingMetricValues::new(
            HashMap::from([
                // No group values
                (
                    TrainingMetricBin::new("2025-09-24".to_string(), None),
                    TrainingMetricValue::Max(10.0),
                ),
                (
                    TrainingMetricBin::new("2025-09-25".to_string(), None),
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
            ]),
            TrainingMetricSummaryValues::default(),
            Unit::Kilometer,
        );

        let grouped_values = group_metric_values(values);

        assert_eq!(
            grouped_values,
            GroupedMetricValues::new(
                HashMap::from([
                    (
                        NO_GROUP.to_string(),
                        HashMap::from([
                            ("2025-09-24".to_string(), 10.0,),
                            ("2025-09-25".to_string(), 15.0),
                        ])
                    ),
                    (
                        "Running".to_string(),
                        HashMap::from([
                            ("2025-09-24".to_string(), 5.0,),
                            ("2025-09-26".to_string(), 8.0),
                        ])
                    )
                ]),
                HashMap::new(),
                Unit::Kilometer
            )
        );
    }

    #[test]
    fn test_two_groups_with_multiple_values() {
        let values = TrainingMetricValues::new(
            HashMap::from([
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
            ]),
            TrainingMetricSummaryValues::default(),
            Unit::Kilometer,
        );

        let grouped_values = group_metric_values(values);

        assert_eq!(
            grouped_values,
            GroupedMetricValues::new(
                HashMap::from([
                    (
                        "Cycling".to_string(),
                        HashMap::from([
                            ("2025-09-24".to_string(), 10.0,),
                            ("2025-09-25".to_string(), 15.0),
                        ])
                    ),
                    (
                        "Running".to_string(),
                        HashMap::from([
                            ("2025-09-24".to_string(), 5.0,),
                            ("2025-09-26".to_string(), 8.0),
                        ])
                    )
                ]),
                HashMap::new(),
                Unit::Kilometer
            )
        );
    }
}

#[cfg(test)]
mod test_fill_grouped_metric_values {
    use chrono::NaiveDate;

    use crate::domain::models::training::TrainingMetricGroupBy;

    use super::*;

    #[test]
    fn test_fill_window_daily_no_gaps() {
        let window = TrainingMetricWindow::new(
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricGroupBy::none(),
        );
        let range = MetricsDateRange {
            start: DateTime::parse_from_rfc3339("2025-09-24T00:00:00Z").unwrap(),
            end: Some(DateTime::parse_from_rfc3339("2025-09-26T00:00:00Z").unwrap()),
        };
        let values = GroupedMetricValues::new(
            HashMap::from([(
                "Cycling".to_string(),
                HashMap::from([
                    ("2025-09-24".to_string(), 10.0),
                    ("2025-09-25".to_string(), 15.0),
                    ("2025-09-26".to_string(), 12.0),
                ]),
            )]),
            HashMap::new(),
            Unit::Kilometer,
        );

        let filled_values = fill_missing_granules(values, &window, &range);

        assert_eq!(
            filled_values,
            GroupedMetricValues::new(
                HashMap::from([(
                    "Cycling".to_string(),
                    HashMap::from([
                        ("2025-09-24".to_string(), 10.0),
                        ("2025-09-25".to_string(), 15.0),
                        ("2025-09-26".to_string(), 12.0),
                    ]),
                )]),
                HashMap::new(),
                Unit::Kilometer,
            )
        );
    }

    #[test]
    fn test_fill_window_daily_with_gaps() {
        let window = TrainingMetricWindow::new(
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
            TrainingMetricGroupBy::none(),
        );
        let range = MetricsDateRange {
            start: DateTime::parse_from_rfc3339("2025-09-23T00:00:00Z").unwrap(),
            end: Some(DateTime::parse_from_rfc3339("2025-09-27T00:00:00Z").unwrap()),
        };
        let values = GroupedMetricValues::new(
            HashMap::from([(
                "Cycling".to_string(),
                HashMap::from([
                    ("2025-09-24".to_string(), 10.0),
                    ("2025-09-26".to_string(), 12.0),
                ]),
            )]),
            HashMap::new(),
            Unit::Kilometer,
        );

        let filled_values = fill_missing_granules(values, &window, &range);

        assert_eq!(
            filled_values,
            GroupedMetricValues::new(
                HashMap::from([(
                    "Cycling".to_string(),
                    HashMap::from([
                        ("2025-09-23".to_string(), 0.),
                        ("2025-09-24".to_string(), 10.0),
                        ("2025-09-25".to_string(), 0.),
                        ("2025-09-26".to_string(), 12.0),
                        ("2025-09-27".to_string(), 0.),
                    ]),
                )]),
                HashMap::new(),
                Unit::Kilometer,
            )
        );
    }

    #[test]
    fn test_fill_window_weekly_with_gaps() {
        let window = TrainingMetricWindow::new(
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Average,
            TrainingMetricGroupBy::none(),
        );
        let range = MetricsDateRange {
            start: DateTime::parse_from_rfc3339("2025-09-24T00:00:00Z").unwrap(),
            end: Some(DateTime::parse_from_rfc3339("2025-10-08T00:00:00Z").unwrap()),
        };
        let values = GroupedMetricValues::new(
            HashMap::from([(
                "Cycling".to_string(),
                HashMap::from([
                    ("2025-09-29".to_string(), 10.0),
                    ("2025-10-06".to_string(), 12.0),
                ]),
            )]),
            HashMap::new(),
            Unit::Kilometer,
        );

        let filled_values = fill_missing_granules(values, &window, &range);

        assert_eq!(
            filled_values,
            GroupedMetricValues::new(
                HashMap::from([(
                    "Cycling".to_string(),
                    HashMap::from([
                        ("2025-09-22".to_string(), 0.),
                        ("2025-09-29".to_string(), 10.0),
                        ("2025-10-06".to_string(), 12.0),
                    ]),
                )]),
                HashMap::new(),
                Unit::Kilometer,
            )
        );
    }

    #[test]
    fn test_fill_window_monthly_with_gaps() {
        let window = TrainingMetricWindow::new(
            TrainingMetricGranularity::Monthly,
            TrainingMetricAggregate::Average,
            TrainingMetricGroupBy::none(),
        );
        let range = MetricsDateRange {
            start: DateTime::parse_from_rfc3339("2025-09-14T00:00:00Z").unwrap(),
            end: Some(DateTime::parse_from_rfc3339("2025-11-02T00:00:00Z").unwrap()),
        };
        let values = GroupedMetricValues::new(
            HashMap::from([(
                "Cycling".to_string(),
                HashMap::from([
                    ("2025-09-01".to_string(), 20.0),
                    ("2025-11-01".to_string(), 30.0),
                ]),
            )]),
            HashMap::new(),
            Unit::Kilometer,
        );

        let filled_values = fill_missing_granules(values, &window, &range);

        assert_eq!(
            filled_values,
            GroupedMetricValues::new(
                HashMap::from([(
                    "Cycling".to_string(),
                    HashMap::from([
                        ("2025-09-01".to_string(), 20.0),
                        ("2025-10-01".to_string(), 0.),
                        ("2025-11-01".to_string(), 30.0),
                    ]),
                )]),
                HashMap::new(),
                Unit::Kilometer,
            )
        );
    }
}
