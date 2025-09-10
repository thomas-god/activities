use chrono::{DateTime, FixedOffset};
use derive_more::{AsRef, Constructor, Deref, Display, From, Into};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Activity {
    id: ActivityId,
    start_time: ActivityStartTime,
    duration: ActivityDuration,
    sport: Sport,
    timeseries: Timeseries,
}

impl Activity {
    pub fn new(
        id: ActivityId,
        start_time: ActivityStartTime,
        duration: ActivityDuration,
        sport: Sport,
        timeseries: Timeseries,
    ) -> Self {
        Self {
            id,
            start_time,
            duration,
            sport,
            timeseries,
        }
    }

    /// Activity's natural key defined from its defining fields. Two activities with identical
    /// natural keys should be considered identical/duplicate regardless of their ID.
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

    pub fn timeseries(&self) -> &Timeseries {
        &self.timeseries
    }
}

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

#[derive(Debug, Clone, PartialEq, Constructor, Default)]
pub struct Timeseries {
    time: TimeseriesTime,
    metrics: Vec<TimeseriesMetric>,
}

impl Timeseries {
    pub fn time(&self) -> &TimeseriesTime {
        &self.time
    }

    pub fn metrics(&self) -> &[TimeseriesMetric] {
        &self.metrics
    }
}

#[derive(Debug, Clone, PartialEq, Constructor, AsRef, Deref, Default)]
pub struct TimeseriesTime(Vec<usize>);

#[derive(Debug, Clone, PartialEq)]
pub enum TimeseriesMetric {
    Speed(Vec<Option<f64>>),
    Power(Vec<Option<usize>>),
    HeartRate(Vec<Option<usize>>),
    Distance(Vec<Option<f32>>),
}

impl TimeseriesMetric {
    pub fn len(&self) -> usize {
        match self {
            Self::Distance(values) => values.len(),
            Self::Power(values) => values.len(),
            Self::HeartRate(values) => values.len(),
            Self::Speed(values) => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn unit(&self) -> String {
        match self {
            Self::Distance(_) => "m".to_string(),
            Self::Power(_) => "W".to_string(),
            Self::HeartRate(_) => "bpm".to_string(),
            Self::Speed(_) => "m/s".to_string(),
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
            Timeseries::default(),
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration(100),
            Sport::Running,
            Timeseries::default(),
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
            Timeseries::default(),
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            ActivityStartTime::from_timestamp(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
            Timeseries::default(),
        );

        assert_eq!(first_activity.natural_key(), second_activity.natural_key());
    }
}
