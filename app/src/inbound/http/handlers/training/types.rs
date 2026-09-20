use std::{
    collections::HashMap,
    fmt::{Display, Write},
    str::FromStr,
};

use axum::http::StatusCode;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::{
            activity::{
                ActivityMetric, ActivityMetricSource, ActivityRpe, ActivityStatistic, BonkStatus,
                Sport, TimeseriesAggregate, TimeseriesMetric, Unit, WorkoutType,
            },
            training::{
                ActivitySource, HooperIndex, HooperIndexPatch, HooperIndexSource, SportFilter,
                SubjectiveScale, TrainingMetricActivityFilters, TrainingMetricAggregate,
                TrainingMetricGranularity, TrainingMetricGroupBy, TrainingMetricScope,
                TrainingMetricSource, TrainingMetricSummary, TrainingMetricSummaryAverage,
                TrainingMetricTarget, TrainingMetricWindow, TrainingPeriodId, TrainingPeriodSports,
                WeightAndNutrition, WeightAndNutritionPatch, WeightAndNutritionSource,
            },
        },
        ports::training::{HooperIndexError, WeightAndNutritionError},
    },
    inbound::http::{handlers::training::utils::GranuleValues, shared::PatchField},
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
pub enum APIActivityMetricSource {
    Statistic(APIActivityStatistic),
    Timeseries((APITimeseriesMetric, APITimeseriesAggregate)),
}

impl From<APIActivityMetricSource> for ActivityMetricSource {
    fn from(value: APIActivityMetricSource) -> Self {
        match value {
            APIActivityMetricSource::Statistic(stat) => {
                ActivityMetricSource::Statistic(ActivityStatistic::from(stat))
            }
            APIActivityMetricSource::Timeseries((metric, aggregate)) => {
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
}

impl APITimeseriesWindow {
    pub fn granularity(&self) -> &APITrainingMetricGranularity {
        &self.granularity
    }
    pub fn aggregate(&self) -> &APITrainingMetricAggregate {
        &self.aggregate
    }
}
impl From<&APITimeseriesWindow> for TrainingMetricWindow {
    fn from(value: &APITimeseriesWindow) -> Self {
        Self::new(
            TrainingMetricGranularity::from(&value.granularity),
            TrainingMetricAggregate::from(&value.aggregate),
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

impl TryFrom<&APITrainingMetricFilters> for TrainingMetricActivityFilters {
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
            value.bonked,
            rpes,
        ))
    }
}

impl TryFrom<APITrainingMetricFilters> for TrainingMetricActivityFilters {
    type Error = String;
    fn try_from(value: APITrainingMetricFilters) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
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

impl From<&TrainingMetricGroupBy> for APITrainingMetricGroupBy {
    fn from(value: &TrainingMetricGroupBy) -> Self {
        match value {
            TrainingMetricGroupBy::Sport => Self::Sport,
            TrainingMetricGroupBy::SportCategory => Self::SportCategory,
            TrainingMetricGroupBy::WorkoutType => Self::WorkoutType,
            TrainingMetricGroupBy::RpeRange => Self::RpeRange,
            TrainingMetricGroupBy::Bonked => Self::Bonked,
        }
    }
}

#[cfg(test)]
impl APITrainingMetricGroupBy {
    pub fn none() -> Option<Self> {
        None
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Constructor)]
pub struct APIActivitySource {
    pub metric: ActivityMetric,
    pub group_by: Option<APITrainingMetricGroupBy>,
}

impl Display for APIActivitySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.metric.to_string())
    }
}

impl From<&APIActivitySource> for ActivitySource {
    fn from(value: &APIActivitySource) -> Self {
        Self::new(
            value.metric,
            value.group_by.as_ref().map(TrainingMetricGroupBy::from),
        )
    }
}

impl From<&ActivitySource> for APIActivitySource {
    fn from(value: &ActivitySource) -> Self {
        Self::new(
            value.metric(),
            value
                .group_by()
                .as_ref()
                .map(APITrainingMetricGroupBy::from),
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", content = "metric", rename_all = "camelCase")]
pub enum APITrainingMetricSource {
    Activity(APIActivitySource),
    HooperIndex(HooperIndexSource),
    WeightAndNutrition(WeightAndNutritionSource),
}

impl Display for APITrainingMetricSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Activity(source) => f.write_str(&source.to_string()),
            Self::HooperIndex(source) => f.write_str(&source.to_string()),
            Self::WeightAndNutrition(source) => f.write_str(&source.to_string()),
        }
    }
}

impl From<&APITrainingMetricSource> for TrainingMetricSource {
    fn from(value: &APITrainingMetricSource) -> Self {
        match value {
            APITrainingMetricSource::Activity(source) => {
                Self::Activity(ActivitySource::from(source))
            }
            APITrainingMetricSource::HooperIndex(source) => Self::HooperIndex(*source),
            APITrainingMetricSource::WeightAndNutrition(source) => {
                Self::WeightAndNutrition(*source)
            }
        }
    }
}

impl From<&TrainingMetricSource> for APITrainingMetricSource {
    fn from(value: &TrainingMetricSource) -> Self {
        match value {
            TrainingMetricSource::Activity(source) => {
                Self::Activity(APIActivitySource::from(source))
            }
            TrainingMetricSource::HooperIndex(source) => Self::HooperIndex(*source),
            TrainingMetricSource::WeightAndNutrition(source) => Self::WeightAndNutrition(*source),
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
    pub source: APITrainingMetricSource,
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
    pub scope: APITrainingMetricScope,
    pub summary: HashMap<String, f64>,
}

pub fn format_source_metric(source: &TrainingMetricSource) -> String {
    match source {
        TrainingMetricSource::Activity(source) => {
            format_activity_source_metric(source.metric().source())
        }
        TrainingMetricSource::HooperIndex(source) => source.to_string(),
        TrainingMetricSource::WeightAndNutrition(source) => source.to_string(),
    }
}

fn format_activity_source_metric(source: ActivityMetricSource) -> String {
    match source {
        ActivityMetricSource::Statistic(stat) => stat.to_string(),
        ActivityMetricSource::Timeseries((metric, aggregate)) => {
            format!("Activity {aggregate:?} {metric:?}")
        }
        ActivityMetricSource::ActiveDuration => "ActiveDuration".into(),
        ActivityMetricSource::NumberOfActivities => "Number of activities".into(),
    }
}

/// Hooper's index values as received from the API. Every measure is optional, and each provided
/// value must be in the `1..=10` range.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct APIHooperIndex {
    pub fatigue: Option<u8>,
    pub sleep: Option<u8>,
    pub pain: Option<u8>,
    pub stress: Option<u8>,
    pub mood: Option<u8>,
}

impl From<&HooperIndex> for APIHooperIndex {
    fn from(value: &HooperIndex) -> Self {
        Self {
            fatigue: value.fatigue().map(|scale| scale.value()),
            sleep: value.sleep().map(|scale| scale.value()),
            pain: value.pain().map(|scale| scale.value()),
            stress: value.stress().map(|scale| scale.value()),
            mood: value.mood().map(|scale| scale.value()),
        }
    }
}

impl TryFrom<APIHooperIndex> for HooperIndex {
    type Error = String;

