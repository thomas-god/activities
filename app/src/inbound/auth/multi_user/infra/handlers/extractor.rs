use axum::{
    extract::{FromRef, FromRequestParts, Request, State},
    http::{StatusCode, request::Parts},
    middleware::Next,
};
use axum::{
    http::header::SET_COOKIE,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use cookie::{Cookie, time::OffsetDateTime};
use std::{marker::PhantomData, sync::Arc};

use crate::inbound::{
    auth::{
        AuthenticatedSession, AuthenticatedUser, SessionRevoked,
        multi_user::{
            GenerateSessionTokenResult, IMultiUserService, SessionToken,
            infra::handlers::AuthAppState,
        },
    },
    http::CookieConfig,
};

/// Extractor that tries to extract user information from the request's session cookie.
#[allow(unused)]
pub struct CookieUserExtractor<US>(PhantomData<US>);

impl<S, US> FromRequestParts<S> for CookieUserExtractor<US>
where
    S: Send + Sync,
    US: IMultiUserService,
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

pub async fn cookie_auth_middleware<US>(
    State(state): State<AuthAppState<US>>,
    mut request: Request,
    next: Next,
) -> Response
where
    US: IMultiUserService,
{
    let jar = CookieJar::from_headers(request.headers());
    let cookie_config = state.cookie_config.for_request(request.headers());
    let Some(cookie) = jar.get("session_token") else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let session_token: SessionToken = cookie.value().into();
    let Ok(result) = state.user_service.check_session_token(&session_token).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    // The current session may have just been rotated by the check above: handlers that revoke the
    // user's other sessions must exclude the *effective* session (the refreshed one), not the
    // token that was sent in the request.
    let current_hash = match result.refreshed() {
        Some(refreshed) => refreshed.token().as_hash(),
        None => session_token.as_hash(),
    };

    request
        .extensions_mut()
        .insert(AuthenticatedUser::new(result.user().clone()));
    request
        .extensions_mut()
        .insert(AuthenticatedSession::new(current_hash));
    let mut response = next.run(request).await;

    // A handler that revoked the current session already cleared the cookie: do not append a
    // rotated one on top of it, or the browser would keep a token pointing at a deleted session.
    if response.extensions().get::<SessionRevoked>().is_none()
        && let Some(refreshed) = result.refreshed()
        && let Some(new_cookie) = build_session_cookie(&cookie_config, refreshed)
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
        middleware::{from_extractor_with_state, from_fn_with_state},
        response::IntoResponse,
        routing::get,
    };
    use axum_extra::extract::cookie::Cookie;
    use axum_test::TestServer;
    use chrono::{TimeDelta, Utc};

    use crate::{
        domain::models::UserId,
        inbound::{
            auth::multi_user::{
                CheckSessionResult, GenerateSessionTokenResult, SessionToken,
                infra::handlers::{
                    UserRegistration,
                    extractor::{CookieUserExtractor, cookie_auth_middleware},
                },
                test_utils::MockUserService,
            },
            http::CookieConfig,
        },
    };

    use super::*;

    fn build_test_server(session_service: MockUserService) -> TestServer {
        let state = AuthAppState {
            user_service: Arc::new(session_service),
            cookie_config: Arc::new(CookieConfig::default()),
            registration_mode: UserRegistration::Allowed,
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
                    AuthAppState<MockUserService>,
                >(state));
        TestServer::new(app)
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
        let state = AuthAppState {
            user_service: Arc::new(session_service),
            cookie_config: Arc::new(CookieConfig::default()),
            registration_mode: UserRegistration::Allowed,
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
                cookie_auth_middleware::<MockUserService>,
            ))
            .with_state(state);
        TestServer::new(app)
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

    fn build_middleware_session_hash_server(session_service: MockUserService) -> TestServer {
        let state = AuthAppState {
            user_service: Arc::new(session_service),
            cookie_config: Arc::new(CookieConfig::default()),
            registration_mode: UserRegistration::Allowed,
        };

        async fn test_route(Extension(session): Extension<AuthenticatedSession>) -> String {
            session.hash().to_string()
        }

        let app = Router::new()
            .route("/", get(test_route))
            .route_layer(from_fn_with_state(
                state.clone(),
                cookie_auth_middleware::<MockUserService>,
            ))
            .with_state(state);
        TestServer::new(app)
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_exposes_effective_session_without_refresh() {
        let mut sessions = MockUserService::new();
        sessions
            .expect_check_session_token()
            .returning(|_| Ok(CheckSessionResult::new(UserId::from("a user"), None)));
        let server = build_middleware_session_hash_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;

        response.assert_status(StatusCode::OK);
        assert_eq!(
            response.text(),
            SessionToken::from("a value").as_hash().to_string()
        );
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_exposes_refreshed_session() {
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
        let server = build_middleware_session_hash_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;

        response.assert_status(StatusCode::OK);
        assert_eq!(
            response.text(),
            SessionToken::from("new_token").as_hash().to_string()
        );
    }

    fn build_middleware_session_revoked_server(session_service: MockUserService) -> TestServer {
        let state = AuthAppState {
            user_service: Arc::new(session_service),
            cookie_config: Arc::new(CookieConfig::default()),
            registration_mode: UserRegistration::Allowed,
        };

        async fn test_route() -> axum::response::Response {
            let mut response = StatusCode::OK.into_response();
            response.extensions_mut().insert(SessionRevoked);
            response
        }

        let app = Router::new()
            .route("/", get(test_route))
            .route_layer(from_fn_with_state(
                state.clone(),
                cookie_auth_middleware::<MockUserService>,
            ))
            .with_state(state);
        TestServer::new(app)
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_skips_refresh_when_session_revoked() {
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
        let server = build_middleware_session_revoked_server(sessions);

        let response = server
            .get("/")
            .add_cookie(Cookie::new("session_token", "a value"))
            .await;

        response.assert_status(StatusCode::OK);
        // The handler revoked the session: no rotated cookie may be appended.
        assert!(response.headers().get(SET_COOKIE).is_none());
    }
}
