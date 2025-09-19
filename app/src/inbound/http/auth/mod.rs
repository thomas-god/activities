use std::{marker::PhantomData, sync::Arc};

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use derive_more::Constructor;
use thiserror::Error;

use crate::{
    domain::{
        models::UserId,
        ports::{IActivityService, ITrainingMetricService},
    },
    inbound::{http::AppState, parser::ParseFile},
};

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

#[derive(Debug, Clone, Error)]
pub enum SessionTokenError {
    #[error("SessionTokenDoesNotExists")]
    DoesNotExist,
}

pub trait ISessionRepository: Clone + Send + Sync + 'static {
    fn check_session_token(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<UserId, SessionTokenError>> + Send;
}

#[derive(Debug, Clone, Constructor)]
pub struct SessionRepository;
impl ISessionRepository for SessionRepository {
    async fn check_session_token(&self, _token: &str) -> Result<UserId, SessionTokenError> {
        todo!()
    }
}

pub struct SessionsRepositoryWrapper<SR: ISessionRepository>(Arc<SR>);

impl<AS, PF, TMS, SR> FromRef<AppState<AS, PF, TMS, SR>> for SessionsRepositoryWrapper<SR>
where
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    SR: ISessionRepository,
{
    fn from_ref(input: &AppState<AS, PF, TMS, SR>) -> Self {
        Self(input.session_repository.clone())
    }
}

/// Extractor that tries to extract user information from the request's session cookie.
pub struct CookieUserExtractor<SR>(PhantomData<SR>);

impl<S, SR> FromRequestParts<S> for CookieUserExtractor<SR>
where
    S: Send + Sync,
    SR: ISessionRepository,
    SessionsRepositoryWrapper<SR>: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = axum_extra::extract::CookieJar::from_headers(&parts.headers);
        let Some(session_token) = jar.get("session_token") else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let repository = SessionsRepositoryWrapper::from_ref(state);

        let Ok(user) = repository
            .0
            .check_session_token(session_token.value())
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
        pub SessionRepository {}

        impl Clone for SessionRepository {
            fn clone(&self) -> Self;
        }

        impl ISessionRepository for SessionRepository {
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

    use crate::{
        domain::services::{
            activity::test_utils::MockActivityService,
            training_metrics::test_utils::MockTrainingMetricService,
        },
        inbound::{
            http::auth::test_utils::MockSessionRepository, parser::test_utils::MockFileParser,
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

    fn build_test_server(sessions_repository: MockSessionRepository) -> TestServer {
        let state = AppState {
            activity_service: Arc::new(MockActivityService::new()),
            training_metrics_service: Arc::new(MockTrainingMetricService::new()),
            file_parser: Arc::new(MockFileParser::new()),
            session_repository: Arc::new(sessions_repository),
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
                    CookieUserExtractor<MockSessionRepository>,
                    AppState<
                        MockActivityService,
                        MockFileParser,
                        MockTrainingMetricService,
                        MockSessionRepository,
                    >,
                >(state));
        TestServer::new(app).expect("unable to create test server")
    }

    #[tokio::test]
    async fn test_cookie_user_extractor_no_sesion_token_cookie() {
        let sessions = MockSessionRepository::new();
        let server = build_test_server(sessions);

        let response = server.get("/").await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_cookie_user_extractor_sesion_token_cookie_rejected() {
        let mut sessions = MockSessionRepository::new();
        sessions
            .expect_check_session_token()
            .returning(|_| Err(SessionTokenError::DoesNotExist));
        let server = build_test_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_cookie_user_extractor_return_user_id() {
        let mut sessions = MockSessionRepository::new();
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
