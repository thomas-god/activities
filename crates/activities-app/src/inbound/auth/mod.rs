use std::fmt::Debug;

use activities_auth::{
    auth::MultiUserAuthStrategy,
    multi_user::{AuthMethods, HashedSessionToken},
};
use activities_domain::shared::UserId;
use activities_infra::config::{AppMode, SingleUserConfig};
use derive_more::Constructor;

pub mod infra;
pub mod multi_user;
pub mod no_auth;
pub mod single_password;

pub use activities_auth::auth::{AuthStrategy, SinglePassword, UserRegistration};

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

pub fn auth_strategy_from_mode(value: &AppMode) -> AuthStrategy {
    match value {
        AppMode::MultiUser(config) => AuthStrategy::MultiUser(MultiUserAuthStrategy::new(
            AuthMethods::new(config.email.is_some()),
            UserRegistration::from(config.allow_registration),
        )),
        AppMode::SingleUser(SingleUserConfig {
            password: Some(pwd),
        }) => AuthStrategy::SinglePassword(SinglePassword::from(pwd.clone())),
        AppMode::SingleUser(SingleUserConfig { password: None }) => AuthStrategy::NoAuth,
    }
}
