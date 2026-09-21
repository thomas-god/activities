use std::{
    collections::{HashMap, HashSet, hash_map::Iter},
    fmt::{self, Display},
    hash::Hash,
};

use chrono::{DateTime, Datelike, Days, FixedOffset, Months, NaiveDate, Utc};
use derive_more::{AsRef, Constructor, Display};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    models::{
        UserId,
        activity::{
            Activity, ActivityMetric, ActivityMetrics, ActivityRpe, BonkStatus, Sport,
            SportCategory, Unit, WorkoutType,
        },
        search::{SearchDocument, SearchDocumentEvent, SearchDocumentType},
    },
    ports::{DateRange, DateTimeRange},
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display)]
pub enum SportFilter {
    Sport(Sport),
    SportCategory(SportCategory),
}

impl SportFilter {
    pub fn matches(&self, activity: &Activity) -> bool {
        match self {
            Self::Sport(sport) => activity.sport() == sport,
            Self::SportCategory(category) => activity.sport().category() == Some(*category),
        }
    }

    /// Returns [`Some`] containing the smallest [`SportFilter`] compatible with `other`,
    /// [`None`] if they're not compatible.
    ///
    /// When comparing a [`SportFilter::Sport`] with a [`SportFilter::SportCategory`], if the sport
    /// is included in the category, then the sport is returned, else [`None`].
    pub fn smallest_compatible(&self, other: &SportFilter) -> Option<SportFilter> {
        match (self, other) {
            // Comparing sport to sport
            (Self::Sport(sport), Self::Sport(other_sport)) if sport == other_sport => {
                Some(Self::Sport(*sport))
            }
            (Self::Sport(_sport), Self::Sport(_other_sport)) => None,

            // Comparing category to category
            (Self::SportCategory(category), Self::SportCategory(other_category))
                if category == other_category =>
            {
                Some(Self::SportCategory(*category))
            }
            (Self::SportCategory(_category), Self::SportCategory(_other_category)) => None,

            // Comparing sport to category
            (Self::Sport(sport), Self::SportCategory(other_category)) => {
                if let Some(category) = sport.category()
                    && category == *other_category
                {
                    return Some(Self::Sport(*sport));
                }
                None
            }
            (Self::SportCategory(category), Self::Sport(other_sport)) => {
                if let Some(other_category) = other_sport.category()
                    && *category == other_category
                {
                    return Some(Self::Sport(*other_sport));
                }
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Constructor, Serialize, Deserialize, Default)]
pub struct TrainingMetricActivityFilters {
    sports: Option<Vec<SportFilter>>,
    workout_types: Option<Vec<WorkoutType>>,
    bonked: Option<BonkStatus>,
    rpes: Option<Vec<ActivityRpe>>,
}

impl TrainingMetricActivityFilters {
    pub fn empty() -> Self {
        Self {
            sports: None,
            workout_types: None,
            bonked: None,
            rpes: None,
        }
    }

    /// Merge `default_sports` into the current sport filters.
    /// - if current sport filters is empty, then use `default_sports` instead,
    /// - if current sport fitlers is not empty, filter it by the elements of `default_sports`
    pub fn merge_default_sports(self, default_sports: &Option<Vec<SportFilter>>) -> Self {
        let Some(default_sports) = default_sports else {
            return self;
        };

        let new_sports = match self.sports {
            Some(sports) => sports
                .iter()
                .flat_map(|sport| {
                    default_sports
                        .iter()
                        .filter_map(|other| sport.smallest_compatible(other))
                })
                .collect::<Vec<_>>(),
            None => default_sports.to_vec(),
        };

        Self {
            sports: Some(new_sports),
            workout_types: self.workout_types,
            bonked: self.bonked,
            rpes: self.rpes,
        }
    }

    pub fn sports(&self) -> &Option<Vec<SportFilter>> {
        &self.sports
    }

    pub fn workout_types(&self) -> &Option<Vec<WorkoutType>> {
        &self.workout_types
    }

    pub fn bonked(&self) -> &Option<BonkStatus> {
        &self.bonked
    }

    pub fn rpes(&self) -> &Option<Vec<ActivityRpe>> {
        &self.rpes
    }

    pub fn matches(&self, activity: &Activity) -> bool {
        let sport_matches = self
            .sports
            .as_ref()
            .map(|sports| sports.iter().any(|filter| filter.matches(activity)))
            .unwrap_or(true);

        let workout_matches = self
            .workout_types
            .as_ref()
            .map(|types| {
                types.iter().any(|workout_type| {
                    activity
                        .workout_type()
                        .map(|activity_workout_type| *workout_type == activity_workout_type)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(true);

        let bonk_status_matches = self
            .bonked
            .as_ref()
            .map(|status| {
                activity
                    .nutrition()
                    .as_ref()
                    .map(|nutrition| nutrition.bonk_status() == *status)
                    .unwrap_or(false)
            })
            .unwrap_or(true);

        let rpes_matches = self
            .rpes
            .as_ref()
            .map(|rpes| {
                rpes.iter().any(|rpe| {
                    activity
                        .rpe()
                        .map(|activity_rpe| *rpe == activity_rpe)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(true);

        sport_matches && workout_matches && bonk_status_matches && rpes_matches
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrainingMetricActivityGroupBy {
    Sport,
    SportCategory,
    WorkoutType,
    RpeRange,
    Bonked,
}

impl TrainingMetricActivityGroupBy {
    pub fn extract_group(&self, activity: &Activity) -> Option<String> {
        match self {
            Self::Sport => Some(activity.sport().to_string()),
            Self::SportCategory => activity.sport().category().map(|cat| cat.to_string()),
            Self::WorkoutType => activity.workout_type().map(|wk| wk.to_string()),
            Self::RpeRange => activity.rpe().map(|rpe| rpe.range().to_string()),
            Self::Bonked => activity
                .nutrition()
                .as_ref()
                .map(|nutrition| nutrition.bonk_status().to_string()),
        }
    }
}

impl Display for TrainingMetricActivityGroupBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bonked => f.write_str("Bonked"),
            Self::RpeRange => f.write_str("RpeRange"),
            Self::SportCategory => f.write_str("SportCategory"),
            Self::Sport => f.write_str("Sport"),
            Self::WorkoutType => f.write_str("WorkoutType"),
        }
    }
}

#[cfg(test)]
impl TrainingMetricActivityGroupBy {
    pub fn none() -> Option<TrainingMetricActivityGroupBy> {
        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrainingMetricName(String);

impl TrainingMetricName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TrainingMetricName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TrainingMetricName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for TrainingMetricName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct TrainingMetric {
    id: TrainingMetricId,
    name: Option<TrainingMetricName>,
    scope: TrainingMetricScope,
    definition: TrainingMetricDefinition,
}

impl TrainingMetric {
    pub fn id(&self) -> &TrainingMetricId {
        &self.id
    }

    pub fn name(&self) -> &Option<TrainingMetricName> {
        &self.name
    }

    pub fn scope(&self) -> &TrainingMetricScope {
        &self.scope
    }

    pub fn definition(&self) -> &TrainingMetricDefinition {
        &self.definition
    }

    pub fn apply_patch(self, patch: TrainingMetricPatch) -> TrainingMetric {
        TrainingMetric {
            id: self.id,
            scope: self.scope,
            name: Some(patch.name),
            definition: self.definition.apply_patch(patch.definition),
        }
    }
}

/// Patch that can be applied to a [`TrainingMetric`], i.e. its fields that are allowed to be
/// updated.
#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct TrainingMetricPatch {
    name: TrainingMetricName,
    definition: TrainingMetricDefinitionPatch,
}

impl TrainingMetricPatch {
    pub fn name(&self) -> &TrainingMetricName {
        &self.name
    }

    pub fn definition(&self) -> &TrainingMetricDefinitionPatch {
        &self.definition
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrainingMetricScope {
    Global,
    TrainingPeriod(TrainingPeriodId),
}

impl From<&Option<TrainingPeriodId>> for TrainingMetricScope {
    fn from(value: &Option<TrainingPeriodId>) -> Self {
        match value {
            None => Self::Global,
            Some(period) => Self::TrainingPeriod(period.clone()),
        }
    }
}

impl From<&TrainingMetricScope> for Option<TrainingPeriodId> {
    fn from(value: &TrainingMetricScope) -> Self {
        match value {
            TrainingMetricScope::Global => None,
            TrainingMetricScope::TrainingPeriod(period) => Some(period.clone()),
        }
    }
}

impl TrainingMetricScope {
    pub fn period(&self) -> Option<TrainingPeriodId> {
        self.into()
    }
}

#[derive(Debug, Clone, PartialEq, Constructor, Serialize, Deserialize)]
pub struct TrainingMetricSummary {
    average: Option<TrainingMetricSummaryAverage>,
}

#[derive(Debug, Clone, PartialEq, Constructor, Serialize, Deserialize)]
pub struct TrainingMetricSummaryAverage {
    include_zeros: bool,
}

impl TrainingMetricSummaryAverage {
    pub fn compute(&self, values: &HashMap<TrainingMetricBin, TrainingMetricValue>) -> Option<f64> {
        let mut values_by_bin: HashMap<&str, f64> = HashMap::new();
        for (bin, value) in values.iter() {
            values_by_bin
                .entry(bin.granule())
                .and_modify(|v| *v += value.value())
                .or_insert(value.value());
        }

        let number_of_valid_bins = values_by_bin.iter().fold(0, |acc, (_, value)| {
            if !self.include_zeros && *value == 0. {
                acc
            } else {
                acc + 1
            }
        });

        if number_of_valid_bins == 0 {
            return None;
        }

        Some(
            values_by_bin.iter().fold(0., |acc, (_, curr)| acc + curr)
                / (number_of_valid_bins as f64),
        )
    }
}

impl TrainingMetricSummary {
    pub fn empty() -> Self {
        Self { average: None }
    }

    pub fn average(&self) -> &Option<TrainingMetricSummaryAverage> {
        &self.average
    }

    pub fn compute(
        &self,
        values: &HashMap<TrainingMetricBin, TrainingMetricValue>,
    ) -> TrainingMetricSummaryValues {
        TrainingMetricSummaryValues {
            average: self.average.as_ref().and_then(|a| a.compute(values)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Constructor, Default)]
pub struct TrainingMetricSummaryValues {
    average: Option<f64>,
}

impl TrainingMetricSummaryValues {
    pub fn as_hash_map(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();

        if let Some(average) = self.average {
            map.insert("average".to_string(), average);
        }

        map
    }
}

/// Serde helper to (de)serialize [`Unit`] using its `Display`/`FromStr` representation
/// (e.g. `"km"`) instead of the derived variant names.
mod unit_as_display {
    use std::str::FromStr;

    use serde::{Deserialize, Deserializer, Serializer};

    use crate::domain::models::activity::Unit;

    pub fn serialize<S>(unit: &Unit, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&unit.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Unit, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Unit::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Constructor, Serialize, Deserialize)]
pub struct TrainingMetricTarget {
    value: f64,
    #[serde(with = "unit_as_display")]
    unit: Unit,
}

impl TrainingMetricTarget {
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn unit(&self) -> Unit {
        self.unit
    }
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct TrainingMetricWindow {
    granularity: TrainingMetricGranularity,
    aggregate: TrainingMetricAggregate,
}

impl TrainingMetricWindow {
    pub fn granularity(&self) -> &TrainingMetricGranularity {
        &self.granularity
    }

    pub fn aggregate(&self) -> &TrainingMetricAggregate {
        &self.aggregate
    }
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct TrainingMetricDefinitionPatch {
    source: TrainingMetricSource,
    window: Option<TrainingMetricWindow>,
    summary: TrainingMetricSummary,
    target: Option<TrainingMetricTarget>,
}

impl TrainingMetricDefinitionPatch {
    pub fn source(&self) -> &TrainingMetricSource {
        &self.source
    }

    pub fn window(&self) -> &Option<TrainingMetricWindow> {
        &self.window
    }

    pub fn summary(&self) -> &TrainingMetricSummary {
        &self.summary
    }

    pub fn target(&self) -> &Option<TrainingMetricTarget> {
        &self.target
    }
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct ActivitySource {
    metric: ActivityMetric,
    group_by: Option<TrainingMetricActivityGroupBy>,
    filters: TrainingMetricActivityFilters,
}

impl ActivitySource {
    pub fn metric(&self) -> ActivityMetric {
        self.metric
    }

    pub fn group_by(&self) -> &Option<TrainingMetricActivityGroupBy> {
        &self.group_by
    }

    pub fn filters(&self) -> &TrainingMetricActivityFilters {
        &self.filters
    }

    pub fn unit(&self) -> Unit {
        self.metric.unit()
    }

    pub fn merge_default_sports(self, default_sports: &Option<Vec<SportFilter>>) -> Self {
        Self {
            metric: self.metric,
            group_by: self.group_by,
            filters: self.filters.merge_default_sports(default_sports),
        }
    }

    pub fn extract_values(
        &self,
        window: &Option<TrainingMetricWindow>,
        activities: impl Iterator<Item = (Activity, f64)>,
    ) -> HashMap<TrainingMetricBin, Vec<IndividualValue>> {
        let filtered_activities =
            activities.filter(|(activity, _metric_value)| self.filters.matches(activity));

        filtered_activities
            .map(|(activity, metric)| {
                let bin = match window {
                    None => TrainingMetricBin::new_without_group(
                        activity.start_time().datetime().to_rfc3339(),
                    ),
                    Some(window) => {
                        let granule = window
                            .granularity()
                            .datetime_key(activity.start_time().datetime());

                        let group = self
                            .group_by
                            .as_ref()
                            .and_then(|group_by| group_by.extract_group(&activity));

                        TrainingMetricBin::new(granule, group)
                    }
                };

                (bin, IndividualValue::new(metric))
            })
            .into_group_map()
    }
}

impl Display for ActivitySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.metric.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrainingMetricSource {
    Activity(ActivitySource),
    HooperIndex(HooperIndexSource),
    WeightAndNutrition(WeightAndNutritionSource),
}

impl TrainingMetricSource {
    pub fn unit(&self) -> Unit {
        match self {
            Self::Activity(source) => source.unit(),
            Self::HooperIndex(source) => source.unit(),
            Self::WeightAndNutrition(source) => source.unit(),
        }
    }

    pub fn merge_default_sports(self, default_sports: &Option<Vec<SportFilter>>) -> Self {
        match self {
            Self::Activity(source) => Self::Activity(source.merge_default_sports(default_sports)),
            source => source,
        }
    }
}

impl Display for TrainingMetricSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Activity(source) => f.write_str(&source.to_string()),
            Self::HooperIndex(source) => f.write_str(&source.to_string()),
            Self::WeightAndNutrition(source) => f.write_str(&source.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingMetricDefinition {
    user: UserId,
    source: TrainingMetricSource,
    window: Option<TrainingMetricWindow>,
    summary: TrainingMetricSummary,
    target: Option<TrainingMetricTarget>,
}

impl TrainingMetricDefinition {
    pub fn new(
        user: UserId,
        source: TrainingMetricSource,
        window: Option<TrainingMetricWindow>,
        summary: TrainingMetricSummary,
        target: Option<TrainingMetricTarget>,
    ) -> Self {
        // By definitions, Hooper values and Weight&Nutrition are day aligned values, so we reflect
        // that by converting None window to be of TrainingMetricGranularity::Daily granularity.
        // With one value per day the chosen aggregate funcion (average) is identity.
        let window = match &source {
            TrainingMetricSource::HooperIndex(_) | TrainingMetricSource::WeightAndNutrition(_) => {
                window.or(Some(TrainingMetricWindow::new(
                    TrainingMetricGranularity::Daily,
                    TrainingMetricAggregate::Average,
                )))
            }
            TrainingMetricSource::Activity(_) => window,
        };

        Self {
            user,
            source,
            window,
            summary,
            target,
        }
    }

    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn source(&self) -> &TrainingMetricSource {
        &self.source
    }

    pub fn window(&self) -> &Option<TrainingMetricWindow> {
        &self.window
    }

    pub fn summary(&self) -> &TrainingMetricSummary {
        &self.summary
    }

    pub fn target(&self) -> &Option<TrainingMetricTarget> {
        &self.target
    }

    pub fn unit(&self) -> Unit {
        match self.source() {
            TrainingMetricSource::Activity(source) => source.unit(),
            TrainingMetricSource::HooperIndex(source) => source.unit(),
            TrainingMetricSource::WeightAndNutrition(source) => source.unit(),
        }
    }

    pub fn apply_patch(self, patch: TrainingMetricDefinitionPatch) -> Self {
        Self {
            user: self.user,
            source: patch.source,
            window: patch.window,
            summary: patch.summary,
            target: patch.target,
        }
    }

    pub fn merge_default_sports(self, default_sports: &Option<Vec<SportFilter>>) -> Self {
        Self {
            user: self.user,
            source: self.source.merge_default_sports(default_sports),
            window: self.window,
            summary: self.summary,
            target: self.target,
        }
    }

    /// Compute for each bin the corresponding training metric value based on the window/aggregate
    /// of the definition and return the final `TrainingMetricValues`.
    pub fn compute_training_metric_values(
        &self,
        values: HashMap<TrainingMetricBin, Vec<IndividualValue>>,
    ) -> TrainingMetricValues {
        let aggregate = self.window().as_ref().map(|w| *w.aggregate());
        let values = values
            .into_iter()
            .filter_map(|(key, values)| {
                let value = match aggregate {
                    None => {
                        // Skip empty bins
                        let value = values.first()?;
                        TrainingMetricValue::SingleValue(value.value())
                    }
                    Some(aggregate) => aggregate.aggregate_values(&values)?,
                };

                Some((key, value))
            })
            .collect();

        let summary = self.summary.compute(&values);

        TrainingMetricValues::new(values, summary, self.unit())
    }
}

/// Intermediate, source-agnostic, struct holding the value and metadata necessary to compute
/// a training metric values. Sources (activity, hooper index) should map/convert to this struct.
#[derive(Debug, Clone, Copy, Constructor)]
pub struct IndividualValue(f64);

impl IndividualValue {
    fn value(&self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Display)]
pub enum TrainingMetricGranularity {
    Daily,
    Weekly,
    Monthly,
}

impl TrainingMetricGranularity {
    pub fn date_key(&self, date: &chrono::NaiveDate) -> String {
        match self {
            TrainingMetricGranularity::Daily => date.to_string(),
            TrainingMetricGranularity::Weekly => {
                date.week(chrono::Weekday::Mon).first_day().to_string()
            }
            TrainingMetricGranularity::Monthly => date.with_day(1).unwrap().to_string(),
        }
    }
    pub fn datetime_key(&self, dt: &DateTime<FixedOffset>) -> String {
        self.date_key(&dt.date_naive())
    }

    /// Computes the bins' keys for the [TrainingMetricGranularity] over the given range [start,
    /// end].
    pub fn bins_keys(&self, start: &chrono::NaiveDate, end: &chrono::NaiveDate) -> Vec<String> {
        let mut dates = vec![];

        #[allow(clippy::type_complexity)]
        let (mut start, end, next_dt): (
            NaiveDate,
            NaiveDate,
            Box<dyn Fn(NaiveDate) -> Option<NaiveDate>>,
        ) = match self {
            Self::Daily => (
                *start,
                *end,
                Box::new(|dt: NaiveDate| dt.checked_add_days(Days::new(1))),
            ),
            Self::Weekly => (
                start.week(chrono::Weekday::Mon).first_day(),
                end.week(chrono::Weekday::Mon).first_day(),
                Box::new(|dt: NaiveDate| dt.checked_add_days(Days::new(7))),
            ),
            Self::Monthly => (
                start.with_day(1).unwrap(),
                end.with_day(1).unwrap(),
                Box::new(|dt: NaiveDate| dt.checked_add_months(Months::new(1))),
            ),
        };

        loop {
            dates.push(start.to_string());
            let Some(new_start) = next_dt(start) else {
                return dates;
            };
            start = new_start;
            if new_start > end {
                break;
            }
        }
        dates
    }

    pub fn bins_from_datetime(&self, range: &DateTimeRange) -> Vec<DateRange> {
        let start = range.start().date_naive();
        let end = range
            .end()
            .map(|date| date.date_naive())
            .unwrap_or(Utc::now().fixed_offset().date_naive());

        self.bins(&DateRange::new(start, end))
    }

    pub fn bins(&self, range: &DateRange) -> Vec<DateRange> {
        #[allow(clippy::type_complexity)]
        let (mut start, last_start, next_start): (
            NaiveDate,
            NaiveDate,
            Box<dyn Fn(NaiveDate) -> Option<NaiveDate>>,
        ) = match self {
            Self::Daily => (
                *range.start(),
                *range.end(),
                Box::new(|dt: NaiveDate| dt.checked_add_days(Days::new(1))),
            ),
            Self::Weekly => (
                range.start().week(chrono::Weekday::Mon).first_day(),
                range.end().week(chrono::Weekday::Mon).first_day(),
                Box::new(|dt: NaiveDate| dt.checked_add_days(Days::new(7))),
            ),
            Self::Monthly => (
                range.start().with_day(1).unwrap(),
                range.end().with_day(1).unwrap(),
                Box::new(|dt: NaiveDate| dt.checked_add_months(Months::new(1))),
            ),
        };

        let mut dates = vec![];
        loop {
            let Some(new_start) = next_start(start) else {
                return dates;
            };
            dates.push(DateRange::new(start, new_start));
            start = new_start;
            if new_start > last_start {
                break;
            }
        }
        dates
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Display, Serialize, Deserialize)]
pub enum TrainingMetricAggregate {
    Min,
    Max,
    Average,
    Sum,
    NumberOfActivities,
}

impl TrainingMetricAggregate {
    fn aggregate_values(&self, values: &[IndividualValue]) -> Option<TrainingMetricValue> {
        if values.is_empty() {
            return None;
        }
        Some(match self {
            TrainingMetricAggregate::Min => TrainingMetricValue::Min(
                values
                    .iter()
                    .fold(f64::MAX, |min, metric| min.min(metric.value())),
            ),
            TrainingMetricAggregate::Max => TrainingMetricValue::Max(
                values
                    .iter()
                    .fold(f64::MIN, |max, metric| max.max(metric.value())),
            ),
            TrainingMetricAggregate::Average => {
                let number_of_metrics = values.len();
                let sum = values.iter().fold(0., |sum, metric| sum + metric.value());

                TrainingMetricValue::Average(sum / number_of_metrics as f64)
            }
            TrainingMetricAggregate::Sum => {
                TrainingMetricValue::Sum(values.iter().fold(0., |sum, metric| sum + metric.value()))
            }
            TrainingMetricAggregate::NumberOfActivities => {
                TrainingMetricValue::NumberOfActivities(values.len())
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrainingMetricValue {
    SingleValue(f64),
    Min(f64),
    Max(f64),
    Sum(f64),
    Average(f64),
    NumberOfActivities(usize),
}

impl TrainingMetricValue {
    pub fn value(&self) -> f64 {
        match self {
            Self::SingleValue(value) => *value,
            Self::Max(max) => *max,
            Self::Min(min) => *min,
            Self::Sum(sum) => *sum,
            Self::Average(avg) => *avg,
            Self::NumberOfActivities(count) => *count as f64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Constructor)]
pub struct TrainingMetricBin {
    granule: String,
    group: Option<String>,
}

impl TrainingMetricBin {
    pub fn new_without_group(granule: String) -> Self {
        Self {
            granule,
            group: None,
        }
    }

    pub fn granule(&self) -> &str {
        &self.granule
    }

    pub fn group(&self) -> &Option<String> {
        &self.group
    }
}

#[cfg(test)]
impl TrainingMetricBin {
    pub fn from_granule(granule: &str) -> Self {
        Self {
            granule: granule.to_string(),
            group: None,
        }
    }
}

#[derive(Debug, Clone, Constructor, PartialEq)]
pub struct TrainingMetricValues {
    values: HashMap<TrainingMetricBin, TrainingMetricValue>,
    summary_values: TrainingMetricSummaryValues,
    unit: Unit,
}

impl TrainingMetricValues {
    pub fn empty(unit: Unit) -> Self {
        Self {
            values: HashMap::new(),
            summary_values: TrainingMetricSummaryValues::default(),
            unit,
        }
    }

    pub fn insert(
        &mut self,
        key: TrainingMetricBin,
        value: TrainingMetricValue,
    ) -> Option<TrainingMetricValue> {
        self.values.insert(key, value)
    }

    pub fn get(&self, key: &TrainingMetricBin) -> Option<&TrainingMetricValue> {
        self.values.get(key)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> Iter<'_, TrainingMetricBin, TrainingMetricValue> {
        self.values.iter()
    }

    pub fn summary_values(&self) -> &TrainingMetricSummaryValues {
        &self.summary_values
    }

    pub fn unit(&self) -> Unit {
        self.unit
    }
}

impl TrainingMetricValues {
    pub fn as_hash_map(self) -> HashMap<TrainingMetricBin, TrainingMetricValue> {
        self.values
    }
}

impl std::iter::IntoIterator for TrainingMetricValues {
    type Item = (TrainingMetricBin, TrainingMetricValue);
    type IntoIter = std::collections::hash_map::IntoIter<TrainingMetricBin, TrainingMetricValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, AsRef, Hash, Serialize, Deserialize)]
pub struct TrainingPeriodId(String);

impl TrainingPeriodId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl fmt::Display for TrainingPeriodId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TrainingPeriodId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Constructor, PartialEq, Serialize, Deserialize)]
pub struct TrainingPeriodSports(Option<Vec<SportFilter>>);

impl TrainingPeriodSports {
    pub fn matches(&self, activity: &Activity) -> bool {
        self.0
            .as_ref()
            .map(|sports| sports.iter().any(|sport| sport.matches(activity)))
            .unwrap_or(true)
    }

    pub fn items(&self) -> Option<&Vec<SportFilter>> {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingPeriod {
    id: TrainingPeriodId,
    user: UserId,
    start: NaiveDate,
    end: Option<NaiveDate>,
    name: String,
    sports: TrainingPeriodSports,
    note: Option<String>,
}

#[derive(Debug, Clone, Error)]
pub enum TrainingPeriodCreationError {
    #[error("End date must be None or after start date")]
    EndDateBeforeStartDate,
}

impl TrainingPeriod {
    pub fn new(
        id: TrainingPeriodId,
        user: UserId,
        start: NaiveDate,
        end: Option<NaiveDate>,
        name: String,
        sports: TrainingPeriodSports,
        note: Option<String>,
    ) -> Result<Self, TrainingPeriodCreationError> {
        if let Some(end_date) = end
            && start > end_date
        {
            return Err(TrainingPeriodCreationError::EndDateBeforeStartDate);
        }

        Ok(Self {
            id,
            user,
            start,
            end,
            name,
            sports,
            note,
        })
    }

    pub fn id(&self) -> &TrainingPeriodId {
        &self.id
    }

    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn start(&self) -> &NaiveDate {
        &self.start
    }

    pub fn end(&self) -> &Option<NaiveDate> {
        &self.end
    }

    /// Returns a DateRange for this training period, ending today for open-ended periods.
    pub fn range_default_today(&self) -> DateRange {
        let end = self.end.unwrap_or_else(|| Utc::now().date_naive());
        DateRange::new(self.start, end)
    }

    /// Returns a DateRange for this training period, ending tomorrow for open-ended periods.
    pub fn range_default_tomorrow(&self) -> DateRange {
        let end = self
            .end
            .unwrap_or_else(|| Utc::now().date_naive() + Days::new(1));
        DateRange::new(self.start, end)
    }

    pub fn matches(&self, activity: &Activity) -> bool {
        let activity_start_date = activity.start_time().datetime().date_naive();
        if activity_start_date < self.start {
            return false;
        }

        if let Some(end) = self.end
            && activity_start_date > end
        {
            return false;
        }

        self.sports.matches(activity)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn sports(&self) -> &TrainingPeriodSports {
        &self.sports
    }

    pub fn sport_items(&self) -> &Option<Vec<SportFilter>> {
        &self.sports.0
    }

    pub fn note(&self) -> &Option<String> {
        &self.note
    }
}

#[derive(Debug, Clone)]
pub struct TrainingPeriodWithActivities {
    period: TrainingPeriod,
    activities: Vec<(Activity, ActivityMetrics)>,
}

impl TrainingPeriodWithActivities {
    pub fn new(period: TrainingPeriod, activities: Vec<(Activity, ActivityMetrics)>) -> Self {
        Self { period, activities }
    }

    pub fn period(&self) -> &TrainingPeriod {
        &self.period
    }

    pub fn activities(&self) -> &[(Activity, ActivityMetrics)] {
        &self.activities
    }
}
// =============================================================================
// Hooper's Index
// =============================================================================

/// Subjective scale with value in [0, 10]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubjectiveScale(u8);

impl TryFrom<u8> for SubjectiveScale {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 10 {
            return Err("subjective scale must be in [0, 10]".to_string());
        }

        Ok(Self(value))
    }
}

impl SubjectiveScale {
    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Hooper's index is a collection of self-reported subjective measures about one's state and
/// well-being.
#[derive(Debug, Clone, Copy, Default)]
pub struct HooperIndex {
    fatigue: Option<SubjectiveScale>,
    sleep: Option<SubjectiveScale>,
    pain: Option<SubjectiveScale>,
    stress: Option<SubjectiveScale>,
    mood: Option<SubjectiveScale>,
}

impl HooperIndex {
    pub fn new(
        fatigue: Option<SubjectiveScale>,
        sleep: Option<SubjectiveScale>,
        pain: Option<SubjectiveScale>,
        stress: Option<SubjectiveScale>,
        mood: Option<SubjectiveScale>,
    ) -> Self {
        Self {
            fatigue,
            sleep,
            pain,
            stress,
            mood,
        }
    }

    pub fn fatigue(&self) -> &Option<SubjectiveScale> {
        &self.fatigue
    }
    pub fn sleep(&self) -> &Option<SubjectiveScale> {
        &self.sleep
    }
    pub fn pain(&self) -> &Option<SubjectiveScale> {
        &self.pain
    }
    pub fn stress(&self) -> &Option<SubjectiveScale> {
        &self.stress
    }
    pub fn mood(&self) -> &Option<SubjectiveScale> {
        &self.mood
    }

    pub fn value(&self, source: &HooperIndexSource) -> &Option<SubjectiveScale> {
        match source {
            HooperIndexSource::Fatigue => &self.fatigue,
            HooperIndexSource::Sleep => &self.sleep,
            HooperIndexSource::Mood => &self.mood,
            HooperIndexSource::Pain => &self.pain,
            HooperIndexSource::Stress => &self.stress,
        }
    }

    pub fn patch(self, patch: HooperIndexPatch) -> Self {
        Self {
            fatigue: patch.fatigue.unwrap_or(self.fatigue),
            sleep: patch.sleep.unwrap_or(self.sleep),
            pain: patch.pain.unwrap_or(self.pain),
            stress: patch.stress.unwrap_or(self.stress),
            mood: patch.mood.unwrap_or(self.mood),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Display, Serialize, Deserialize)]
pub enum HooperIndexSource {
    Fatigue,
    Sleep,
    Pain,
    Stress,
    Mood,
}

impl HooperIndexSource {
    pub fn unit(&self) -> Unit {
        Unit::Null
    }

    pub fn extract_values(
        &self,
        window: &Option<TrainingMetricWindow>,
        values: impl Iterator<Item = (chrono::NaiveDate, HooperIndex)>,
    ) -> HashMap<TrainingMetricBin, Vec<IndividualValue>> {
        // Hooper values are day-aligned
        let granularity = window
            .as_ref()
            .map(|w| *w.granularity())
            .unwrap_or(TrainingMetricGranularity::Daily);

        values
            .filter_map(|(date, value)| {
                let Some(value) = value.value(self) else {
                    return None;
                };

                // Hooper index values have no intrinsic group
                Some((
                    TrainingMetricBin::new_without_group(granularity.date_key(&date)),
                    IndividualValue::new(value.value() as f64),
                ))
            })
            .into_group_map()
    }
}

#[derive(Debug, Clone, Copy, Constructor, Default)]
pub struct HooperIndexPatch {
    fatigue: Option<Option<SubjectiveScale>>,
    sleep: Option<Option<SubjectiveScale>>,
    pain: Option<Option<SubjectiveScale>>,
    stress: Option<Option<SubjectiveScale>>,
    mood: Option<Option<SubjectiveScale>>,
}

impl HooperIndexPatch {
    pub fn fatigue(&self) -> &Option<Option<SubjectiveScale>> {
        &self.fatigue
    }
    pub fn sleep(&self) -> &Option<Option<SubjectiveScale>> {
        &self.sleep
    }
    pub fn pain(&self) -> &Option<Option<SubjectiveScale>> {
        &self.pain
    }
    pub fn stress(&self) -> &Option<Option<SubjectiveScale>> {
        &self.stress
    }
    pub fn mood(&self) -> &Option<Option<SubjectiveScale>> {
        &self.mood
    }
}

// =============================================================================
// Weight and nutrition
// =============================================================================

#[derive(Debug, Clone, Copy, Constructor, Default)]
pub struct WeightAndNutrition {
    // Weight
    weight: Option<f32>,
    fat: Option<f32>,
    muscle: Option<f32>,
    // Nutrition
    calories: Option<f32>,
    lipid: Option<f32>,
    carbs: Option<f32>,
    protein: Option<f32>,
    // Hydration
    water: Option<f32>,
    alcohol: Option<f32>,
}

impl WeightAndNutrition {
    pub fn weight(&self) -> Option<f32> {
        self.weight
    }
    pub fn fat(&self) -> Option<f32> {
        self.fat
    }
    pub fn muscle(&self) -> Option<f32> {
        self.muscle
    }
    pub fn calories(&self) -> Option<f32> {
        self.calories
    }
    pub fn lipid(&self) -> Option<f32> {
        self.lipid
    }
    pub fn carbs(&self) -> Option<f32> {
        self.carbs
    }
    pub fn protein(&self) -> Option<f32> {
        self.protein
    }
    pub fn water(&self) -> Option<f32> {
        self.water
    }
    pub fn alcohol(&self) -> Option<f32> {
        self.alcohol
    }

    pub fn patch(self, patch: WeightAndNutritionPatch) -> Self {
        Self {
            weight: patch.weight.unwrap_or(self.weight),
            muscle: patch.muscle.unwrap_or(self.muscle),
            fat: patch.fat.unwrap_or(self.fat),
            calories: patch.calories.unwrap_or(self.calories),
            lipid: patch.lipid.unwrap_or(self.lipid),
            carbs: patch.carbs.unwrap_or(self.carbs),
            protein: patch.protein.unwrap_or(self.protein),
            water: patch.water.unwrap_or(self.water),
            alcohol: patch.alcohol.unwrap_or(self.alcohol),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Display, Serialize, Deserialize)]
pub enum WeightAndNutritionSource {
    TotalWeight,
    BodyComposition,
    Calories,
    Macros,
    Water,
    Alcohol,
}

impl WeightAndNutritionSource {
    pub fn unit(&self) -> Unit {
        match self {
            Self::TotalWeight | Self::BodyComposition => Unit::Kilogram,
            Self::Calories => Unit::KiloCalorie,
            Self::Macros => Unit::Gram,
            Self::Water => Unit::Liter,
            Self::Alcohol => Unit::AlcoholUnit,
        }
    }

    pub fn extract_values(
        &self,
        window: &Option<TrainingMetricWindow>,
        values: impl Iterator<Item = (chrono::NaiveDate, WeightAndNutrition)>,
    ) -> HashMap<TrainingMetricBin, Vec<IndividualValue>> {
        // Weight and nutrition values are day-aligned
        let granularity = window
            .as_ref()
            .map(|w| *w.granularity())
            .unwrap_or(TrainingMetricGranularity::Daily);

        values
            .map(|(date, value)| {
                let bin = granularity.date_key(&date);

                let mut values = vec![];
                match self {
                    Self::TotalWeight => {
                        if let Some(weight) = value.weight() {
                            values.push((
                                TrainingMetricBin::new_without_group(bin),
                                IndividualValue::new(weight as f64),
                            ));
                        }
                    }
                    Self::BodyComposition => {
                        if let Some(fat) = value.fat() {
                            values.push((
                                TrainingMetricBin::new(bin.clone(), Some("fat".to_string())),
                                IndividualValue::new(fat as f64),
                            ));
                        }
                        if let Some(muscle) = value.muscle() {
                            values.push((
                                TrainingMetricBin::new(bin, Some("muscle".to_string())),
                                IndividualValue::new(muscle as f64),
                            ));
                        }
                    }
                    Self::Calories => {
                        if let Some(calories) = value.calories() {
                            values.push((
                                TrainingMetricBin::new_without_group(bin),
                                IndividualValue::new(calories as f64),
                            ));
                        }
                    }
                    Self::Macros => {
                        if let Some(lipids) = value.lipid() {
                            values.push((
                                TrainingMetricBin::new(bin.clone(), Some("lipids".to_string())),
                                IndividualValue::new(lipids as f64),
                            ));
                        }
                        if let Some(carbs) = value.carbs() {
                            values.push((
                                TrainingMetricBin::new(bin.clone(), Some("carbs".to_string())),
                                IndividualValue::new(carbs as f64),
                            ));
                        }
                        if let Some(proteins) = value.protein() {
                            values.push((
                                TrainingMetricBin::new(bin, Some("proteins".to_string())),
                                IndividualValue::new(proteins as f64),
                            ));
                        }
                    }
                    Self::Water => {
                        if let Some(water) = value.water() {
                            values.push((
                                TrainingMetricBin::new_without_group(bin),
                                IndividualValue::new(water as f64),
                            ));
                        }
                    }
                    Self::Alcohol => {
                        if let Some(alcohol) = value.alcohol() {
                            values.push((
                                TrainingMetricBin::new_without_group(bin),
                                IndividualValue::new(alcohol as f64),
                            ));
                        }
                    }
                };

                values
            })
            .flatten()
            .into_group_map()
    }
}

#[derive(Debug, Clone, Copy, Constructor, Default)]
pub struct WeightAndNutritionPatch {
    pub weight: Option<Option<f32>>,
    pub fat: Option<Option<f32>>,
    pub muscle: Option<Option<f32>>,
    pub calories: Option<Option<f32>>,
    pub lipid: Option<Option<f32>>,
    pub carbs: Option<Option<f32>>,
    pub protein: Option<Option<f32>>,
    pub water: Option<Option<f32>>,
    pub alcohol: Option<Option<f32>>,
}

// =============================================================================
// Training Notes
// =============================================================================

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, AsRef, Hash)]
pub struct TrainingNoteId(String);

impl TrainingNoteId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl fmt::Display for TrainingNoteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TrainingNoteId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrainingNoteContent(String);

impl TrainingNoteContent {
    pub fn new(content: impl Into<String>) -> Self {
        Self(content.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TrainingNoteContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TrainingNoteContent {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for TrainingNoteContent {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrainingNoteTitle(String);

impl TrainingNoteTitle {
    pub fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TrainingNoteTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TrainingNoteTitle {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for TrainingNoteTitle {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainingNoteDate(NaiveDate);

impl TrainingNoteDate {
    pub fn new(date: NaiveDate) -> Self {
        Self(date)
    }

    pub fn as_naive_date(&self) -> &NaiveDate {
        &self.0
    }

    #[cfg(test)]
    pub fn today() -> Self {
        Self(Utc::now().date_naive())
    }
}

impl fmt::Display for TrainingNoteDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<NaiveDate> for TrainingNoteDate {
    fn from(date: NaiveDate) -> Self {
        Self(date)
    }
}

impl TryFrom<&str> for TrainingNoteDate {
    type Error = chrono::ParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").map(Self)
    }
}

impl TryFrom<String> for TrainingNoteDate {
    type Error = chrono::ParseError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(s.as_str())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrainingNote {
    id: TrainingNoteId,
    user: UserId,
    title: Option<TrainingNoteTitle>,
    content: TrainingNoteContent,
    date: TrainingNoteDate,
    created_at: DateTime<FixedOffset>,
}

impl TrainingNote {
    pub fn new(
        id: TrainingNoteId,
        user: UserId,
        title: Option<TrainingNoteTitle>,
        content: TrainingNoteContent,
        date: TrainingNoteDate,
        created_at: DateTime<FixedOffset>,
    ) -> Self {
        Self {
            id,
            user,
            title,
            content,
            date,
            created_at,
        }
    }

    pub fn id(&self) -> &TrainingNoteId {
        &self.id
    }

    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn title(&self) -> &Option<TrainingNoteTitle> {
        &self.title
    }

    pub fn content(&self) -> &TrainingNoteContent {
        &self.content
    }

    pub fn date(&self) -> &TrainingNoteDate {
        &self.date
    }

    pub fn created_at(&self) -> &DateTime<FixedOffset> {
        &self.created_at
    }

    pub fn to_search_document(
        &self,
        event: SearchDocumentEvent,
        now: chrono::DateTime<chrono::Utc>,
    ) -> SearchDocument {
        let content = [
            self.title()
                .as_ref()
                .map(|name| name.to_string())
                .unwrap_or_default(),
            self.content().to_string(),
        ]
        .join(" ")
        .trim()
        .to_string();
        SearchDocument::new(
            SearchDocumentType::TrainingNote,
            self.id().to_string(),
            self.user.clone(),
            event,
            content,
            now,
        )
    }

    pub fn update(
        self,
        title: Option<TrainingNoteTitle>,
        content: TrainingNoteContent,
        date: TrainingNoteDate,
    ) -> Self {
        Self {
            // Immutable fields
            id: self.id,
            user: self.user,
            created_at: self.created_at,
            // Mutable fields
            title,
            content,
            date,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct TrainingMetricsOrdering(Vec<TrainingMetricId>);

impl TryFrom<Vec<TrainingMetricId>> for TrainingMetricsOrdering {
    type Error = ();

    fn try_from(value: Vec<TrainingMetricId>) -> Result<Self, Self::Error> {
        let mut unique_ids = HashSet::new();
        for id in value.iter() {
            if !unique_ids.insert(id) {
                return Err(());
            }
        }
        Ok(Self(value))
    }
}

impl TrainingMetricsOrdering {
    pub fn sort(&self, mut metrics: Vec<TrainingMetric>) -> Vec<TrainingMetric> {
        let mut sorted_metrics = Vec::new();

        // Sort known metrics to the ordering
        for id in self.0.iter() {
            if let Some(index) = metrics.iter().position(|metric| metric.id == *id) {
                let metric = metrics.swap_remove(index);
                sorted_metrics.push(metric);
            }
        }

        // Sort remaining unknown metrics alphabetically by metric.id
        metrics.sort_by_key(|metric| metric.id().clone());

        sorted_metrics.append(&mut metrics);

        sorted_metrics
    }

    pub fn remove_metric(mut self, id: &TrainingMetricId) -> Self {
        self.0.retain(|_id| _id != id);
        Self(self.0)
    }

    pub fn ids(&self) -> &[TrainingMetricId] {
        &self.0
    }
}

#[cfg(test)]
mod test_training_metric_definition_new {
    use super::*;

    fn activity_source() -> TrainingMetricSource {
        TrainingMetricSource::Activity(ActivitySource::new(
            ActivityMetric::NumberOfActivity,
            TrainingMetricActivityGroupBy::none(),
            TrainingMetricActivityFilters::empty(),
        ))
    }

    fn hooper_source() -> TrainingMetricSource {
        TrainingMetricSource::HooperIndex(HooperIndexSource::Fatigue)
    }

    fn weight_source() -> TrainingMetricSource {
        TrainingMetricSource::WeightAndNutrition(WeightAndNutritionSource::TotalWeight)
    }

    fn summary() -> TrainingMetricSummary {
        TrainingMetricSummary::new(Some(TrainingMetricSummaryAverage::new(true)))
    }

    #[test]
    fn test_activity_source_keeps_none_window() {
        let definition = TrainingMetricDefinition::new(
            UserId::test_default(),
            activity_source(),
            None,
            summary(),
            None,
        );

        assert_eq!(definition.window(), &None);
    }

    #[test]
    fn test_activity_source_keeps_provided_window() {
        let window = Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Sum,
        ));
        let definition = TrainingMetricDefinition::new(
            UserId::test_default(),
            activity_source(),
            window.clone(),
            summary(),
            None,
        );

        assert_eq!(definition.window(), &window);
    }

    #[test]
    fn test_hooper_source_defaults_to_daily_average_window() {
        let definition = TrainingMetricDefinition::new(
            UserId::test_default(),
            hooper_source(),
            None,
            summary(),
            None,
        );

        assert_eq!(
            definition.window(),
            &Some(TrainingMetricWindow::new(
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            ))
        );
    }

    #[test]
    fn test_hooper_source_keeps_provided_window() {
        let window = Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Max,
        ));
        let definition = TrainingMetricDefinition::new(
            UserId::test_default(),
            hooper_source(),
            window.clone(),
            summary(),
            None,
        );

        assert_eq!(definition.window(), &window);
    }

    #[test]
    fn test_weight_and_nutrition_source_defaults_to_daily_average_window() {
        let definition = TrainingMetricDefinition::new(
            UserId::test_default(),
            weight_source(),
            None,
            summary(),
            None,
        );

        assert_eq!(
            definition.window(),
            &Some(TrainingMetricWindow::new(
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Average,
            ))
        );
    }

    #[test]
    fn test_weight_and_nutrition_source_keeps_provided_window() {
        let window = Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Monthly,
            TrainingMetricAggregate::Min,
        ));
        let definition = TrainingMetricDefinition::new(
            UserId::test_default(),
            weight_source(),
            window.clone(),
            summary(),
            None,
        );

        assert_eq!(definition.window(), &window);
    }

    #[test]
    fn test_fields_are_stored() {
        let user = UserId::test_default();
        let target = Some(TrainingMetricTarget::new(5., Unit::Kilometer));
        let source = activity_source();
        let window = Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Sum,
        ));

        let definition = TrainingMetricDefinition::new(
            user.clone(),
            source.clone(),
            window.clone(),
            summary(),
            target.clone(),
        );

        assert_eq!(definition.user(), &user);
        assert_eq!(definition.source(), &source);
        assert_eq!(definition.window(), &window);
        assert_eq!(definition.summary(), &summary());
        assert_eq!(definition.target(), &target);
    }
}

#[cfg(test)]
mod test_training_metrics {

    use super::*;

    #[test]
    fn test_granularity_bins_daily() {
        let start = "2025-09-03".parse::<chrono::NaiveDate>().unwrap();
        let end = "2025-09-06".parse::<chrono::NaiveDate>().unwrap();
        let granularity = TrainingMetricGranularity::Daily;

        let res = granularity.bins_keys(&start, &end);

        assert_eq!(
            res,
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
        let start = "2025-08-23".parse::<chrono::NaiveDate>().unwrap();
        let end = "2025-09-09".parse::<chrono::NaiveDate>().unwrap();
        let granularity = TrainingMetricGranularity::Weekly;

        let res = granularity.bins_keys(&start, &end);

        assert_eq!(
            res,
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
        let start = "2025-07-23".parse::<chrono::NaiveDate>().unwrap();
        let end = "2025-09-09".parse::<chrono::NaiveDate>().unwrap();
        let granularity = TrainingMetricGranularity::Monthly;

        let res = granularity.bins_keys(&start, &end);

        assert_eq!(
            res,
            vec![
                "2025-07-01".to_string(),
                "2025-08-01".to_string(),
                "2025-09-01".to_string(),
            ]
        )
    }
}

#[cfg(test)]
mod test_training_metric_aggregate_values {
    use super::*;

    fn values(values: &[f64]) -> Vec<IndividualValue> {
        values
            .iter()
            .map(|value| IndividualValue::new(*value))
            .collect()
    }

    #[test]
    fn test_returns_none_for_empty_values() {
        for aggregate in [
            TrainingMetricAggregate::Min,
            TrainingMetricAggregate::Max,
            TrainingMetricAggregate::Average,
            TrainingMetricAggregate::Sum,
            TrainingMetricAggregate::NumberOfActivities,
        ] {
            assert_eq!(
                aggregate.aggregate_values(&[]),
                None,
                "{aggregate} should return None for empty input"
            );
        }
    }

    #[test]
    fn test_min_returns_smallest_value() {
        let aggregate = TrainingMetricAggregate::Min;

        assert_eq!(
            aggregate.aggregate_values(&values(&[3.0, 1.0, 2.0])),
            Some(TrainingMetricValue::Min(1.0))
        );
    }

    #[test]
    fn test_max_returns_largest_value() {
        let aggregate = TrainingMetricAggregate::Max;

        assert_eq!(
            aggregate.aggregate_values(&values(&[3.0, 1.0, 2.0])),
            Some(TrainingMetricValue::Max(3.0))
        );
    }

    #[test]
    fn test_min_and_max_handle_negative_values() {
        assert_eq!(
            TrainingMetricAggregate::Min.aggregate_values(&values(&[-5.0, 3.0, 0.0])),
            Some(TrainingMetricValue::Min(-5.0))
        );
        assert_eq!(
            TrainingMetricAggregate::Max.aggregate_values(&values(&[-5.0, 3.0, 0.0])),
            Some(TrainingMetricValue::Max(3.0))
        );
    }

    #[test]
    fn test_sum_adds_all_values() {
        let aggregate = TrainingMetricAggregate::Sum;

        assert_eq!(
            aggregate.aggregate_values(&values(&[1.0, 2.0, 3.0, 4.0])),
            Some(TrainingMetricValue::Sum(10.0))
        );
    }

    #[test]
    fn test_average_returns_value_sum_and_element_count() {
        let aggregate = TrainingMetricAggregate::Average;

        assert_eq!(
            aggregate.aggregate_values(&values(&[1.0, 2.0, 3.0, 4.0])),
            Some(TrainingMetricValue::Average(2.5))
        );
    }

    #[test]
    fn test_number_of_activities_counts_values() {
        let aggregate = TrainingMetricAggregate::NumberOfActivities;

        assert_eq!(
            aggregate.aggregate_values(&values(&[1.0, 2.0, 3.0])),
            Some(TrainingMetricValue::NumberOfActivities(3))
        );
    }

    #[test]
    fn test_single_value_is_handled_by_every_aggregate() {
        let single = values(&[7.5]);

        assert_eq!(
            TrainingMetricAggregate::Min.aggregate_values(&single),
            Some(TrainingMetricValue::Min(7.5))
        );
        assert_eq!(
            TrainingMetricAggregate::Max.aggregate_values(&single),
            Some(TrainingMetricValue::Max(7.5))
        );
        assert_eq!(
            TrainingMetricAggregate::Sum.aggregate_values(&single),
            Some(TrainingMetricValue::Sum(7.5))
        );
        assert_eq!(
            TrainingMetricAggregate::Average.aggregate_values(&single),
            Some(TrainingMetricValue::Average(7.5))
        );
        assert_eq!(
            TrainingMetricAggregate::NumberOfActivities.aggregate_values(&single),
            Some(TrainingMetricValue::NumberOfActivities(1))
        );
    }

    #[test]
    fn test_duplicate_values_are_all_included() {
        let duplicates = values(&[2.0, 2.0, 2.0]);

        assert_eq!(
            TrainingMetricAggregate::Sum.aggregate_values(&duplicates),
            Some(TrainingMetricValue::Sum(6.0))
        );
        assert_eq!(
            TrainingMetricAggregate::Average.aggregate_values(&duplicates),
            Some(TrainingMetricValue::Average(2.0))
        );
    }
}

#[cfg(test)]
mod test_granularity_bins {
    use crate::domain::ports::DateRange;

    use super::*;

    #[test]
    fn test_daily_granularity() {
        let range = DateTimeRange::new(
            "2025-07-23T12:03:00+02:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
            Some(
                "2025-07-24T17:03:00+02:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
        );

        let bins = TrainingMetricGranularity::Daily.bins_from_datetime(&range);

        assert_eq!(
            bins,
            vec![
                DateRange::new(
                    "2025-07-23".parse::<NaiveDate>().unwrap(),
                    "2025-07-24".parse::<NaiveDate>().unwrap(),
                ),
                DateRange::new(
                    "2025-07-24".parse::<NaiveDate>().unwrap(),
                    "2025-07-25".parse::<NaiveDate>().unwrap(),
                )
            ]
        );
    }

    #[test]
    fn test_daily_granularity_same_day() {
        let range = DateTimeRange::new(
            "2025-07-23T12:03:00+02:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
            Some(
                "2025-07-23T17:03:00+02:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
        );

        let bins = TrainingMetricGranularity::Daily.bins_from_datetime(&range);

        assert_eq!(
            bins,
            vec![DateRange::new(
                "2025-07-23".parse::<NaiveDate>().unwrap(),
                "2025-07-24".parse::<NaiveDate>().unwrap(),
            ),]
        );
    }

    #[test]
    fn test_weekly_granularity() {
        let range = DateTimeRange::new(
            "2025-10-01T12:03:00+02:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
            Some(
                "2025-10-09T17:03:00+02:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
        );

        let bins = TrainingMetricGranularity::Weekly.bins_from_datetime(&range);

        assert_eq!(
            bins,
            vec![
                DateRange::new(
                    "2025-09-29".parse::<NaiveDate>().unwrap(),
                    "2025-10-06".parse::<NaiveDate>().unwrap(),
                ),
                DateRange::new(
                    "2025-10-06".parse::<NaiveDate>().unwrap(),
                    "2025-10-13".parse::<NaiveDate>().unwrap(),
                )
            ]
        );
    }

    #[test]
    fn test_weekly_granularity_same_week() {
        let range = DateTimeRange::new(
            "2025-10-01T12:03:00+02:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
            Some(
                "2025-10-05T17:03:00+02:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
        );

        let bins = TrainingMetricGranularity::Weekly.bins_from_datetime(&range);

        assert_eq!(
            bins,
            vec![DateRange::new(
                "2025-09-29".parse::<NaiveDate>().unwrap(),
                "2025-10-06".parse::<NaiveDate>().unwrap(),
            ),]
        );
    }

    #[test]
    fn test_monthly_granularity() {
        let range = DateTimeRange::new(
            "2025-09-14T12:03:00+02:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
            Some(
                "2025-10-09T17:03:00+02:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
        );

        let bins = TrainingMetricGranularity::Monthly.bins_from_datetime(&range);

        assert_eq!(
            bins,
            vec![
                DateRange::new(
                    "2025-09-01".parse::<NaiveDate>().unwrap(),
                    "2025-10-01".parse::<NaiveDate>().unwrap()
                ),
                DateRange::new(
                    "2025-10-01".parse::<NaiveDate>().unwrap(),
                    "2025-11-01".parse::<NaiveDate>().unwrap(),
                )
            ]
        );
    }

    #[test]
    fn test_monthly_granularity_same_month() {
        let range = DateTimeRange::new(
            "2025-10-01T12:03:00+02:00"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
            Some(
                "2025-10-05T17:03:00+02:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
        );

        let bins = TrainingMetricGranularity::Monthly.bins_from_datetime(&range);

        assert_eq!(
            bins,
            vec![DateRange::new(
                "2025-10-01".parse::<NaiveDate>().unwrap(),
                "2025-11-01".parse::<NaiveDate>().unwrap(),
            ),]
        );
    }
}

#[cfg(test)]
mod test_granularity_key {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn test_daily_date_key_is_the_date_itself() {
        assert_eq!(
            TrainingMetricGranularity::Daily.date_key(&date(2025, 10, 1)),
            "2025-10-01"
        );
        assert_eq!(
            TrainingMetricGranularity::Daily.date_key(&date(2025, 10, 5)),
            "2025-10-05"
        );
    }

    #[test]
    fn test_weekly_date_key_on_monday_is_the_same_day() {
        // 2025-09-29 is a Monday.
        assert_eq!(
            TrainingMetricGranularity::Weekly.date_key(&date(2025, 9, 29)),
            "2025-09-29"
        );
    }

    #[test]
    fn test_weekly_date_key_mid_week_returns_monday() {
        // 2025-10-01 is a Wednesday of the week starting 2025-09-29.
        assert_eq!(
            TrainingMetricGranularity::Weekly.date_key(&date(2025, 10, 1)),
            "2025-09-29"
        );
    }

    #[test]
    fn test_weekly_date_key_on_sunday_returns_preceding_monday() {
        // 2025-10-05 is a Sunday of the week starting 2025-09-29.
        assert_eq!(
            TrainingMetricGranularity::Weekly.date_key(&date(2025, 10, 5)),
            "2025-09-29"
        );
    }

    #[test]
    fn test_weekly_date_key_spans_month_and_year_boundaries() {
        // 2026-01-01 is a Thursday of the week starting 2025-12-29.
        assert_eq!(
            TrainingMetricGranularity::Weekly.date_key(&date(2026, 1, 1)),
            "2025-12-29"
        );
    }

    #[test]
    fn test_monthly_date_key_returns_first_day_of_month() {
        assert_eq!(
            TrainingMetricGranularity::Monthly.date_key(&date(2025, 9, 14)),
            "2025-09-01"
        );
        assert_eq!(
            TrainingMetricGranularity::Monthly.date_key(&date(2025, 9, 30)),
            "2025-09-01"
        );
    }

    #[test]
    fn test_monthly_date_key_on_first_is_the_same_day() {
        assert_eq!(
            TrainingMetricGranularity::Monthly.date_key(&date(2025, 10, 1)),
            "2025-10-01"
        );
    }

    #[test]
    fn test_datetime_key_uses_the_datetime_date_portion() {
        // The naive date is 2025-10-01 even though the instant is 2025-10-02 UTC.
        let dt = "2025-10-01T23:30:00-05:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();

        assert_eq!(
            TrainingMetricGranularity::Daily.datetime_key(&dt),
            "2025-10-01"
        );
        assert_eq!(
            TrainingMetricGranularity::Weekly.datetime_key(&dt),
            "2025-09-29"
        );
        assert_eq!(
            TrainingMetricGranularity::Monthly.datetime_key(&dt),
            "2025-10-01"
        );
    }

    #[test]
    fn test_datetime_key_matches_date_key_for_the_same_date() {
        let dt = "2025-10-05T08:00:00+02:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        let naive = date(2025, 10, 5);

        for granularity in [
            TrainingMetricGranularity::Daily,
            TrainingMetricGranularity::Weekly,
            TrainingMetricGranularity::Monthly,
        ] {
            assert_eq!(granularity.datetime_key(&dt), granularity.date_key(&naive));
        }
    }
}

#[cfg(test)]
mod test_training_metric_filters {
    use crate::domain::models::activity::{
        ActivityDuration, ActivityFeedback, ActivityId, ActivityName, ActivityNutrition,
        ActivityStartTime,
    };

    use super::*;

    // Helper functions to reduce test boilerplate
    fn default_start_time() -> ActivityStartTime {
        ActivityStartTime::new(
            "2025-09-03T00:00:00Z"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
        )
    }

    fn create_activity_with_sport(sport: Sport) -> Activity {
        Activity::new_empty(
            ActivityId::default(),
            UserId::test_default(),
            default_start_time(),
            ActivityDuration::default(),
            sport,
        )
    }

    fn create_activity_with_workout_type(workout_type: WorkoutType) -> Activity {
        Activity::new(
            ActivityId::default(),
            UserId::test_default(),
            ActivityName::empty(),
            default_start_time(),
            ActivityDuration::default(),
            Sport::Running,
            ActivityRpe::empty(),
            Some(workout_type),
            ActivityNutrition::empty(),
            ActivityFeedback::empty(),
        )
    }

    fn create_activity_with_bonk_status(bonk_status: BonkStatus) -> Activity {
        Activity::new(
            ActivityId::default(),
            UserId::test_default(),
            ActivityName::empty(),
            default_start_time(),
            ActivityDuration::default(),
            Sport::Running,
            ActivityRpe::empty(),
            WorkoutType::empty(),
            Some(ActivityNutrition::new(bonk_status, None)),
            ActivityFeedback::empty(),
        )
    }

    fn create_activity_with_rpe(rpe: ActivityRpe) -> Activity {
        Activity::new(
            ActivityId::default(),
            UserId::test_default(),
            ActivityName::empty(),
            default_start_time(),
            ActivityDuration::default(),
            Sport::Running,
            Some(rpe),
            WorkoutType::empty(),
            ActivityNutrition::empty(),
            ActivityFeedback::empty(),
        )
    }

    fn create_activity_with_all_filters(
        sport: Sport,
        workout_type: WorkoutType,
        bonk_status: BonkStatus,
    ) -> Activity {
        Activity::new(
            ActivityId::default(),
            UserId::test_default(),
            ActivityName::empty(),
            default_start_time(),
            ActivityDuration::default(),
            sport,
            Some(ActivityRpe::Eight),
            Some(workout_type),
            Some(ActivityNutrition::new(bonk_status, None)),
            ActivityFeedback::empty(),
        )
    }

    fn create_activity_without_optional_fields() -> Activity {
        Activity::new_empty(
            ActivityId::default(),
            UserId::test_default(),
            default_start_time(),
            ActivityDuration::default(),
            Sport::Running,
        )
    }

    fn create_filter_with_sports(sports: Vec<SportFilter>) -> TrainingMetricActivityFilters {
        TrainingMetricActivityFilters::new(Some(sports), None, None, None)
    }

    fn create_filter_with_workout_types(
        workout_types: Vec<WorkoutType>,
    ) -> TrainingMetricActivityFilters {
        TrainingMetricActivityFilters::new(None, Some(workout_types), None, None)
    }

    fn create_filter_with_bonk_status(bonk_status: BonkStatus) -> TrainingMetricActivityFilters {
        TrainingMetricActivityFilters::new(None, None, Some(bonk_status), None)
    }

    fn create_filter_with_rpes(rpes: Vec<ActivityRpe>) -> TrainingMetricActivityFilters {
        TrainingMetricActivityFilters::new(None, None, None, Some(rpes))
    }

    #[test]
    fn test_sport_filter_matches_activity() {
        let activity = create_activity_with_sport(Sport::IndoorCycling);

        assert!(SportFilter::Sport(Sport::IndoorCycling).matches(&activity));
        assert!(!SportFilter::Sport(Sport::Cycling).matches(&activity));

        assert!(SportFilter::SportCategory(SportCategory::Cycling).matches(&activity));
        assert!(!SportFilter::SportCategory(SportCategory::Running).matches(&activity));
    }

    #[test]
    fn test_sport_filter_smallest_compatible() {
        // Same category
        assert_eq!(
            SportFilter::SportCategory(SportCategory::Running)
                .smallest_compatible(&SportFilter::SportCategory(SportCategory::Running)),
            Some(SportFilter::SportCategory(SportCategory::Running))
        );

        // Category with one of its child sport
        assert_eq!(
            SportFilter::SportCategory(SportCategory::Running)
                .smallest_compatible(&SportFilter::Sport(Sport::TrailRunning)),
            Some(SportFilter::Sport(Sport::TrailRunning))
        );

        // Category with a sport from another category
        assert_eq!(
            SportFilter::SportCategory(SportCategory::Running)
                .smallest_compatible(&SportFilter::Sport(Sport::AlpineSki)),
            None
        );

        // Sport with itself
        assert_eq!(
            SportFilter::Sport(Sport::TrailRunning)
                .smallest_compatible(&SportFilter::Sport(Sport::TrailRunning)),
            Some(SportFilter::Sport(Sport::TrailRunning))
        );

        // Sport with another sport
        assert_eq!(
            SportFilter::Sport(Sport::TrailRunning)
                .smallest_compatible(&SportFilter::Sport(Sport::Rowing)),
            None
        );

        // Sport with another sport fromt he same category
        assert_eq!(
            SportFilter::Sport(Sport::TrailRunning)
                .smallest_compatible(&SportFilter::Sport(Sport::TrackRunning)),
            None
        );

        // Sport with its parent category
        assert_eq!(
            SportFilter::Sport(Sport::TrailRunning)
                .smallest_compatible(&SportFilter::SportCategory(SportCategory::Running)),
            Some(SportFilter::Sport(Sport::TrailRunning))
        );

        // Sport with another category
        assert_eq!(
            SportFilter::Sport(Sport::TrailRunning)
                .smallest_compatible(&SportFilter::SportCategory(SportCategory::Cycling)),
            None
        );
    }

    #[test]
    fn test_filter_by_sport() {
        let activity = create_activity_with_sport(Sport::Cycling);

        assert!(
            create_filter_with_sports(vec![SportFilter::Sport(Sport::Cycling)]).matches(&activity)
        );
        assert!(
            create_filter_with_sports(vec![
                SportFilter::Sport(Sport::Cycling),
                SportFilter::Sport(Sport::Running)
            ])
            .matches(&activity)
        );
        assert!(
            !create_filter_with_sports(vec![SportFilter::Sport(Sport::Running)]).matches(&activity)
        );
        assert!(!create_filter_with_sports(vec![]).matches(&activity));
    }

    #[test]
    fn test_filter_by_workout_type() {
        let activity_with_workout = create_activity_with_workout_type(WorkoutType::Tempo);
        let activity_without_workout = create_activity_without_optional_fields();

        // Should match when activity has one of the specified workout types
        assert!(
            create_filter_with_workout_types(vec![WorkoutType::Tempo])
                .matches(&activity_with_workout)
        );
        assert!(
            create_filter_with_workout_types(vec![WorkoutType::Tempo, WorkoutType::Easy])
                .matches(&activity_with_workout)
        );

        // Should not match when activity has different workout type
        assert!(
            !create_filter_with_workout_types(vec![WorkoutType::Easy])
                .matches(&activity_with_workout)
        );

        // Should not match when activity has no workout type
        assert!(
            !create_filter_with_workout_types(vec![WorkoutType::Tempo])
                .matches(&activity_without_workout)
        );

        // Empty list should not match anything
        assert!(!create_filter_with_workout_types(vec![]).matches(&activity_with_workout));
    }

    #[test]
    fn test_filter_by_bonked_status() {
        let activity_bonked = create_activity_with_bonk_status(BonkStatus::Bonked);
        let activity_not_bonked = create_activity_with_bonk_status(BonkStatus::None);
        let activity_no_nutrition = create_activity_without_optional_fields();

        // Should match when activity has matching bonk status
        assert!(create_filter_with_bonk_status(BonkStatus::Bonked).matches(&activity_bonked));
        assert!(create_filter_with_bonk_status(BonkStatus::None).matches(&activity_not_bonked));

        // Should not match when bonk status is different
        assert!(!create_filter_with_bonk_status(BonkStatus::Bonked).matches(&activity_not_bonked));
        assert!(!create_filter_with_bonk_status(BonkStatus::None).matches(&activity_bonked));

        // Should not match when activity has no nutrition data
        assert!(
            !create_filter_with_bonk_status(BonkStatus::Bonked).matches(&activity_no_nutrition)
        );
    }

    #[test]
    fn test_filter_by_rpe() {
        let activity_rpe_eight = create_activity_with_rpe(ActivityRpe::Eight);
        let activity_rpe_five = create_activity_with_rpe(ActivityRpe::Five);
        let activity_no_rpe = create_activity_without_optional_fields();

        // Should match when activity has one of the specified RPEs
        assert!(create_filter_with_rpes(vec![ActivityRpe::Eight]).matches(&activity_rpe_eight));
        assert!(
            create_filter_with_rpes(vec![ActivityRpe::Eight, ActivityRpe::Five])
                .matches(&activity_rpe_eight)
        );
        assert!(
            create_filter_with_rpes(vec![ActivityRpe::Eight, ActivityRpe::Five])
                .matches(&activity_rpe_five)
        );

        // Should not match when activity has different RPE
        assert!(!create_filter_with_rpes(vec![ActivityRpe::Five]).matches(&activity_rpe_eight));
        assert!(!create_filter_with_rpes(vec![ActivityRpe::Eight]).matches(&activity_rpe_five));

        // Should not match when activity has no RPE
        assert!(!create_filter_with_rpes(vec![ActivityRpe::Eight]).matches(&activity_no_rpe));

        // Empty list should not match anything
        assert!(!create_filter_with_rpes(vec![]).matches(&activity_rpe_eight));
    }

    #[test]
    fn test_filter_by_multiple_criteria() {
        let activity = create_activity_with_all_filters(
            Sport::Running,
            WorkoutType::Tempo,
            BonkStatus::Bonked,
        );

        // Should match when all filters match
        assert!(
            TrainingMetricActivityFilters::new(
                Some(vec![SportFilter::Sport(Sport::Running)]),
                Some(vec![WorkoutType::Tempo]),
                Some(BonkStatus::Bonked),
                Some(vec![ActivityRpe::Eight])
            )
            .matches(&activity)
        );

        // Should not match when sport doesn't match
        assert!(
            !TrainingMetricActivityFilters::new(
                Some(vec![SportFilter::Sport(Sport::Cycling)]),
                Some(vec![WorkoutType::Tempo]),
                Some(BonkStatus::Bonked),
                Some(vec![ActivityRpe::Eight])
            )
            .matches(&activity)
        );

        // Should not match when workout type doesn't match
        assert!(
            !TrainingMetricActivityFilters::new(
                Some(vec![SportFilter::Sport(Sport::Running)]),
                Some(vec![WorkoutType::Easy]),
                Some(BonkStatus::Bonked),
                Some(vec![ActivityRpe::Eight])
            )
            .matches(&activity)
        );

        // Should not match when bonk status doesn't match
        assert!(
            !TrainingMetricActivityFilters::new(
                Some(vec![SportFilter::Sport(Sport::Running)]),
                Some(vec![WorkoutType::Tempo]),
                Some(BonkStatus::None),
                Some(vec![ActivityRpe::Eight])
            )
            .matches(&activity)
        );

        // Should not match when RPE doesn't match
        assert!(
            !TrainingMetricActivityFilters::new(
                Some(vec![SportFilter::Sport(Sport::Running)]),
                Some(vec![WorkoutType::Tempo]),
                Some(BonkStatus::None),
                Some(vec![ActivityRpe::Nine])
            )
            .matches(&activity)
        );
    }

    #[test]
    fn test_filter_with_none_values_matches_any_activity() {
        let activity_cycling = create_activity_with_sport(Sport::Cycling);
        let activity_running = create_activity_with_sport(Sport::Running);
        let activity_with_workout = create_activity_with_workout_type(WorkoutType::Tempo);
        let activity_bonked = create_activity_with_bonk_status(BonkStatus::Bonked);
        let activity_rpe_eight = create_activity_with_rpe(ActivityRpe::Eight);
        let activity_minimal = create_activity_without_optional_fields();

        // When sports filter is None, should match any sport
        let filter_no_sport = TrainingMetricActivityFilters::new(None, None, None, None);
        assert!(filter_no_sport.matches(&activity_cycling));
        assert!(filter_no_sport.matches(&activity_running));

        // When workout_types filter is None, should match any workout type
        let filter_no_workout = TrainingMetricActivityFilters::new(
            Some(vec![SportFilter::Sport(Sport::Running)]),
            None,
            None,
            None,
        );
        assert!(filter_no_workout.matches(&activity_with_workout));
        assert!(filter_no_workout.matches(&activity_minimal));

        // When bonked filter is None, should match any bonk status
        let filter_no_bonk = TrainingMetricActivityFilters::new(
            Some(vec![SportFilter::Sport(Sport::Running)]),
            None,
            None,
            None,
        );
        assert!(filter_no_bonk.matches(&activity_bonked));
        assert!(filter_no_bonk.matches(&activity_minimal));

        // When rpe filter is None, should match any RPE
        let filter_no_rpe = TrainingMetricActivityFilters::new(
            Some(vec![SportFilter::Sport(Sport::Running)]),
            None,
            None,
            None,
        );
        assert!(filter_no_rpe.matches(&activity_rpe_eight));
        assert!(filter_no_rpe.matches(&activity_minimal));

        // Empty filter (all None) should match any activity
        let empty_filter = TrainingMetricActivityFilters::empty();
        assert!(empty_filter.matches(&activity_cycling));
        assert!(empty_filter.matches(&activity_running));
        assert!(empty_filter.matches(&activity_with_workout));
        assert!(empty_filter.matches(&activity_bonked));
        assert!(empty_filter.matches(&activity_rpe_eight));
        assert!(empty_filter.matches(&activity_minimal));
    }

    #[test]
    fn test_merge_none_default_sports_to_none_sport_filter() {
        let filter = TrainingMetricActivityFilters::empty();
        let default_sports = None;

        let new_filter = filter.merge_default_sports(&default_sports);

        assert!(new_filter.sports().is_none(),)
    }

    #[test]
    fn test_merge_none_default_sports_to_sport_filter() {
        let filter = create_filter_with_sports(vec![SportFilter::Sport(Sport::AlpineSki)]);
        let default_sports = None;

        let new_filter = filter.merge_default_sports(&default_sports);

        assert_eq!(
            new_filter.sports(),
            &Some(vec![SportFilter::Sport(Sport::AlpineSki)])
        )
    }

    #[test]
    fn test_merge_default_sports_to_none_sport_filter() {
        let filter = TrainingMetricActivityFilters::empty();
        let default_sports = Some(vec![
            SportFilter::Sport(Sport::AlpineSki),
            SportFilter::Sport(Sport::Running),
        ]);

        let new_filter = filter.merge_default_sports(&default_sports);

        assert_eq!(
            new_filter.sports(),
            &Some(vec![
                SportFilter::Sport(Sport::AlpineSki),
                SportFilter::Sport(Sport::Running)
            ])
        )
    }

    #[test]
    fn test_merge_default_sports_to_sport_filter_included_in_default() {
        let filter = create_filter_with_sports(vec![SportFilter::Sport(Sport::AlpineSki)]);
        let default_sports = Some(vec![
            SportFilter::Sport(Sport::AlpineSki),
            SportFilter::Sport(Sport::Running),
        ]);

        let new_filter = filter.merge_default_sports(&default_sports);

        assert_eq!(
            new_filter.sports(),
            &Some(vec![SportFilter::Sport(Sport::AlpineSki),])
        )
    }

    #[test]
    fn test_merge_default_sports_to_sport_filter_not_included_in_default() {
        let filter = create_filter_with_sports(vec![
            SportFilter::Sport(Sport::AlpineSki),
            SportFilter::Sport(Sport::Rugby),
        ]);
        let default_sports = Some(vec![
            SportFilter::Sport(Sport::AlpineSki),
            SportFilter::Sport(Sport::Running),
        ]);

        let new_filter = filter.merge_default_sports(&default_sports);

        assert_eq!(
            new_filter.sports(),
            &Some(vec![SportFilter::Sport(Sport::AlpineSki),])
        )
    }

    #[test]
    fn test_merge_default_sports_to_sport_filter_with_category() {
        let filter =
            create_filter_with_sports(vec![SportFilter::SportCategory(SportCategory::Running)]);
        let default_sports = Some(vec![
            SportFilter::Sport(Sport::TrackRunning),
            SportFilter::Sport(Sport::TrailRunning),
        ]);

        let new_filter = filter.merge_default_sports(&default_sports);

        assert_eq!(
            new_filter.sports(),
            &Some(vec![
                SportFilter::Sport(Sport::TrackRunning),
                SportFilter::Sport(Sport::TrailRunning),
            ])
        )
    }

    #[test]
    fn test_merge_category_default_sports_to_sport_filter() {
        let filter = create_filter_with_sports(vec![
            SportFilter::Sport(Sport::TrackRunning),
            SportFilter::Sport(Sport::TrailRunning),
        ]);
        let default_sports = Some(vec![SportFilter::SportCategory(SportCategory::Running)]);

        let new_filter = filter.merge_default_sports(&default_sports);

        assert_eq!(
            new_filter.sports(),
            &Some(vec![
                SportFilter::Sport(Sport::TrackRunning),
                SportFilter::Sport(Sport::TrailRunning),
            ])
        )
    }
}

#[cfg(test)]
mod test_activity_source_extract_values {
    use crate::domain::models::activity::{
        Activity, ActivityDuration, ActivityId, ActivityStartTime,
    };

    use super::*;

    fn activity_at(start: &str, sport: Sport) -> Activity {
        Activity::new_empty(
            ActivityId::default(),
            UserId::test_default(),
            ActivityStartTime::new(start.parse::<DateTime<FixedOffset>>().unwrap()),
            ActivityDuration::default(),
            sport,
        )
    }

    fn daily_window() -> Option<TrainingMetricWindow> {
        Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Sum,
        ))
    }

    fn values<'a>(
        result: &'a HashMap<TrainingMetricBin, Vec<IndividualValue>>,
        granule: &str,
        group: Option<&str>,
    ) -> Vec<f64> {
        let bin = TrainingMetricBin::new(granule.to_string(), group.map(|g| g.to_string()));
        result[&bin].iter().map(|value| value.value()).collect()
    }

    #[test]
    fn test_no_window_bins_by_exact_start_time() {
        let source = ActivitySource::new(
            ActivityMetric::NumberOfActivity,
            TrainingMetricActivityGroupBy::none(),
            TrainingMetricActivityFilters::empty(),
        );
        let activities = vec![
            (activity_at("2025-09-03T10:00:00Z", Sport::Cycling), 10.0),
            (activity_at("2025-09-04T12:00:00Z", Sport::Running), 20.0),
        ];

        let result = source.extract_values(&None, activities.into_iter());

        assert_eq!(result.len(), 2);
        assert_eq!(
            values(&result, "2025-09-03T10:00:00+00:00", None),
            vec![10.0]
        );
        assert_eq!(
            values(&result, "2025-09-04T12:00:00+00:00", None),
            vec![20.0]
        );
    }

    #[test]
    fn test_daily_window_groups_activities_of_same_day() {
        let source = ActivitySource::new(
            ActivityMetric::NumberOfActivity,
            TrainingMetricActivityGroupBy::none(),
            TrainingMetricActivityFilters::empty(),
        );
        let activities = vec![
            (activity_at("2025-09-03T10:00:00Z", Sport::Cycling), 10.0),
            (activity_at("2025-09-03T18:00:00Z", Sport::Running), 20.0),
            (activity_at("2025-09-04T08:00:00Z", Sport::Cycling), 30.0),
        ];

        let result = source.extract_values(&daily_window(), activities.into_iter());

        assert_eq!(result.len(), 2);
        let mut same_day = values(&result, "2025-09-03", None);
        same_day.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(same_day, vec![10.0, 20.0]);
        assert_eq!(values(&result, "2025-09-04", None), vec![30.0]);
    }

    #[test]
    fn test_weekly_window_groups_by_monday_of_week() {
        let source = ActivitySource::new(
            ActivityMetric::NumberOfActivity,
            TrainingMetricActivityGroupBy::none(),
            TrainingMetricActivityFilters::empty(),
        );
        // 2025-09-03 is a Wednesday and 2025-09-07 the Sunday of the same week (Monday
        // 2025-09-01); 2025-09-08 starts the next week.
        let activities = vec![
            (activity_at("2025-09-03T10:00:00Z", Sport::Cycling), 10.0),
            (activity_at("2025-09-07T18:00:00Z", Sport::Running), 20.0),
            (activity_at("2025-09-08T08:00:00Z", Sport::Cycling), 30.0),
        ];
        let window = Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Sum,
        ));

        let result = source.extract_values(&window, activities.into_iter());

        assert_eq!(result.len(), 2);
        let mut same_week = values(&result, "2025-09-01", None);
        same_week.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(same_week, vec![10.0, 20.0]);
        assert_eq!(values(&result, "2025-09-08", None), vec![30.0]);
    }

    #[test]
    fn test_monthly_window_groups_by_first_day_of_month() {
        let source = ActivitySource::new(
            ActivityMetric::NumberOfActivity,
            TrainingMetricActivityGroupBy::none(),
            TrainingMetricActivityFilters::empty(),
        );
        let activities = vec![
            (activity_at("2025-09-03T10:00:00Z", Sport::Cycling), 10.0),
            (activity_at("2025-09-28T18:00:00Z", Sport::Running), 20.0),
            (activity_at("2025-10-15T08:00:00Z", Sport::Cycling), 30.0),
        ];
        let window = Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Monthly,
            TrainingMetricAggregate::Sum,
        ));

        let result = source.extract_values(&window, activities.into_iter());

        assert_eq!(result.len(), 2);
        let mut same_month = values(&result, "2025-09-01", None);
        same_month.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(same_month, vec![10.0, 20.0]);
        assert_eq!(values(&result, "2025-10-01", None), vec![30.0]);
    }

    #[test]
    fn test_filters_exclude_non_matching_activities() {
        let source = ActivitySource::new(
            ActivityMetric::NumberOfActivity,
            TrainingMetricActivityGroupBy::none(),
            TrainingMetricActivityFilters::new(
                Some(vec![SportFilter::Sport(Sport::Cycling)]),
                None,
                None,
                None,
            ),
        );
        let activities = vec![
            (activity_at("2025-09-03T10:00:00Z", Sport::Cycling), 10.0),
            (activity_at("2025-09-03T18:00:00Z", Sport::Running), 20.0),
        ];

        let result = source.extract_values(&daily_window(), activities.into_iter());

        assert_eq!(result.len(), 1);
        assert_eq!(values(&result, "2025-09-03", None), vec![10.0]);
    }

    #[test]
    fn test_group_by_sport_adds_group_to_bin() {
        let source = ActivitySource::new(
            ActivityMetric::NumberOfActivity,
            Some(TrainingMetricActivityGroupBy::Sport),
            TrainingMetricActivityFilters::empty(),
        );
        let activities = vec![
            (activity_at("2025-09-03T10:00:00Z", Sport::Cycling), 10.0),
            (activity_at("2025-09-03T18:00:00Z", Sport::Running), 20.0),
            (activity_at("2025-09-04T08:00:00Z", Sport::Cycling), 30.0),
        ];

        let result = source.extract_values(&daily_window(), activities.into_iter());

        assert_eq!(result.len(), 3);
        assert_eq!(values(&result, "2025-09-03", Some("Cycling")), vec![10.0]);
        assert_eq!(values(&result, "2025-09-03", Some("Running")), vec![20.0]);
        assert_eq!(values(&result, "2025-09-04", Some("Cycling")), vec![30.0]);
    }

    #[test]
    fn test_group_by_none_yields_bin_without_group() {
        let source = ActivitySource::new(
            ActivityMetric::NumberOfActivity,
            TrainingMetricActivityGroupBy::none(),
            TrainingMetricActivityFilters::empty(),
        );
        let activities = vec![
            (activity_at("2025-09-03T10:00:00Z", Sport::Cycling), 10.0),
            (activity_at("2025-09-04T08:00:00Z", Sport::Cycling), 20.0),
        ];

        let result = source.extract_values(&daily_window(), activities.into_iter());

        assert_eq!(result.len(), 2);
        assert_eq!(values(&result, "2025-09-03", None), vec![10.0]);
        assert_eq!(values(&result, "2025-09-04", None), vec![20.0]);
    }
}

#[cfg(test)]
mod test_hooper_index_source_extract_values {
    use super::*;

    fn scale(value: u8) -> SubjectiveScale {
        SubjectiveScale::try_from(value).unwrap()
    }

    fn date(date: &str) -> NaiveDate {
        date.parse::<NaiveDate>().unwrap()
    }

    fn daily_window() -> Option<TrainingMetricWindow> {
        Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
        ))
    }

    fn value_at<'a>(
        result: &'a HashMap<TrainingMetricBin, Vec<IndividualValue>>,
        granule: &str,
    ) -> f64 {
        result[&TrainingMetricBin::from_granule(granule)][0].value()
    }

    #[test]
    fn test_no_window_bins_by_date() {
        let source = HooperIndexSource::Fatigue;
        let values = vec![
            (
                date("2025-09-03"),
                HooperIndex::new(Some(scale(2)), None, None, None, None),
            ),
            (
                date("2025-09-04"),
                HooperIndex::new(Some(scale(5)), None, None, None, None),
            ),
        ];

        let result = source.extract_values(&None, values.into_iter());

        assert_eq!(result.len(), 2);
        assert_eq!(value_at(&result, "2025-09-03"), 2.0);
        assert_eq!(value_at(&result, "2025-09-04"), 5.0);
    }

    #[test]
    fn test_daily_window_groups_activities_of_same_day() {
        let source = HooperIndexSource::Sleep;
        let values = vec![
            (
                date("2025-09-03"),
                HooperIndex::new(None, Some(scale(7)), None, None, None),
            ),
            (
                date("2025-09-04"),
                HooperIndex::new(None, Some(scale(8)), None, None, None),
            ),
        ];

        let result = source.extract_values(&daily_window(), values.into_iter());

        assert_eq!(result.len(), 2);
        assert_eq!(value_at(&result, "2025-09-03"), 7.0);
        assert_eq!(value_at(&result, "2025-09-04"), 8.0);
    }

    #[test]
    fn test_weekly_window_groups_by_monday_of_week() {
        let source = HooperIndexSource::Stress;
        // 2025-09-03 is a Wednesday and 2025-09-07 the Sunday of the week starting
        // Monday 2025-09-01.
        let values = vec![
            (
                date("2025-09-03"),
                HooperIndex::new(None, None, None, Some(scale(3)), None),
            ),
            (
                date("2025-09-07"),
                HooperIndex::new(None, None, None, Some(scale(4)), None),
            ),
        ];
        let window = Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Average,
        ));

        let result = source.extract_values(&window, values.into_iter());

        assert_eq!(result.len(), 1);
        let grouped = &result[&TrainingMetricBin::from_granule("2025-09-01")];
        let mut values = grouped
            .iter()
            .map(|value| value.value())
            .collect::<Vec<_>>();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(values, vec![3.0, 4.0]);
    }

    #[test]
    fn test_skips_entries_missing_the_source_value() {
        let source = HooperIndexSource::Mood;
        let values = vec![
            (
                date("2025-09-03"),
                HooperIndex::new(Some(scale(1)), None, None, None, None),
            ),
            (
                date("2025-09-04"),
                HooperIndex::new(None, None, None, None, Some(scale(6))),
            ),
            (date("2025-09-05"), HooperIndex::default()),
        ];

        let result = source.extract_values(&None, values.into_iter());

        assert_eq!(result.len(), 1);
        assert_eq!(value_at(&result, "2025-09-04"), 6.0);
    }
}

#[cfg(test)]
mod test_weight_and_nutrition_source_extract_values {
    use super::*;

    fn date(date: &str) -> NaiveDate {
        date.parse::<NaiveDate>().unwrap()
    }

    fn daily_window() -> Option<TrainingMetricWindow> {
        Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Daily,
            TrainingMetricAggregate::Average,
        ))
    }

    fn bin(granule: &str, group: Option<&str>) -> TrainingMetricBin {
        TrainingMetricBin::new(granule.to_string(), group.map(|g| g.to_string()))
    }

    fn value_at(result: &HashMap<TrainingMetricBin, Vec<IndividualValue>>, granule: &str) -> f64 {
        result[&bin(granule, None)][0].value()
    }

    fn grouped_value_at(
        result: &HashMap<TrainingMetricBin, Vec<IndividualValue>>,
        granule: &str,
        group: &str,
    ) -> f64 {
        result[&bin(granule, Some(group))][0].value()
    }

    #[test]
    fn test_no_window_bins_by_date() {
        let source = WeightAndNutritionSource::TotalWeight;
        let values = vec![
            (
                date("2025-09-03"),
                WeightAndNutrition {
                    weight: Some(75.5),
                    ..Default::default()
                },
            ),
            (
                date("2025-09-04"),
                WeightAndNutrition {
                    weight: Some(76.0),
                    ..Default::default()
                },
            ),
        ];

        let result = source.extract_values(&None, values.into_iter());

        assert_eq!(result.len(), 2);
        assert_eq!(value_at(&result, "2025-09-03"), 75.5);
        assert_eq!(value_at(&result, "2025-09-04"), 76.0);
    }

    #[test]
    fn test_ungrouped_sources_bin_a_single_value_without_group_per_date() {
        // TotalWeight, Calories, Water and Alcohol each extract a single
        // value per date, so they bin it without a group.
        let cases = [
            (
                WeightAndNutritionSource::TotalWeight,
                75.5,
                WeightAndNutrition {
                    weight: Some(75.5),
                    ..Default::default()
                },
            ),
            (
                WeightAndNutritionSource::Calories,
                2500.0,
                WeightAndNutrition {
                    calories: Some(2500.0),
                    ..Default::default()
                },
            ),
            (
                WeightAndNutritionSource::Water,
                2.0,
                WeightAndNutrition {
                    water: Some(2.0),
                    ..Default::default()
                },
            ),
            (
                WeightAndNutritionSource::Alcohol,
                1.5,
                WeightAndNutrition {
                    alcohol: Some(1.5),
                    ..Default::default()
                },
            ),
        ];

        for (source, expected, value) in cases {
            let values = vec![(date("2025-09-03"), value), (date("2025-09-04"), value)];

            let result = source.extract_values(&None, values.into_iter());

            assert_eq!(result.len(), 2);
            assert!(result.keys().all(|k| k.group().is_none()));
            assert_eq!(value_at(&result, "2025-09-03"), expected);
            assert_eq!(value_at(&result, "2025-09-04"), expected);
        }
    }

    #[test]
    fn test_ungrouped_sources_ignore_other_fields() {
        let value = WeightAndNutrition {
            weight: Some(75.0),
            fat: Some(20.0),
            muscle: Some(40.0),
            calories: Some(2500.0),
            lipid: Some(70.0),
            carbs: Some(300.0),
            protein: Some(100.0),
            water: Some(2.0),
            alcohol: Some(1.0),
        };
        let cases = [
            (WeightAndNutritionSource::TotalWeight, 75.0),
            (WeightAndNutritionSource::Calories, 2500.0),
            (WeightAndNutritionSource::Water, 2.0),
            (WeightAndNutritionSource::Alcohol, 1.0),
        ];

        for (source, expected) in cases {
            let result =
                source.extract_values(&None, vec![(date("2025-09-03"), value)].into_iter());

            assert_eq!(result.len(), 1);
            assert_eq!(value_at(&result, "2025-09-03"), expected);
        }
    }

    #[test]
    fn test_monthly_window_groups_by_first_day_of_month() {
        let source = WeightAndNutritionSource::Calories;
        let values = vec![
            (
                date("2025-09-03"),
                WeightAndNutrition {
                    calories: Some(2500.0),
                    ..Default::default()
                },
            ),
            (
                date("2025-09-28"),
                WeightAndNutrition {
                    calories: Some(2600.0),
                    ..Default::default()
                },
            ),
            (
                date("2025-10-15"),
                WeightAndNutrition {
                    calories: Some(2700.0),
                    ..Default::default()
                },
            ),
        ];
        let window = Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Monthly,
            TrainingMetricAggregate::Average,
        ));

        let result = source.extract_values(&window, values.into_iter());

        assert_eq!(result.len(), 2);
        let september = &result[&TrainingMetricBin::from_granule("2025-09-01")];
        let mut values = september
            .iter()
            .map(|value| value.value())
            .collect::<Vec<_>>();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(values, vec![2500.0, 2600.0]);
        assert_eq!(value_at(&result, "2025-10-01"), 2700.0);
    }

    #[test]
    fn test_extracts_the_requested_field() {
        let source = WeightAndNutritionSource::Water;
        let values = vec![(
            date("2025-09-03"),
            WeightAndNutrition {
                weight: Some(75.0),
                calories: Some(2500.0),
                water: Some(120.0),
                ..Default::default()
            },
        )];

        let result = source.extract_values(&daily_window(), values.into_iter());

        assert_eq!(result.len(), 1);
        assert_eq!(value_at(&result, "2025-09-03"), 120.0);
    }

    #[test]
    fn test_skips_entries_missing_the_source_value() {
        let source = WeightAndNutritionSource::Water;
        let values = vec![
            (
                date("2025-09-03"),
                WeightAndNutrition {
                    weight: Some(75.0),
                    ..Default::default()
                },
            ),
            (
                date("2025-09-04"),
                WeightAndNutrition {
                    weight: Some(76.0),
                    water: Some(15.0),
                    ..Default::default()
                },
            ),
        ];

        let result = source.extract_values(&None, values.into_iter());

        assert_eq!(result.len(), 1);
        assert_eq!(value_at(&result, "2025-09-04"), 15.0);
    }

    #[test]
    fn test_body_composition_groups_fat_and_muscle_into_separate_bins() {
        let source = WeightAndNutritionSource::BodyComposition;
        let values = vec![(
            date("2025-09-03"),
            WeightAndNutrition {
                fat: Some(20.0),
                muscle: Some(40.0),
                ..Default::default()
            },
        )];

        let result = source.extract_values(&daily_window(), values.into_iter());

        assert_eq!(result.len(), 2);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "fat"), 20.0);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "muscle"), 40.0);
    }

    #[test]
    fn test_body_composition_skips_missing_values() {
        let source = WeightAndNutritionSource::BodyComposition;
        let values = vec![
            (
                date("2025-09-03"),
                WeightAndNutrition {
                    fat: Some(20.0),
                    ..Default::default()
                },
            ),
            (
                date("2025-09-04"),
                WeightAndNutrition {
                    muscle: Some(41.0),
                    ..Default::default()
                },
            ),
        ];

        let result = source.extract_values(&daily_window(), values.into_iter());

        assert_eq!(result.len(), 2);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "fat"), 20.0);
        assert_eq!(grouped_value_at(&result, "2025-09-04", "muscle"), 41.0);
    }

    #[test]
    fn test_body_composition_ignores_other_fields() {
        let source = WeightAndNutritionSource::BodyComposition;
        let values = vec![(
            date("2025-09-03"),
            WeightAndNutrition {
                weight: Some(75.0),
                fat: Some(20.0),
                muscle: Some(40.0),
                calories: Some(2500.0),
                ..Default::default()
            },
        )];

        let result = source.extract_values(&daily_window(), values.into_iter());

        assert_eq!(result.len(), 2);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "fat"), 20.0);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "muscle"), 40.0);
    }

    #[test]
    fn test_macros_groups_lipids_carbs_and_proteins_into_separate_bins() {
        let source = WeightAndNutritionSource::Macros;
        let values = vec![(
            date("2025-09-03"),
            WeightAndNutrition {
                lipid: Some(70.0),
                carbs: Some(300.0),
                protein: Some(100.0),
                ..Default::default()
            },
        )];

        let result = source.extract_values(&daily_window(), values.into_iter());

        assert_eq!(result.len(), 3);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "lipids"), 70.0);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "carbs"), 300.0);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "proteins"), 100.0);
    }

    #[test]
    fn test_macros_skips_missing_values() {
        let source = WeightAndNutritionSource::Macros;
        let values = vec![(
            date("2025-09-03"),
            WeightAndNutrition {
                carbs: Some(300.0),
                ..Default::default()
            },
        )];

        let result = source.extract_values(&daily_window(), values.into_iter());

        assert_eq!(result.len(), 1);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "carbs"), 300.0);
    }

    #[test]
    fn test_macros_ignores_other_fields() {
        let source = WeightAndNutritionSource::Macros;
        let values = vec![(
            date("2025-09-03"),
            WeightAndNutrition {
                weight: Some(75.0),
                lipid: Some(70.0),
                carbs: Some(300.0),
                protein: Some(100.0),
                water: Some(2.0),
                ..Default::default()
            },
        )];

        let result = source.extract_values(&daily_window(), values.into_iter());

        assert_eq!(result.len(), 3);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "lipids"), 70.0);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "carbs"), 300.0);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "proteins"), 100.0);
    }

    #[test]
    fn test_monthly_window_groups_grouped_sources_by_first_day_of_month() {
        let source = WeightAndNutritionSource::Macros;
        let values = vec![
            (
                date("2025-09-03"),
                WeightAndNutrition {
                    lipid: Some(70.0),
                    ..Default::default()
                },
            ),
            (
                date("2025-09-28"),
                WeightAndNutrition {
                    lipid: Some(80.0),
                    ..Default::default()
                },
            ),
            (
                date("2025-10-15"),
                WeightAndNutrition {
                    lipid: Some(90.0),
                    ..Default::default()
                },
            ),
        ];
        let window = Some(TrainingMetricWindow::new(
            TrainingMetricGranularity::Monthly,
            TrainingMetricAggregate::Average,
        ));

        let result = source.extract_values(&window, values.into_iter());

        assert_eq!(result.len(), 2);
        let september = &result[&bin("2025-09-01", Some("lipids"))];
        let mut values = september
            .iter()
            .map(|value| value.value())
            .collect::<Vec<_>>();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(values, vec![70.0, 80.0]);
        assert_eq!(grouped_value_at(&result, "2025-10-01", "lipids"), 90.0);
    }

    #[test]
    fn test_grouped_bins_only_exist_for_their_own_granule() {
        // Dates falling in different windows must not share a bin,
        // even though the group names are the same.
        let source = WeightAndNutritionSource::BodyComposition;
        let values = vec![
            (
                date("2025-09-03"),
                WeightAndNutrition {
                    fat: Some(20.0),
                    ..Default::default()
                },
            ),
            (
                date("2025-10-15"),
                WeightAndNutrition {
                    fat: Some(21.0),
                    ..Default::default()
                },
            ),
        ];

        let result = source.extract_values(&daily_window(), values.into_iter());

        assert_eq!(result.len(), 2);
        assert_eq!(grouped_value_at(&result, "2025-09-03", "fat"), 20.0);
        assert_eq!(grouped_value_at(&result, "2025-10-15", "fat"), 21.0);
    }
}

