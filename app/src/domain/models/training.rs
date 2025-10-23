use std::{
    collections::{HashMap, hash_map::Iter},
    fmt,
    hash::Hash,
};

use chrono::{DateTime, Datelike, Days, FixedOffset, Months, NaiveDate, Utc};
use derive_more::{AsRef, Constructor, Display};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    models::{
        UserId,
        activity::{
            Activity, ActivityStartTime, ActivityStatistic, ActivityWithTimeseries, Sport,
            SportCategory, TimeseriesMetric, ToUnit, Unit,
        },
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
}

#[derive(Debug, Clone, PartialEq, Constructor, Serialize, Deserialize)]
pub struct TrainingMetricFilters {
    sports: Option<Vec<SportFilter>>,
}

impl TrainingMetricFilters {
    pub fn empty() -> Self {
        Self { sports: None }
    }

    pub fn sports(&self) -> &Option<Vec<SportFilter>> {
        &self.sports
    }

    pub fn matches(&self, activity: &Activity) -> bool {
        let sport_matches = self
            .sports
            .as_ref()
            .map(|sports| sports.iter().any(|filter| filter.matches(activity)))
            .unwrap_or(true);

        // More explicit with multiple filters
        #[allow(clippy::let_and_return)]
        sport_matches
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrainingMetricGroupBy {
    Sport,
    SportCategory,
    WorkoutType,
    RpeRange,
    Bonked,
}

impl TrainingMetricGroupBy {
    pub fn none() -> Option<TrainingMetricGroupBy> {
        None
    }

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

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct TrainingMetricDefinition {
    id: TrainingMetricId,
    user: UserId,
    source: ActivityMetricSource,
    granularity: TrainingMetricGranularity,
    granularity_aggregate: TrainingMetricAggregate,
    filters: TrainingMetricFilters,
    group_by: Option<TrainingMetricGroupBy>,
}

impl TrainingMetricDefinition {
    pub fn id(&self) -> &TrainingMetricId {
        &self.id
    }

    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn source(&self) -> &ActivityMetricSource {
        &self.source
    }

    pub fn granularity(&self) -> &TrainingMetricGranularity {
        &self.granularity
    }

    pub fn aggregate(&self) -> &TrainingMetricAggregate {
        &self.granularity_aggregate
    }

    pub fn filters(&self) -> &TrainingMetricFilters {
        &self.filters
    }

    pub fn group_by(&self) -> &Option<TrainingMetricGroupBy> {
        &self.group_by
    }

    pub fn source_requirement(&self) -> ComputeMetricRequirement {
        self.source.input_min_requirement()
    }
}

impl TrainingMetricDefinition {
    pub fn compute_values_from_timeseries(
        &self,
        activities: &[ActivityWithTimeseries],
    ) -> TrainingMetricValues {
        let activity_metrics = activities
            .iter()
            .filter_map(|activity| {
                if self.filters.matches(activity.activity()) {
                    self.source.metric_from_activity_with_timeseries(activity)
                } else {
                    None
                }
            })
            .collect();
        self.aggregate_metrics(activity_metrics)
    }

    pub fn compute_values(&self, activities: &[Activity]) -> TrainingMetricValues {
        let activity_metrics = activities
            .iter()
            .filter_map(|activity| {
                if self.filters.matches(activity) {
                    self.source.metric_from_activity(activity).ok().flatten()
                } else {
                    None
                }
            })
            .collect();
        self.aggregate_metrics(activity_metrics)
    }

    fn aggregate_metrics(&self, activity_metrics: Vec<ActivityMetric>) -> TrainingMetricValues {
        let grouped_metrics = group_metrics_by_bin(&self.granularity, activity_metrics);
        TrainingMetricValues::new(aggregate_metrics(
            &self.granularity_aggregate,
            grouped_metrics,
        ))
    }
}

#[derive(Debug, Clone, Constructor, PartialEq)]
pub struct DateGranule {
    start: NaiveDate,
    end: NaiveDate,
}

/// An [ActivityMetric] represents the value of an [ActivityMetricSource] extracted from
/// a single [ActivityWithTimeseries]. On top of the metric value, it contains metadata like
/// the activity start time and duration that can be used in later computations.
#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct ActivityMetric {
    value: f64,
    activity_start_time: ActivityStartTime,
    activity_duration: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActivityMetricSource {
    Statistic(ActivityStatistic),
    Timeseries((TimeseriesMetric, TimeseriesAggregate)),
}

#[derive(Debug, Clone)]
pub enum ComputeMetricRequirement {
    Activity,
    ActivityWithTimeseries,
}

#[derive(Debug, Clone, Error)]
pub enum ComputeMetricError {
    #[error(
        "Trying to compute a timseries-based metric without supplying the activity's timeseries"
    )]
    MissingTimeseries,
}

impl ActivityMetricSource {
    /// What is the minimum required to compute this metric activity values.
    pub fn input_min_requirement(&self) -> ComputeMetricRequirement {
        match self {
            Self::Statistic(_) => ComputeMetricRequirement::Activity,
            Self::Timeseries(_) => ComputeMetricRequirement::ActivityWithTimeseries,
        }
    }