    fn try_from(value: APIHooperIndex) -> Result<Self, Self::Error> {
        Ok(HooperIndex::new(
            value.fatigue.map(SubjectiveScale::try_from).transpose()?,
            value.sleep.map(SubjectiveScale::try_from).transpose()?,
            value.pain.map(SubjectiveScale::try_from).transpose()?,
            value.stress.map(SubjectiveScale::try_from).transpose()?,
            value.mood.map(SubjectiveScale::try_from).transpose()?,
        ))
    }
}

/// Patch of Hooper's index values. Mirrors the domain `HooperIndexPatch` (double `Option`
/// convention):
/// - a field **absent** from the body leaves the current value untouched,
/// - a field set to **`null`** clears/removes the current value,
/// - a field with a **value** sets it (and must be in the `1..=10` range).
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct APIHooperIndexPatch {
    #[serde(default)]
    pub fatigue: PatchField<u8>,
    #[serde(default)]
    pub sleep: PatchField<u8>,
    #[serde(default)]
    pub pain: PatchField<u8>,
    #[serde(default)]
    pub stress: PatchField<u8>,
    #[serde(default)]
    pub mood: PatchField<u8>,
}

fn patch_field_to_domain(field: PatchField<u8>) -> Result<Option<Option<SubjectiveScale>>, String> {
    match field {
        PatchField::Absent => Ok(None),
        PatchField::Clear => Ok(Some(None)),
        PatchField::Set(value) => Ok(Some(Some(SubjectiveScale::try_from(value)?))),
    }
}

impl TryFrom<APIHooperIndexPatch> for HooperIndexPatch {
    type Error = String;

