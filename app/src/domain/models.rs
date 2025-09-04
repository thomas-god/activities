use chrono::{DateTime, NaiveDateTime};
use derive_more::{AsRef, Constructor, Deref, Display, From, Into};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Activity {
    id: ActivityId,
    start_time: ActivityStartTime,
    duration: ActivityDuration,
    sport: Sport,
}

impl Activity {
    pub fn new(
        id: ActivityId,
        start_time: ActivityStartTime,
        duration: ActivityDuration,
        sport: Sport,
    ) -> Self {
        Self {
            id,
            start_time,
            duration,
            sport,
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
}

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, AsRef, Deref, Hash)]
pub struct ActivityId(String);

impl ActivityId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
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
pub struct ActivityStartTime(NaiveDateTime);

impl ActivityStartTime {
    pub fn new(datetime: usize) -> Option<Self> {
        DateTime::from_timestamp(datetime as i64, 0).map(|dt| Self(dt.naive_utc()))
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_different_activities_different_natural_keys() {
        let first_activity = Activity::new(
            ActivityId::new(),
            ActivityStartTime::new(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            ActivityStartTime::new(0).unwrap(),
            ActivityDuration(100),
            Sport::Running,
        );

        assert_ne!(first_activity.natural_key(), second_activity.natural_key());
    }

    #[test]
    fn test_similar_activities_same_natural_keys() {
        let first_activity = Activity::new(
            ActivityId::new(),
            ActivityStartTime::new(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
        );
        let second_activity = Activity::new(
            ActivityId::new(),
            ActivityStartTime::new(0).unwrap(),
            ActivityDuration(100),
            Sport::Cycling,
        );

        assert_eq!(first_activity.natural_key(), second_activity.natural_key());
    }
}