    pub fn metric_from_activity_with_timeseries(
        &self,
        activity: &ActivityWithTimeseries,
    ) -> Option<ActivityMetric> {
        match self {
            Self::Statistic(statistic) => activity.statistics().get(statistic).cloned(),
            Self::Timeseries((metric, aggregate)) => {
                aggregate.value_from_timeseries(metric, activity)
            }
        }
        .map(|value| {
            ActivityMetric::new(
                value,
                *activity.start_time(),
                activity
                    .statistics()
                    .get(&ActivityStatistic::Duration)
                    .cloned(),
            )
        })
    }

    pub fn metric_from_activity(
        &self,
        activity: &Activity,
    ) -> Result<Option<ActivityMetric>, ComputeMetricError> {
        Ok(match self {
            Self::Statistic(statistic) => activity.statistics().get(statistic).cloned(),
            Self::Timeseries(_) => return Err(ComputeMetricError::MissingTimeseries),
        }
        .map(|value| {
            ActivityMetric::new(
                value,
                *activity.start_time(),
                activity
                    .statistics()
                    .get(&ActivityStatistic::Duration)
                    .cloned(),
            )
        }))
    }
}

impl ToUnit for ActivityMetricSource {
    fn unit(&self) -> Unit {
        match self {
            Self::Statistic(stat) => stat.unit(),
            Self::Timeseries((metric, _)) => metric.unit(),
        }
    }
}

fn group_metrics_by_bin(
    granularity: &TrainingMetricGranularity,
    metrics: Vec<ActivityMetric>,
) -> HashMap<TrainingMetricBin, Vec<ActivityMetric>> {
    let mut grouped_values: HashMap<TrainingMetricBin, Vec<ActivityMetric>> = HashMap::new();
    for value in metrics {
        // TODO: group by TrainingMetricGroupBy
        let bin = TrainingMetricBin::from_granule(
            &granularity.datetime_key(value.activity_start_time.date()),
        );
        grouped_values.entry(bin).or_default().push(value);
    }
    grouped_values
}

fn aggregate_metrics(
    aggregate: &TrainingMetricAggregate,
    metrics: HashMap<TrainingMetricBin, Vec<ActivityMetric>>,
) -> HashMap<TrainingMetricBin, TrainingMetricValue> {
    let mut res = HashMap::new();

    for (key, values) in metrics.into_iter() {
        let Some(training_metric_value) = aggregate.aggregate(values) else {
            continue;
        };
        res.insert(key, training_metric_value);
    }

    res
}

#[derive(Debug, Clone, PartialEq, Display)]
pub enum TrainingMetricGranularity {
    Daily,
    Weekly,
    Monthly,
}

impl TrainingMetricGranularity {
    pub fn datetime_key(&self, dt: &DateTime<FixedOffset>) -> String {
        match self {
            TrainingMetricGranularity::Daily => dt.date_naive().to_string(),
            TrainingMetricGranularity::Weekly => dt
                .date_naive()
                .week(chrono::Weekday::Mon)
                .first_day()
                .to_string(),
            TrainingMetricGranularity::Monthly => dt.date_naive().with_day(1).unwrap().to_string(),
        }
    }