    fn try_from(value: APIHooperIndexPatch) -> Result<Self, Self::Error> {
        Ok(HooperIndexPatch::new(
            patch_field_to_domain(value.fatigue)?,
            patch_field_to_domain(value.sleep)?,
            patch_field_to_domain(value.pain)?,
            patch_field_to_domain(value.stress)?,
            patch_field_to_domain(value.mood)?,
        ))
    }
}

impl From<HooperIndexError> for StatusCode {
    fn from(_value: HooperIndexError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

/// Weight and nutrition values as received from the API. Every measure is optional.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct APIWeightAndNutrition {
    pub weight: Option<f32>,
    pub fat: Option<f32>,
    pub muscle: Option<f32>,
    pub bmi: Option<f32>,
    pub calories: Option<f32>,
    pub lipid: Option<f32>,
    pub carbs: Option<f32>,
    pub protein: Option<f32>,
    pub water: Option<f32>,
    pub alcohol: Option<f32>,
}

impl From<&WeightAndNutrition> for APIWeightAndNutrition {
    fn from(value: &WeightAndNutrition) -> Self {
        Self {
            weight: value.weight(),
            fat: value.fat(),
            muscle: value.muscle(),
            bmi: value.bmi(),
            calories: value.calories(),
            lipid: value.lipid(),
            carbs: value.carbs(),
            protein: value.protein(),
            water: value.water(),
            alcohol: value.alcohol(),
        }
    }
}

impl From<APIWeightAndNutrition> for WeightAndNutrition {
    fn from(value: APIWeightAndNutrition) -> Self {
        WeightAndNutrition::new(
            value.weight,
            value.fat,
            value.muscle,
            value.bmi,
            value.calories,
            value.lipid,
            value.carbs,
            value.protein,
            value.water,
            value.alcohol,
        )
    }
}

/// Patch of weight and nutrition values. Mirrors the domain `WeightAndNutritionPatch` (double
/// `Option` convention):
/// - a field **absent** from the body leaves the current value untouched,
/// - a field set to **`null`** clears/removes the current value,
/// - a field with a **value** sets it.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct APIWeightAndNutritionPatch {
    #[serde(default)]
    pub weight: PatchField<f32>,
    #[serde(default)]
    pub fat: PatchField<f32>,
    #[serde(default)]
    pub muscle: PatchField<f32>,
    #[serde(default)]
    pub bmi: PatchField<f32>,
    #[serde(default)]
    pub calories: PatchField<f32>,
    #[serde(default)]
    pub lipid: PatchField<f32>,
    #[serde(default)]
    pub carbs: PatchField<f32>,
    #[serde(default)]
    pub protein: PatchField<f32>,
    #[serde(default)]
    pub water: PatchField<f32>,
    #[serde(default)]
    pub alcohol: PatchField<f32>,
}

fn patch_field_to_domain_f32(field: PatchField<f32>) -> Option<Option<f32>> {
    match field {
        PatchField::Absent => None,
        PatchField::Clear => Some(None),
        PatchField::Set(value) => Some(Some(value)),
    }
}

impl From<APIWeightAndNutritionPatch> for WeightAndNutritionPatch {
    fn from(value: APIWeightAndNutritionPatch) -> Self {
        WeightAndNutritionPatch::new(
            patch_field_to_domain_f32(value.weight),
            patch_field_to_domain_f32(value.fat),
            patch_field_to_domain_f32(value.muscle),
            patch_field_to_domain_f32(value.bmi),
            patch_field_to_domain_f32(value.calories),
            patch_field_to_domain_f32(value.lipid),
            patch_field_to_domain_f32(value.carbs),
            patch_field_to_domain_f32(value.protein),
            patch_field_to_domain_f32(value.water),
            patch_field_to_domain_f32(value.alcohol),
        )
    }
}

impl From<WeightAndNutritionError> for StatusCode {
    fn from(_value: WeightAndNutritionError) -> Self {
        Self::UNPROCESSABLE_ENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::models::activity::{Sport, SportCategory, Unit};
    use crate::domain::models::training::{
        HooperIndex, HooperIndexPatch, SportFilter, SubjectiveScale, TrainingMetricTarget,
        TrainingPeriodSports, WeightAndNutrition, WeightAndNutritionPatch,
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

    #[test]
    fn test_format_source_metric() {
        assert_eq!(
            format_source_metric(&TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::Calories,
                TrainingMetricGroupBy::none()
            ))),
            "Calories".to_string()
        );
        assert_eq!(
            format_source_metric(&TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::MaxCadence,
                TrainingMetricGroupBy::none()
            ))),
            "Activity Max Cadence".to_string()
        );
    }

    #[test]
    fn test_api_hooper_index_converts_all_values() {
        let api = APIHooperIndex {
            fatigue: Some(1),
            sleep: Some(2),
            pain: Some(3),
            stress: Some(4),
            mood: Some(5),
        };

        let index = HooperIndex::try_from(api).unwrap();

        assert_eq!(
            index.fatigue(),
            &Some(SubjectiveScale::try_from(1).unwrap())
        );
        assert_eq!(index.sleep(), &Some(SubjectiveScale::try_from(2).unwrap()));
        assert_eq!(index.pain(), &Some(SubjectiveScale::try_from(3).unwrap()));
        assert_eq!(index.stress(), &Some(SubjectiveScale::try_from(4).unwrap()));
        assert_eq!(index.mood(), &Some(SubjectiveScale::try_from(5).unwrap()));
    }

