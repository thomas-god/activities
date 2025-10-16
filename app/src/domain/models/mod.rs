use std::fmt;

use derive_more::Constructor;

pub mod activity;
pub mod training;

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct User {
    id: UserId,
}

impl User {
    pub fn id(&self) -> &UserId {
        &self.id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(String);

impl UserId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    #[cfg(test)]
    pub fn test_default() -> Self {
        Self("test_user".to_string())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self("default_user".to_string())
    }
}

impl From<String> for UserId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for UserId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
