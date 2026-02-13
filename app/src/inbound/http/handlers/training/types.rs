/// Mappings between domain types and types part of the HTTP API
use serde::{Deserialize, Serialize};

use crate::domain::models::{
    activity::{ActivityStatistic, Sport, TimeseriesMetric},
    training::{
        ActivityMetricSource, SportFilter, TimeseriesAggregate, TrainingMetricAggregate,
        TrainingMetricFilters, TrainingMetricGranularity, TrainingMetricGroupBy,
        TrainingMetricScope, TrainingPeriodId, TrainingPeriodSports,
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
    Pace,
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
            APITimeseriesMetric::Pace => Self::Pace,
            APITimeseriesMetric::Power => Self::Power,
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
    NumberOfActivities,
}

impl From<APITrainingMetricAggregate> for TrainingMetricAggregate {
    fn from(value: APITrainingMetricAggregate) -> Self {
        match value {
            APITrainingMetricAggregate::Min => Self::Min,
            APITrainingMetricAggregate::Max => Self::Max,
            APITrainingMetricAggregate::Average => Self::Average,
            APITrainingMetricAggregate::Sum => Self::Sum,
            APITrainingMetricAggregate::NumberOfActivities => Self::NumberOfActivities,
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

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct APITrainingMetricFilters {
    sports: Option<Vec<SportFilter>>,
}

impl From<APITrainingMetricFilters> for TrainingMetricFilters {
    fn from(value: APITrainingMetricFilters) -> Self {
        Self::new(value.sports)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum APITrainingMetricGroupBy {
    Sport,
    SportCategory,
    WorkoutType,
    RpeRange,
    Bonked,
}

impl From<APITrainingMetricGroupBy> for TrainingMetricGroupBy {
    fn from(value: APITrainingMetricGroupBy) -> Self {
        match value {
            APITrainingMetricGroupBy::Sport => Self::Sport,
            APITrainingMetricGroupBy::SportCategory => Self::SportCategory,
            APITrainingMetricGroupBy::WorkoutType => Self::WorkoutType,
            APITrainingMetricGroupBy::RpeRange => Self::RpeRange,
            APITrainingMetricGroupBy::Bonked => Self::Bonked,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct APITrainingPeriodSports(Option<Vec<SportFilter>>);

impl From<APITrainingPeriodSports> for TrainingPeriodSports {
    fn from(value: APITrainingPeriodSports) -> Self {
        Self::new(value.0)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ScopePayload {
    Global,
    #[serde(rename_all = "camelCase")]
    TrainingPeriod {
        training_period_id: String,
    },
}

impl From<ScopePayload> for TrainingMetricScope {
    fn from(payload: ScopePayload) -> Self {
        match payload {
            ScopePayload::Global => TrainingMetricScope::Global,
            ScopePayload::TrainingPeriod { training_period_id } => {
                TrainingMetricScope::TrainingPeriod(TrainingPeriodId::from(&training_period_id))
            }
        }
    }
}

impl From<&TrainingMetricScope> for ScopePayload {
    fn from(value: &TrainingMetricScope) -> Self {
        match value {
            TrainingMetricScope::Global => ScopePayload::Global,
            TrainingMetricScope::TrainingPeriod(period) => ScopePayload::TrainingPeriod {
                training_period_id: period.to_string(),
            },
        }
    }
}
