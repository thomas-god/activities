use std::{marker::PhantomData, sync::Arc};

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use base64::{Engine, engine::general_purpose};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use rand::Rng;
use subtle::ConstantTimeEq;

use crate::{
    domain::{
        models::UserId,
        ports::{IActivityService, ITrainingMetricService},
    },
    inbound::{http::AppState, parser::ParseFile},
};

pub mod infra;
pub mod services;

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

#[derive(Clone, Debug)]
pub struct MagicToken(String);

impl MagicToken {
    pub fn new() -> Self {
        let mut rng = rand::rng();
        let mut random_bytes = [0u8; 24];
        rng.fill(&mut random_bytes);

        Self(general_purpose::URL_SAFE_NO_PAD.encode(random_bytes))
    }

    /// Constant-time comparison between two [MagicToken]
    pub fn match_token_secure(&self, other: &MagicToken) -> bool {
        self.0.as_bytes().ct_eq(other.0.as_bytes()).into()
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

#[derive(Clone, Debug, Constructor)]
pub struct Session {
    user: UserId,
    token: SessionToken,
    expire_at: chrono::DateTime<Utc>,
}

impl Session {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn token(&self) -> &SessionToken {
        &self.token
    }

    pub fn expire_at(&self) -> &chrono::DateTime<Utc> {
        &self.expire_at
    }

    pub fn is_expired(&self, reference: &chrono::DateTime<Utc>) -> bool {
        reference >= &self.expire_at
    }
}

#[derive(Clone, Debug)]
pub struct SessionToken(String);

impl SessionToken {
    pub fn new() -> Self {
        let mut rng = rand::rng();
        let mut random_bytes = [0u8; 24];
        rng.fill(&mut random_bytes);

        Self(general_purpose::URL_SAFE_NO_PAD.encode(random_bytes))
    }

    /// Constant-time comparison between two [SessionToken]
    pub fn match_token_secure(&self, other: &SessionToken) -> bool {
        self.0.as_bytes().ct_eq(other.0.as_bytes()).into()
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

impl std::fmt::Display for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserRegistrationResult {
    Success,
    Retry,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserLoginResult {
    Success,
    Retry,
}

#[derive(Debug, Clone)]
pub enum MagicLinkValidationResult {
    Success(GenerateSessionTokenResult),
    Invalid,
}

pub trait IUserService: Clone + Send + Sync + 'static {
    fn register_user(
        &self,
        email: EmailAddress,
    ) -> impl Future<Output = UserRegistrationResult> + Send;

    fn login_user(&self, email: EmailAddress) -> impl Future<Output = UserLoginResult> + Send;

    fn validate_magic_link(
        &self,
        magic_token: MagicToken,
    ) -> impl Future<Output = Result<MagicLinkValidationResult, ()>> + Send;

    fn check_session_token(
        &self,
        token: &SessionToken,
    ) -> impl Future<Output = Result<UserId, ()>> + Send;
}

#[derive(Debug, Clone, Constructor)]
pub struct GenerateMagicLinkRequest {
    user: UserId,
    email: EmailAddress,
}

impl GenerateMagicLinkRequest {
    pub fn user(&self) -> &UserId {
        &self.user
    }

    pub fn email(&self) -> &EmailAddress {
        &self.email
    }
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

pub trait IMagicLinkService: Clone + Send + Sync + 'static {
    fn generate_magic_link(
        &self,
        req: GenerateMagicLinkRequest,
    ) -> impl Future<Output = GenerateMagicLinkResult> + Send;

    fn validate_magic_token(
        &self,
        token: &MagicToken,
    ) -> impl Future<Output = Result<Option<UserId>, ()>> + Send;
}

#[derive(Clone, Debug, Constructor)]
pub struct GenerateSessionTokenResult {
    token: SessionToken,
    expire_at: DateTime<Utc>,
}

impl GenerateSessionTokenResult {
    pub fn token(&self) -> &SessionToken {
        &self.token
    }

    pub fn expire_at(&self) -> &DateTime<Utc> {
        &self.expire_at
    }
}

pub trait ISessionService: Clone + Send + Sync + 'static {
    fn generate_session_token(
        &self,
        user: &UserId,
    ) -> impl Future<Output = Result<GenerateSessionTokenResult, ()>> + Send;

    fn check_session_token(
        &self,
        token: &SessionToken,
    ) -> impl Future<Output = Result<UserId, ()>> + Send;
}

impl<AS, PF, TMS, UR> FromRef<AppState<AS, PF, TMS, UR>> for Option<Arc<UR>>
where
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    UR: IUserService,
{
    fn from_ref(input: &AppState<AS, PF, TMS, UR>) -> Self {
        input.user_service.clone()
    }
}

/// Extractor that tries to extract user information from the request's session cookie.
pub struct CookieUserExtractor<SR>(PhantomData<SR>);

impl<S, UR> FromRequestParts<S> for CookieUserExtractor<UR>
where
    S: Send + Sync,
    UR: IUserService,
    Option<Arc<UR>>: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Some(service) = Option::<Arc<UR>>::from_ref(state) else {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };
        let jar = axum_extra::extract::CookieJar::from_headers(&parts.headers);
        let Some(session_token) = jar.get("session_token") else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        let Ok(user) = service
            .check_session_token(&session_token.value().into())
            .await
        else {
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
        pub UserService {}

        impl Clone for UserService {
            fn clone(&self) -> Self;
        }

        impl IUserService for UserService {
            async fn register_user(
                &self,
                email: EmailAddress,
            ) -> UserRegistrationResult;

            async fn login_user(
                &self,
                email: EmailAddress
            ) -> UserLoginResult;

            async fn validate_magic_link(
                &self,
                magic_token: MagicToken,
            ) -> Result<MagicLinkValidationResult, ()>;

            async fn check_session_token(
                &self,
                _token: &SessionToken
            ) -> Result<UserId, ()>;
        }
    }

    mock! {
        pub MagicLinkService {}

        impl Clone for MagicLinkService {
            fn clone(&self) -> Self;
        }

        impl IMagicLinkService for MagicLinkService {
            async fn generate_magic_link(
                &self,
                req: GenerateMagicLinkRequest
            ) -> GenerateMagicLinkResult;

            async fn validate_magic_token(
                &self,
                token: &MagicToken
            ) -> Result<Option<UserId>, ()>;
        }
    }

    mock! {
        pub SessionService {}

        impl Clone for SessionService {
            fn clone(&self) -> Self;
        }

        impl ISessionService for SessionService {
            async fn generate_session_token(
                &self,
                user: &UserId,
            ) ->Result<GenerateSessionTokenResult, ()>;

            async fn check_session_token(
                &self,
                _token: &SessionToken
            ) -> Result<UserId, ()>;
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
        inbound::{
            http::{CookieConfig, auth::test_utils::MockUserService},
            parser::test_utils::MockFileParser,
        },
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
        let link = MagicLink::new(UserId::test_default(), MagicToken::new(), expire_at);
        assert!(link.is_expired(&(expire_at + TimeDelta::seconds(1))));
        assert!(!link.is_expired(&(expire_at - TimeDelta::seconds(1))));
    }

    fn build_test_server(session_service: MockUserService) -> TestServer {
        let state = AppState {
            activity_service: Arc::new(MockActivityService::new()),
            training_metrics_service: Arc::new(MockTrainingMetricService::new()),
            file_parser: Arc::new(MockFileParser::new()),
            user_service: Some(Arc::new(session_service)),
            cookie_config: Arc::new(CookieConfig::default()),
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
                    CookieUserExtractor<MockUserService>,
                    AppState<
                        MockActivityService,
                        MockFileParser,
                        MockTrainingMetricService,
                        MockUserService,
                    >,
                >(state));
        TestServer::new(app).expect("unable to create test server")
    }

    #[tokio::test]
    async fn test_cookie_user_extractor_no_sesion_token_cookie() {
        let sessions = MockUserService::new();
        let server = build_test_server(sessions);

        let response = server.get("/").await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_cookie_user_extractor_sesion_token_cookie_rejected() {
        let mut sessions = MockUserService::new();
        sessions.expect_check_session_token().returning(|_| Err(()));
        let server = build_test_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_cookie_user_extractor_return_user_id() {
        let mut sessions = MockUserService::new();
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
