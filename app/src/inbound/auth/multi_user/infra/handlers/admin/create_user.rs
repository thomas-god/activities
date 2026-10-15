use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::inbound::auth::multi_user::{
    EmailAddress, Password,
    admin::{AdminActor, CreateUserOutcome, IAdminService},
};

use super::{AdminGuard, AdminServiceHandle, admin_error_status};

#[derive(Debug, Deserialize)]
pub struct CreateUserBody {
    email: String,
    password: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CreateUserResponse {
    user_id: String,
}

/// Creates a user even when registration is closed. The email is left unverified.
#[tracing::instrument(skip_all)]
pub async fn create_user<AS: IAdminService>(
    State(admin): State<AdminServiceHandle<AS>>,
    guard: AdminGuard<AS>,
    Json(body): Json<CreateUserBody>,
) -> impl IntoResponse {
    let Ok(email) = EmailAddress::try_from(body.email.as_str()) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(password) = Password::try_from(body.password) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let actor = AdminActor::Web(guard.user().clone());
    match admin.service().create_user(&actor, email, password).await {
        Ok(CreateUserOutcome::Created(user_id)) => (
            StatusCode::CREATED,
            Json(CreateUserResponse {
                user_id: user_id.to_string(),
            }),
        )
            .into_response(),
        Ok(CreateUserOutcome::EmailAlreadyUsed) => StatusCode::CONFLICT.into_response(),
        Err(err) => admin_error_status(err).into_response(),
    }
}
