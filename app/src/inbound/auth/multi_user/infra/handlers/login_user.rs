use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{AppendHeaders, IntoResponse},
};
use serde::Deserialize;

use crate::inbound::auth::multi_user::{
    EmailAddress, IMultiUserService, Password, UserLoginResult,
    infra::handlers::{AuthAppState, extractor::build_session_cookie},
};

#[derive(Debug, Deserialize)]
pub struct LoginUserBody {
    email: String,
    #[serde(default)]
    password: Option<String>,
}

#[tracing::instrument(skip_all)]
pub async fn login_user<UR: IMultiUserService>(
    State(state): State<AuthAppState<UR>>,
    headers: HeaderMap,
    Json(body): Json<LoginUserBody>,
) -> impl IntoResponse {
    let Ok(email) = EmailAddress::try_from(body.email) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let password = body
        .password
        .and_then(|password| Password::try_from(password).ok());

    match state.user_service.login_user(email, password).await {
        UserLoginResult::EmailSent => StatusCode::OK.into_response(),
        UserLoginResult::Session(session) => {
            let cookie_config = state.cookie_config.for_request(&headers);
            let Some(cookie) = build_session_cookie(&cookie_config, &session) else {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            };

            let headers = AppendHeaders([(SET_COOKIE, cookie.encoded().to_string())]);
            (headers, StatusCode::OK).into_response()
        }
        UserLoginResult::Invalid => (StatusCode::UNAUTHORIZED, "invalid login").into_response(),
        UserLoginResult::Retry => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use axum::{Router, http::StatusCode, routing::post};
    use axum_test::TestServer;
    use chrono::{TimeDelta, Utc};
    use serde_json::json;

    use crate::inbound::{
        auth::multi_user::{
            GenerateSessionTokenResult, Password, SessionToken, UserLoginResult,
            infra::handlers::UserRegistration, test_utils::MockUserService,
        },
        http::CookieConfig,
    };

    use super::*;

    fn build_test_server(user_service: MockUserService) -> TestServer {
        let state = AuthAppState {
            user_service: Arc::new(user_service),
            cookie_config: Arc::new(CookieConfig::default()),
            registration_mode: UserRegistration::Allowed,
        };

        let app = Router::new()
            .route("/login", post(login_user::<MockUserService>))
            .with_state(state);
        TestServer::new(app)
    }

    #[tokio::test]
    async fn test_mail_login_returns_ok() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_login_user()
            .times(1)
            .withf(|email, password| {
                email == &EmailAddress::try_from("test@mail.test").unwrap() && password.is_none()
            })
            .returning(|_, _| UserLoginResult::EmailSent);
        let server = build_test_server(user_service);

        let response = server
            .post("/login")
            .json(&json!({"email": "test@mail.test"}))
            .await;

        response.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn test_login_forwards_provided_password() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_login_user()
            .times(1)
            .withf(|_, password| {
                password == &Some(Password::try_from("secret".to_string()).unwrap())
            })
            .returning(|_, _| UserLoginResult::EmailSent);
        let server = build_test_server(user_service);

        let response = server
            .post("/login")
            .json(&json!({"email": "test@mail.test", "password": "secret"}))
            .await;

        response.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn test_empty_password_falls_back_to_mail_flow() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_login_user()
            .times(1)
            .withf(|_, password| password.is_none())
            .returning(|_, _| UserLoginResult::EmailSent);
        let server = build_test_server(user_service);

        let response = server
            .post("/login")
            .json(&json!({"email": "test@mail.test", "password": ""}))
            .await;

        response.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn test_password_login_sets_session_cookie() {
        let token = SessionToken::from("session-token");
        let expire_at = Utc::now() + TimeDelta::days(30);
        let mut user_service = MockUserService::new();
        user_service
            .expect_login_user()
            .times(1)
            .returning(move |_, _| {
                UserLoginResult::Session(GenerateSessionTokenResult::new(token.clone(), expire_at))
            });
        let server = build_test_server(user_service);

        let response = server
            .post("/login")
            .json(&json!({"email": "test@mail.test", "password": "secret"}))
            .await;

        response.assert_status(StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .expect("expected a session cookie");
        assert!(set_cookie.to_str().unwrap().contains("session-token"));
    }

    #[tokio::test]
    async fn test_password_login_cookie_is_secure_only_over_https() {
        let token = SessionToken::from("session-token");
        let expire_at = Utc::now() + TimeDelta::days(30);
        let mut user_service = MockUserService::new();
        user_service.expect_login_user().returning(move |_, _| {
            UserLoginResult::Session(GenerateSessionTokenResult::new(token.clone(), expire_at))
        });
        let server = build_test_server(user_service);

        // Plain HTTP: the cookie must not be `Secure`, otherwise the browser silently drops it
        // and every subsequent request answers 401 (e.g. password-only mode on a plain-HTTP host).
        let response = server
            .post("/login")
            .json(&json!({"email": "test@mail.test", "password": "secret"}))
            .await;
        response.assert_status(StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(!set_cookie.contains("Secure"), "got: {set_cookie}");

        // Behind a TLS-terminating proxy reporting HTTPS: the cookie must be `Secure`.
        let response = server
            .post("/login")
            .add_header("x-forwarded-proto", "https")
            .json(&json!({"email": "test@mail.test", "password": "secret"}))
            .await;
        response.assert_status(StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(set_cookie.contains("Secure"), "got: {set_cookie}");
    }

    #[tokio::test]
    async fn test_invalid_login_returns_unauthorized_with_generic_body() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_login_user()
            .returning(|_, _| UserLoginResult::Invalid);
        let server = build_test_server(user_service);

        let response = server
            .post("/login")
            .json(&json!({"email": "test@mail.test", "password": "secret"}))
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
        assert_eq!(response.text(), "invalid login");

        // The same response is returned for an unknown user.
        let response = server
            .post("/login")
            .json(&json!({"email": "unknown@mail.test", "password": "secret"}))
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
        assert_eq!(response.text(), "invalid login");
    }

    #[tokio::test]
    async fn test_login_returns_service_unavailable_on_retry() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_login_user()
            .returning(|_, _| UserLoginResult::Retry);
        let server = build_test_server(user_service);

        let response = server
            .post("/login")
            .json(&json!({"email": "test@mail.test"}))
            .await;

        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_login_rejects_invalid_email() {
        let mut user_service = MockUserService::new();
        user_service.expect_login_user().times(0);
        let server = build_test_server(user_service);

        let response = server
            .post("/login")
            .json(&json!({"email": "not-an-email", "password": "secret"}))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }
}
