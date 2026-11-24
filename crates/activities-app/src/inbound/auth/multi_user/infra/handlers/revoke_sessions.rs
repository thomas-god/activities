use axum::{
    Extension,
    extract::State,
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{AppendHeaders, IntoResponse},
};

use crate::inbound::{
    auth::{
        AuthenticatedSession, AuthenticatedUser, SessionRevoked,
        multi_user::{IMultiUserService, RevokeSessionsResult, infra::handlers::AuthAppState},
    },
    http::build_removal_cookie,
};

#[tracing::instrument(skip_all)]
pub async fn revoke_other_sessions<UR: IMultiUserService>(
    State(state): State<AuthAppState<UR>>,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(session): Extension<AuthenticatedSession>,
) -> impl IntoResponse {
    match state
        .user_service
        .revoke_other_sessions(user.user(), session.hash())
        .await
    {
        RevokeSessionsResult::Success => StatusCode::OK.into_response(),
        RevokeSessionsResult::Retry => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

#[tracing::instrument(skip_all)]
pub async fn revoke_all_sessions<UR: IMultiUserService>(
    State(state): State<AuthAppState<UR>>,
    headers: HeaderMap,
    Extension(user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    match state.user_service.revoke_all_sessions(user.user()).await {
        RevokeSessionsResult::Success => {
            let cookie_config = state.cookie_config.for_request(&headers);
            let cookie = build_removal_cookie("session_token", &cookie_config);
            let headers = AppendHeaders([(SET_COOKIE, cookie.encoded().to_string())]);
            let mut response = (headers, StatusCode::OK).into_response();
            // Tell the cookie middleware not to append a rotated cookie over the removal.
            response.extensions_mut().insert(SessionRevoked);
            response
        }
        RevokeSessionsResult::Retry => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use axum::{
        Router,
        http::{StatusCode, header::SET_COOKIE},
        routing::post,
    };
    use axum_test::TestServer;

    use crate::{
        domain::models::UserId,
        inbound::auth::multi_user::{
            HashedSessionToken, RevokeSessionsResult, SessionToken, test_utils::MockUserService,
        },
        inbound::http::CookieConfig,
    };

    use super::*;

    fn current_hash() -> HashedSessionToken {
        SessionToken::from("current".to_string()).as_hash()
    }

    /// Middleware simulating an authenticated session by injecting the user and current session
    /// directly, so handler tests can focus on their own logic.
    async fn authed_middleware(
        mut request: axum::extract::Request,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request
            .extensions_mut()
            .insert(AuthenticatedUser::new(UserId::test_default()));
        request
            .extensions_mut()
            .insert(AuthenticatedSession::new(current_hash()));
        next.run(request).await
    }

    fn build_test_server(user_service: MockUserService) -> TestServer {
        let state = AuthAppState {
            user_service: Arc::new(user_service),
            cookie_config: Arc::new(CookieConfig::default()),
            registration_mode:
                crate::inbound::auth::multi_user::infra::handlers::UserRegistration::Allowed,
        };

        let app = Router::new()
            .route(
                "/account/sessions/revoke-others",
                post(revoke_other_sessions::<MockUserService>),
            )
            .route(
                "/account/sessions/revoke-all",
                post(revoke_all_sessions::<MockUserService>),
            )
            .layer(axum::middleware::from_fn(authed_middleware))
            .with_state(state);
        TestServer::new(app)
    }

    #[tokio::test]
    async fn test_revoke_other_sessions_passes_current_hash_and_keeps_cookie() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_revoke_other_sessions()
            .times(1)
            .withf(|user, hash| user == &UserId::test_default() && hash == &current_hash())
            .returning(|_, _| RevokeSessionsResult::Success);
        let server = build_test_server(user_service);

        let response = server.post("/account/sessions/revoke-others").await;

        response.assert_status(StatusCode::OK);
        assert!(response.headers().get(SET_COOKIE).is_none());
    }

    #[tokio::test]
    async fn test_revoke_other_sessions_unavailable_on_retry() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_revoke_other_sessions()
            .returning(|_, _| RevokeSessionsResult::Retry);
        let server = build_test_server(user_service);

        let response = server.post("/account/sessions/revoke-others").await;

        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_revoke_all_sessions_clears_cookie() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_revoke_all_sessions()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(|_| RevokeSessionsResult::Success);
        let server = build_test_server(user_service);

        let response = server.post("/account/sessions/revoke-all").await;

        response.assert_status(StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .expect("expected Set-Cookie header")
            .to_str()
            .unwrap();
        assert!(set_cookie.contains("session_token="));
    }

    #[tokio::test]
    async fn test_revoke_all_sessions_unavailable_on_retry() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_revoke_all_sessions()
            .returning(|_| RevokeSessionsResult::Retry);
        let server = build_test_server(user_service);

        let response = server.post("/account/sessions/revoke-all").await;

        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }
}