#[cfg(test)]
mod test_training_period {

    use crate::domain::models::activity::{ActivityDuration, ActivityId, ActivityStartTime};

    use super::*;

    #[test]
    fn test_create_training_period() {
        let id = TrainingPeriodId::new();
        let user = UserId::test_default();
        let start = "2025-10-17".parse::<NaiveDate>().unwrap();
        let name = "test period".to_string();
        let sports = TrainingPeriodSports::new(None);
        let note = None;

        let end = None;
        assert!(
            TrainingPeriod::new(
                id.clone(),
                user.clone(),
                start,
                end,
                name.clone(),
                sports.clone(),
                note.clone()
            )
            .is_ok()
        );

        let end = Some("2025-10-18".parse::<NaiveDate>().unwrap());
        assert!(
            TrainingPeriod::new(
                id.clone(),
                user.clone(),
                start,
                end,
                name.clone(),
                sports.clone(),
                note.clone()
            )
            .is_ok()
        );

        let end = Some("2025-10-16".parse::<NaiveDate>().unwrap());
        assert!(
            TrainingPeriod::new(
                id.clone(),
                user.clone(),
                start,
                end,
                name.clone(),
                sports.clone(),
                note.clone()
            )
            .is_err()
        );
    }

    fn activity_with_start_time(start: &str) -> Activity {
        Activity::new_empty(
            ActivityId::default(),
            UserId::test_default(),
            ActivityStartTime::new(start.parse::<DateTime<FixedOffset>>().unwrap()),
            ActivityDuration::default(),
            Sport::Running,
        )
    }

