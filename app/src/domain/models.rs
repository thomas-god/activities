use derive_more::{AsRef, Deref, Display};
use uuid::Uuid;

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Sport {
    Running,
    Cycling,
    Other,
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(Default))]
pub struct Activity {
    id: ActivityId,
    calories: Option<usize>,
    duration: Option<usize>,
    sport: Option<Sport>,
}

impl Activity {
    pub fn new(
        id: ActivityId,
        calories: Option<usize>,
        duration: Option<usize>,
        sport: Option<Sport>,
    ) -> Self {
        Self {
            id,
            calories,
            duration,
            sport,
        }
    }

    pub fn id(&self) -> &ActivityId {
        &self.id
    }

    pub fn calories(&self) -> &Option<usize> {
        &self.calories
    }

    pub fn duration(&self) -> &Option<usize> {
        &self.duration
    }

    pub fn sport(&self) -> &Option<Sport> {
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
