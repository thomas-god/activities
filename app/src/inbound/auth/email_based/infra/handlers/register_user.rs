use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::inbound::auth::email_based::{
    EmailAddress, IUserService, UserRegistrationResult, infra::handlers::AuthAppState,
};

#[derive(Debug, Deserialize)]
pub struct RegisterUserQuery {
    email: String,
}

pub async fn register_user<UR: IUserService>(
    State(state): State<AuthAppState<UR>>,
    Query(query): Query<RegisterUserQuery>,
) -> StatusCode {
    let Ok(email) = EmailAddress::try_from(query.email) else {
        return StatusCode::BAD_REQUEST;
    };

    match state.user_service.register_user(email).await {
        UserRegistrationResult::Success => StatusCode::OK,
        UserRegistrationResult::Retry => StatusCode::SERVICE_UNAVAILABLE,
    }
}
