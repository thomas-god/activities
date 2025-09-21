use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::{
    domain::ports::{IActivityService, ITrainingMetricService},
    inbound::{
        http::{
            AppState, UserLoginResult,
            auth::{EmailAddress, IUserService, UserRegistrationResult},
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Deserialize)]
pub struct RegisterUserQuery {
    email: String,
}

pub async fn register_user<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    UR: IUserService,
>(
    State(state): State<AppState<AS, PF, TMS, UR>>,
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
