use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;

use crate::inbound::auth::multi_user::admin::{AdminUser, IAdminService};

use super::{AdminGuard, AdminServiceHandle, admin_error_status};

#[derive(Debug, Serialize, PartialEq)]
pub struct AdminUserResponse {
    user_id: String,
    email: String,
    email_valid: bool,
    verified: bool,
    has_password: bool,
    is_admin: bool,
}

impl From<AdminUser> for AdminUserResponse {
    fn from(user: AdminUser) -> Self {
        Self {
            user_id: user.user_id().to_string(),
            email: user.email().to_string(),
            email_valid: user.email_valid(),
            verified: user.verified(),
            has_password: user.has_password(),
            is_admin: user.is_admin(),
        }
    }
}

/// Lists every user with the flags the admin page needs.
#[tracing::instrument(skip_all)]
pub async fn list_users<AS: IAdminService>(
    State(admin): State<AdminServiceHandle<AS>>,
    _admin: AdminGuard<AS>,
) -> impl IntoResponse {
    match admin.service().list_users().await {
        Ok(users) => Json(
            users
                .into_iter()
                .map(AdminUserResponse::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(err) => admin_error_status(err).into_response(),
    }
}
