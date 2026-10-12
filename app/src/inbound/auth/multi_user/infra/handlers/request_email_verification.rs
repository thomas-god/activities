use axum::{Extension, extract::State, http::StatusCode, response::IntoResponse};

use crate::inbound::auth::AuthenticatedUser;
use crate::inbound::auth::multi_user::{
    EmailVerificationResult, IMultiUserService, infra::handlers::AuthAppState,
};

#[tracing::instrument(skip_all)]
pub async fn request_email_verification<UR: IMultiUserService>(
    State(state): State<AuthAppState<UR>>,
    Extension(user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    match state
        .user_service
        .request_email_verification(user.user())
        .await
    {
        EmailVerificationResult::EmailSent => StatusCode::ACCEPTED.into_response(),
        // Idempotent: there is nothing to do when the address is already verified.
        EmailVerificationResult::AlreadyVerified => StatusCode::OK.into_response(),
        EmailVerificationResult::EmailAuthDisabled => StatusCode::FORBIDDEN.into_response(),
        EmailVerificationResult::Retry => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use axum::{Router, http::StatusCode, routing::post};
    use axum_test::TestServer;

    use crate::{
        domain::models::UserId,
        inbound::auth::multi_user::{EmailVerificationResult, test_utils::MockUserService},
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
            .route(
                "/account/email/verify",
                post(request_email_verification::<MockUserService>),
            )
            .layer(axum::middleware::from_fn(authed_user_middleware))
            .with_state(state);
        TestServer::new(app)
    }

    #[tokio::test]
    async fn test_request_email_verification_accepted_when_email_sent() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_request_email_verification()
            .times(1)
            .withf(|user| user == &UserId::test_default())
            .returning(|_| EmailVerificationResult::EmailSent);
        let server = build_test_server(user_service);

        let response = server.post("/account/email/verify").await;

        response.assert_status(StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn test_request_email_verification_ok_when_already_verified() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_request_email_verification()
            .returning(|_| EmailVerificationResult::AlreadyVerified);
        let server = build_test_server(user_service);

        let response = server.post("/account/email/verify").await;

        response.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn test_request_email_verification_forbidden_when_email_auth_disabled() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_request_email_verification()
            .returning(|_| EmailVerificationResult::EmailAuthDisabled);
        let server = build_test_server(user_service);

        let response = server.post("/account/email/verify").await;

        response.assert_status(StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_request_email_verification_unavailable_on_retry() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_request_email_verification()
            .returning(|_| EmailVerificationResult::Retry);
        let server = build_test_server(user_service);

        let response = server.post("/account/email/verify").await;

        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }
}
