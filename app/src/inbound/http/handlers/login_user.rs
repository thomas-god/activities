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
            auth::{EmailAddress, IUserService},
        },
        parser::ParseFile,
    },
};

#[derive(Debug, Deserialize)]
pub struct LoginUserQuery {
    email: String,
}

pub async fn login_user<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    UR: IUserService,
>(
    State(state): State<AppState<AS, PF, TMS, UR>>,
    Query(query): Query<LoginUserQuery>,
) -> StatusCode {
    let email = EmailAddress::from(query.email);

    match state.user_service.login_user(email).await {
        UserLoginResult::Success => StatusCode::OK,
        UserLoginResult::Retry => StatusCode::SERVICE_UNAVAILABLE,
    }
}
