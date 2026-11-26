use std::fmt::Debug;

use activities_core::infra::config::UserAuthConfig;

use crate::domain::AuthMethods;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserRegistration {
    Allowed,
    Closed,
}

impl From<&bool> for UserRegistration {
    fn from(value: &bool) -> Self {
        if *value { Self::Allowed } else { Self::Closed }
    }
}

impl From<bool> for UserRegistration {
    fn from(value: bool) -> Self {
        Self::from(&value)
    }
}

impl From<&UserRegistration> for bool {
    fn from(value: &UserRegistration) -> Self {
        match value {
            UserRegistration::Allowed => true,
            UserRegistration::Closed => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UserAuthStrategy {
    auth_methods: AuthMethods,
    registration: UserRegistration,
}

impl UserAuthStrategy {
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

impl From<&UserAuthConfig> for UserAuthStrategy {
    fn from(config: &UserAuthConfig) -> Self {
        Self {
            auth_methods: AuthMethods::new(config.email.is_some()),
            registration: config.allow_registration.into(),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {

    use axum::{
        extract::FromRequestParts,
        http::{StatusCode, request::Parts},
    };

    use activities_core::domain::shared::UserId;

    use crate::infra::http::AuthenticatedUser;

    /// Dummy extractor that always returns the default [UserId], regardless of the request.
    pub struct DefaultUserExtractor;

    impl<S> FromRequestParts<S> for DefaultUserExtractor
    where
        S: Send + Sync,
    {
        type Rejection = StatusCode;

        async fn from_request_parts(
            parts: &mut Parts,
            _state: &S,
        ) -> Result<Self, Self::Rejection> {
            let user = AuthenticatedUser::new(UserId::default());

            parts.extensions.insert(user);

            Ok(Self)
        }
    }
}