    #[test]
    fn test_api_hooper_index_rejects_out_of_range_values() {
        for invalid in [11, 255] {
            let api = APIHooperIndex {
                fatigue: Some(invalid),
                ..Default::default()
            };

            assert!(HooperIndex::try_from(api).is_err());
        }
    }

    #[test]
    fn test_api_hooper_index_patch_semantics() {
        let body: APIHooperIndexPatch =
            serde_json::from_str(r#"{ "fatigue": 9, "sleep": null }"#).unwrap();
        let patch = HooperIndexPatch::try_from(body).unwrap();

        let existing = HooperIndex::new(
            Some(SubjectiveScale::try_from(1).unwrap()),
            Some(SubjectiveScale::try_from(2).unwrap()),
            Some(SubjectiveScale::try_from(3).unwrap()),
            Some(SubjectiveScale::try_from(4).unwrap()),
            Some(SubjectiveScale::try_from(5).unwrap()),
        );

        let patched = existing.patch(patch);

        assert_eq!(
            patched.fatigue(),
            &Some(SubjectiveScale::try_from(9).unwrap())
        );
        assert_eq!(patched.sleep(), &None);
        assert_eq!(patched.pain(), &Some(SubjectiveScale::try_from(3).unwrap()));
        assert_eq!(
            patched.stress(),
            &Some(SubjectiveScale::try_from(4).unwrap())
        );
        assert_eq!(patched.mood(), &Some(SubjectiveScale::try_from(5).unwrap()));
    }

    #[test]
    fn test_api_hooper_index_patch_rejects_out_of_range_values() {
        let body: APIHooperIndexPatch = serde_json::from_str(r#"{ "mood": 11 }"#).unwrap();

        assert!(HooperIndexPatch::try_from(body).is_err());
    }

    #[test]
    fn test_api_weight_and_nutrition_converts_all_values() {
        let api = APIWeightAndNutrition {
            weight: Some(70.5),
            fat: Some(15.0),
            muscle: Some(30.0),
            bmi: Some(22.0),
            calories: Some(2000.0),
            lipid: Some(50.0),
            carbs: Some(250.0),
            protein: Some(150.0),
            water: Some(2.5),
            alcohol: Some(0.0),
        };

        let value = WeightAndNutrition::from(api.clone());

        assert_eq!(value.weight(), Some(70.5));
        assert_eq!(value.fat(), Some(15.0));
        assert_eq!(value.muscle(), Some(30.0));
        assert_eq!(value.bmi(), Some(22.0));
        assert_eq!(value.calories(), Some(2000.0));
        assert_eq!(value.lipid(), Some(50.0));
        assert_eq!(value.carbs(), Some(250.0));
        assert_eq!(value.protein(), Some(150.0));
        assert_eq!(value.water(), Some(2.5));
        assert_eq!(value.alcohol(), Some(0.0));

        // Round-trips back to the same API representation.
        assert_eq!(APIWeightAndNutrition::from(&value), api);
    }

    #[test]
    fn test_api_weight_and_nutrition_defaults_every_measure_to_none() {
        let value = WeightAndNutrition::from(APIWeightAndNutrition::default());

        assert_eq!(value.weight(), None);
        assert_eq!(value.fat(), None);
        assert_eq!(value.muscle(), None);
        assert_eq!(value.bmi(), None);
        assert_eq!(value.calories(), None);
        assert_eq!(value.lipid(), None);
        assert_eq!(value.carbs(), None);
        assert_eq!(value.protein(), None);
        assert_eq!(value.water(), None);
        assert_eq!(value.alcohol(), None);
    }

    #[test]
    fn test_api_weight_and_nutrition_patch_semantics() {
        let body: APIWeightAndNutritionPatch =
            serde_json::from_str(r#"{ "weight": 72.0, "muscle": null }"#).unwrap();
        let patch = WeightAndNutritionPatch::from(body);

        let existing = WeightAndNutrition::new(
            Some(70.0),
            Some(15.0),
            Some(30.0),
            Some(22.0),
            Some(2000.0),
            Some(50.0),
            Some(250.0),
            Some(150.0),
            Some(2.5),
            Some(0.0),
        );

        let patched = existing.patch(patch);

        // Overridden, cleared and untouched fields respectively.
        assert_eq!(patched.weight(), Some(72.0));
        assert_eq!(patched.muscle(), None);
        assert_eq!(patched.fat(), Some(15.0));
        assert_eq!(patched.bmi(), Some(22.0));
        assert_eq!(patched.calories(), Some(2000.0));
        assert_eq!(patched.lipid(), Some(50.0));
        assert_eq!(patched.carbs(), Some(250.0));
        assert_eq!(patched.protein(), Some(150.0));
        assert_eq!(patched.water(), Some(2.5));
        assert_eq!(patched.alcohol(), Some(0.0));
    }
}
