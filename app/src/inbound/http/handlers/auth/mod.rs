use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum::{
    extract::{FromRef, Request, State},
    middleware::Next,
};
use axum::{
    http::header::SET_COOKIE,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use cookie::{Cookie, time::OffsetDateTime};
use std::{marker::PhantomData, sync::Arc};

use crate::{
    domain::{
        models::UserId,
        ports::{IActivityService, IPreferencesService, ITrainingService},
    },
    inbound::{
        http::{AppState, CookieConfig, IUserService, auth::GenerateSessionTokenResult},
        parser::ParseFile,
    },
};

use crate::inbound::http::auth::AuthenticatedUser;

mod login_user;
mod register_user;
mod validate_login;

pub use login_user::login_user;
pub use register_user::register_user;
pub use validate_login::validate_login;

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

impl<AS, PF, TMS, US, PS> FromRef<AppState<AS, PF, TMS, US, PS>> for Arc<US>
where
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    US: IUserService,
    PS: IPreferencesService,
{
    fn from_ref(input: &AppState<AS, PF, TMS, US, PS>) -> Self {
        input.user_service.clone()
    }
}

/// Extractor that tries to extract user information from the request's session cookie.
#[allow(unused)]
pub struct CookieUserExtractor<US>(PhantomData<US>);

impl<S, US> FromRequestParts<S> for CookieUserExtractor<US>
where
    S: Send + Sync,
    US: IUserService,
    Arc<US>: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let service = Arc::<US>::from_ref(state);

        let jar = axum_extra::extract::CookieJar::from_headers(&parts.headers);
        let Some(session_token) = jar.get("session_token") else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        let Ok(res) = service
            .check_session_token(&session_token.value().into())
            .await
        else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        parts
            .extensions
            .insert(AuthenticatedUser::new(res.user().clone()));

        Ok(Self(PhantomData))
    }
}

pub async fn cookie_auth_middleware<US, AS, PF, TMS, PS>(
    State(state): State<AppState<AS, PF, TMS, US, PS>>,
    mut request: Request,
    next: Next,
) -> Response
where
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    US: IUserService,
    PS: IPreferencesService,
{
    let jar = CookieJar::from_headers(request.headers());
    let Some(cookie) = jar.get("session_token") else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let Ok(result) = state
        .user_service
        .check_session_token(&cookie.value().into())
        .await
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    request
        .extensions_mut()
        .insert(AuthenticatedUser::new(result.user().clone()));
    let mut response = next.run(request).await;

    if let Some(refreshed) = result.refreshed()
        && let Some(new_cookie) = build_session_cookie(&state.cookie_config, refreshed)
        && let Ok(header_value) = new_cookie.to_string().parse()
    {
        response.headers_mut().append(SET_COOKIE, header_value);
    }

    response
}

pub fn build_session_cookie<'a>(
    cookie_config: &CookieConfig,
    session: &'a GenerateSessionTokenResult,
) -> Option<Cookie<'a>> {
    let expire_at = OffsetDateTime::from_unix_timestamp(session.expire_at().timestamp()).ok()?;
    let mut builder = Cookie::build(("session_token", session.token().to_string()))
        .expires(expire_at)
        .secure(cookie_config.secure)
        .http_only(cookie_config.http_only)
        .same_site(cookie_config.same_site)
        .path("/");
    if let Some(domain) = cookie_config.domain.clone() {
        builder = builder.domain(domain);
    }
    let cookie = builder.build();
    Some(cookie)
}

#[cfg(test)]
mod test {
    use axum::{
        Extension, Router,
        http::header::SET_COOKIE,
        middleware::{from_extractor, from_extractor_with_state, from_fn_with_state},
        routing::get,
    };
    use axum_extra::extract::cookie::Cookie;
    use axum_test::TestServer;
    use chrono::{TimeDelta, Utc};

