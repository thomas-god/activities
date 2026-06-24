use axum::{Router, extract::State, response::IntoResponse, routing::get};

use crate::inbound::auth::{
    AuthStrategy,
    email_based::{IUserService, infra::handlers::email_based_login_routes},
    no_auth::no_auth_login_routes,
    single_password::single_password_login_routes,
};

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
        AuthStrategy::EmailBased => email_based_login_routes(base_router, user_service),
    };

    let info_router = Router::new()
        .route("/auth_info", get(auth_info))
        .with_state(format!("{strategy:?}"));
    router.nest("/api", info_router)
}

pub async fn auth_info(State(state): State<String>) -> impl IntoResponse {
    state.clone().into_response()
}
