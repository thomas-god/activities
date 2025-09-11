use chrono::{DateTime, FixedOffset};
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

pub fn aggregate_activity_metric(
    aggregate: &TrainingMetricAggregate,
    metric: &Metric,
    activity: &Activity,
) -> Option<f64> {
    let values: Vec<f64> = activity.timeseries().metrics().iter().find_map(|m| {
        if m.metric() == metric {
            Some(
                m.values()
                    .iter()
                    .filter_map(|val| val.as_ref().and_then(|v| Some(f64::from(v))))
                    .collect(),
            )
        } else {
            None
        }
    })?;

    let length = values.len();
    if length == 0 {
        return None;
    }

    match aggregate {
        TrainingMetricAggregate::Min => values.into_iter().reduce(|a, b| f64::min(a, b)),
        TrainingMetricAggregate::Max => values.into_iter().reduce(|a, b| f64::max(a, b)),
        TrainingMetricAggregate::Average => values
            .into_iter()
            .reduce(|acc, e| acc + e)
            .and_then(|val| Some(val / length as f64)),
        TrainingMetricAggregate::Sum => values.into_iter().reduce(|acc, e| acc + e),
    }
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
    fn test_compute_aggregate_on_activity_no_metric_found() {
        let metric = Metric::Speed;
        let aggregate = TrainingMetricAggregate::Min;
        let activity = default_activity();

        let res = aggregate_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_compute_aggregate_on_activity_metric_is_empty() {
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

        let res = aggregate_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_compute_aggregate_on_activity_min_value() {
        let metric = Metric::Power;
        let aggregate = TrainingMetricAggregate::Min;
        let activity = default_activity();

        let res = aggregate_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 10.)
    }

    #[test]
    fn test_compute_aggregate_on_activity_max_value() {
        let metric = Metric::Power;
        let aggregate = TrainingMetricAggregate::Max;
        let activity = default_activity();

        let res = aggregate_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 30.)
    }

    #[test]
    fn test_compute_aggregate_on_activity_average_value() {
        let metric = Metric::Power;
        let aggregate = TrainingMetricAggregate::Average;
        let activity = default_activity();

        let res = aggregate_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 20.)
    }

    #[test]
    fn test_compute_aggregate_on_activity_total_value() {
        let metric = Metric::Power;
        let aggregate = TrainingMetricAggregate::Sum;
        let activity = default_activity();

        let res = aggregate_activity_metric(&aggregate, &metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 60.)
    }
}