    use crate::{
        domain::services::{
            activity::test_utils::MockActivityService,
            preferences::tests_utils::MockPreferencesService,
            training::test_utils::MockTrainingService,
        },
        inbound::{
            http::{
                CookieConfig,
                auth::{
                    CheckSessionResult, GenerateSessionTokenResult, SessionToken,
                    test_utils::MockUserService,
                },
            },
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

    fn build_test_server(session_service: MockUserService) -> TestServer {
        let state = AppState {
            activity_service: Arc::new(MockActivityService::new()),
            training_metrics_service: Arc::new(MockTrainingService::new()),
            file_parser: Arc::new(MockFileParser::new()),
            user_service: Arc::new(session_service),
            preferences_service: Arc::new(MockPreferencesService::new()),
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
                        MockTrainingService,
                        MockUserService,
                        MockPreferencesService,
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
            .returning(|_| Ok(CheckSessionResult::new(UserId::from("a user"), None)));
        let server = build_test_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;

        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), UserId::from("a user").to_string());
    }

    fn build_middleware_test_server(session_service: MockUserService) -> TestServer {
        let state = AppState {
            activity_service: Arc::new(MockActivityService::new()),
            training_metrics_service: Arc::new(MockTrainingService::new()),
            file_parser: Arc::new(MockFileParser::new()),
            user_service: Arc::new(session_service),
            preferences_service: Arc::new(MockPreferencesService::new()),
            cookie_config: Arc::new(CookieConfig::default()),
        };

        async fn test_route(
            Extension(user): Extension<AuthenticatedUser>,
        ) -> impl axum::response::IntoResponse {
            user.user().to_string()
        }

        let app = Router::new()
            .route("/", get(test_route))
            .route_layer(from_fn_with_state(
                state.clone(),
                cookie_auth_middleware::<
                    MockUserService,
                    MockActivityService,
                    MockFileParser,
                    MockTrainingService,
                    MockPreferencesService,
                >,
            ))
            .with_state(state);
        TestServer::new(app).expect("unable to create test server")
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_no_session_token_cookie() {
        let server = build_middleware_test_server(MockUserService::new());

        let response = server.get("/").await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_session_token_rejected() {
        let mut sessions = MockUserService::new();
        sessions.expect_check_session_token().returning(|_| Err(()));
        let server = build_middleware_test_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_valid_session_no_refresh() {
        let mut sessions = MockUserService::new();
        sessions
            .expect_check_session_token()
            .returning(|_| Ok(CheckSessionResult::new(UserId::from("a user"), None)));
        let server = build_middleware_test_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;

        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), UserId::from("a user").to_string());
        assert!(response.headers().get(SET_COOKIE).is_none());
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_valid_session_with_refresh() {
        let mut sessions = MockUserService::new();
        sessions.expect_check_session_token().returning(|_| {
            Ok(CheckSessionResult::new(
                UserId::from("a user"),
                Some(GenerateSessionTokenResult::new(
                    SessionToken::from("new_token"),
                    Utc::now() + TimeDelta::days(30),
                )),
            ))
        });
        let server = build_middleware_test_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;

        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), UserId::from("a user").to_string());
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .expect("expected Set-Cookie header on refresh");
        assert!(set_cookie.to_str().unwrap().contains("new_token"));
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_refresh_cookie_build_failure_does_not_logout() {
        // build_session_cookie fails when expire_at is out of OffsetDateTime's range (post year 9999)
        let mut sessions = MockUserService::new();
        sessions.expect_check_session_token().returning(|_| {
            Ok(CheckSessionResult::new(
                UserId::from("a user"),
                Some(GenerateSessionTokenResult::new(
                    SessionToken::from("new_token"),
                    chrono::DateTime::<Utc>::MAX_UTC,
                )),
            ))
        });
        let server = build_middleware_test_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;

        // User is still authenticated despite the refresh cookie failing to build
        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), UserId::from("a user").to_string());
        assert!(response.headers().get(SET_COOKIE).is_none());
    }
}
