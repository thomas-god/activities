use axum::{Json, Router, extract::State, response::IntoResponse, routing::get};
use serde::Serialize;

use crate::inbound::auth::{
    AuthStrategy,
    email_based::{IUserService, infra::handlers::email_based_login_routes},
    no_auth::no_auth_login_routes,
    single_password::single_password_login_routes,
};

/// Public information about the configured authentication strategy, exposed at `/api/auth_info`.
#[derive(Debug, Clone, Serialize)]
pub struct AuthInfoResponse {
    strategy: String,
    registration_open: Option<bool>,
}

pub fn add_auth_router<S, US: IUserService>(
    strategy: AuthStrategy,
    base_router: Router<S>,
    user_service: US,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let router = match &strategy {
        AuthStrategy::NoAuth => no_auth_login_routes(base_router),
        AuthStrategy::SinglePassword(pwd) => single_password_login_routes(base_router, pwd),
        AuthStrategy::EmailBased(allow_registration) => {
            email_based_login_routes(base_router, user_service, allow_registration.clone())
        }
    };

    let registration_open = match &strategy {
        AuthStrategy::EmailBased(allow_registration) => Some(allow_registration.into()),
        _ => None,
    };
    let info_router = Router::new()
        .route("/auth_info", get(auth_info))
        .with_state(AuthInfoResponse {
            strategy: format!("{strategy:?}"),
            registration_open,
        });
    router.nest("/api", info_router)
}

pub async fn auth_info(State(state): State<AuthInfoResponse>) -> impl IntoResponse {
    Json(state)
}
