use chrono::{DateTime, FixedOffset};
use derive_more::{AsRef, Constructor, Deref, Display};
use uuid::Uuid;

use crate::domain::models::activity::Metric;

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
}
