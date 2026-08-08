use std::{
    collections::HashMap,
    fmt::{Display, Write},
    str::FromStr,
};

use derive_more::Constructor;
/// Mappings between domain types and types part of the HTTP API
use serde::{Deserialize, Serialize};

use crate::{
    domain::models::{
        activity::{
            ActivityMetricSource, ActivityRpe, ActivityStatistic, BonkStatus, Sport,
            TimeseriesAggregate, TimeseriesMetric, Unit, WorkoutType,
        },
        training::{
            SportFilter, TrainingMetricAggregate, TrainingMetricFilters, TrainingMetricGranularity,
            TrainingMetricGroupBy, TrainingMetricScope, TrainingMetricSummary,
            TrainingMetricSummaryAverage, TrainingMetricTarget, TrainingMetricWindow,
            TrainingPeriodId, TrainingPeriodSports,
        },
    },
    inbound::http::handlers::training::utils::GranuleValues,
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

#[derive(Debug, Clone, PartialEq, Constructor, Deserialize, Default)]
pub struct APITrainingMetricSummary {
    pub average: Option<APITrainingMetricSummaryAverage>,
}

#[derive(Debug, Clone, PartialEq, Constructor, Deserialize)]
pub struct APITrainingMetricSummaryAverage {
    pub include_zeros: bool,
}
impl From<&APITrainingMetricSummaryAverage> for TrainingMetricSummaryAverage {
    fn from(value: &APITrainingMetricSummaryAverage) -> Self {
        Self::new(value.include_zeros)
    }
}

impl From<APITrainingMetricSummaryAverage> for TrainingMetricSummaryAverage {
    fn from(value: APITrainingMetricSummaryAverage) -> Self {
        Self::from(&value)
    }
}

impl From<APITrainingMetricSummary> for TrainingMetricSummary {
    fn from(value: APITrainingMetricSummary) -> Self {
        Self::from(&value)
    }
}

impl From<&APITrainingMetricSummary> for TrainingMetricSummary {
    fn from(value: &APITrainingMetricSummary) -> Self {
        Self::new(
            value
                .average
                .as_ref()
                .map(TrainingMetricSummaryAverage::from),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Constructor, Deserialize)]
pub struct APITrainingMetricTarget {
    value: f64,
    unit: String,
}

impl TryFrom<&APITrainingMetricTarget> for TrainingMetricTarget {
    type Error = String;

    fn try_from(value: &APITrainingMetricTarget) -> Result<Self, Self::Error> {
        let unit = Unit::from_str(&value.unit)?;
        Ok(TrainingMetricTarget::new(value.value, unit))
    }
}

impl TryFrom<APITrainingMetricTarget> for TrainingMetricTarget {
    type Error = String;

    fn try_from(value: APITrainingMetricTarget) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

#[derive(Debug, Clone, Constructor, PartialEq, Deserialize)]
pub struct APITimeseriesWindow {
    granularity: APITrainingMetricGranularity,
    aggregate: APITrainingMetricAggregate,
    group_by: Option<APITrainingMetricGroupBy>,
}

impl APITimeseriesWindow {
    pub fn granularity(&self) -> &APITrainingMetricGranularity {
        &self.granularity
    }
    pub fn aggregate(&self) -> &APITrainingMetricAggregate {
        &self.aggregate
    }
    pub fn group_by(&self) -> &Option<APITrainingMetricGroupBy> {
        &self.group_by
    }
}
impl From<&APITimeseriesWindow> for TrainingMetricWindow {
    fn from(value: &APITimeseriesWindow) -> Self {
        Self::new(
            TrainingMetricGranularity::from(&value.granularity),
            TrainingMetricAggregate::from(&value.aggregate),
            value.group_by.as_ref().map(TrainingMetricGroupBy::from),
        )
    }
}

impl From<APITimeseriesWindow> for TrainingMetricWindow {
    fn from(value: APITimeseriesWindow) -> Self {
        Self::from(&value)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum APITimeseriesAggregate {
    Min,
    Max,
    Average,
}

impl From<APITimeseriesAggregate> for TimeseriesAggregate {
    fn from(value: APITimeseriesAggregate) -> Self {
        match value {
            APITimeseriesAggregate::Min => Self::Min,
            APITimeseriesAggregate::Max => Self::Max,
            APITimeseriesAggregate::Average => Self::Average,
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

impl Display for APITrainingMetricAggregate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = match self {
            Self::Min => "Min",
            Self::Max => "Max",
            Self::Average => "Average",
            Self::Sum => "Sum",
            Self::NumberOfActivities => "NumberOfActivities",
        };
        f.write_str(d)
    }
}
impl From<&APITrainingMetricAggregate> for TrainingMetricAggregate {
    fn from(value: &APITrainingMetricAggregate) -> Self {
        match value {
            APITrainingMetricAggregate::Min => Self::Min,
            APITrainingMetricAggregate::Max => Self::Max,
            APITrainingMetricAggregate::Average => Self::Average,
            APITrainingMetricAggregate::Sum => Self::Sum,
            APITrainingMetricAggregate::NumberOfActivities => Self::NumberOfActivities,
        }
    }
}

impl From<APITrainingMetricAggregate> for TrainingMetricAggregate {
    fn from(value: APITrainingMetricAggregate) -> Self {
        Self::from(&value)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum APITrainingMetricGranularity {
    Daily,
    Weekly,
    Monthly,
}

impl From<&APITrainingMetricGranularity> for TrainingMetricGranularity {
    fn from(value: &APITrainingMetricGranularity) -> Self {
        match value {
            APITrainingMetricGranularity::Daily => Self::Daily,
            APITrainingMetricGranularity::Weekly => Self::Weekly,
            APITrainingMetricGranularity::Monthly => Self::Monthly,
        }
    }
}

impl From<APITrainingMetricGranularity> for TrainingMetricGranularity {
    fn from(value: APITrainingMetricGranularity) -> Self {
        Self::from(&value)
    }
}

impl Display for APITrainingMetricGranularity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = match self {
            Self::Daily => "Daily",
            Self::Weekly => "Weekly",
            Self::Monthly => "Monthly",
        };
        f.write_str(d)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct APITrainingMetricFilters {
    pub sports: Option<Vec<SportFilter>>,
    pub workout_types: Option<Vec<WorkoutType>>,
    pub bonked: Option<BonkStatus>,
    pub rpes: Option<Vec<u8>>,
}

impl TryFrom<&APITrainingMetricFilters> for TrainingMetricFilters {
    type Error = String;

    fn try_from(value: &APITrainingMetricFilters) -> Result<Self, Self::Error> {
        let rpes = value
            .rpes
            .as_ref()
            .map(|raw_rpes| {
                raw_rpes
                    .iter()
                    .map(|&raw_rpe| ActivityRpe::try_from(raw_rpe))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;

        Ok(Self::new(
            value.sports.clone(),
            value.workout_types.clone(),
            value.bonked.clone(),
            rpes,
        ))
    }
}

impl TryFrom<APITrainingMetricFilters> for TrainingMetricFilters {
    type Error = String;
    fn try_from(value: APITrainingMetricFilters) -> Result<Self, Self::Error> {
        Self::try_from(&value)
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

impl Display for APITrainingMetricGroupBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = match self {
            Self::Sport => "Sport",
            Self::SportCategory => "SportCategory",
            Self::WorkoutType => "WorkoutType",
            Self::RpeRange => "RpeRange",
            Self::Bonked => "Bonked",
        };
        f.write_str(d)
    }
}

impl From<&APITrainingMetricGroupBy> for TrainingMetricGroupBy {
    fn from(value: &APITrainingMetricGroupBy) -> Self {
        match value {
            APITrainingMetricGroupBy::Sport => Self::Sport,
            APITrainingMetricGroupBy::SportCategory => Self::SportCategory,
            APITrainingMetricGroupBy::WorkoutType => Self::WorkoutType,
            APITrainingMetricGroupBy::RpeRange => Self::RpeRange,
            APITrainingMetricGroupBy::Bonked => Self::Bonked,
        }
    }
}
impl From<APITrainingMetricGroupBy> for TrainingMetricGroupBy {
    fn from(value: APITrainingMetricGroupBy) -> Self {
        Self::from(&value)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct APITrainingPeriodSports(Option<Vec<SportFilter>>);

impl From<APITrainingPeriodSports> for TrainingPeriodSports {
    fn from(value: APITrainingPeriodSports) -> Self {
        Self::new(value.0)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum APITrainingMetricScope {
    Global,
    #[serde(rename_all = "camelCase")]
    TrainingPeriod {
        training_period_id: String,
    },
}

impl From<APITrainingMetricScope> for TrainingMetricScope {
    fn from(payload: APITrainingMetricScope) -> Self {
        match payload {
            APITrainingMetricScope::Global => TrainingMetricScope::Global,
            APITrainingMetricScope::TrainingPeriod { training_period_id } => {
                TrainingMetricScope::TrainingPeriod(TrainingPeriodId::from(&training_period_id))
            }
        }
    }
}

impl From<&TrainingMetricScope> for APITrainingMetricScope {
    fn from(value: &TrainingMetricScope) -> Self {
        match value {
            TrainingMetricScope::Global => APITrainingMetricScope::Global,
            TrainingMetricScope::TrainingPeriod(period) => APITrainingMetricScope::TrainingPeriod {
                training_period_id: period.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SportsResponse {
    pub categories: Vec<String>,
    pub sports: Vec<String>,
}

impl From<&TrainingPeriodSports> for SportsResponse {
    fn from(value: &TrainingPeriodSports) -> Self {
        let Some(items) = value.items() else {
            return Self {
                categories: vec![],
                sports: vec![],
            };
        };

        let mut sports = Vec::new();
        let mut categories = Vec::new();

        for sport in items {
            match sport {
                SportFilter::Sport(sport) => sports.push(sport.to_string()),
                SportFilter::SportCategory(category) => categories.push(category.to_string()),
            }
        }

        Self { categories, sports }
    }
}

impl From<&Option<Vec<SportFilter>>> for SportsResponse {
    fn from(value: &Option<Vec<SportFilter>>) -> Self {
        let Some(items) = value else {
            return Self {
                categories: vec![],
                sports: vec![],
            };
        };

        let mut sports = Vec::new();
        let mut categories = Vec::new();

        for sport in items {
            match sport {
                SportFilter::Sport(sport) => sports.push(sport.to_string()),
                SportFilter::SportCategory(category) => categories.push(category.to_string()),
            }
        }

        Self { categories, sports }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TrainingMetricBody {
    pub id: String,
    pub name: Option<String>,
    pub metric: String,
    pub metric_formated: String,
    pub unit: String,
    pub granularity: Option<String>,
    pub aggregate: Option<String>,
    pub sports: SportsResponse,
    pub workout_types: Option<Vec<String>>,
    pub bonked: Option<String>,
    pub rpes: Option<Vec<u8>>,
    pub show_average: Option<TrainingMetricSummaryAverage>,
    pub target: Option<TrainingMetricTarget>,
    pub values: HashMap<String, GranuleValues>,
    pub group_by: Option<String>,
    pub scope: APITrainingMetricScope,
    pub summary: HashMap<String, f64>,
}

pub fn format_source_metric(source: &ActivityMetricSource) -> String {
    match source {
        ActivityMetricSource::Statistic(stat) => stat.to_string(),
        ActivityMetricSource::Timeseries((metric, aggregate)) => {
            format!("Activity {aggregate:?} {metric:?}")
        }
        ActivityMetricSource::ActiveDuration => "ActiveDuration".into(),
        ActivityMetricSource::NumberOfActivities => "Number of activities".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::models::activity::{Sport, SportCategory, Unit};
    use crate::domain::models::training::{
        SportFilter, TrainingMetricTarget, TrainingPeriodSports,
    };

    #[test]
    fn test_api_target_conversion_ok() {
        let api = APITrainingMetricTarget::new(100.0, "km".to_string());
        let target: TrainingMetricTarget = api.try_into().unwrap();
        assert_eq!(target, TrainingMetricTarget::new(100.0, Unit::Kilometer));
    }

    #[test]
    fn test_api_target_conversion_invalid_unit_fails() {
        let api = APITrainingMetricTarget::new(100.0, "parsec".to_string());
        let result: Result<TrainingMetricTarget, _> = api.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_response_sports_from_training_period_sports_with_mixed_filters() {
        let sports = TrainingPeriodSports::new(Some(vec![
            SportFilter::Sport(Sport::Running),
            SportFilter::SportCategory(SportCategory::Cycling),
            SportFilter::Sport(Sport::Swimming),
            SportFilter::SportCategory(SportCategory::Climbing),
        ]));

        let response = SportsResponse::from(&sports);

        assert_eq!(
            response.sports,
            vec!["Running".to_string(), "Swimming".to_string()]
        );
        assert_eq!(
            response.categories,
            vec!["Cycling".to_string(), "Climbing".to_string()]
        );
    }

    #[test]
    fn test_response_sports_from_training_period_sports_only_sports() {
        let sports = TrainingPeriodSports::new(Some(vec![
            SportFilter::Sport(Sport::TrailRunning),
            SportFilter::Sport(Sport::IndoorCycling),
        ]));

        let response = SportsResponse::from(&sports);

        assert_eq!(
            response.sports,
            vec!["TrailRunning".to_string(), "IndoorCycling".to_string()]
        );
        assert!(response.categories.is_empty());
    }

    #[test]
    fn test_response_sports_from_training_period_sports_only_categories() {
        let sports = TrainingPeriodSports::new(Some(vec![
            SportFilter::SportCategory(SportCategory::Running),
            SportFilter::SportCategory(SportCategory::WaterSports),
        ]));

        let response = SportsResponse::from(&sports);

        assert!(response.sports.is_empty());
        assert_eq!(
            response.categories,
            vec!["Running".to_string(), "WaterSports".to_string()]
        );
    }

    #[test]
    fn test_response_sports_from_training_period_sports_none_is_empty() {
        let sports = TrainingPeriodSports::new(None);

        let response = SportsResponse::from(&sports);

        assert!(response.sports.is_empty());
        assert!(response.categories.is_empty());
    }

    #[test]
    fn test_response_sports_from_training_period_sports_empty_vec_is_empty() {
        let sports = TrainingPeriodSports::new(Some(vec![]));

        let response = SportsResponse::from(&sports);

        assert!(response.sports.is_empty());
        assert!(response.categories.is_empty());
    }

    #[test]
    fn test_response_sports_from_option_sports_with_mixed_filters() {
        let sports = Some(vec![
            SportFilter::Sport(Sport::Hiking),
            SportFilter::SportCategory(SportCategory::Racket),
            SportFilter::Sport(Sport::Kayaking),
        ]);

        let response = SportsResponse::from(&sports);

        assert_eq!(
            response.sports,
            vec!["Hiking".to_string(), "Kayaking".to_string()]
        );
        assert_eq!(response.categories, vec!["Racket".to_string()]);
    }

    #[test]
    fn test_response_sports_from_option_sports_none_is_empty() {
        let sports: Option<Vec<SportFilter>> = None;

        let response = SportsResponse::from(&sports);

        assert!(response.sports.is_empty());
        assert!(response.categories.is_empty());
    }

    #[test]
    fn test_response_sports_from_option_sports_empty_vec_is_empty() {
        let sports: Option<Vec<SportFilter>> = Some(vec![]);

        let response = SportsResponse::from(&sports);

        assert!(response.sports.is_empty());
        assert!(response.categories.is_empty());
    }
}
