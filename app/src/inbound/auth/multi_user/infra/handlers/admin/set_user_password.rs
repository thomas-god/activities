use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    domain::models::UserId,
    inbound::auth::multi_user::{
        Password,
        admin::{AdminActor, IAdminService, SetUserPasswordOutcome},
    },
};

use super::{AdminGuard, AdminServiceHandle, admin_error_status};

#[derive(Debug, Deserialize)]
pub struct SetUserPasswordBody {
    password: String,
}

/// Sets or resets a user's password without requiring the current one.
#[tracing::instrument(skip_all)]
pub async fn set_user_password<AS: IAdminService>(
    State(admin): State<AdminServiceHandle<AS>>,
    guard: AdminGuard<AS>,
    Path(user_id): Path<String>,
    Json(body): Json<SetUserPasswordBody>,
) -> impl IntoResponse {
    let Ok(password) = Password::try_from(body.password) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let actor = AdminActor::Web(guard.user().clone());
    match admin
        .service()
        .set_user_password(&actor, UserId::from(user_id), password)
        .await
    {
        Ok(SetUserPasswordOutcome::Updated) => StatusCode::OK.into_response(),
        Ok(SetUserPasswordOutcome::UnknownUser) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => admin_error_status(err).into_response(),
    }
}
