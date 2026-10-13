use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    domain::models::UserId,
    inbound::auth::multi_user::admin::{IAdminService, SetAdminOutcome},
};

use super::{AdminGuard, AdminServiceHandle, admin_error_status};

#[derive(Debug, Deserialize)]
pub struct SetAdminBody {
    is_admin: bool,
}

/// Grants or revokes the admin flag. Refused when it would remove the last admin.
#[tracing::instrument(skip_all)]
pub async fn set_admin<AS: IAdminService>(
    State(admin): State<AdminServiceHandle<AS>>,
    _admin: AdminGuard<AS>,
    Path(user_id): Path<String>,
    Json(body): Json<SetAdminBody>,
) -> impl IntoResponse {
    match admin
        .service()
        .set_admin(UserId::from(user_id), body.is_admin)
        .await
    {
        Ok(SetAdminOutcome::Updated) => StatusCode::OK.into_response(),
        Ok(SetAdminOutcome::UnknownUser) => StatusCode::NOT_FOUND.into_response(),
        Ok(SetAdminOutcome::LastAdmin) => StatusCode::CONFLICT.into_response(),
        Err(err) => admin_error_status(err).into_response(),
    }
}
