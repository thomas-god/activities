use std::{
    collections::{HashMap, hash_map::Iter},
    fmt,
};

use chrono::{DateTime, Datelike, Days, FixedOffset, Months, NaiveDate, SecondsFormat};
use derive_more::{AsRef, Constructor, Display};
use uuid::Uuid;

use crate::domain::models::{
    UserId,
    activity::{Activity, ActivityStatistic, TimeseriesMetric, ToUnit, Unit},
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
    source: TrainingMetricSource,
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

    pub fn source(&self) -> &TrainingMetricSource {
        &self.source
    }

    pub fn granularity(&self) -> &TrainingMetricGranularity {
        &self.granularity
    }

    pub fn aggregate(&self) -> &TrainingMetricAggregate {
        &self.granularity_aggregate
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrainingMetricSource {
    Statistic(ActivityStatistic),
    Timeseries((TimeseriesMetric, TrainingMetricAggregate)),
}

impl TrainingMetricSource {
    pub fn extract_from_activity(
        &self,
        activity: &Activity,
    ) -> Option<(DateTime<FixedOffset>, f64)> {
        match self {
            Self::Statistic(statistic) => activity.statistics().get(statistic).cloned(),
            Self::Timeseries((metric, aggregate)) => {
                extract_aggregated_activity_metric(aggregate, metric, activity)
            }
        }
        .map(|value| (*activity.start_time().date(), value))
    }
}

impl ToUnit for TrainingMetricSource {
    fn unit(&self) -> Unit {
        match self {
            Self::Statistic(stat) => stat.unit(),
            Self::Timeseries((metric, _)) => metric.unit(),
        }
    }
}

impl TrainingMetricDefinition {
    pub fn compute_values(&self, activities: &[Activity]) -> TrainingMetricValues {
        let metrics_per_activity = activities
            .iter()
            .filter_map(|activity| self.source.extract_from_activity(activity))
            .collect();
        let grouped_metrics = group_metrics_by_granularity(&self.granularity, metrics_per_activity);
        TrainingMetricValues(aggregate_metrics(
            &self.granularity_aggregate,
            grouped_metrics,
        ))
    }
}

fn extract_aggregated_activity_metric(
    aggregate: &TrainingMetricAggregate,
    metric: &TimeseriesMetric,
    activity: &Activity,
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
    aggregate.aggregate(values)
}

fn group_metrics_by_granularity(
    granularity: &TrainingMetricGranularity,
    metrics: Vec<(DateTime<FixedOffset>, f64)>,
) -> HashMap<String, Vec<f64>> {
    let mut grouped_values: HashMap<String, Vec<f64>> = HashMap::new();
    for (date, value) in metrics {
        let key = granularity.datetime_key(&date);
        grouped_values.entry(key).or_default().push(value);
    }
    grouped_values
}

fn aggregate_metrics(
    aggregate: &TrainingMetricAggregate,
    metrics: HashMap<String, Vec<f64>>,
) -> HashMap<String, f64> {
    let mut res = HashMap::new();

    for (key, values) in metrics.into_iter() {
        let Some(aggregated_value) = aggregate.aggregate(values) else {
            continue;
        };
        res.insert(key, aggregated_value);
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

#[derive(Debug, Clone, PartialEq, Display)]
pub enum TrainingMetricAggregate {
    Min,
    Max,
    Average,
    Sum,
}

impl TrainingMetricAggregate {
    fn aggregate(&self, values: Vec<f64>) -> Option<f64> {
        if values.is_empty() {
            return None;
        }
        let length = values.len();
        match self {
            TrainingMetricAggregate::Min => values.into_iter().reduce(f64::min),
            TrainingMetricAggregate::Max => values.into_iter().reduce(f64::max),
            TrainingMetricAggregate::Average => values
                .into_iter()
                .reduce(|acc, e| acc + e)
                .map(|val| val / length as f64),
            TrainingMetricAggregate::Sum => values.into_iter().reduce(|acc, e| acc + e),
        }
    }
}

#[derive(Debug, Clone, Constructor, Default)]
pub struct TrainingMetricValues(HashMap<String, f64>);

impl TrainingMetricValues {
    pub fn insert(&mut self, key: String, value: f64) -> Option<f64> {
        self.0.insert(key, value)
    }

    pub fn get(&self, key: &str) -> Option<&f64> {
        self.0.get(key)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> Iter<'_, String, f64> {
        self.0.iter()
    }
}

impl TrainingMetricValues {
    pub fn as_hash_map(self) -> HashMap<String, f64> {
        self.0
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

    fn default_activity() -> Activity {
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
        let aggregate = TrainingMetricAggregate::Min;
        let activity = default_activity();

        let res = extract_aggregated_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_aggregated_activity_metric_metric_is_empty() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TrainingMetricAggregate::Average;
        let activity = Activity::new(
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
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![]),
                vec![Timeseries::new(TimeseriesMetric::Power, vec![])],
            ),
        );

        let res = extract_aggregated_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_aggregated_activity_metric_min_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TrainingMetricAggregate::Min;
        let activity = default_activity();

        let res = extract_aggregated_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 10.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_max_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TrainingMetricAggregate::Max;
        let activity = default_activity();

        let res = extract_aggregated_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 30.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_average_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TrainingMetricAggregate::Average;
        let activity = default_activity();

        let res = extract_aggregated_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 20.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_total_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TrainingMetricAggregate::Sum;
        let activity = default_activity();

        let res = extract_aggregated_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 60.)
    }

    #[test]
    fn test_group_metric_by_granularity_activity() {
        let metrics = vec![
            (
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                12.3,
            ),
            (
                "2025-09-03T02:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                18.1,
            ),
        ];
        let granularity = TrainingMetricGranularity::Activity;

        let res = group_metrics_by_granularity(&granularity, metrics);
        assert_eq!(res.len(), 2);
        assert_eq!(res.get("2025-09-03T00:00:00Z").unwrap(), &vec![12.3]);
        assert_eq!(res.get("2025-09-03T02:00:00Z").unwrap(), &vec![18.1]);
    }

    #[test]
    fn test_group_metric_by_granularity_daily() {
        let metrics = vec![
            (
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                12.3,
            ),
            (
                "2025-09-03T02:00:00+03:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                18.1,
            ),
            (
                "2025-09-04T02:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                67.1,
            ),
        ];
        let granularity = TrainingMetricGranularity::Daily;

        let res = group_metrics_by_granularity(&granularity, metrics);
        assert_eq!(res.len(), 2);
        assert_eq!(res.get("2025-09-03").unwrap(), &vec![12.3, 18.1]);
        assert_eq!(res.get("2025-09-04").unwrap(), &vec![67.1]);
    }

    #[test]
    fn test_group_metric_by_granularity_weekly() {
        let metrics = vec![
            (
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                12.3,
            ),
            (
                "2025-09-05T02:00:00+03:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                18.1,
            ),
            (
                "2025-09-14T02:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                67.1,
            ),
        ];
        let granularity = TrainingMetricGranularity::Weekly;

        let res = group_metrics_by_granularity(&granularity, metrics);
        assert_eq!(res.len(), 2);
        assert_eq!(res.get("2025-09-01").unwrap(), &vec![12.3, 18.1]);
        assert_eq!(res.get("2025-09-08").unwrap(), &vec![67.1]);
    }

    #[test]
    fn test_group_metric_by_granularity_monthly() {
        let metrics = vec![
            (
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                12.3,
            ),
            (
                "2025-09-05T02:00:00+03:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                18.1,
            ),
            (
                "2025-08-14T02:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                67.1,
            ),
        ];
        let granularity = TrainingMetricGranularity::Monthly;

        let res = group_metrics_by_granularity(&granularity, metrics);
        assert_eq!(res.len(), 2);
        assert_eq!(res.get("2025-09-01").unwrap(), &vec![12.3, 18.1]);
        assert_eq!(res.get("2025-08-01").unwrap(), &vec![67.1]);
    }

    #[test]
    fn test_aggregate_metrics_min() {
        let metrics = HashMap::from([("2025-09-01".to_string(), vec![1., 2., 3.])]);
        let aggregate = TrainingMetricAggregate::Min;

        let res = aggregate_metrics(&aggregate, metrics);

        assert_eq!(*res.get("2025-09-01").unwrap(), 1.);
    }

    #[test]
    fn test_compute_training_metrics() {
        let activities = vec![default_activity()];
        let metric_definition = TrainingMetricDefinition::new(
            TrainingMetricId::default(),
            UserId::test_default(),
            TrainingMetricSource::Timeseries((
                TimeseriesMetric::Power,
                TrainingMetricAggregate::Average,
            )),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Max,
        );

        let metrics = metric_definition.compute_values(&activities);

        assert_eq!(*metrics.get("2025-09-01").unwrap(), 20.);
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
