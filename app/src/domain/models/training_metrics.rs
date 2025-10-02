use std::{
    collections::{HashMap, hash_map::Iter},
    fmt,
    hash::Hash,
};

use chrono::{DateTime, Datelike, Days, FixedOffset, Months, NaiveDate, SecondsFormat};
use derive_more::{AsRef, Constructor, Display};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::{
    UserId,
    activity::{
        ActivityId, ActivityStartTime, ActivityStatistic, ActivityWithTimeseries, TimeseriesMetric,
        ToUnit, Unit,
    },
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, AsRef, Hash)]
pub struct TrainingMetricId(String);

impl TrainingMetricId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl fmt::Display for TrainingMetricId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TrainingMetricId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct TrainingMetricDefinition {
    id: TrainingMetricId,
    user: UserId,
    source: ActivityMetricSource,
    granularity: TrainingMetricGranularity,
    granularity_aggregate: TrainingMetricAggregate,
}

impl TrainingMetricDefinition {
    pub fn id(&self) -> &TrainingMetricId {
        &self.id
    }

    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn source(&self) -> &ActivityMetricSource {
        &self.source
    }

    pub fn granularity(&self) -> &TrainingMetricGranularity {
        &self.granularity
    }

    pub fn aggregate(&self) -> &TrainingMetricAggregate {
        &self.granularity_aggregate
    }
}

impl TrainingMetricDefinition {
    pub fn compute_values(&self, activities: &[ActivityWithTimeseries]) -> TrainingMetricValues {
        let activity_metrics = activities
            .iter()
            .filter_map(|activity| self.source.from_activity(activity))
            .collect();
        let grouped_metrics = group_metrics_by_granularity(&self.granularity, activity_metrics);
        TrainingMetricValues::new(aggregate_metrics(
            &self.granularity_aggregate,
            grouped_metrics,
        ))
    }

    pub fn get_corresponding_granule(&self, activity: &ActivityWithTimeseries) -> Option<Granule> {
        self.granularity.granule(activity)
    }
}

/// A [Granule] represents a set on which a definition will yield a single value. It can represent
/// a single activity with [Granule::Activity], or a temporal range with [Granule::Date].
#[derive(Debug, Clone, PartialEq)]
pub enum Granule {
    Activity(ActivityId),
    Date(DateGranule),
}

#[derive(Debug, Clone, Constructor, PartialEq)]
pub struct DateGranule {
    start: NaiveDate,
    end: NaiveDate,
}

/// An [ActivityMetric] represents the value of an [ActivityMetricSource] extracted from
/// a single [ActivityWithTimeseries]. On top of the metric value, it contains metadata like
/// the activity start time and duration that can be used in later computations.
#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct ActivityMetric {
    value: f64,
    activity_start_time: ActivityStartTime,
    activity_duration: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActivityMetricSource {
    Statistic(ActivityStatistic),
    Timeseries((TimeseriesMetric, TimeseriesAggregate)),
}

impl ActivityMetricSource {
    pub fn from_activity(&self, activity: &ActivityWithTimeseries) -> Option<ActivityMetric> {
        match self {
            Self::Statistic(statistic) => activity.statistics().get(statistic).cloned(),
            Self::Timeseries((metric, aggregate)) => {
                aggregate.value_from_timeseries(metric, activity)
            }
        }
        .map(|value| {
            ActivityMetric::new(
                value,
                *activity.start_time(),
                activity
                    .statistics()
                    .get(&ActivityStatistic::Duration)
                    .cloned(),
            )
        })
    }
}

impl ToUnit for ActivityMetricSource {
    fn unit(&self) -> Unit {
        match self {
            Self::Statistic(stat) => stat.unit(),
            Self::Timeseries((metric, _)) => metric.unit(),
        }
    }
}

fn group_metrics_by_granularity(
    granularity: &TrainingMetricGranularity,
    metrics: Vec<ActivityMetric>,
) -> HashMap<String, Vec<ActivityMetric>> {
    let mut grouped_values: HashMap<String, Vec<ActivityMetric>> = HashMap::new();
    for value in metrics {
        let key = granularity.datetime_key(value.activity_start_time.date());
        grouped_values.entry(key).or_default().push(value);
    }
    grouped_values
}

