use std::{marker::PhantomData, sync::Arc};

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use chrono::Utc;
use derive_more::Constructor;
use thiserror::Error;

use crate::{
    domain::{
        models::UserId,
        ports::{IActivityService, ITrainingMetricService},
    },
    inbound::{http::AppState, parser::ParseFile},
};

pub mod infra;
pub mod service;

#[derive(Debug, Clone, Constructor)]
pub struct AuthenticatedUser(UserId);

impl AuthenticatedUser {
    pub fn user(&self) -> &UserId {
        &self.0
    }
}

/// Dummy extractor that always returns the default [UserId], regardless of the request.
pub struct DefaultUserExtractor;

impl<S> FromRequestParts<S> for DefaultUserExtractor
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::new(UserId::default());

        parts.extensions.insert(user);

        Ok(Self)
    }
}

#[derive(Clone, Debug, Constructor, PartialEq, Eq, Hash)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl From<&str> for EmailAddress {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for EmailAddress {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Constructor, PartialEq, Eq)]
pub struct MagicToken(String);

impl MagicToken {
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl From<&str> for MagicToken {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for MagicToken {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Constructor)]
pub struct MagicLink {
    user: UserId,
    token: MagicToken,
    expire_at: chrono::DateTime<Utc>,
}

impl MagicLink {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn token(&self) -> &MagicToken {
        &self.token
    }

    pub fn expire_at(&self) -> &chrono::DateTime<Utc> {
        &self.expire_at
    }

    pub fn is_expired(&self, reference: &chrono::DateTime<Utc>) -> bool {
        reference >= &self.expire_at
    }
}

#[derive(Clone, Debug, Constructor, PartialEq, Eq)]
pub struct SessionToken(String);

impl SessionToken {
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SessionToken {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for SessionToken {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Error)]
pub enum SessionTokenError {
    #[error("Session token {0:?} does not exist")]
    SessionTokenDoesNotExistsError(SessionToken),
}

#[derive(Debug, Clone)]
pub enum GenerateMagicLinkResult {
    /// [GenerateMagicLinkResult::Success] covers all functional cases, regardless of user actually
    /// existing to not leak that information.
    Success,
    /// [GenerateMagicLinkResult::Retry] only covers infrastructure related issues for which the
    /// user can actually retry (e.g. we fail to send the email containing the magic link).
    Retry,
}

pub trait ISessionService: Clone + Send + Sync + 'static {
    fn generate_magic_link(
        &self,
        email: &EmailAddress,
    ) -> impl Future<Output = GenerateMagicLinkResult> + Send;

    fn check_session_token(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<UserId, SessionTokenError>> + Send;
}

impl<AS, PF, TMS, SR> FromRef<AppState<AS, PF, TMS, SR>> for Option<Arc<SR>>
where
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    SR: ISessionService,
{
    fn from_ref(input: &AppState<AS, PF, TMS, SR>) -> Self {
        input.session_service.clone()
    }
}

/// Extractor that tries to extract user information from the request's session cookie.
pub struct CookieUserExtractor<SR>(PhantomData<SR>);

impl<S, SR> FromRequestParts<S> for CookieUserExtractor<SR>
where
    S: Send + Sync,
    SR: ISessionService,
    Option<Arc<SR>>: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Some(service) = Option::<Arc<SR>>::from_ref(state) else {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };
        let jar = axum_extra::extract::CookieJar::from_headers(&parts.headers);
        let Some(session_token) = jar.get("session_token") else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        let Ok(user) = service.check_session_token(session_token.value()).await else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        parts.extensions.insert(AuthenticatedUser::new(user));

        Ok(Self(PhantomData))
    }
}

#[cfg(test)]
pub mod test_utils {
    use mockall::mock;

    use super::*;

    mock! {
        pub SessionService {}

        impl Clone for SessionService {
            fn clone(&self) -> Self;
        }

        impl ISessionService for SessionService {
            async fn generate_magic_link(
                &self,
                email: &EmailAddress
            ) -> GenerateMagicLinkResult;

            async fn check_session_token(
                &self,
                _token: &str
            ) -> Result<UserId, SessionTokenError>;
        }
    }
}

#[cfg(test)]
mod test {
    use axum::{
        Extension, Router,
        middleware::{from_extractor, from_extractor_with_state},
        routing::get,
    };
    use axum_extra::extract::cookie::Cookie;
    use axum_test::TestServer;
    use chrono::TimeDelta;

    use crate::{
        domain::services::{
            activity::test_utils::MockActivityService,
            training_metrics::test_utils::MockTrainingMetricService,
        },
        inbound::{http::auth::test_utils::MockSessionService, parser::test_utils::MockFileParser},
    };

    use super::*;

    #[tokio::test]
    async fn test_default_user_extractor() {
        async fn test_route(
            Extension(user): Extension<AuthenticatedUser>,
        ) -> impl axum::response::IntoResponse {
            user.user().to_string()
        }

        let app = Router::new()
            .route("/", get(test_route))
            .route_layer(from_extractor::<DefaultUserExtractor>());
        let server = TestServer::new(app).expect("unable to create test server");

        let response = server.get("/").await;
        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), UserId::default().to_string());
    }

    #[test]
    fn test_magic_link_expiry() {
        let expire_at = chrono::Utc::now();
        let link = MagicLink::new(UserId::test_default(), "a link".into(), expire_at);
        assert!(link.is_expired(&(expire_at + TimeDelta::seconds(1))));
        assert!(!link.is_expired(&(expire_at - TimeDelta::seconds(1))));
    }

    fn build_test_server(session_service: MockSessionService) -> TestServer {
        let state = AppState {
            activity_service: Arc::new(MockActivityService::new()),
            training_metrics_service: Arc::new(MockTrainingMetricService::new()),
            file_parser: Arc::new(MockFileParser::new()),
            session_service: Some(Arc::new(session_service)),
        };

        async fn test_route(
            Extension(user): Extension<AuthenticatedUser>,
        ) -> impl axum::response::IntoResponse {
            user.user().to_string()
        }

        let app =
            Router::new()
                .route("/", get(test_route))
                .route_layer(from_extractor_with_state::<
                    CookieUserExtractor<MockSessionService>,
                    AppState<
                        MockActivityService,
                        MockFileParser,
                        MockTrainingMetricService,
                        MockSessionService,
                    >,
                >(state));
        TestServer::new(app).expect("unable to create test server")
    }

    #[tokio::test]
    async fn test_cookie_user_extractor_no_sesion_token_cookie() {
        let sessions = MockSessionService::new();
        let server = build_test_server(sessions);

        let response = server.get("/").await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_cookie_user_extractor_sesion_token_cookie_rejected() {
        let mut sessions = MockSessionService::new();
        sessions.expect_check_session_token().returning(|token| {
            Err(SessionTokenError::SessionTokenDoesNotExistsError(
                token.into(),
            ))
        });
        let server = build_test_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_cookie_user_extractor_return_user_id() {
        let mut sessions = MockSessionService::new();
        sessions
            .expect_check_session_token()
            .returning(|_| Ok(UserId::from("a user")));
        let server = build_test_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;

        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), UserId::from("a user").to_string());
    }
}