    fn activity_with_sport(sport: Sport) -> Activity {
        Activity::new_empty(
            ActivityId::default(),
            UserId::test_default(),
            ActivityStartTime::new(
                "2025-10-01T12:00:00+02:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            ActivityDuration::default(),
            sport,
        )
    }

    #[test]
    fn test_closed_period_match_activity() {
        let id = TrainingPeriodId::new();
        let user = UserId::test_default();
        let end = Some("2025-10-18".parse::<NaiveDate>().unwrap());
        let period = TrainingPeriod::new(
            id.clone(),
            user.clone(),
            "2025-09-17".parse::<NaiveDate>().unwrap(),
            end,
            "test period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        assert!(!period.matches(&activity_with_start_time("2025-09-16T12:00:00+02:00")));
        assert!(period.matches(&activity_with_start_time("2025-10-01T12:00:00+02:00")));
        assert!(!period.matches(&activity_with_start_time("2025-10-19T12:00:00+02:00")));
    }

    #[test]
    fn test_open_period_match_activity() {
        let id = TrainingPeriodId::new();
        let user = UserId::test_default();
        let end = None;
        let period = TrainingPeriod::new(
            id.clone(),
            user.clone(),
            "2025-09-17".parse::<NaiveDate>().unwrap(),
            end,
            "test period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        assert!(!period.matches(&activity_with_start_time("2025-09-16T12:00:00+02:00")));
        assert!(period.matches(&activity_with_start_time("2025-10-01T12:00:00+02:00")));
        assert!(period.matches(&activity_with_start_time("2025-10-19T12:00:00+02:00")));
    }

    #[test]
    fn test_no_sport_period_match_activity_sport() {
        let id = TrainingPeriodId::new();
        let user = UserId::test_default();
        let end = Some("2025-10-18".parse::<NaiveDate>().unwrap());
        let period = TrainingPeriod::new(
            id.clone(),
            user.clone(),
            "2025-09-17".parse::<NaiveDate>().unwrap(),
            end,
            "test period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        assert!(period.matches(&activity_with_sport(Sport::Running)));
        assert!(period.matches(&activity_with_sport(Sport::Cycling)));
        assert!(period.matches(&activity_with_sport(Sport::StrengthTraining)));
    }

    #[test]
    fn test_period_with_sport_match_activity_sport() {
        let id = TrainingPeriodId::new();
        let user = UserId::test_default();
        let end = Some("2025-10-18".parse::<NaiveDate>().unwrap());
        let period = TrainingPeriod::new(
            id.clone(),
            user.clone(),
            "2025-09-17".parse::<NaiveDate>().unwrap(),
            end,
            "test period".to_string(),
            TrainingPeriodSports::new(Some(vec![SportFilter::Sport(Sport::Running)])),
            None,
        )
        .unwrap();

        assert!(period.matches(&activity_with_sport(Sport::Running)));
        assert!(!period.matches(&activity_with_sport(Sport::TrackRunning)));
        assert!(!period.matches(&activity_with_sport(Sport::Cycling)));
        assert!(!period.matches(&activity_with_sport(Sport::StrengthTraining)));
    }

    #[test]
    fn test_period_with_sport_category_match_activity_sport() {
        let id = TrainingPeriodId::new();
        let user = UserId::test_default();
        let end = Some("2025-10-18".parse::<NaiveDate>().unwrap());
        let period = TrainingPeriod::new(
            id.clone(),
            user.clone(),
            "2025-09-17".parse::<NaiveDate>().unwrap(),
            end,
            "test period".to_string(),
            TrainingPeriodSports::new(Some(vec![SportFilter::SportCategory(
                SportCategory::Running,
            )])),
            None,
        )
        .unwrap();

        assert!(period.matches(&activity_with_sport(Sport::Running)));
        assert!(period.matches(&activity_with_sport(Sport::TrackRunning)));
        assert!(!period.matches(&activity_with_sport(Sport::Cycling)));
        assert!(!period.matches(&activity_with_sport(Sport::StrengthTraining)));
    }

    #[test]
    fn test_period_range_with_end_date() {
        let id = TrainingPeriodId::new();
        let user = UserId::test_default();
        let start = "2025-10-17".parse::<NaiveDate>().unwrap();
        let end = "2025-10-21".parse::<NaiveDate>().unwrap();

        let period = TrainingPeriod::new(
            id,
            user,
            start,
            Some(end),
            "test period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        // Both methods should return the same result when end date is specified
        let range_today = period.range_default_today();
        let range_tomorrow = period.range_default_tomorrow();
        let expected_end = end;

        assert_eq!(range_today.start(), &start);
        assert_eq!(range_today.end(), &expected_end);
        assert_eq!(range_tomorrow.start(), &start);
        assert_eq!(range_tomorrow.end(), &expected_end);
    }

    #[test]
    fn test_period_range_default_today() {
        use chrono::Utc;

        let id = TrainingPeriodId::new();
        let user = UserId::test_default();
        let start = "2025-10-17".parse::<NaiveDate>().unwrap();
        let end = None; // Open-ended period

        let period = TrainingPeriod::new(
            id,
            user,
            start,
            end,
            "test period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        let range = period.range_default_today();
        let today = Utc::now().date_naive();

        assert_eq!(range.start(), &start);
        // Should use today as end (exclusive, so won't include today's activities)
        assert_eq!(range.end(), &today);
    }

    #[test]
    fn test_period_range_default_tomorrow() {
        use chrono::{Days, Utc};

        let id = TrainingPeriodId::new();
        let user = UserId::test_default();
        let start = "2025-10-17".parse::<NaiveDate>().unwrap();
        let end = None; // Open-ended period

        let period = TrainingPeriod::new(
            id,
            user,
            start,
            end,
            "test period".to_string(),
            TrainingPeriodSports::new(None),
            None,
        )
        .unwrap();

        let range = period.range_default_tomorrow();
        let today = Utc::now().date_naive();
        let tomorrow = today + Days::new(1);

        assert_eq!(range.start(), &start);
        // Should use tomorrow as end (exclusive, so will include today's activities)
        assert_eq!(range.end(), &tomorrow);
    }
}

#[cfg(test)]
mod test_training_metric_group_by {
    use crate::domain::models::activity::{
        ActivityDuration, ActivityFeedback, ActivityId, ActivityName, ActivityNutrition,
        ActivityRpe, ActivityStartTime, BonkStatus, WorkoutType,
    };

    use super::*;

    #[test]
    fn test_extract_group_from_activity() {
        let activity = Activity::new(
            ActivityId::default(),
            UserId::test_default(),
            ActivityName::empty(),
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            ActivityDuration::default(),
            Sport::TrailRunning,
            Some(ActivityRpe::Six),
            Some(WorkoutType::Intervals),
            Some(ActivityNutrition::new(BonkStatus::Bonked, None)),
            ActivityFeedback::empty(),
        );

        assert_eq!(
            TrainingMetricActivityGroupBy::Sport.extract_group(&activity),
            Some("TrailRunning".to_string())
        );

        assert_eq!(
            TrainingMetricActivityGroupBy::SportCategory.extract_group(&activity),
            Some("Running".to_string())
        );

        assert_eq!(
            TrainingMetricActivityGroupBy::WorkoutType.extract_group(&activity),
            Some("intervals".to_string())
        );

        assert_eq!(
            TrainingMetricActivityGroupBy::RpeRange.extract_group(&activity),
            Some("moderate".to_string())
        );

        assert_eq!(
            TrainingMetricActivityGroupBy::Bonked.extract_group(&activity),
            Some("bonked".to_string())
        );
    }

    #[test]
    fn test_extract_group_from_activity_none() {
        let activity = Activity::new_empty(
            ActivityId::default(),
            UserId::test_default(),
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            ActivityDuration::default(),
            Sport::Golf,
        );

        assert_eq!(
            TrainingMetricActivityGroupBy::SportCategory.extract_group(&activity),
            None
        );

        assert_eq!(
            TrainingMetricActivityGroupBy::WorkoutType.extract_group(&activity),
            None
        );

        assert_eq!(
            TrainingMetricActivityGroupBy::RpeRange.extract_group(&activity),
            None
        );

        assert_eq!(
            TrainingMetricActivityGroupBy::Bonked.extract_group(&activity),
            None
        );
    }

    #[test]
    fn test_training_metric_scope_from_none() {
        let period_id: Option<TrainingPeriodId> = None;
        let scope = TrainingMetricScope::from(&period_id);
        assert_eq!(scope, TrainingMetricScope::Global);
    }

    #[test]
    fn test_training_metric_scope_from_some_period_id() {
        let period_id = TrainingPeriodId::new();
        let scope = TrainingMetricScope::from(&Some(period_id.clone()));
        assert_eq!(scope, TrainingMetricScope::TrainingPeriod(period_id));
    }

    #[test]
    fn test_option_training_period_id_from_global_scope() {
        let scope = TrainingMetricScope::Global;
        let period_id: Option<TrainingPeriodId> = (&scope).into();
        assert_eq!(period_id, None);
    }

    #[test]
    fn test_option_training_period_id_from_training_period_scope() {
        let period_id = TrainingPeriodId::new();
        let scope = TrainingMetricScope::TrainingPeriod(period_id.clone());
        let result: Option<TrainingPeriodId> = (&scope).into();
        assert_eq!(result, Some(period_id));
    }

    #[test]
    fn test_training_metric_scope_period_returns_none_for_global() {
        let scope = TrainingMetricScope::Global;
        assert_eq!(scope.period(), None);
    }

    #[test]
    fn test_training_metric_scope_period_returns_some_for_training_period() {
        let period_id = TrainingPeriodId::new();
        let scope = TrainingMetricScope::TrainingPeriod(period_id.clone());
        assert_eq!(scope.period(), Some(period_id));
    }
}

#[cfg(test)]
mod test_training_metrics_ordering {

    use super::*;

    fn generate_test_metrics() -> Vec<TrainingMetric> {
        let definition = TrainingMetricDefinition::new(
            UserId::test_default(),
            TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::Distance,
                TrainingMetricActivityGroupBy::none(),
                TrainingMetricActivityFilters::empty(),
            )),
            Some(TrainingMetricWindow::new(
                TrainingMetricGranularity::Daily,
                TrainingMetricAggregate::Sum,
            )),
            TrainingMetricSummary::empty(),
            None,
        );

        let ids = ["a", "c", "b", "d"];

        ids.iter()
            .map(|id| {
                TrainingMetric::new(
                    TrainingMetricId::from(id),
                    None,
                    TrainingMetricScope::Global,
                    definition.clone(),
                )
            })
            .collect()
    }

    fn test_ordering(ids: Vec<&str>, metrics: Vec<TrainingMetric>) {
        let expected_ids: Vec<TrainingMetricId> =
            ids.iter().map(|id| TrainingMetricId::from(id)).collect();
        assert_eq!(
            metrics
                .iter()
                .map(|metric| metric.id().clone())
                .collect::<Vec<TrainingMetricId>>(),
            expected_ids
        );
    }

    fn ordering_from_vec(ids: Vec<&str>) -> TrainingMetricsOrdering {
        TrainingMetricsOrdering::try_from(
            ids.iter()
                .map(|id| TrainingMetricId::from(id))
                .collect::<Vec<TrainingMetricId>>(),
        )
        .unwrap()
    }

    #[test]
    fn test_building_ordering_from_empty_vec_ok() {
        let ordering = TrainingMetricsOrdering::try_from(vec![]).expect("Should not err");

        assert_eq!(ordering.0, vec![]);
    }

    #[test]
    fn test_building_ordering_from_vec_ok() {
        let ordering = TrainingMetricsOrdering::try_from(vec![
            TrainingMetricId::from("a"),
            TrainingMetricId::from("b"),
        ])
        .expect("Should not err");

        assert_eq!(
            ordering.0,
            vec![TrainingMetricId::from("a"), TrainingMetricId::from("b"),]
        );
    }

    #[test]
    fn test_building_ordering_from_vec_with_duplicates_err() {
        TrainingMetricsOrdering::try_from(vec![
            TrainingMetricId::from("a"),
            TrainingMetricId::from("b"),
            TrainingMetricId::from("a"),
        ])
        .expect_err("Should return an err");
    }

    #[test]
    fn test_sort_with_empty_ordering_uses_metric_id_ordering() {
        let ordering = ordering_from_vec(vec![]);

        let metrics = generate_test_metrics();

        let sorted_metrics = ordering.sort(metrics);

        test_ordering(vec!["a", "b", "c", "d"], sorted_metrics);
    }

    #[test]
    fn test_sort_with_empty_ordering_and_empty_metrics() {
        let ordering = ordering_from_vec(vec![]);

        let metrics = vec![];

        let sorted_metrics = ordering.sort(metrics);

        test_ordering(vec![], sorted_metrics);
    }

    #[test]
    fn test_sort_with_all_metrics_in_ordering() {
        let ordering = ordering_from_vec(vec!["d", "c", "a", "b"]);

        let metrics = generate_test_metrics();

        let sorted_metrics = ordering.sort(metrics);

        test_ordering(vec!["d", "c", "a", "b"], sorted_metrics);
    }

    #[test]
    fn test_sort_with_metrics_unknown_to_the_ordering() {
        let ordering = ordering_from_vec(vec!["d", "c"]);

        let metrics = generate_test_metrics();

        let sorted_metrics = ordering.sort(metrics);

        test_ordering(vec!["d", "c", "a", "b"], sorted_metrics);
    }

    #[test]
    fn test_sort_with_metrics_missing_the_ordering() {
        let ordering = ordering_from_vec(vec!["d", "c", "w", "z"]);

        let metrics = generate_test_metrics();

        let sorted_metrics = ordering.sort(metrics);

        test_ordering(vec!["d", "c", "a", "b"], sorted_metrics);
    }

    #[test]
    fn test_removing_metric_from_empty_ordering() {
        let ordering = ordering_from_vec(vec![]);

        let new_ordering = ordering.remove_metric(&TrainingMetricId::from("toto"));

        assert_eq!(new_ordering, ordering_from_vec(vec![]))
    }

    #[test]
    fn test_removing_metric_not_in_the_ordering() {
        let ordering = ordering_from_vec(vec!["a", "b", "c"]);

        let new_ordering = ordering.remove_metric(&TrainingMetricId::from("toto"));

        assert_eq!(new_ordering, ordering_from_vec(vec!["a", "b", "c"]))
    }

    #[test]
    fn test_removing_metric_in_the_ordering() {
        let ordering = ordering_from_vec(vec!["a", "b", "c"]);

        let new_ordering = ordering.remove_metric(&TrainingMetricId::from("b"));

        assert_eq!(new_ordering, ordering_from_vec(vec!["a", "c"]))
    }
}

#[cfg(test)]
mod test_training_metric_summary {
    use super::*;

    #[test]
    fn test_compute_average_empty_values() {
        let values = HashMap::new();

        let include_zeros = true;
        let average = TrainingMetricSummaryAverage::new(include_zeros).compute(&values);

        assert!(average.is_none())
    }

    #[test]
    fn test_compute_average_include_zeros() {
        let values = HashMap::from([
            (
                TrainingMetricBin::new("2025-09-24".to_string(), Some("Running".to_string())),
                TrainingMetricValue::Max(10.0),
            ),
            (
                TrainingMetricBin::new("2025-09-24".to_string(), None),
                TrainingMetricValue::Max(15.0),
            ),
            (
                TrainingMetricBin::new("2025-09-25".to_string(), None),
                TrainingMetricValue::Max(15.0),
            ),
            (
                TrainingMetricBin::new("2025-09-26".to_string(), None),
                TrainingMetricValue::Max(0.),
            ),
        ]);

        let include_zeros = true;
        let average = TrainingMetricSummaryAverage::new(include_zeros).compute(&values);

        assert_eq!(average, Some(((10. + 15.) + 15. + 0.) / 3.))
    }

    #[test]
    fn test_compute_average_exclude_zeros() {
        let values = HashMap::from([
            (
                TrainingMetricBin::new("2025-09-24".to_string(), Some("Running".to_string())),
                TrainingMetricValue::Max(10.0),
            ),
            (
                TrainingMetricBin::new("2025-09-24".to_string(), None),
                TrainingMetricValue::Max(15.0),
            ),
            (
                TrainingMetricBin::new("2025-09-25".to_string(), None),
                TrainingMetricValue::Max(15.0),
            ),
            (
                TrainingMetricBin::new("2025-09-26".to_string(), None),
                TrainingMetricValue::Max(0.),
            ),
        ]);

        let include_zeros = false;
        let average = TrainingMetricSummaryAverage::new(include_zeros).compute(&values);

        assert_eq!(average, Some(((10. + 15.) + 15.) / 2.))
    }

    #[test]
    fn test_compute_summary_average() {
        let values = HashMap::from([
            (
                TrainingMetricBin::new("2025-09-24".to_string(), Some("Running".to_string())),
                TrainingMetricValue::Max(10.0),
            ),
            (
                TrainingMetricBin::new("2025-09-25".to_string(), None),
                TrainingMetricValue::Max(20.0),
            ),
        ]);

        let summary = TrainingMetricSummary::new(Some(TrainingMetricSummaryAverage::new(true)));

        assert_eq!(summary.compute(&values).average, Some(15.0));
    }
}

#[cfg(test)]
mod test_training_metric_target {
    use super::*;

    fn definition_with_target() -> TrainingMetricDefinition {
        TrainingMetricDefinition::new(
            UserId::test_default(),
            TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::Distance,
                TrainingMetricActivityGroupBy::none(),
                TrainingMetricActivityFilters::empty(),
            )),
            None,
            TrainingMetricSummary::empty(),
            Some(TrainingMetricTarget::new(100.0, Unit::Kilometer)),
        )
    }

    #[test]
    fn test_target_accessors() {
        let target = TrainingMetricTarget::new(42.0, Unit::KiloCalorie);

        assert_eq!(target.value(), 42.0);
        assert_eq!(target.unit(), Unit::KiloCalorie);
    }

    #[test]
    fn test_definition_target_is_some_when_set() {
        let definition = definition_with_target();

        let target = definition.target().expect("target should be set");
        assert_eq!(target.value(), 100.0);
        assert_eq!(target.unit(), Unit::Kilometer);
    }

    #[test]
    fn test_definition_target_is_none_when_not_set() {
        let definition = TrainingMetricDefinition::new(
            UserId::test_default(),
            TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::Distance,
                TrainingMetricActivityGroupBy::none(),
                TrainingMetricActivityFilters::empty(),
            )),
            None,
            TrainingMetricSummary::empty(),
            None,
        );

        assert!(definition.target().is_none());
    }

