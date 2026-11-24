use std::fmt::{Debug, Display};

use derive_more::{Constructor, From};
use subtle::ConstantTimeEq;

use crate::{
    config::{AppMode, SingleUserConfig},
    domain::models::UserId,
    inbound::auth::multi_user::{
        AuthMethods, HashedSessionToken, infra::handlers::UserRegistration,
    },
};

pub mod infra;
pub mod multi_user;
pub mod no_auth;
pub mod single_password;

#[derive(Clone, Constructor, From)]
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

/// Constant-time comparison to avoid leaking information through response timing attacks.
impl PartialEq for SinglePassword {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_bytes().ct_eq(other.0.as_bytes()).into()
    }
}

impl Eq for SinglePassword {}

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
    MultiUser(MultiUserAuthStrategy),
}

#[derive(Clone, Debug)]
pub struct MultiUserAuthStrategy {
    auth_methods: AuthMethods,
    registration: UserRegistration,
}

impl MultiUserAuthStrategy {
    pub fn new(auth_methods: AuthMethods, registration: UserRegistration) -> Self {
        Self {
            auth_methods,
            registration,
        }
    }

    pub fn auth_methods(&self) -> &AuthMethods {
        &self.auth_methods
    }

    pub fn registration(&self) -> &UserRegistration {
        &self.registration
    }
}

/// Manual impl of Debug to avoid leaking the value of [AuthStrategy::SinglePassword].
impl Debug for AuthStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAuth => write!(f, "NoAuth"),
            AuthStrategy::MultiUser(strategy) => write!(f, "MultiUser with config: {:?}", strategy),
            AuthStrategy::SinglePassword(_) => write!(f, "SinglePassword"),
        }
    }
}

/// Manual impl of Display to avoid leaking the value of [AuthStrategy::SinglePassword].
impl Display for AuthStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAuth => write!(f, "NoAuth"),
            AuthStrategy::MultiUser(_) => write!(f, "MultiUser",),
            AuthStrategy::SinglePassword(_) => write!(f, "SinglePassword"),
        }
    }
}

impl From<&AppMode> for AuthStrategy {
    fn from(value: &AppMode) -> Self {
        match value {
            AppMode::MultiUser(config) => AuthStrategy::MultiUser(MultiUserAuthStrategy::new(
                AuthMethods::from(config),
                UserRegistration::from(config.allow_registration),
            )),
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

#[derive(Debug, Clone, Constructor)]
pub struct AuthenticatedSession(HashedSessionToken);

impl AuthenticatedSession {
    pub fn hash(&self) -> &HashedSessionToken {
        &self.0
    }
}

/// Marker inserted on a response by a handler that has already revoked the current session. The
/// cookie middleware must then not append a freshly rotated session cookie, which would override
/// the handler's session-cookie removal with a cookie pointing at a deleted session.
#[derive(Debug, Clone, Copy)]
pub struct SessionRevoked;

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

    #[test]
    fn test_single_password_eq_identical_values() {
        assert_eq!(
            SinglePassword::from("secret"),
            SinglePassword::from("secret")
        );
    }

    #[test]
    fn test_single_password_eq_both_empty() {
        assert_eq!(SinglePassword::from(""), SinglePassword::from(""));
    }

    #[test]
    fn test_single_password_not_eq_different_values_same_length() {
        assert_ne!(
            SinglePassword::from("secret1"),
            SinglePassword::from("secreu1")
        );
    }

    #[test]
    fn test_single_password_not_eq_different_length() {
        assert_ne!(
            SinglePassword::from("secret"),
            SinglePassword::from("secret-but-longer")
        );
        assert_ne!(SinglePassword::from("secret"), SinglePassword::from(""));
    }

    #[test]
    fn test_single_password_not_eq_prefix_of_other() {
        assert_ne!(
            SinglePassword::from("secret"),
            SinglePassword::from("secretsecret")
        );
    }
}
