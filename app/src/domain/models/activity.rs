use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};
use derive_more::{AsRef, Constructor, Deref, Display, From, Into};
use uuid::Uuid;

///////////////////////////////////////////////////////////////////
/// ACTIVITY
///////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct Activity {
    id: ActivityId,
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
        start_time: ActivityStartTime,
        duration: ActivityDuration,
        sport: Sport,
        statistics: ActivityStatistics,
        timeseries: ActivityTimeseries,
    ) -> Self {
        Self {
            id,
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
            "{:?}:{:?}:{:?}",
            self.sport, self.start_time, self.duration
        ))
    }

    pub fn id(&self) -> &ActivityId {
        &self.id
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityStatistic {
    Calories,
    Elevation,
    Distance,
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

impl TimeseriesMetric {
    pub fn unit(&self) -> String {
        match self {
            Self::Distance => "m".to_string(),
            Self::Power => "W".to_string(),
            Self::HeartRate => "bpm".to_string(),
            Self::Speed => "m/s".to_string(),
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
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
            ActivityStatistics::default(),
            ActivityTimeseries::default(),
        );
        let second_activity = Activity::new(
            ActivityId::new(),
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
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
            ActivityStatistics::default(),
            ActivityTimeseries::default(),
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
            ActivityStatistics::default(),
            ActivityTimeseries::default(),
        );

        assert_eq!(first_activity.natural_key(), second_activity.natural_key());
    }
}