    #[test]
    fn test_apply_patch_updates_target() {
        let definition = definition_with_target();
        let patch = TrainingMetricDefinitionPatch::new(
            TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::Calories,
                TrainingMetricActivityGroupBy::none(),
                TrainingMetricActivityFilters::empty(),
            )),
            None,
            TrainingMetricSummary::empty(),
            Some(TrainingMetricTarget::new(2000.0, Unit::KiloCalorie)),
        );

        let patched = definition.apply_patch(patch);

        let target = patched.target().expect("target should be set");
        assert_eq!(target.value(), 2000.0);
        assert_eq!(target.unit(), Unit::KiloCalorie);
        assert_eq!(
            *patched.source(),
            TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::Calories,
                TrainingMetricActivityGroupBy::none(),
                TrainingMetricActivityFilters::empty(),
            ))
        );
    }

    #[test]
    fn test_apply_patch_clears_target() {
        let definition = definition_with_target();
        let patch = TrainingMetricDefinitionPatch::new(
            TrainingMetricSource::Activity(ActivitySource::new(
                ActivityMetric::Calories,
                TrainingMetricActivityGroupBy::none(),
                TrainingMetricActivityFilters::empty(),
            )),
            None,
            TrainingMetricSummary::empty(),
            None,
        );

        let patched = definition.apply_patch(patch);

        assert!(patched.target().is_none());
    }

    #[test]
    fn test_merge_default_sports_preserves_target() {
        let definition = definition_with_target();

        let merged =
            definition.merge_default_sports(&Some(vec![SportFilter::Sport(Sport::Running)]));

        let target = merged.target().expect("target should be preserved");
        assert_eq!(target.value(), 100.0);
        assert_eq!(target.unit(), Unit::Kilometer);
    }

    #[test]
    fn test_target_serde_uses_display_unit() {
        let target = TrainingMetricTarget::new(3.5, Unit::Kilometer);

        let json = serde_json::to_string(&target).unwrap();

        assert_eq!(json, r#"{"value":3.5,"unit":"km"}"#);
    }

    #[test]
    fn test_target_deserialize_from_display_unit() {
        let target: TrainingMetricTarget =
            serde_json::from_str(r#"{"value":3.5,"unit":"km"}"#).unwrap();

        assert_eq!(target, TrainingMetricTarget::new(3.5, Unit::Kilometer));
    }

    #[test]
    fn test_target_deserialize_unknown_unit_fails() {
        let result: Result<TrainingMetricTarget, _> =
            serde_json::from_str(r#"{"value":3.5,"unit":"parsec"}"#);

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod test_training_note_search_document {

    use super::*;

    fn now() -> chrono::DateTime<chrono::Utc> {
        "2025-09-03T00:00:00Z"
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap()
    }

    fn note(
        id: TrainingNoteId,
        title: Option<TrainingNoteTitle>,
        content: TrainingNoteContent,
    ) -> TrainingNote {
        TrainingNote::new(
            id,
            UserId::test_default(),
            title,
            content,
            TrainingNoteDate::new(NaiveDate::from_ymd_opt(2025, 9, 3).unwrap()),
            "2025-09-03T00:00:00Z"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
        )
    }

    #[test]
    fn test_search_document_is_for_training_note() {
        let id = TrainingNoteId::from("note-1");
        let doc = note(id, None, "Great session".into())
            .to_search_document(SearchDocumentEvent::Updated, now());

        assert_eq!(doc.document_type(), &SearchDocumentType::TrainingNote);
    }

    #[test]
    fn test_search_document_uses_note_id() {
        let id = TrainingNoteId::from("note-42");
        let doc = note(id, None, "Great session".into())
            .to_search_document(SearchDocumentEvent::Updated, now());

        assert_eq!(doc.document_id(), "note-42");
    }

    #[test]
    fn test_search_document_preserves_event() {
        let doc_updated = note(TrainingNoteId::new(), None, "Great session".into())
            .to_search_document(SearchDocumentEvent::Updated, now());
        let doc_deleted = note(TrainingNoteId::new(), None, "Great session".into())
            .to_search_document(SearchDocumentEvent::Deleted, now());

        assert!(matches!(doc_updated.event(), SearchDocumentEvent::Updated));
        assert!(matches!(doc_deleted.event(), SearchDocumentEvent::Deleted));
    }

    #[test]
    fn test_search_document_preserves_occurred_at() {
        let now = now();
        let doc = note(TrainingNoteId::new(), None, "Great session".into())
            .to_search_document(SearchDocumentEvent::Updated, now);

        assert_eq!(doc.occurred_at(), &now);
    }

    #[test]
    fn test_search_document_content_contains_title_and_content() {
        let doc = note(
            TrainingNoteId::new(),
            Some("Long Run".into()),
            "Great session".into(),
        )
        .to_search_document(SearchDocumentEvent::Updated, now());

        assert_eq!(doc.content(), "Long Run Great session");
    }

    #[test]
    fn test_search_document_content_without_title() {
        let doc = note(TrainingNoteId::new(), None, "Great session".into())
            .to_search_document(SearchDocumentEvent::Updated, now());

        assert_eq!(doc.content(), "Great session");
    }
}
