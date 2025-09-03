use derive_more::{AsRef, Deref, Display, From, Into};
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

#[derive(
    Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, AsRef, Deref, Hash, From, Into, Copy,
)]
pub struct ActivityStartTime(pub usize);

#[derive(
    Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, AsRef, Deref, Hash, From, Into, Copy,
)]
pub struct ActivityDuration(pub usize);

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Sport {
    Running,
    Cycling,
    Other,
}