fn aggregate_metrics(
    aggregate: &TrainingMetricAggregate,
    metrics: HashMap<String, Vec<ActivityMetric>>,
) -> HashMap<String, TrainingMetricValue> {
    let mut res = HashMap::new();

    for (key, values) in metrics.into_iter() {
        let Some(training_metric_value) = aggregate.aggregate(values) else {
            continue;
        };
        res.insert(key, training_metric_value);
    }

    res
}

#[derive(Debug, Clone, PartialEq, Display)]
pub enum TrainingMetricGranularity {
    Activity,
    Daily,
    Weekly,
    Monthly,
}

impl TrainingMetricGranularity {
    pub fn datetime_key(&self, dt: &DateTime<FixedOffset>) -> String {
        match self {
            TrainingMetricGranularity::Activity => dt.to_rfc3339_opts(SecondsFormat::Secs, true),
            TrainingMetricGranularity::Daily => dt.date_naive().to_string(),
            TrainingMetricGranularity::Weekly => dt
                .date_naive()
                .week(chrono::Weekday::Mon)
                .first_day()
                .to_string(),
            TrainingMetricGranularity::Monthly => dt.date_naive().with_day(1).unwrap().to_string(),
        }
    }

    pub fn granule(&self, activity: &ActivityWithTimeseries) -> Option<Granule> {
        let start = activity.start_time().date().date_naive();
        Some(match self {
            TrainingMetricGranularity::Activity => Granule::Activity(activity.id().clone()),
            TrainingMetricGranularity::Daily => Granule::Date(DateGranule::new(
                start,
                start.checked_add_days(Days::new(1))?,
            )),
            TrainingMetricGranularity::Weekly => Granule::Date(DateGranule::new(
                start.week(chrono::Weekday::Mon).first_day(),
                start.week(chrono::Weekday::Mon).last_day(),
            )),
            TrainingMetricGranularity::Monthly => Granule::Date(DateGranule::new(
                start.with_day(1)?,
                start.with_day(start.num_days_in_month().into())?,
            )),
        })
    }

    /// Computes the bins values for the [TrainingMetricGranularity] for the given range [start,
    /// end].
    /// If the results is [None] that means all datetimes are valid within the range
    /// ([TrainingMetricGranularity::Activity]).
    pub fn bins(
        &self,
        start: &DateTime<FixedOffset>,
        end: &DateTime<FixedOffset>,
    ) -> Option<Vec<String>> {
        let mut dates = vec![];

        #[allow(clippy::type_complexity)]
        let (mut start, end, next_dt): (
            NaiveDate,
            NaiveDate,
            Box<dyn Fn(NaiveDate) -> Option<NaiveDate>>,
        ) = match self {
            Self::Daily => (
                start.date_naive(),
                end.date_naive(),
                Box::new(|dt: NaiveDate| dt.checked_add_days(Days::new(1))),
            ),
            Self::Weekly => (
                start.date_naive().week(chrono::Weekday::Mon).first_day(),
                end.date_naive().week(chrono::Weekday::Mon).first_day(),
                Box::new(|dt: NaiveDate| dt.checked_add_days(Days::new(7))),
            ),
            Self::Monthly => (
                start.date_naive().with_day(1).unwrap(),
                end.date_naive().with_day(1).unwrap(),
                Box::new(|dt: NaiveDate| dt.checked_add_months(Months::new(1))),
            ),
            Self::Activity => return None,
        };

        loop {
            dates.push(start.to_string());
            let Some(new_start) = next_dt(start) else {
                return Some(dates);
            };
            start = new_start;
            if new_start > end {
                break;
            }
        }
        Some(dates)
    }
}

#[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
pub enum TimeseriesAggregate {
    Min,
    Max,
    Average,
    Sum,
}

