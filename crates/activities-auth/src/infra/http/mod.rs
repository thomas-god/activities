use activities_core::domain::shared::UserId;
use derive_more::Constructor;

pub mod auth_info;
pub mod cookie;
pub mod multi_user;
pub mod no_auth;
pub mod rate_limit;
pub mod single_password;

pub use auth_info::add_auth_router;
pub use cookie::{CookieConfig, build_removal_cookie};

#[derive(Debug, Clone, Constructor)]
pub struct AuthenticatedUser(UserId);

impl AuthenticatedUser {
    pub fn user(&self) -> &UserId {
        &self.0
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct AuthenticatedSession(crate::multi_user::HashedSessionToken);

impl AuthenticatedSession {
    pub fn hash(&self) -> &crate::multi_user::HashedSessionToken {
        &self.0
    }
}

/// Marker inserted on a response by a handler that has already revoked the current session. The
/// cookie middleware must then not append a freshly rotated session cookie, which would override
/// the handler's session-cookie removal with a cookie pointing at a deleted session.
#[derive(Debug, Clone, Copy)]
pub struct SessionRevoked;
