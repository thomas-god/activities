use std::fmt::Debug;

use derive_more::{Constructor, From};

use crate::{
    config::{AppMode, SingleUserConfig},
    domain::models::UserId,
};

pub mod email_based;
pub mod infra;
pub mod no_auth;
pub mod single_password;

#[derive(Clone, Constructor, From, PartialEq)]
#[from(String, &str)]
pub struct SinglePassword(String);

impl SinglePassword {
    pub fn value(&self) -> &str {
        &self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// Manual impl of Debug to avoid leaking the inner value.
impl Debug for SinglePassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SinglePassword")
    }
}

#[derive(Clone)]
pub enum AuthStrategy {
    NoAuth,
    SinglePassword(SinglePassword),
    EmailBased,
}

/// Manual impl of Debug to avoid leaking the value of [AuthStrategy::SinglePassword].
impl Debug for AuthStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAuth => write!(f, "NoAuth"),
            AuthStrategy::EmailBased => write!(f, "EmailBased"),
            AuthStrategy::SinglePassword(_) => write!(f, "SinglePassword"),
        }
    }
}

impl From<&AppMode> for AuthStrategy {
    fn from(value: &AppMode) -> Self {
        match value {
            AppMode::MultiUser(_) => AuthStrategy::EmailBased,
            AppMode::SingleUser(SingleUserConfig {
                password: Some(pwd),
            }) => AuthStrategy::SinglePassword(SinglePassword::from(pwd.clone())),
            AppMode::SingleUser(SingleUserConfig { password: None }) => AuthStrategy::NoAuth,
        }
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct AuthenticatedUser(UserId);

impl AuthenticatedUser {
    pub fn user(&self) -> &UserId {
        &self.0
    }
}

#[cfg(test)]
mod test_auth {
    use super::*;

    #[test]
    fn test_do_not_leak_single_password_content_when_debug() {
        let password = SinglePassword::from("secret");
        assert_eq!(format!("{password:?}"), "SinglePassword");
        let strategy = AuthStrategy::SinglePassword(SinglePassword::from("secret"));
        assert_eq!(format!("{strategy:?}"), "SinglePassword")
    }
}
