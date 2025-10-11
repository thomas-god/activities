use std::{
    collections::HashMap,
    fmt::{self},
    hash::Hash,
};

use chrono::{DateTime, FixedOffset};
use derive_more::{AsRef, Constructor, Display, From, Into};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::UserId;

///////////////////////////////////////////////////////////////////
/// ACTIVITY
///////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct Activity {
    id: ActivityId,
    user: UserId,
    name: Option<ActivityName>,
    start_time: ActivityStartTime,
    sport: Sport,
    statistics: ActivityStatistics,
}

#[allow(clippy::too_many_arguments)]
/// An [Activity] is an entity representing a single sport activity or training session.
impl Activity {
    pub fn new(
        id: ActivityId,
        user: UserId,
        name: Option<ActivityName>,
        start_time: ActivityStartTime,
        sport: Sport,
        statistics: ActivityStatistics,
    ) -> Self {
        Self {
            id,
            user,
            name,
            start_time,
            sport,
            statistics,
        }
    }

    /// An [Activity]'s natural key if a key generated from its defining fields. Two activities with
    /// identical natural keys should be considered identical/duplicate regardless of their
    /// technical [Activity::id].
    pub fn natural_key(&self) -> ActivityNaturalKey {
        let duration = self
            .statistics
            .get(&ActivityStatistic::Duration)
            .unwrap_or(&0.);
        ActivityNaturalKey(format!(
            "{}:{}:{}:{}",
            self.user, self.sport, self.start_time, duration
        ))
    }

    pub fn id(&self) -> &ActivityId {
        &self.id
    }

    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn name(&self) -> Option<&ActivityName> {
        self.name.as_ref()
    }

    pub fn start_time(&self) -> &ActivityStartTime {
        &self.start_time
    }

    pub fn sport(&self) -> &Sport {
        &self.sport
    }

    pub fn statistics(&self) -> &ActivityStatistics {
        &self.statistics
    }
}

#[derive(Clone, Debug, Constructor)]
pub struct ActivityWithTimeseries {
    activity: Activity,
    timeseries: ActivityTimeseries,
}

impl ActivityWithTimeseries {
    pub fn activity(&self) -> &Activity {
        &self.activity
    }

    pub fn id(&self) -> &ActivityId {
        self.activity.id()
    }

    pub fn natural_key(&self) -> ActivityNaturalKey {
        self.activity.natural_key()
    }

    pub fn user(&self) -> &UserId {
        self.activity.user()
    }

    pub fn name(&self) -> Option<&ActivityName> {
        self.activity.name()
    }

    pub fn start_time(&self) -> &ActivityStartTime {
        self.activity.start_time()
    }

    pub fn sport(&self) -> &Sport {
        self.activity.sport()
    }

    pub fn statistics(&self) -> &ActivityStatistics {
        self.activity.statistics()
    }
    pub fn timeseries(&self) -> &ActivityTimeseries {
        &self.timeseries
    }
}

/// Technical ID of an [Activity].
#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, AsRef, Hash)]
pub struct ActivityId(String);

impl ActivityId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl Default for ActivityId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Constructor)]
pub struct ActivityName(String);

impl fmt::Display for ActivityName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ActivityName {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActivityNaturalKey(String);

impl From<&str> for ActivityNaturalKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into, Copy)]
pub struct ActivityStartTime(DateTime<FixedOffset>);

impl ActivityStartTime {
    pub fn new(datetime: DateTime<FixedOffset>) -> Self {
        Self(datetime)
    }

    pub fn from_timestamp(timestamp: usize) -> Option<Self> {
        DateTime::from_timestamp(timestamp as i64, 0).map(|dt| Self(dt.fixed_offset()))
    }

    pub fn date(&self) -> &DateTime<FixedOffset> {
        &self.0
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Display, Serialize, Deserialize)]
pub enum Sport {
    Running,
    Cycling,
    Swimming,
    AlpineSKi,
    StrengthTraining,
    Other,
}

#[derive(Clone, Debug, Constructor, Default, Serialize, Deserialize, PartialEq)]
pub struct ActivityStatistics(HashMap<ActivityStatistic, f64>);

impl ActivityStatistics {
    pub fn get(&self, stat: &ActivityStatistic) -> Option<&f64> {
        self.0.get(stat)
    }

