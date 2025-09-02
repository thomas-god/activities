use derive_more::{AsRef, Deref, Display};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Activity {
    id: ActivityId,
    calories: Option<usize>,
}

impl Activity {
    pub fn new(id: ActivityId, calories: Option<usize>) -> Self {
        Self { id, calories }
    }

    pub fn id(&self) -> &ActivityId {
        &self.id
    }

    pub fn calories(&self) -> &Option<usize> {
        &self.calories
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, AsRef, Deref)]
pub struct ActivityId(String);

impl ActivityId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}
