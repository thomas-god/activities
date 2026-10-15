use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use serde::Serialize;

use crate::{
    domain::models::UserId,
    inbound::auth::multi_user::admin::{AdminActor, IAdminService},
};

use super::{AdminGuard, AdminServiceHandle, admin_error_status};

#[derive(Debug, Serialize, PartialEq)]
pub struct RevokeSessionsResponse {
    revoked: u64,
}

/// Revokes every session of the given user.
#[tracing::instrument(skip_all)]
pub async fn revoke_user_sessions<AS: IAdminService>(
    State(admin): State<AdminServiceHandle<AS>>,
    guard: AdminGuard<AS>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    let actor = AdminActor::Web(guard.user().clone());
    match admin
        .service()
        .revoke_user_sessions(&actor, UserId::from(user_id))
        .await
    {
        Ok(revoked) => Json(RevokeSessionsResponse { revoked }).into_response(),
        Err(err) => admin_error_status(err).into_response(),
    }
}
