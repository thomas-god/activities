use std::collections::HashMap;

use chrono::{DateTime, Datelike, FixedOffset, SecondsFormat};
use derive_more::{AsRef, Constructor, Deref, Display};
use uuid::Uuid;

use crate::domain::models::activity::{Activity, ActivityStatistic, TimeseriesMetric};

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, AsRef, Deref, Hash)]
pub struct TrainingMetricId(String);

impl TrainingMetricId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from(id: &str) -> Self {
        Self(id.to_string())
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
    source: TrainingMetricSource,
    granularity: TrainingMetricGranularity,
    granularity_aggregate: TrainingMetricAggregate,
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
        .map(|value| (**activity.start_time(), value))
    }
}

#[derive(Debug, Clone)]
pub enum MetricValueComputationError {}

impl TrainingMetricDefinition {
    pub fn compute_values(
        &self,
        activities: &[Activity],
    ) -> Result<HashMap<String, f64>, MetricValueComputationError> {
        let metrics_per_activity = activities
            .iter()
            .filter_map(|activity| self.source.extract_from_activity(activity))
            .collect();
        let grouped_metrics = group_metrics_by_granularity(&self.granularity, metrics_per_activity);
        Ok(aggregate_metrics(
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

#[derive(Debug, Clone, PartialEq)]
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
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Constructor, Deref, Default)]
pub struct TrainingMetricValues(Vec<(String, f64)>);

impl TrainingMetricValues {
    pub fn append(self, new_value: (String, f64)) -> Self {
        let mut values = self.0;
        values.push(new_value);

        Self(values)
    }

    pub fn push(&mut self, new_value: (String, f64)) {
        self.0.push(new_value);
    }
}

#[cfg(test)]
mod test_training_metrics {

    use crate::domain::models::activity::{
        Activity, ActivityDuration, ActivityId, ActivityStartTime, ActivityStatistics,
        ActivityTimeseries, Sport, Timeseries, TimeseriesTime, TimeseriesValue,
    };

    use super::*;

    #[test]
    fn test_append_new_value_to_existing_metric() {
        let metric = TrainingMetricValues::new(vec![]);
        let new_value = ("2025-09-03T00:00:00Z".to_string(), 12.3);

        let updated_metric = metric.append(new_value);

        assert_eq!(
            *updated_metric,
            vec![("2025-09-03T00:00:00Z".to_string(), 12.3,)]
        );
    }

    #[test]
    fn test_push_new_value_to_existing_metric() {
        let mut metric = TrainingMetricValues::new(vec![]);
        let new_value = ("2025-09-03T00:00:00Z".to_string(), 12.3);

        metric.push(new_value);

        assert_eq!(*metric, vec![("2025-09-03T00:00:00Z".to_string(), 12.3,)]);
    }

    fn default_activity() -> Activity {
        Activity::new(
            ActivityId::default(),
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            ActivityDuration::new(3),
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
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            ActivityDuration::new(1),
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
            TrainingMetricSource::Timeseries((
                TimeseriesMetric::Power,
                TrainingMetricAggregate::Average,
            )),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Max,
        );

        let metrics = metric_definition.compute_values(&activities);

        assert!(metrics.is_ok());
        assert_eq!(*metrics.unwrap().get("2025-09-01").unwrap(), 20.);
    }
}