    /// Computes the bins' keys for the [TrainingMetricGranularity] over the given range [start,
    /// end].
    pub fn bins_keys(
        &self,
        start: &DateTime<FixedOffset>,
        end: &DateTime<FixedOffset>,
    ) -> Vec<String> {
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

#[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
pub enum TimeseriesAggregate {
    Min,
    Max,
    Average,
    Sum,
}

impl TimeseriesAggregate {
    fn value_from_timeseries(
        &self,
        metric: &TimeseriesMetric,
        activity: &ActivityWithTimeseries,
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
        if values.is_empty() {
            return None;
        }
        let length = values.len();
        match self {
            Self::Min => values.into_iter().reduce(f64::min),
            Self::Max => values.into_iter().reduce(f64::max),
            Self::Average => values
                .into_iter()
                .reduce(|acc, e| acc + e)
                .map(|val| val / length as f64),
            Self::Sum => values.into_iter().reduce(|acc, e| acc + e),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
pub enum TrainingMetricAggregate {
    Min,
    Max,
    Average,
    Sum,
}

impl TrainingMetricAggregate {
    fn aggregate(&self, activity_metrics: Vec<ActivityMetric>) -> Option<TrainingMetricValue> {
        if activity_metrics.is_empty() {
            return None;
        }
        Some(match self {
            TrainingMetricAggregate::Min => TrainingMetricValue::Min(
                activity_metrics
                    .into_iter()
                    .fold(f64::MAX, |min, metric| min.min(metric.value)),
            ),
            TrainingMetricAggregate::Max => TrainingMetricValue::Max(
                activity_metrics
                    .into_iter()
                    .fold(f64::MIN, |max, metric| max.max(metric.value)),
            ),
            TrainingMetricAggregate::Average => {
                let number_of_metrics = activity_metrics.len();
                let sum = activity_metrics
                    .into_iter()
                    .fold(0., |sum, metric| sum + metric.value);

                TrainingMetricValue::Average {
                    value: sum / number_of_metrics as f64,
                    sum,
                    number_of_elements: number_of_metrics,
                }
            }
            TrainingMetricAggregate::Sum => TrainingMetricValue::Sum(
                activity_metrics
                    .into_iter()
                    .fold(0., |sum, metric| sum + metric.value),
            ),
        })
    }

    pub fn initial_value(&self, new_metric: &ActivityMetric) -> Option<TrainingMetricValue> {
        Some(match self {
            Self::Max => TrainingMetricValue::Max(new_metric.value),
            Self::Min => TrainingMetricValue::Min(new_metric.value),
            Self::Sum => TrainingMetricValue::Sum(new_metric.value),
            Self::Average => TrainingMetricValue::Average {
                value: new_metric.value,
                sum: new_metric.value,
                number_of_elements: 1,
            },
        })
    }

    pub fn update_value(
        &self,
        previous_value: &TrainingMetricValue,
        new_metric: &ActivityMetric,
    ) -> Option<TrainingMetricValue> {
        match self {
            Self::Min => {
                let TrainingMetricValue::Min(min) = previous_value else {
                    return None;
                };
                Some(TrainingMetricValue::Min(min.min(new_metric.value)))
            }
            Self::Max => {
                let TrainingMetricValue::Max(max) = previous_value else {
                    return None;
                };
                Some(TrainingMetricValue::Max(max.max(new_metric.value)))
            }
            Self::Sum => {
                let TrainingMetricValue::Sum(sum) = previous_value else {
                    return None;
                };
                Some(TrainingMetricValue::Sum(sum + new_metric.value))
            }
            Self::Average => {
                let TrainingMetricValue::Average {
                    sum,
                    value: _,
                    number_of_elements,
                } = previous_value
                else {
                    return None;
                };
                Some(TrainingMetricValue::Average {
                    sum: *sum + new_metric.value,
                    number_of_elements: *number_of_elements + 1,
                    value: (*sum + new_metric.value) / (*number_of_elements as f64 + 1.),
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrainingMetricValue {
    Min(f64),
    Max(f64),
    Sum(f64),
    Average {
        value: f64,
        sum: f64,
        number_of_elements: usize,
    },
}

impl TrainingMetricValue {
    pub fn value(&self) -> &f64 {
        match self {
            Self::Max(max) => max,
            Self::Min(min) => min,
            Self::Sum(sum) => sum,
            Self::Average {
                value,
                sum: _,
                number_of_elements: _,
            } => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Constructor)]
pub struct TrainingMetricBin {
    granule: String,
    group: Option<String>,
}

impl TrainingMetricBin {
    // TODO: should no longer be required once all todos are resolved ?
    pub fn from_granule(granule: &str) -> Self {
        Self {
            granule: granule.to_string(),
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

#[derive(Debug, Clone, Constructor, Default)]
pub struct TrainingMetricValues(HashMap<TrainingMetricBin, TrainingMetricValue>);

impl TrainingMetricValues {
    pub fn insert(
        &mut self,
        key: TrainingMetricBin,
        value: TrainingMetricValue,
    ) -> Option<TrainingMetricValue> {
        self.0.insert(key, value)
    }

    pub fn get(&self, key: &TrainingMetricBin) -> Option<&TrainingMetricValue> {
        self.0.get(key)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> Iter<'_, TrainingMetricBin, TrainingMetricValue> {
        self.0.iter()
    }
}

impl TrainingMetricValues {
    pub fn as_hash_map(self) -> HashMap<TrainingMetricBin, TrainingMetricValue> {
        self.0
    }
}

impl std::iter::IntoIterator for TrainingMetricValues {
    type Item = (TrainingMetricBin, TrainingMetricValue);
    type IntoIter = std::collections::hash_map::IntoIter<TrainingMetricBin, TrainingMetricValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, AsRef, Hash)]
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

    /// Returns a DateRange for this training period.
    /// For open-ended periods (no end date), defaults to today as the end date.
    /// Note: DateRange end is exclusive, so this will NOT include today's activities.
    pub fn range_default_today(&self) -> DateRange {
        let end = self.end.unwrap_or_else(|| Utc::now().date_naive());
        DateRange::new(self.start, end)
    }

    /// Returns a DateRange for this training period.
    /// For open-ended periods (no end date), defaults to tomorrow as the end date.
    /// Note: DateRange end is exclusive, so this WILL include today's activities.
    pub fn range_default_tomorrow(&self) -> DateRange {
        let end = self
            .end
            .unwrap_or_else(|| Utc::now().date_naive() + Days::new(1));
        DateRange::new(self.start, end)
    }

    pub fn matches(&self, activity: &Activity) -> bool {
        let activity_start_date = activity.start_time().date().date_naive();
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

    pub fn note(&self) -> &Option<String> {
        &self.note
    }
}

#[derive(Debug, Clone)]
pub struct TrainingPeriodWithActivities {
    period: TrainingPeriod,
    activities: Vec<Activity>,
}

impl TrainingPeriodWithActivities {
    pub fn new(period: TrainingPeriod, activities: Vec<Activity>) -> Self {
        Self { period, activities }
    }

    pub fn period(&self) -> &TrainingPeriod {
        &self.period
    }

    pub fn activities(&self) -> &[Activity] {
        &self.activities
    }
}

#[cfg(test)]
mod test_training_metrics {

    use crate::domain::models::{
        UserId,
        activity::{
            ActiveTime, Activity, ActivityId, ActivityStartTime, ActivityStatistics,
            ActivityTimeseries, Sport, Timeseries, TimeseriesActiveTime, TimeseriesTime,
            TimeseriesValue,
        },
    };

    use super::*;

    fn default_activity() -> ActivityWithTimeseries {
        ActivityWithTimeseries::new(
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
                ActivityStatistics::new(HashMap::from([(ActivityStatistic::Calories, 123.3)])),
                None,
                None,
                None,
            ),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![0, 1, 2]),
                TimeseriesActiveTime::new(vec![
                    ActiveTime::Running(0),
                    ActiveTime::Running(1),
                    ActiveTime::Running(2),
                ]),
                vec![],
                vec![Timeseries::new(
                    TimeseriesMetric::Power,
                    vec![
                        Some(TimeseriesValue::Int(10)),
                        Some(TimeseriesValue::Int(20)),
                        Some(TimeseriesValue::Int(30)),
                    ],
                )],
            )
            .unwrap(),
        )
    }

    #[test]
    fn test_extract_aggregated_activity_metric_no_metric_found() {
        let metric = TimeseriesMetric::Speed;
        let aggregate = TimeseriesAggregate::Min;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_aggregated_activity_metric_metric_is_empty() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Average;
        let activity = ActivityWithTimeseries::new(
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
                None,
                None,
                None,
            ),
            ActivityTimeseries::new(
                TimeseriesTime::new(vec![]),
                TimeseriesActiveTime::new(vec![]),
                vec![],
                vec![Timeseries::new(TimeseriesMetric::Power, vec![])],
            )
            .unwrap(),
        );

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_aggregated_activity_metric_min_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Min;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 10.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_max_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Max;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 30.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_average_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Average;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 20.)
    }

    #[test]
    fn test_extract_aggregated_activity_metric_total_value() {
        let metric = TimeseriesMetric::Power;
        let aggregate = TimeseriesAggregate::Sum;
        let activity = default_activity();

        let res = aggregate.value_from_timeseries(&metric, &activity);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 60.)
    }

    #[test]
    fn test_group_metric_by_granularity_daily() {
        let metric_1 = ActivityMetric::new(
            12.3,
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(120.),
        );
        let metric_2 = ActivityMetric::new(
            18.1,
            ActivityStartTime::new(
                "2025-09-03T02:00:00+03:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(120.),
        );

        let metric_3 = ActivityMetric::new(
            67.1,
            ActivityStartTime::new(
                "2025-09-04T02:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(120.),
        );
        let metrics = vec![metric_1.clone(), metric_2.clone(), metric_3.clone()];
        let granularity = TrainingMetricGranularity::Daily;

        let res = group_metrics_by_bin(&granularity, metrics);
        assert_eq!(res.len(), 2);
        assert_eq!(
            res.get(&TrainingMetricBin::from_granule("2025-09-03"))
                .unwrap(),
            &vec![metric_1, metric_2]
        );
        assert_eq!(
            res.get(&TrainingMetricBin::from_granule("2025-09-04"))
                .unwrap(),
            &vec![metric_3]
        );
    }

    #[test]
    fn test_group_metric_by_granularity_weekly() {
        let metric_1 = ActivityMetric::new(
            12.3,
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(12.),
        );
        let metric_2 = ActivityMetric::new(
            18.1,
            ActivityStartTime::new(
                "2025-09-05T02:00:00+03:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(12.),
        );
        let metric_3 = ActivityMetric::new(
            67.1,
            ActivityStartTime::new(
                "2025-09-14T02:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Some(12.),
        );
        let metrics = vec![metric_1.clone(), metric_2.clone(), metric_3.clone()];
        let granularity = TrainingMetricGranularity::Weekly;

        let res = group_metrics_by_bin(&granularity, metrics);
        assert_eq!(res.len(), 2);
        assert_eq!(
            res.get(&TrainingMetricBin::from_granule("2025-09-01"))
                .unwrap(),
            &vec![metric_1, metric_2]
        );
        assert_eq!(
            res.get(&TrainingMetricBin::from_granule("2025-09-08"))
                .unwrap(),
            &vec![metric_3]
        );
    }

    #[test]
    fn test_group_metric_by_granularity_monthly() {
        let metric_1 = ActivityMetric::new(
            12.3,
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            None,
        );
        let metric_2 = ActivityMetric::new(
            18.1,
            ActivityStartTime::new(
                "2025-09-05T02:00:00+03:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            None,
        );
        let metric_3 = ActivityMetric::new(
            67.1,
            ActivityStartTime::new(
                "2025-08-14T02:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            None,
        );
        let metrics = vec![metric_1.clone(), metric_2.clone(), metric_3.clone()];
        let granularity = TrainingMetricGranularity::Monthly;

        let res = group_metrics_by_bin(&granularity, metrics);
        assert_eq!(res.len(), 2);
        assert_eq!(
            res.get(&TrainingMetricBin::from_granule("2025-09-01"))
                .unwrap(),
            &vec![metric_1, metric_2]
        );
        assert_eq!(
            res.get(&TrainingMetricBin::from_granule("2025-08-01"))
                .unwrap(),
            &vec![metric_3]
        );
    }

    #[test]
    fn test_aggregate_metrics_min() {
        let metrics = HashMap::from([(
            TrainingMetricBin::from_granule("2025-09-01"),
            vec![
                ActivityMetric::new(
                    12.3,
                    ActivityStartTime::new(
                        "2025-09-03T00:00:00Z"
                            .parse::<DateTime<FixedOffset>>()
                            .unwrap(),
                    ),
                    None,
                ),
                ActivityMetric::new(
                    1.3,
                    ActivityStartTime::new(
                        "2025-09-03T00:00:00Z"
                            .parse::<DateTime<FixedOffset>>()
                            .unwrap(),
                    ),
                    None,
                ),
            ],
        )]);
        let aggregate = TrainingMetricAggregate::Min;

        let res = aggregate_metrics(&aggregate, metrics);

        assert_eq!(
            *res.get(&TrainingMetricBin::from_granule("2025-09-01"))
                .unwrap(),
            TrainingMetricValue::Min(1.3)
        );
    }

    #[test]
    fn test_compute_training_metrics_from_timeseries() {
        let activities = vec![default_activity()];
        let metric_definition = TrainingMetricDefinition::new(
            TrainingMetricId::default(),
            UserId::test_default(),
            ActivityMetricSource::Timeseries((
                TimeseriesMetric::Power,
                TimeseriesAggregate::Average,
            )),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Max,
            TrainingMetricFilters::empty(),
            TrainingMetricGroupBy::none(),
        );

        let metrics = metric_definition.compute_values_from_timeseries(&activities);

        assert_eq!(
            *metrics
                .get(&TrainingMetricBin::from_granule("2025-09-01"))
                .unwrap(),
            TrainingMetricValue::Max(20.)
        );
    }

    #[test]
    fn test_compute_training_metrics_from_timeseries_with_filters() {
        let activities = vec![default_activity()];
        let metric_definition = TrainingMetricDefinition::new(
            TrainingMetricId::default(),
            UserId::test_default(),
            ActivityMetricSource::Timeseries((
                TimeseriesMetric::Power,
                TimeseriesAggregate::Average,
            )),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Max,
            TrainingMetricFilters::new(Some(vec![SportFilter::Sport(Sport::Running)])),
            TrainingMetricGroupBy::none(),
        );

        let metrics = metric_definition.compute_values_from_timeseries(&activities);

        assert!(metrics.is_empty());
    }

    #[test]
    fn test_compute_training_metrics_with_filters() {
        let activities: Vec<Activity> = vec![default_activity()]
            .iter()
            .map(|activity| activity.activity().clone())
            .collect();
        let metric_definition = TrainingMetricDefinition::new(
            TrainingMetricId::default(),
            UserId::test_default(),
            ActivityMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Max,
            TrainingMetricFilters::new(Some(vec![SportFilter::Sport(Sport::Running)])),
            TrainingMetricGroupBy::none(),
        );

        let metrics = metric_definition.compute_values(&activities);

        assert!(metrics.is_empty());
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
        let start = "2025-08-23T12:03:00+02:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        let end = "2025-09-09T12:03:00+10:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
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
        let start = "2025-07-23T12:03:00+02:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
        let end = "2025-09-09T12:03:00+10:00"
            .parse::<DateTime<FixedOffset>>()
            .unwrap();
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
mod test_training_metric_aggregate_initial_value {

    use super::*;

    #[test]
    fn test_min_value() {
        let aggregate = TrainingMetricAggregate::Min;
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.initial_value(&new_metric),
            Some(TrainingMetricValue::Min(10.1))
        );
    }

    #[test]
    fn test_max_value() {
        let aggregate = TrainingMetricAggregate::Max;
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.initial_value(&new_metric),
            Some(TrainingMetricValue::Max(10.1))
        );
    }

    #[test]
    fn test_sum_value() {
        let aggregate = TrainingMetricAggregate::Sum;
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.initial_value(&new_metric),
            Some(TrainingMetricValue::Sum(10.1))
        );
    }

    #[test]
    fn test_average_value() {
        let aggregate = TrainingMetricAggregate::Average;
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.initial_value(&new_metric),
            Some(TrainingMetricValue::Average {
                value: 10.1,
                sum: 10.1,
                number_of_elements: 1
            })
        );
    }
}

#[cfg(test)]
mod test_training_metric_aggregate_update_value {

    use super::*;

    use assert_approx_eq::assert_approx_eq;

    #[test]
    fn test_update_min_value() {
        let aggregate = TrainingMetricAggregate::Min;
        let previous = TrainingMetricValue::Min(12.2);
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.update_value(&previous, &new_metric),
            Some(TrainingMetricValue::Min(10.1))
        );
    }

    #[test]
    fn test_do_not_update_min_value() {
        let aggregate = TrainingMetricAggregate::Min;
        let previous = TrainingMetricValue::Min(12.2);
        let new_metric = ActivityMetric::new(
            13.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.update_value(&previous, &new_metric),
            Some(TrainingMetricValue::Min(12.2))
        );
    }

    #[test]
    fn test_update_min_value_wrong_previous_value_variant() {
        let previous_values_wrong_variant = vec![
            TrainingMetricValue::Max(12.),
            TrainingMetricValue::Sum(12.),
            TrainingMetricValue::Average {
                value: 12.,
                sum: 12.,
                number_of_elements: 2,
            },
        ];
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        let aggregate = TrainingMetricAggregate::Min;

        for previous in previous_values_wrong_variant {
            assert!(aggregate.update_value(&previous, &new_metric).is_none());
        }
    }

    #[test]
    fn test_update_max_value() {
        let aggregate = TrainingMetricAggregate::Max;
        let previous = TrainingMetricValue::Max(12.2);
        let new_metric = ActivityMetric::new(
            13.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.update_value(&previous, &new_metric),
            Some(TrainingMetricValue::Max(13.1))
        );
    }

    #[test]
    fn test_do_not_update_max_value() {
        let aggregate = TrainingMetricAggregate::Max;
        let previous = TrainingMetricValue::Max(12.2);
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        assert_eq!(
            aggregate.update_value(&previous, &new_metric),
            Some(TrainingMetricValue::Max(12.2))
        );
    }

    #[test]
    fn test_update_max_value_wrong_previous_value_variant() {
        let aggregate = TrainingMetricAggregate::Max;
        let previous_values_wrong_variant = vec![
            TrainingMetricValue::Min(12.),
            TrainingMetricValue::Sum(12.),
            TrainingMetricValue::Average {
                value: 12.,
                sum: 12.,
                number_of_elements: 2,
            },
        ];
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        for previous in previous_values_wrong_variant {
            assert!(aggregate.update_value(&previous, &new_metric).is_none());
        }
    }

    #[test]
    fn test_update_sum_value() {
        let aggregate = TrainingMetricAggregate::Sum;
        let previous = TrainingMetricValue::Sum(12.2);
        let new_metric = ActivityMetric::new(
            13.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        let Some(TrainingMetricValue::Sum(sum)) = aggregate.update_value(&previous, &new_metric)
        else {
            unreachable!("Should have returned Some(TrainingMetricValue::Sum(sum))");
        };
        assert_approx_eq!(sum, 25.3);
    }

    #[test]
    fn test_update_sum_value_wrong_previous_value_variant() {
        let aggregate = TrainingMetricAggregate::Sum;
        let previous_values_wrong_variant = vec![
            TrainingMetricValue::Min(12.),
            TrainingMetricValue::Max(12.),
            TrainingMetricValue::Average {
                value: 12.,
                sum: 12.,
                number_of_elements: 2,
            },
        ];
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        for previous in previous_values_wrong_variant {
            assert!(aggregate.update_value(&previous, &new_metric).is_none());
        }
    }

    #[test]
    fn test_update_average_value() {
        let aggregate = TrainingMetricAggregate::Average;
        let previous = TrainingMetricValue::Average {
            value: 12.,
            sum: 12.,
            number_of_elements: 2,
        };
        let new_metric = ActivityMetric::new(
            13.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        let Some(TrainingMetricValue::Average {
            sum,
            value,
            number_of_elements,
        }) = aggregate.update_value(&previous, &new_metric)
        else {
            unreachable!("Should have returned Some(TrainingMetricValue::Average)");
        };
        assert_approx_eq!(sum, 25.1);
        assert_approx_eq!(value, 25.1 / 3.);
        assert_eq!(number_of_elements, 3);
    }

    #[test]
    fn test_update_average_value_wrong_previous_value_variant() {
        let aggregate = TrainingMetricAggregate::Average;
        let previous_values_wrong_variant = vec![
            TrainingMetricValue::Min(12.),
            TrainingMetricValue::Max(12.),
            TrainingMetricValue::Sum(12.),
        ];
        let new_metric = ActivityMetric::new(
            10.1,
            ActivityStartTime::from_timestamp(1200).unwrap(),
            Some(1200.),
        );

        for previous in previous_values_wrong_variant {
            assert!(aggregate.update_value(&previous, &new_metric).is_none());
        }
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
mod test_training_metric_filters {
    use crate::domain::models::activity::{ActivityId, ActivityStatistics};

    use super::*;

    #[test]
    fn test_sport_filter_matches_activity() {
        let activity = Activity::new(
            ActivityId::default(),
            UserId::test_default(),
            None,
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Sport::IndoorCycling,
            ActivityStatistics::default(),
            None,
            None,
            None,
        );

        assert!(SportFilter::Sport(Sport::IndoorCycling).matches(&activity));
        assert!(!SportFilter::Sport(Sport::Cycling).matches(&activity));

        assert!(SportFilter::SportCategory(SportCategory::Cycling).matches(&activity));
        assert!(!SportFilter::SportCategory(SportCategory::Running).matches(&activity));
    }

    #[test]
    fn test_filter_by_sport() {
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
            None,
            None,
            None,
        );

        assert!(
            TrainingMetricFilters::new(Some(vec![SportFilter::Sport(Sport::Cycling)]))
                .matches(&activity)
        );
        assert!(
            TrainingMetricFilters::new(Some(vec![
                SportFilter::Sport(Sport::Cycling),
                SportFilter::Sport(Sport::Running)
            ]))
            .matches(&activity)
        );
        assert!(
            !TrainingMetricFilters::new(Some(vec![SportFilter::Sport(Sport::Running)]))
                .matches(&activity)
        );
        assert!(!TrainingMetricFilters::new(Some(vec![])).matches(&activity));
    }
}

#[cfg(test)]
mod test_training_period {

    use crate::domain::models::activity::{ActivityId, ActivityStatistics};

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
        Activity::new(
            ActivityId::default(),
            UserId::test_default(),
            None,
            ActivityStartTime::new(start.parse::<DateTime<FixedOffset>>().unwrap()),
            Sport::Running,
            ActivityStatistics::new(HashMap::new()),
            None,
            None,
            None,
        )
    }

    fn activity_with_sport(sport: Sport) -> Activity {
        Activity::new(
            ActivityId::default(),
            UserId::test_default(),
            None,
            ActivityStartTime::new(
                "2025-10-01T12:00:00+02:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            sport,
            ActivityStatistics::new(HashMap::new()),
            None,
            None,
            None,
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
        let end = Some("2025-10-21".parse::<NaiveDate>().unwrap());

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

        // Both methods should return the same result when end date is specified
        let range_today = period.range_default_today();
        let range_tomorrow = period.range_default_tomorrow();

        assert_eq!(range_today.start(), &start);
        assert_eq!(range_today.end(), &end.unwrap());
        assert_eq!(range_tomorrow.start(), &start);
        assert_eq!(range_tomorrow.end(), &end.unwrap());
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
        ActivityId, ActivityNutrition, ActivityRpe, ActivityStatistics, BonkStatus, WorkoutType,
    };

    use super::*;

    #[test]
    fn test_extract_group_from_activity() {
        let activity = Activity::new(
            ActivityId::default(),
            UserId::test_default(),
            None,
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Sport::TrailRunning,
            ActivityStatistics::new(HashMap::new()),
            Some(ActivityRpe::Six),
            Some(WorkoutType::Intervals),
            Some(ActivityNutrition::new(BonkStatus::Bonked, None)),
        );

        assert_eq!(
            TrainingMetricGroupBy::Sport.extract_group(&activity),
            Some("TrailRunning".to_string())
        );

        assert_eq!(
            TrainingMetricGroupBy::SportCategory.extract_group(&activity),
            Some("Running".to_string())
        );

        assert_eq!(
            TrainingMetricGroupBy::WorkoutType.extract_group(&activity),
            Some("intervals".to_string())
        );

        assert_eq!(
            TrainingMetricGroupBy::RpeRange.extract_group(&activity),
            Some("moderate".to_string())
        );

        assert_eq!(
            TrainingMetricGroupBy::Bonked.extract_group(&activity),
            Some("bonked".to_string())
        );
    }

    #[test]
    fn test_extract_group_from_activity_none() {
        let activity = Activity::new(
            ActivityId::default(),
            UserId::test_default(),
            None,
            ActivityStartTime::new(
                "2025-09-03T00:00:00Z"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap(),
            ),
            Sport::Golf,
            ActivityStatistics::new(HashMap::new()),
            None,
            None,
            None,
        );

        assert_eq!(
            TrainingMetricGroupBy::SportCategory.extract_group(&activity),
            None
        );

        assert_eq!(
            TrainingMetricGroupBy::WorkoutType.extract_group(&activity),
            None
        );

        assert_eq!(
            TrainingMetricGroupBy::RpeRange.extract_group(&activity),
            None
        );

        assert_eq!(TrainingMetricGroupBy::Bonked.extract_group(&activity), None);
    }
}