impl TimeseriesAggregate {
    fn value_from_timeseries(
        &self,
        metric: &TimeseriesMetric,
        activity: &ActivityWithTimeseries,
    ) -> Option<f64> {
        let values: Vec<f64> = activity.timeseries().metrics().iter().find_map(|m| {
            if m.metric() == metric {
                Some(
                    m.values()
                        .iter()
                        .filter_map(|val| val.as_ref().map(f64::from))
                        .collect(),
                )
            } else {
                None
            }
        })?;
        if values.is_empty() {
            return None;
        }
        let length = values.len();
        match self {
            Self::Min => values.into_iter().reduce(f64::min),
            Self::Max => values.into_iter().reduce(f64::max),
            Self::Average => values
                .into_iter()
                .reduce(|acc, e| acc + e)
                .map(|val| val / length as f64),
            Self::Sum => values.into_iter().reduce(|acc, e| acc + e),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
pub enum TrainingMetricAggregate {
    Min,
    Max,
    Average,
    Sum,
}

impl TrainingMetricAggregate {
    fn aggregate(&self, activity_metrics: Vec<ActivityMetric>) -> Option<TrainingMetricValue> {
        if activity_metrics.is_empty() {
            return None;
        }
        Some(match self {
            TrainingMetricAggregate::Min => TrainingMetricValue::Min(
                activity_metrics
                    .into_iter()
                    .fold(f64::MAX, |min, metric| min.min(metric.value)),
            ),
            TrainingMetricAggregate::Max => TrainingMetricValue::Max(
                activity_metrics
                    .into_iter()
                    .fold(f64::MIN, |max, metric| max.max(metric.value)),
            ),
            TrainingMetricAggregate::Average => {
                let number_of_metrics = activity_metrics.len();
                let sum = activity_metrics
                    .into_iter()
                    .fold(0., |sum, metric| sum + metric.value);

                TrainingMetricValue::Average {
                    value: sum / number_of_metrics as f64,
                    sum,
                    number_of_elements: number_of_metrics,
                }
            }
            TrainingMetricAggregate::Sum => TrainingMetricValue::Sum(
                activity_metrics
                    .into_iter()
                    .fold(0., |sum, metric| sum + metric.value),
            ),
        })
    }

    pub fn update_value(
        &self,
        previous_value: &TrainingMetricValue,
        new_metric: &ActivityMetric,
    ) -> Option<TrainingMetricValue> {
        match self {
            Self::Min => {
                let TrainingMetricValue::Min(min) = previous_value else {
                    return None;
                };
                Some(TrainingMetricValue::Min(min.min(new_metric.value)))
            }
            Self::Max => {
                let TrainingMetricValue::Max(max) = previous_value else {
                    return None;
                };
                Some(TrainingMetricValue::Max(max.max(new_metric.value)))
            }
            Self::Sum => {
                let TrainingMetricValue::Sum(sum) = previous_value else {
                    return None;
                };
                Some(TrainingMetricValue::Sum(sum + new_metric.value))
            }
            Self::Average => {
                let TrainingMetricValue::Average {
                    sum,
                    value: _,
                    number_of_elements,
                } = previous_value
                else {
                    return None;
                };
                Some(TrainingMetricValue::Average {
                    sum: *sum + new_metric.value,
                    number_of_elements: *number_of_elements + 1,
                    value: (*sum + new_metric.value) / (*number_of_elements as f64 + 1.),
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrainingMetricValue {
    Min(f64),
    Max(f64),
    Sum(f64),
    Average {
        value: f64,
        sum: f64,
        number_of_elements: usize,
    },
}

impl TrainingMetricValue {
    pub fn value(&self) -> &f64 {
        match self {
            Self::Max(max) => max,
            Self::Min(min) => min,
            Self::Sum(sum) => sum,
            Self::Average {
                value,
                sum: _,
                number_of_elements: _,
            } => value,
        }
    }
}

#[derive(Debug, Clone, Constructor, Default)]
pub struct TrainingMetricValues(HashMap<String, TrainingMetricValue>);

impl TrainingMetricValues {
    pub fn insert(
        &mut self,
        key: String,
        value: TrainingMetricValue,
    ) -> Option<TrainingMetricValue> {
        self.0.insert(key, value)
    }

    pub fn get(&self, key: &str) -> Option<&TrainingMetricValue> {
        self.0.get(key)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> Iter<'_, String, TrainingMetricValue> {
        self.0.iter()
    }
}

impl TrainingMetricValues {
    pub fn as_hash_map(self) -> HashMap<String, TrainingMetricValue> {
        self.0
    }

    pub fn as_float_hash_map(&self) -> HashMap<String, f64> {
        HashMap::from_iter(
            self.0
                .iter()
                .map(|(bin, value)| (bin.clone(), *value.value())),
        )
    }
}

impl std::iter::IntoIterator for TrainingMetricValues {
    type Item = (String, TrainingMetricValue);
    type IntoIter = std::collections::hash_map::IntoIter<String, TrainingMetricValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod test_training_metrics {

    use crate::domain::models::{
        UserId,
        activity::{
            Activity, ActivityId, ActivityStartTime, ActivityStatistics, ActivityTimeseries, Sport,
            Timeseries, TimeseriesTime, TimeseriesValue,
        },
    };

    use super::*;

    fn default_activity() -> ActivityWithTimeseries {
        ActivityWithTimeseries::new(
            Activity::new(
                ActivityId::default(),
                UserId::test_default(),
                None,
                ActivityStartTime::new(
                    "2025-09-03T00:00:00Z"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                ),
                Sport::Cycling,
                ActivityStatistics::default(),
            ),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![0, 1, 2]),
                vec![Timeseries::new(
                    TimeseriesMetric::Power,
                    vec![
                        Some(TimeseriesValue::Int(10)),
                        Some(TimeseriesValue::Int(20)),
                        Some(TimeseriesValue::Int(30)),
                    ],
                )],
            ),
        )
    }

    #[test]
    fn test_extract_aggregated_activity_metric_no_metric_found() {
        let metric = TimeseriesMetric::Speed;
        let aggregate = TimeseriesAggregate::Min;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_aggregated_activity_metric_metric_is_empty() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Average;
        let activity = ActivityWithTimeseries::new(
            Activity::new(
                ActivityId::default(),
                UserId::test_default(),
                None,
                ActivityStartTime::new(
                    "2025-09-03T00:00:00Z"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                ),
                Sport::Cycling,
                ActivityStatistics::default(),
            ),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![]),
                vec![Timeseries::new(TimeseriesMetric::Power, vec![])],
            ),
        );

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_aggregated_activity_metric_min_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Min;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 10.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_max_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Max;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 30.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_average_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Average;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 20.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_total_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Sum;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 60.)
    }

    #[test]
    fn test_group_metric_by_granularity_activity() {
        let metric_1 = ActivityMetric::new(
            12.3,
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(120.),
        );
        let metric_2 = ActivityMetric::new(
            18.1,
            ActivityStartTime::new(
                "2025-09-03T02:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(120.),
        );
        let metrics = vec![metric_1.clone(), metric_2.clone()];
        let granularity = TrainingMetricGranularity::Activity;

        let res = group_metrics_by_granularity(&granularity, metrics);
        assert_eq!(res.len(), 2);
        assert_eq!(res.get("2025-09-03T00:00:00Z").unwrap(), &vec![metric_1]);
        assert_eq!(res.get("2025-09-03T02:00:00Z").unwrap(), &vec![metric_2]);
    }

    #[test]
    fn test_group_metric_by_granularity_daily() {
        let metric_1 = ActivityMetric::new(
            12.3,
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(120.),
        );
        let metric_2 = ActivityMetric::new(
            18.1,
            ActivityStartTime::new(
                "2025-09-03T02:00:00+03:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(120.),
        );

        let metric_3 = ActivityMetric::new(
            67.1,
            ActivityStartTime::new(
                "2025-09-04T02:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(120.),
        );
        let metrics = vec![metric_1.clone(), metric_2.clone(), metric_3.clone()];
        let granularity = TrainingMetricGranularity::Daily;

        let res = group_metrics_by_granularity(&granularity, metrics);
        assert_eq!(res.len(), 2);
        assert_eq!(res.get("2025-09-03").unwrap(), &vec![metric_1, metric_2]);
        assert_eq!(res.get("2025-09-04").unwrap(), &vec![metric_3]);
    }

    #[test]
    fn test_group_metric_by_granularity_weekly() {
        let metric_1 = ActivityMetric::new(
            12.3,
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(12.),
        );
        let metric_2 = ActivityMetric::new(
            18.1,
            ActivityStartTime::new(
                "2025-09-05T02:00:00+03:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(12.),
        );
        let metric_3 = ActivityMetric::new(
            67.1,
            ActivityStartTime::new(
                "2025-09-14T02:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(12.),
        );
        let metrics = vec![metric_1.clone(), metric_2.clone(), metric_3.clone()];
        let granularity = TrainingMetricGranularity::Weekly;

        let res = group_metrics_by_granularity(&granularity, metrics);
        assert_eq!(res.len(), 2);
        assert_eq!(res.get("2025-09-01").unwrap(), &vec![metric_1, metric_2]);
        assert_eq!(res.get("2025-09-08").unwrap(), &vec![metric_3]);
    }

    #[test]
    fn test_group_metric_by_granularity_monthly() {
        let metric_1 = ActivityMetric::new(
            12.3,
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            None,
        );
        let metric_2 = ActivityMetric::new(
            18.1,
            ActivityStartTime::new(
                "2025-09-05T02:00:00+03:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            None,
        );
        let metric_3 = ActivityMetric::new(
            67.1,
            ActivityStartTime::new(
                "2025-08-14T02:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            None,
        );
        let metrics = vec![metric_1.clone(), metric_2.clone(), metric_3.clone()];
        let granularity = TrainingMetricGranularity::Monthly;

        let res = group_metrics_by_granularity(&granularity, metrics);
        assert_eq!(res.len(), 2);
        assert_eq!(res.get("2025-09-01").unwrap(), &vec![metric_1, metric_2]);
        assert_eq!(res.get("2025-08-01").unwrap(), &vec![metric_3]);
    }

    #[test]
    fn test_aggregate_metrics_min() {
        let metrics = HashMap::from([(
            "2025-09-01".to_string(),
            vec![
                ActivityMetric::new(
                    12.3,
                    ActivityStartTime::new(
                        "2025-09-03T00:00:00Z"
                            .parse::<DateTime<FixedOffset>>()
                            .unwrap(),
                    ),
                    None,
                ),
                ActivityMetric::new(
                    1.3,
                    ActivityStartTime::new(
                        "2025-09-03T00:00:00Z"
                            .parse::<DateTime<FixedOffset>>()
                            .unwrap(),
                    ),
                    None,
                ),
            ],
        )]);
        let aggregate = TrainingMetricAggregate::Min;

        let res = aggregate_metrics(&aggregate, metrics);

        assert_eq!(
            *res.get("2025-09-01").unwrap(),
            TrainingMetricValue::Min(1.3)
        );
    }

    #[test]
    fn test_compute_training_metrics() {
        let activities = vec![default_activity()];
        let metric_definition = TrainingMetricDefinition::new(
            TrainingMetricId::default(),
            UserId::test_default(),
            ActivityMetricSource::Timeseries((
                TimeseriesMetric::Power,
                TimeseriesAggregate::Average,
            )),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Max,
        );

        let metrics = metric_definition.compute_values(&activities);

        assert_eq!(
            *metrics.get("2025-09-01").unwrap(),
            TrainingMetricValue::Max(20.)
        );
    }

    #[test]
    fn test_granularity_bins_activity_is_none() {
        let start = "2025-07-23T12:03:00+02:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        let end = "2025-09-09T12:03:00+10:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        let granularity = TrainingMetricGranularity::Activity;

        assert!(granularity.bins(&start, &end).is_none())
    }

    #[test]
    fn test_granularity_bins_daily() {
        let start = "2025-09-03T12:03:00+02:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        let end = "2025-09-06T12:03:00+10:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        let granularity = TrainingMetricGranularity::Daily;

        let res = granularity.bins(&start, &end);

        assert!(res.is_some());
        assert_eq!(
            res.unwrap(),
            vec![
                "2025-09-03".to_string(),
                "2025-09-04".to_string(),
                "2025-09-05".to_string(),
                "2025-09-06".to_string(),
            ]
        )
    }

    #[test]
    fn test_granularity_bins_weekly() {
        let start = "2025-08-23T12:03:00+02:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        let end = "2025-09-09T12:03:00+10:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        let granularity = TrainingMetricGranularity::Weekly;

        let res = granularity.bins(&start, &end);

        assert!(res.is_some());
        assert_eq!(
            res.unwrap(),
            vec![
                "2025-08-18".to_string(),
                "2025-08-25".to_string(),
                "2025-09-01".to_string(),
                "2025-09-08".to_string(),
            ]
        )
    }

    #[test]
    fn test_granularity_bins_monthly() {
        let start = "2025-07-23T12:03:00+02:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        let end = "2025-09-09T12:03:00+10:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        let granularity = TrainingMetricGranularity::Monthly;

        let res = granularity.bins(&start, &end);

        assert!(res.is_some());
        assert_eq!(
            res.unwrap(),
            vec![
                "2025-07-01".to_string(),
                "2025-08-01".to_string(),
                "2025-09-01".to_string(),
            ]
        )
    }
}

#[cfg(test)]
mod test_training_metric_definition_extract_granule {
    use crate::domain::models::activity::{
        Activity, ActivityId, ActivityStartTime, ActivityStatistics, ActivityTimeseries, Sport,
        Timeseries, TimeseriesTime, TimeseriesValue,
    };

    use super::*;

    fn default_activity() -> ActivityWithTimeseries {
        ActivityWithTimeseries::new(
            Activity::new(
                ActivityId::default(),
                UserId::test_default(),
                None,
                ActivityStartTime::new(
                    "2025-09-03T00:00:00Z"
                        .parse::<DateTime<FixedOffset>>()
                        .unwrap(),
                ),
                Sport::Cycling,
                ActivityStatistics::default(),
            ),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![0, 1, 2]),
                vec![Timeseries::new(
                    TimeseriesMetric::Power,
                    vec![
                        Some(TimeseriesValue::Int(10)),
                        Some(TimeseriesValue::Int(20)),
                        Some(TimeseriesValue::Int(30)),
                    ],
                )],
            ),
        )
    }

    #[test]
    fn test_granularity_datetime_granule() {
        let activity = default_activity();

        assert_eq!(
            TrainingMetricGranularity::Activity.granule(&activity),
            Some(Granule::Activity(activity.id().clone()))
        );

        assert_eq!(
            TrainingMetricGranularity::Daily
                .granule(&activity)
                .expect("Shoule be Some"),
            Granule::Date(DateGranule::new(
                NaiveDate::from_ymd_opt(2025, 09, 3).unwrap(),
                NaiveDate::from_ymd_opt(2025, 09, 4).unwrap(),
            ))
        );

        assert_eq!(
            TrainingMetricGranularity::Weekly
                .granule(&activity)
                .expect("Should be Some"),
            Granule::Date(DateGranule::new(
                NaiveDate::from_ymd_opt(2025, 09, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 09, 7).unwrap(),
            ))
        );

        assert_eq!(
            TrainingMetricGranularity::Monthly
                .granule(&activity)
                .expect("Should be Some"),
            Granule::Date(DateGranule::new(
                NaiveDate::from_ymd_opt(2025, 09, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 09, 30).unwrap(),
            ))
        );
    }
}

#[cfg(test)]
mod test_training_metric_aggregate_update_value {

    use super::*;

    use assert_approx_eq::assert_approx_eq;

    #[test]
    fn test_update_min_value() {
        let aggregate = TrainingMetricAggregate::Min;
        let previous = TrainingMetricValue::Min(12.2);
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.update_value(&previous, &new_metric),
            Some(TrainingMetricValue::Min(10.1))
        );
    }

    #[test]
    fn test_do_not_update_min_value() {
        let aggregate = TrainingMetricAggregate::Min;
        let previous = TrainingMetricValue::Min(12.2);
        let new_metric = ActivityMetric::new(
            13.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.update_value(&previous, &new_metric),
            Some(TrainingMetricValue::Min(12.2))
        );
    }

    #[test]
    fn test_update_min_value_wrong_previous_value_variant() {
        let previous_values_wrong_variant = vec![
            TrainingMetricValue::Max(12.),
            TrainingMetricValue::Sum(12.),
            TrainingMetricValue::Average {
                value: 12.,
                sum: 12.,
                number_of_elements: 2,
            },
        ];
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        let aggregate = TrainingMetricAggregate::Min;

        for previous in previous_values_wrong_variant {
            assert!(aggregate.update_value(&previous, &new_metric).is_none());
        }
    }

    #[test]
    fn test_update_max_value() {
        let aggregate = TrainingMetricAggregate::Max;
        let previous = TrainingMetricValue::Max(12.2);
        let new_metric = ActivityMetric::new(
            13.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.update_value(&previous, &new_metric),
            Some(TrainingMetricValue::Max(13.1))
        );
    }

    #[test]
    fn test_do_not_update_max_value() {
        let aggregate = TrainingMetricAggregate::Max;
        let previous = TrainingMetricValue::Max(12.2);
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.update_value(&previous, &new_metric),
            Some(TrainingMetricValue::Max(12.2))
        );
    }

    #[test]
    fn test_update_max_value_wrong_previous_value_variant() {
        let aggregate = TrainingMetricAggregate::Max;
        let previous_values_wrong_variant = vec![
            TrainingMetricValue::Min(12.),
            TrainingMetricValue::Sum(12.),
            TrainingMetricValue::Average {
                value: 12.,
                sum: 12.,
                number_of_elements: 2,
            },
        ];
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        for previous in previous_values_wrong_variant {
            assert!(aggregate.update_value(&previous, &new_metric).is_none());
        }
    }

    #[test]
    fn test_update_sum_value() {
        let aggregate = TrainingMetricAggregate::Sum;
        let previous = TrainingMetricValue::Sum(12.2);
        let new_metric = ActivityMetric::new(
            13.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        let Some(TrainingMetricValue::Sum(sum)) = aggregate.update_value(&previous, &new_metric)
        else {
            unreachable!("Should have returned Some(TrainingMetricValue::Sum(sum))");
        };
        assert_approx_eq!(sum, 25.3);
    }

    #[test]
    fn test_update_sum_value_wrong_previous_value_variant() {
        let aggregate = TrainingMetricAggregate::Sum;
        let previous_values_wrong_variant = vec![
            TrainingMetricValue::Min(12.),
            TrainingMetricValue::Max(12.),
            TrainingMetricValue::Average {
                value: 12.,
                sum: 12.,
                number_of_elements: 2,
            },
        ];
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        for previous in previous_values_wrong_variant {
            assert!(aggregate.update_value(&previous, &new_metric).is_none());
        }
    }

    #[test]
    fn test_update_average_value() {
        let aggregate = TrainingMetricAggregate::Average;
        let previous = TrainingMetricValue::Average {
            value: 12.,
            sum: 12.,
            number_of_elements: 2,
        };
        let new_metric = ActivityMetric::new(
            13.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        let Some(TrainingMetricValue::Average {
            sum,
            value,
            number_of_elements,
        }) = aggregate.update_value(&previous, &new_metric)
        else {
            unreachable!("Should have returned Some(TrainingMetricValue::Average)");
        };
        assert_approx_eq!(sum, 25.1);
        assert_approx_eq!(value, 25.1 / 3.);
        assert_eq!(number_of_elements, 3);
    }

    #[test]
    fn test_update_average_value_wrong_previous_value_variant() {
        let aggregate = TrainingMetricAggregate::Average;
        let previous_values_wrong_variant = vec![
            TrainingMetricValue::Min(12.),
            TrainingMetricValue::Max(12.),
            TrainingMetricValue::Sum(12.),
        ];
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        for previous in previous_values_wrong_variant {
            assert!(aggregate.update_value(&previous, &new_metric).is_none());
        }
    }
}
