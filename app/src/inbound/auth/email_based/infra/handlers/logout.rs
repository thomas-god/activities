use axum::{
    extract::State,
    http::{StatusCode, header::SET_COOKIE},
    response::{AppendHeaders, IntoResponse},
};
use axum_extra::extract::CookieJar;

use crate::inbound::{
    auth::email_based::{IUserService, infra::handlers::AuthAppState},
    http::build_removal_cookie,
};

/// Always succeeds and clears the session cookie, whether or not a session was found: from the
/// client's point of view logout is idempotent, it just means "no session" afterwards.
#[tracing::instrument(skip_all)]
pub async fn logout<UR: IUserService>(
    State(state): State<AuthAppState<UR>>,
    jar: CookieJar,
) -> impl IntoResponse {
    if let Some(cookie) = jar.get("session_token") {
        let _ = state.user_service.logout(&cookie.value().into()).await;
    }

    let cookie = build_removal_cookie("session_token", &state.cookie_config);
    let headers = AppendHeaders([(SET_COOKIE, cookie.encoded().to_string())]);
    (headers, StatusCode::OK)
}

#[cfg(test)]
mod test {
    use axum::{Router, routing::post};
    use axum_test::TestServer;

    use crate::inbound::auth::email_based::{
        infra::handlers::UserRegistration, test_utils::MockUserService,
    };

    use super::*;

    fn build_test_server(user_service: MockUserService) -> TestServer {
        let state = AuthAppState {
            user_service: std::sync::Arc::new(user_service),
            cookie_config: std::sync::Arc::new(crate::inbound::http::CookieConfig::default()),
            registration_mode: UserRegistration::Allowed,
        };

        let app = Router::new()
            .route("/logout", post(logout::<MockUserService>))
            .with_state(state);
        TestServer::new(app)
    }

    #[tokio::test]
    async fn test_logout_without_cookie_still_clears_cookie_and_succeeds() {
        let user_service = MockUserService::new();
        let server = build_test_server(user_service);

        let response = server.post("/logout").await;

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
    async fn test_logout_with_cookie_calls_user_service_logout_and_clears_cookie() {
        let mut user_service = MockUserService::new();
        user_service.expect_logout().times(1).returning(|_| Ok(()));
        let server = build_test_server(user_service);

        let response = server
            .post("/logout")
            .add_cookie(axum_extra::extract::cookie::Cookie::new(
                "session_token",
                "some_token",
            ))
            .await;

        response.assert_status(StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .expect("expected Set-Cookie header")
            .to_str()
            .unwrap();
        assert!(set_cookie.contains("session_token="));
    }
}
