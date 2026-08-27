use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::inbound::auth::email_based::{
    EmailAddress, IUserService, UserRegistrationResult,
    infra::handlers::{AuthAppState, UserRegistration},
};

#[derive(Debug, Deserialize)]
pub struct RegisterUserBody {
    email: String,
}

#[tracing::instrument(skip_all)]
pub async fn register_user<UR: IUserService>(
    State(state): State<AuthAppState<UR>>,
    Json(body): Json<RegisterUserBody>,
) -> StatusCode {
    if let UserRegistration::Closed = state.registration_mode {
        return StatusCode::FORBIDDEN;
    }

    let Ok(email) = EmailAddress::try_from(body.email) else {
        return StatusCode::BAD_REQUEST;
    };

    match state.user_service.register_user(email).await {
        UserRegistrationResult::Success => StatusCode::OK,
        UserRegistrationResult::Retry => StatusCode::SERVICE_UNAVAILABLE,
    }
}

#[cfg(test)]
mod test {
    use axum::{Router, http::StatusCode, routing::post};
    use axum_test::TestServer;
    use serde_json::json;

    use crate::inbound::auth::email_based::{
        EmailAddress, UserRegistrationResult,
        infra::handlers::UserRegistration, test_utils::MockUserService,
    };

    use super::*;

    fn build_test_server(
        user_service: MockUserService,
        registration_mode: UserRegistration,
    ) -> TestServer {
        let state = AuthAppState {
            user_service: std::sync::Arc::new(user_service),
            cookie_config: std::sync::Arc::new(crate::inbound::http::CookieConfig::default()),
            registration_mode,
        };

        let app = Router::new()
            .route("/register", post(register_user::<MockUserService>))
            .with_state(state);
        TestServer::new(app)
    }

    #[tokio::test]
    async fn test_register_allowed_registers_new_user() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_register_user()
            .times(1)
            .withf(|email| email == &EmailAddress::try_from("test@mail.test").unwrap())
            .returning(|_| UserRegistrationResult::Success);
        let server = build_test_server(user_service, UserRegistration::Allowed);

        let response = server
            .post("/register")
            .json(&json!({"email": "test@mail.test"}))
            .await;

        response.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn test_register_allowed_returns_service_unavailable_on_retry() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_register_user()
            .times(1)
            .returning(|_| UserRegistrationResult::Retry);
        let server = build_test_server(user_service, UserRegistration::Allowed);

        let response = server
            .post("/register")
            .json(&json!({"email": "test@mail.test"}))
            .await;

        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_register_allowed_rejects_invalid_email() {
        let mut user_service = MockUserService::new();
        user_service.expect_register_user().times(0);
        let server = build_test_server(user_service, UserRegistration::Allowed);

        let response = server
            .post("/register")
            .json(&json!({"email": "not-an-email"}))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_register_closed_rejects_registration() {
        let mut user_service = MockUserService::new();
        user_service.expect_register_user().times(0);
        let server = build_test_server(user_service, UserRegistration::Closed);

        let response = server
            .post("/register")
            .json(&json!({"email": "test@mail.test"}))
            .await;

        response.assert_status(StatusCode::FORBIDDEN);
    }
}
