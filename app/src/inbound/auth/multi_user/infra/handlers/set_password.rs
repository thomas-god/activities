use axum::{
    Extension, Json,
    extract::State,
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{AppendHeaders, IntoResponse},
};
use serde::Deserialize;

use crate::inbound::{
    auth::{
        AuthenticatedUser,
        multi_user::{
            IMultiUserService, RegisterPasswordRequest, RegisterPasswordResult,
            UpdatePasswordRequest, UpdatePasswordResult,
            infra::handlers::{AuthAppState, extractor::build_session_cookie},
        },
    },
    http::CookieConfig,
};

#[derive(Debug, Deserialize)]
pub struct RegisterPasswordBody {
    new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordBody {
    current_password: String,
    new_password: String,
}

fn session_response(
    cookie_config: &CookieConfig,
    headers: &HeaderMap,
    session: &crate::inbound::auth::multi_user::GenerateSessionTokenResult,
) -> axum::response::Response {
    let cookie_config = cookie_config.for_request(headers);
    let Some(cookie) = build_session_cookie(&cookie_config, session) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let headers = AppendHeaders([(SET_COOKIE, cookie.encoded().to_string())]);
    (headers, StatusCode::OK).into_response()
}

/// Registers the first password for the authenticated user, who must not have one yet. Once a
/// password is set, `POST /api/password/change` must be used instead.
#[tracing::instrument(skip_all)]
pub async fn register_password<UR: IMultiUserService>(
    State(state): State<AuthAppState<UR>>,
    Extension(user): Extension<AuthenticatedUser>,
    headers: HeaderMap,
    Json(body): Json<RegisterPasswordBody>,
) -> impl IntoResponse {
    let Ok(new) = crate::inbound::auth::multi_user::Password::try_from(body.new_password) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    match state
        .user_service
        .register_password(RegisterPasswordRequest::new(user.user().clone(), new))
        .await
    {
        RegisterPasswordResult::Success(session) => {
            session_response(&state.cookie_config, &headers, &session)
        }
        RegisterPasswordResult::PasswordAlreadySet => StatusCode::CONFLICT.into_response(),
        RegisterPasswordResult::Retry => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

/// Replaces the password of the authenticated user, requiring the current one. If the user has no
/// password yet, `POST /api/password` (registration flow) must be used instead; no-password and
/// wrong-password attempts are indistinguishable from the outside.
#[tracing::instrument(skip_all)]
pub async fn update_password<UR: IMultiUserService>(
    State(state): State<AuthAppState<UR>>,
    Extension(user): Extension<AuthenticatedUser>,
    headers: HeaderMap,
    Json(body): Json<UpdatePasswordBody>,
) -> impl IntoResponse {
    let Ok(new) = crate::inbound::auth::multi_user::Password::try_from(body.new_password) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(current) = crate::inbound::auth::multi_user::Password::try_from(body.current_password)
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    match state
        .user_service
        .update_password(UpdatePasswordRequest::new(
            user.user().clone(),
            current,
            new,
        ))
        .await
    {
        UpdatePasswordResult::Success(session) => {
            session_response(&state.cookie_config, &headers, &session)
        }
        UpdatePasswordResult::InvalidCurrentPassword => {
            (StatusCode::UNAUTHORIZED, "invalid current password").into_response()
        }
        UpdatePasswordResult::Retry => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use axum::{Router, http::StatusCode, routing::post};
    use axum_test::TestServer;
    use chrono::{TimeDelta, Utc};
    use serde_json::json;

    use crate::{
        domain::models::UserId,
        inbound::auth::multi_user::{
            GenerateSessionTokenResult, Password, RegisterPasswordResult, SessionToken,
            UpdatePasswordResult, test_utils::MockUserService,
        },
        inbound::http::CookieConfig,
    };

    use super::*;

    /// Middleware simulating an authenticated session by injecting the user directly, so handler
    /// tests can focus on their own logic (the cookie extractor is tested in `extractor.rs`).
    async fn authed_user_middleware(
        mut request: axum::extract::Request,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request
            .extensions_mut()
            .insert(AuthenticatedUser::new(UserId::test_default()));
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
            .route("/password", post(register_password::<MockUserService>))
            .route("/password/change", post(update_password::<MockUserService>))
            .layer(axum::middleware::from_fn(authed_user_middleware))
            .with_state(state);
        TestServer::new(app)
    }

    fn session_result() -> GenerateSessionTokenResult {
        GenerateSessionTokenResult::new(
            SessionToken::from("session-token"),
            Utc::now() + TimeDelta::days(30),
        )
    }

    #[tokio::test]
    async fn test_register_password_sets_session_cookie() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_register_password()
            .times(1)
            .withf(|req| {
                req.user() == &UserId::test_default()
                    && req.password() == &Password::try_from("secret".to_string()).unwrap()
            })
            .returning(|_| RegisterPasswordResult::Success(session_result()));
        let server = build_test_server(user_service);

        let response = server
            .post("/password")
            .json(&json!({"new_password": "secret"}))
            .await;

        response.assert_status(StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .expect("expected a session cookie");
        assert!(set_cookie.to_str().unwrap().contains("session-token"));
    }

    #[tokio::test]
    async fn test_register_password_conflict_when_already_set() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_register_password()
            .returning(|_| RegisterPasswordResult::PasswordAlreadySet);
        let server = build_test_server(user_service);

        let response = server
            .post("/password")
            .json(&json!({"new_password": "secret"}))
            .await;

        response.assert_status(StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_register_password_rejects_empty_password() {
        let mut user_service = MockUserService::new();
        user_service.expect_register_password().times(0);
        let server = build_test_server(user_service);

        let response = server
            .post("/password")
            .json(&json!({"new_password": ""}))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_register_password_retry_maps_to_service_unavailable() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_register_password()
            .returning(|_| RegisterPasswordResult::Retry);
        let server = build_test_server(user_service);

        let response = server
            .post("/password")
            .json(&json!({"new_password": "secret"}))
            .await;

        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_update_password_sets_session_cookie() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_update_password()
            .times(1)
            .withf(|req| {
                req.user() == &UserId::test_default()
                    && req.current() == &Password::try_from("old".to_string()).unwrap()
                    && req.password() == &Password::try_from("secret".to_string()).unwrap()
            })
            .returning(|_| UpdatePasswordResult::Success(session_result()));
        let server = build_test_server(user_service);

        let response = server
            .post("/password/change")
            .json(&json!({"current_password": "old", "new_password": "secret"}))
            .await;

        response.assert_status(StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .expect("expected a session cookie");
        assert!(set_cookie.to_str().unwrap().contains("session-token"));
    }

    #[tokio::test]
    async fn test_update_password_unauthorized_on_invalid_current_password() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_update_password()
            .returning(|_| UpdatePasswordResult::InvalidCurrentPassword);
        let server = build_test_server(user_service);

        let response = server
            .post("/password/change")
            .json(&json!({"current_password": "wrong", "new_password": "secret"}))
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
        assert_eq!(response.text(), "invalid current password");
    }

    #[tokio::test]
    async fn test_update_password_rejects_empty_current_password() {
        let mut user_service = MockUserService::new();
        user_service.expect_update_password().times(0);
        let server = build_test_server(user_service);

        let response = server
            .post("/password/change")
            .json(&json!({"current_password": "", "new_password": "secret"}))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_update_password_retry_maps_to_service_unavailable() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_update_password()
            .returning(|_| UpdatePasswordResult::Retry);
        let server = build_test_server(user_service);

        let response = server
            .post("/password/change")
            .json(&json!({"current_password": "old", "new_password": "secret"}))
            .await;

        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }
}
