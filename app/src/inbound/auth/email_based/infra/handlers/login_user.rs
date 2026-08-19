use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::inbound::auth::email_based::{
    EmailAddress, IUserService, UserLoginResult, infra::handlers::AuthAppState,
};

#[derive(Debug, Deserialize)]
pub struct LoginUserBody {
    email: String,
}

pub async fn login_user<UR: IUserService>(
    State(state): State<AuthAppState<UR>>,
    Json(body): Json<LoginUserBody>,
) -> StatusCode {
    let Ok(email) = EmailAddress::try_from(body.email) else {
        return StatusCode::BAD_REQUEST;
    };

    match state.user_service.login_user(email).await {
        UserLoginResult::Success => StatusCode::OK,
        UserLoginResult::Retry => StatusCode::SERVICE_UNAVAILABLE,
    }
}
