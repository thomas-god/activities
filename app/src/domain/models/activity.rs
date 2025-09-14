use std::{
    collections::HashMap,
    fmt::{self},
};

use chrono::{DateTime, FixedOffset};
use derive_more::{AsRef, Constructor, Deref, Display, From, Into};
use uuid::Uuid;

use crate::domain::models::UserId;

///////////////////////////////////////////////////////////////////
/// ACTIVITY
///////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct Activity {
    id: ActivityId,
    user: UserId,
    start_time: ActivityStartTime,
    duration: ActivityDuration,
    sport: Sport,
    statistics: ActivityStatistics,
    timeseries: ActivityTimeseries,
}

/// An [Activity] is an entity representing a single sport activity or training session.
impl Activity {
    pub fn new(
        id: ActivityId,
        user: UserId,
        start_time: ActivityStartTime,
        duration: ActivityDuration,
        sport: Sport,
        statistics: ActivityStatistics,
        timeseries: ActivityTimeseries,
    ) -> Self {
        Self {
            id,
            user,
            start_time,
            duration,
            sport,
            statistics,
            timeseries,
        }
    }

    /// An [Activity]'s natural key if a key generated from its defining fields. Two activities with
    /// identical natural keys should be considered identical/duplicate regardless of their
    /// technical [Activity::id].
    pub fn natural_key(&self) -> ActivityNaturalKey {
        ActivityNaturalKey(format!(
            "{:?}{:?}:{:?}:{:?}",
            self.user, self.sport, self.start_time, self.duration
        ))
    }

    pub fn id(&self) -> &ActivityId {
        &self.id
    }

    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn start_time(&self) -> &ActivityStartTime {
        &self.start_time
    }

    pub fn duration(&self) -> &ActivityDuration {
        &self.duration
    }

    pub fn sport(&self) -> &Sport {
        &self.sport
    }

    pub fn statistics(&self) -> &ActivityStatistics {
        &self.statistics
    }

    pub fn timeseries(&self) -> &ActivityTimeseries {
        &self.timeseries
    }
}

/// Technical ID of an [Activity].
#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, AsRef, Deref, Hash)]
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

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, AsRef, Deref, Hash)]
pub struct ActivityNaturalKey(String);

#[derive(
    Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, AsRef, Deref, Hash, From, Into, Copy,
)]
pub struct ActivityStartTime(DateTime<FixedOffset>);

impl ActivityStartTime {
    pub fn new(datetime: DateTime<FixedOffset>) -> Self {
        Self(datetime)
    }

    pub fn from_timestamp(timestamp: usize) -> Option<Self> {
        DateTime::from_timestamp(timestamp as i64, 0).map(|dt| Self(dt.fixed_offset()))
    }
}

#[derive(
    Clone,
    Debug,
    Display,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    AsRef,
    Deref,
    Hash,
    From,
    Into,
    Copy,
    Constructor,
)]

pub struct ActivityDuration(pub usize);

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Sport {
    Running,
    Cycling,
    Other,
}

#[derive(Clone, Debug, Constructor, Default, Deref)]
pub struct ActivityStatistics(HashMap<ActivityStatistic, f64>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
pub enum ActivityStatistic {
    Calories,
    Elevation,
    Distance,
}

impl ToUnit for ActivityStatistic {
    fn unit(&self) -> Unit {
        match self {
            Self::Calories => Unit::KiloCalorie,
            Self::Elevation => Unit::Meter,
            Self::Distance => Unit::Meter,
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
#[derive(Debug, Clone, PartialEq, Constructor, AsRef, Deref, Default)]
pub struct TimeseriesTime(Vec<usize>);

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

#[derive(Debug, Clone, PartialEq, Display)]
pub enum TimeseriesMetric {
    Speed,
    Power,
    HeartRate,
    Distance,
}

impl ToUnit for TimeseriesMetric {
    fn unit(&self) -> Unit {
        match self {
            Self::Distance => Unit::Meter,
            Self::Power => Unit::Watt,
            Self::HeartRate => Unit::BeatPerMinute,
            Self::Speed => Unit::MeterPerSecond,
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
            UserId::default(),
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
            ActivityStatistics::default(),
            ActivityTimeseries::default(),
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            UserId::default(),
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration(100),
            Sport::Running,
            ActivityStatistics::default(),
            ActivityTimeseries::default(),
        );

        assert_ne!(first_activity.natural_key(), second_activity.natural_key());
    }

    #[test]
    fn test_similar_activities_same_natural_keys() {
        let first_activity = Activity::new(
            ActivityId::new(),
            UserId::default(),
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
            ActivityStatistics::default(),
            ActivityTimeseries::default(),
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            UserId::default(),
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
            ActivityStatistics::default(),
            ActivityTimeseries::default(),
        );

        assert_eq!(first_activity.natural_key(), second_activity.natural_key());
    }

    #[test]
    fn test_same_activity_different_user_natural_keys_not_equal() {
        let first_activity = Activity::new(
            ActivityId::new(),
            UserId::default(),
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
            ActivityStatistics::default(),
            ActivityTimeseries::default(),
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            "another_user".to_string().into(),
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
            ActivityStatistics::default(),
            ActivityTimeseries::default(),
        );

        assert_ne!(first_activity.natural_key(), second_activity.natural_key());
    }
}