    pub fn items(&self) -> HashMap<String, f64> {
        HashMap::from_iter(
            self.0
                .iter()
                .map(|(stat, value)| (stat.to_string(), *value)),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
pub enum ActivityStatistic {
    Duration,
    Calories,
    Elevation,
    Distance,
    NormalizedPower,
}

impl ToUnit for ActivityStatistic {
    fn unit(&self) -> Unit {
        match self {
            Self::Duration => Unit::Second,
            Self::Calories => Unit::KiloCalorie,
            Self::Elevation => Unit::Meter,
            Self::Distance => Unit::Meter,
            Self::NormalizedPower => Unit::Watt,
        }
    }
}

/// Trait to represent the associated physical unit (e.g., meters, watt) of some value.
pub trait ToUnit {
    fn unit(&self) -> Unit;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    KiloCalorie,
    Meter,
    Kilometer,
    MeterPerSecond,
    KilometerPerHour,
    Watt,
    BeatPerMinute,
    RevolutionPerMinute,
    Second,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = match self {
            Self::KiloCalorie => "kcal",
            Self::Meter => "m",
            Self::Kilometer => "km",
            Self::MeterPerSecond => "m/s",
            Self::KilometerPerHour => "km/h",
            Self::Watt => "W",
            Self::BeatPerMinute => "bpm",
            Self::RevolutionPerMinute => "rpm",
            Self::Second => "s",
        };

        write!(f, "{}", unit)
    }
}

///////////////////////////////////////////////////////////////////
// TIMESERIES
///////////////////////////////////////////////////////////////////

/// An [ActivityTimeseries] is a coherent set of time dependant [TimeseriesMetric] (plural)
/// from the same [Activity].
#[derive(Debug, Clone, PartialEq, Constructor, Default)]
pub struct ActivityTimeseries {
    time: TimeseriesTime,
    metrics: Vec<Timeseries>,
}

impl ActivityTimeseries {
    pub fn time(&self) -> &TimeseriesTime {
        &self.time
    }

    pub fn metrics(&self) -> &[Timeseries] {
        &self.metrics
    }
}

/// [TimeseriesTime] represents the relative timestamp of a timeseries, starting from the
/// [Activity::start_time].
#[derive(Debug, Clone, PartialEq, Constructor, AsRef, Default)]
pub struct TimeseriesTime(Vec<usize>);

impl TimeseriesTime {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn values(&self) -> &[usize] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct Timeseries {
    metric: TimeseriesMetric,
    values: Vec<Option<TimeseriesValue>>,
}

impl Timeseries {
    pub fn metric(&self) -> &TimeseriesMetric {
        &self.metric
    }

    pub fn values(&self) -> &[Option<TimeseriesValue>] {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
pub enum TimeseriesMetric {
    Speed,
    Power,
    HeartRate,
    Distance,
    Cadence,
    Altitude,
}

impl ToUnit for TimeseriesMetric {
    fn unit(&self) -> Unit {
        match self {
            Self::Distance => Unit::Meter,
            Self::Power => Unit::Watt,
            Self::HeartRate => Unit::BeatPerMinute,
            Self::Speed => Unit::MeterPerSecond,
            Self::Altitude => Unit::Meter,
            Self::Cadence => Unit::RevolutionPerMinute,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeseriesValue {
    Int(usize),
    Float(f64),
}

impl From<&TimeseriesValue> for f64 {
    fn from(value: &TimeseriesValue) -> Self {
        match value {
            TimeseriesValue::Int(val) => *val as f64,
            TimeseriesValue::Float(val) => *val,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_different_activities_different_natural_keys() {
        let first_activity = Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Cycling,
            ActivityStatistics::default(),
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Running,
            ActivityStatistics::default(),
        );

        assert_ne!(first_activity.natural_key(), second_activity.natural_key());
    }

    #[test]
    fn test_similar_activities_same_natural_keys() {
        let first_activity = Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Cycling,
            ActivityStatistics::default(),
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Cycling,
            ActivityStatistics::default(),
        );

        assert_eq!(first_activity.natural_key(), second_activity.natural_key());
    }

    #[test]
    fn test_same_activity_different_user_natural_keys_not_equal() {
        let first_activity = Activity::new(
            ActivityId::new(),
            UserId::test_default(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Cycling,
            ActivityStatistics::default(),
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            "another_user".to_string().into(),
            None,
            ActivityStartTime::from_timestamp(0).unwrap(),
            Sport::Cycling,
            ActivityStatistics::default(),
        );

        assert_ne!(first_activity.natural_key(), second_activity.natural_key());
    }
}
