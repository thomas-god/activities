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
        EmailAddress,
        admin::{AdminActor, IAdminService, SetUserEmailOutcome},
    },
};

use super::{AdminGuard, AdminServiceHandle, admin_error_status};

#[derive(Debug, Deserialize)]
pub struct SetUserEmailBody {
    email: String,
}

#[tracing::instrument(skip_all)]
pub async fn set_user_email<AS: IAdminService>(
    State(admin): State<AdminServiceHandle<AS>>,
    guard: AdminGuard<AS>,
    Path(user_id): Path<String>,
    Json(body): Json<SetUserEmailBody>,
) -> impl IntoResponse {
    let Ok(email) = EmailAddress::try_from(body.email) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let actor = AdminActor::Web(guard.user().clone());
    match admin
        .service()
        .set_user_email(&actor, UserId::from(user_id), email)
        .await
    {
        Ok(SetUserEmailOutcome::Updated) => StatusCode::OK.into_response(),
        Ok(SetUserEmailOutcome::UnknownUser) => StatusCode::NOT_FOUND.into_response(),
        Ok(SetUserEmailOutcome::EmailAddressTaken) => StatusCode::CONFLICT.into_response(),
        Err(err) => admin_error_status(err).into_response(),
    }
}
