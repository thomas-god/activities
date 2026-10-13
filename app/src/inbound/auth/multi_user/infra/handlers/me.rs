use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::inbound::auth::AuthenticatedUser;
use crate::inbound::auth::multi_user::{
    IMultiUserService, UserInfo, UserInfoResult, infra::handlers::AuthAppState,
};

/// Metadata about the authenticated user's account, used by the settings page to know which
/// account actions apply (email verification state, whether a password is registered). Auth mode
/// itself is advertised publicly through `/api/auth_info`.
#[derive(Debug, Serialize)]
pub struct UserInfoResponse {
    email: String,
    email_valid: bool,
    email_verified: bool,
    has_password: bool,
    is_admin: bool,
}

impl From<UserInfo> for UserInfoResponse {
    fn from(info: UserInfo) -> Self {
        Self {
            email: info.email().to_string(),
            email_valid: info.email_valid(),
            email_verified: info.email_verified(),
            has_password: info.has_password(),
            is_admin: info.is_admin(),
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn get_me<UR: IMultiUserService>(
    State(state): State<AuthAppState<UR>>,
    Extension(user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    match state.user_service.get_user_info(user.user()).await {
        UserInfoResult::Success(info) => Json(UserInfoResponse::from(info)).into_response(),
        // Includes the "session valid but user gone" case: there is nothing better to answer.
        UserInfoResult::Retry => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
#[cfg(test)]
mod test {
    use std::sync::Arc;

    use axum::{Router, http::StatusCode, routing::get};
    use axum_test::TestServer;
    use serde_json::json;

    use crate::{
        domain::models::UserId,
        inbound::auth::multi_user::{UserInfo, UserInfoResult, test_utils::MockUserService},
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
            .route("/account/me", get(get_me::<MockUserService>))
            .layer(axum::middleware::from_fn(authed_user_middleware))
            .with_state(state);
        TestServer::new(app)
    }

    #[tokio::test]
    async fn test_get_me_returns_user_info() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_get_user_info()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(|_| {
                UserInfoResult::Success(UserInfo::new(
                    "test@mail.test".to_string(),
                    true,
                    true,
                    true,
                    false,
                ))
            });
        let server = build_test_server(user_service);

        let response = server.get("/account/me").await;

        response.assert_status(StatusCode::OK);
        response.assert_json(&json!({
            "email": "test@mail.test",
            "email_verified": true,
            "email_valid": true,
            "has_password": true,
            "is_admin": false
        }));
    }

    #[tokio::test]
    async fn test_get_me_unavailable_when_service_fails() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_get_user_info()
            .returning(|_| UserInfoResult::Retry);
        let server = build_test_server(user_service);

        let response = server.get("/account/me").await;

        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }
}
