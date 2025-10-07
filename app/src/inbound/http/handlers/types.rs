/// Mappings between domain types and types part of the HTTP API
use serde::Deserialize;

use crate::domain::models::{
    activity::{ActivityStatistic, TimeseriesMetric},
    training_metrics::{
        ActivityMetricSource, TimeseriesAggregate, TrainingMetricAggregate,
        TrainingMetricGranularity,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum APIActivityStatistic {
    Calories,
    Elevation,
    Distance,
    Duration,
    NormalizedPower,
}

impl From<APIActivityStatistic> for ActivityStatistic {
    fn from(value: APIActivityStatistic) -> Self {
        match value {
            APIActivityStatistic::Calories => Self::Calories,
            APIActivityStatistic::Elevation => Self::Elevation,
            APIActivityStatistic::Distance => Self::Distance,
            APIActivityStatistic::Duration => Self::Duration,
            APIActivityStatistic::NormalizedPower => Self::NormalizedPower,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum APITimeseriesMetric {
    Speed,
    Power,
    HeartRate,
    Distance,
    Altitude,
    Cadence,
}

impl From<APITimeseriesMetric> for TimeseriesMetric {
    fn from(value: APITimeseriesMetric) -> Self {
        match value {
            APITimeseriesMetric::Speed => Self::Speed,
            APITimeseriesMetric::Power => Self::Distance,
            APITimeseriesMetric::HeartRate => Self::HeartRate,
            APITimeseriesMetric::Distance => Self::Distance,
            APITimeseriesMetric::Altitude => Self::Altitude,
            APITimeseriesMetric::Cadence => Self::Cadence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum APITrainingMetricSource {
    Statistic(APIActivityStatistic),
    Timeseries((APITimeseriesMetric, APITimeseriesAggregate)),
}

impl From<APITrainingMetricSource> for ActivityMetricSource {
    fn from(value: APITrainingMetricSource) -> Self {
        match value {
            APITrainingMetricSource::Statistic(stat) => {
                ActivityMetricSource::Statistic(ActivityStatistic::from(stat))
            }
            APITrainingMetricSource::Timeseries((metric, aggregate)) => {
                ActivityMetricSource::Timeseries((
                    TimeseriesMetric::from(metric),
                    TimeseriesAggregate::from(aggregate),
                ))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum APITimeseriesAggregate {
    Min,
    Max,
    Average,
    Sum,
}

impl From<APITimeseriesAggregate> for TimeseriesAggregate {
    fn from(value: APITimeseriesAggregate) -> Self {
        match value {
            APITimeseriesAggregate::Min => Self::Min,
            APITimeseriesAggregate::Max => Self::Max,
            APITimeseriesAggregate::Average => Self::Average,
            APITimeseriesAggregate::Sum => Self::Sum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum APITrainingMetricAggregate {
    Min,
    Max,
    Average,
    Sum,
}

impl From<APITrainingMetricAggregate> for TrainingMetricAggregate {
    fn from(value: APITrainingMetricAggregate) -> Self {
        match value {
            APITrainingMetricAggregate::Min => Self::Min,
            APITrainingMetricAggregate::Max => Self::Max,
            APITrainingMetricAggregate::Average => Self::Average,
            APITrainingMetricAggregate::Sum => Self::Sum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum APITrainingMetricGranularity {
    Daily,
    Weekly,
    Monthly,
}

impl From<APITrainingMetricGranularity> for TrainingMetricGranularity {
    fn from(value: APITrainingMetricGranularity) -> Self {
        match value {
            APITrainingMetricGranularity::Daily => Self::Daily,
            APITrainingMetricGranularity::Weekly => Self::Weekly,
            APITrainingMetricGranularity::Monthly => Self::Monthly,
        }
    }
}
