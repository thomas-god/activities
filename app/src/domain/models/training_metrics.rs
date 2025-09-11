use std::collections::HashMap;

use chrono::{DateTime, Datelike, FixedOffset, SecondsFormat};
use derive_more::{AsRef, Constructor, Deref, Display};
use uuid::Uuid;

use crate::domain::models::activity::{Activity, Metric};

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
    activity_metric: Metric,
    activity_metric_aggregate: TrainingMetricAggregate,
    granularity: TrainingMetricGranularity,
    granularity_aggregate: TrainingMetricAggregate,
}

#[derive(Debug, Clone)]
pub enum MetricValueComputationError {}

impl TrainingMetricDefinition {
    pub fn compute_values(
        &self,
        activities: &[Activity],
    ) -> Result<HashMap<String, f64>, MetricValueComputationError> {
        Ok(aggregate_metrics(
            &self.granularity_aggregate,
            group_metrics_by_granularity(
                &self.granularity,
                activities
                    .iter()
                    .filter_map(|activity| {
                        extract_aggregated_activity_metric(
                            &self.activity_metric_aggregate,
                            &self.activity_metric,
                            activity,
                        )
                        .map(|val| (**activity.start_time(), val))
                    })
                    .collect(),
            ),
        ))
    }
}

fn extract_aggregated_activity_metric(
    aggregate: &TrainingMetricAggregate,
    metric: &Metric,
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
    let key_lambda = match granularity {
        TrainingMetricGranularity::Activity => {
            |dt: DateTime<FixedOffset>| dt.to_rfc3339_opts(SecondsFormat::Secs, true)
        }
        TrainingMetricGranularity::Daily => |dt: DateTime<FixedOffset>| dt.date_naive().to_string(),
        TrainingMetricGranularity::Weekly => |dt: DateTime<FixedOffset>| {
            dt.date_naive()
                .week(chrono::Weekday::Mon)
                .first_day()
                .to_string()
        },
        TrainingMetricGranularity::Monthly => {
            |dt: DateTime<FixedOffset>| dt.date_naive().with_day(1).unwrap().to_string()
        }
    };
    let mut grouped_value: HashMap<String, Vec<f64>> = HashMap::new();
    for (date, value) in metrics {
        let key = key_lambda(date);
        grouped_value.entry(key).or_default().push(value);
    }
    grouped_value
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

#[derive(Debug, Clone, Constructor)]
pub struct TrainingMetricValues {
    granularity: TrainingMetricGranularity,
    values: Vec<(DateTime<FixedOffset>, f64)>,
}

impl TrainingMetricValues {
    pub fn append(self, new_value: (DateTime<FixedOffset>, f64)) -> Self {
        let mut values = self.values;
        values.push(new_value);

        Self {
            granularity: self.granularity,
            values,
        }
    }

    pub fn granularity(&self) -> &TrainingMetricGranularity {
        &self.granularity
    }

    pub fn values(&self) -> &[(DateTime<FixedOffset>, f64)] {
        &self.values
    }
}

#[cfg(test)]
mod test_training_metrics {

    use crate::domain::models::activity::{
        Activity, ActivityDuration, ActivityId, ActivityStartTime, Sport, Timeseries,
        TimeseriesMetric, TimeseriesTime, TimeseriesValue,
    };

    use super::*;

    #[test]
    fn test_append_new_value_to_existing_metric() {
        let metric = TrainingMetricValues::new(TrainingMetricGranularity::Weekly, vec![]);
        let new_value = (
            "2025-09-03T00:00:00Z"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
            12.3,
        );

        let updated_metric = metric.append(new_value);

        assert_eq!(
            updated_metric.values(),
            vec![(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
                12.3,
            )]
        );
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
            Timeseries::new(
                TimeseriesTime::new(vec![0, 1, 2]),
                vec![TimeseriesMetric::new(
                    Metric::Power,
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
        let metric = Metric::Speed;
        let aggregate = TrainingMetricAggregate::Min;
        let activity = default_activity();

        let res = extract_aggregated_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_aggregated_activity_metric_metric_is_empty() {
        let metric = Metric::Power;
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
            Timeseries::new(
                TimeseriesTime::new(vec![]),
                vec![TimeseriesMetric::new(Metric::Power, vec![])],
            ),
        );

        let res = extract_aggregated_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_aggregated_activity_metric_min_value() {
        let metric = Metric::Power;
        let aggregate = TrainingMetricAggregate::Min;
        let activity = default_activity();

        let res = extract_aggregated_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 10.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_max_value() {
        let metric = Metric::Power;
        let aggregate = TrainingMetricAggregate::Max;
        let activity = default_activity();

        let res = extract_aggregated_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 30.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_average_value() {
        let metric = Metric::Power;
        let aggregate = TrainingMetricAggregate::Average;
        let activity = default_activity();

        let res = extract_aggregated_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 20.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_total_value() {
        let metric = Metric::Power;
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
        dbg!(&res);
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
            Metric::Power,
            TrainingMetricAggregate::Average,
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Max,
        );

        let metrics = metric_definition.compute_values(&activities);

        assert!(metrics.is_ok());
        dbg!(&metrics);
        assert_eq!(*metrics.unwrap().get("2025-09-01").unwrap(), 20.);
    }
}
