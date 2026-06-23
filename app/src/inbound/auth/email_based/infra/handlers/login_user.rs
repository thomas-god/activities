use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::inbound::auth::email_based::{
    EmailAddress, IUserService, UserLoginResult, infra::handlers::AuthAppState,
};

#[derive(Debug, Deserialize)]
pub struct LoginUserQuery {
    email: String,
}

pub async fn login_user<UR: IUserService>(
    State(state): State<AuthAppState<UR>>,
    Query(query): Query<LoginUserQuery>,
) -> StatusCode {
    let Ok(email) = EmailAddress::try_from(query.email) else {
        return StatusCode::BAD_REQUEST;
    };

    match state.user_service.login_user(email).await {
        UserLoginResult::Success => StatusCode::OK,
        UserLoginResult::Retry => StatusCode::SERVICE_UNAVAILABLE,
    }
}
