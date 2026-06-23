use axum::Router;
use axum::extract::FromRef;
use axum::routing::post;

use std::sync::Arc;

use crate::inbound::auth::email_based::IUserService;
use crate::inbound::http::CookieConfig;

pub use extractor::cookie_auth_middleware;
pub use login_user::login_user;
pub use register_user::register_user;
pub use validate_login::validate_login;

pub mod extractor;
pub mod login_user;
pub mod register_user;
pub mod validate_login;

#[derive(Debug, Clone)]
pub struct AuthAppState<UR: IUserService> {
    user_service: Arc<UR>,
    cookie_config: Arc<CookieConfig>,
}

impl<US> FromRef<AuthAppState<US>> for Arc<US>
where
    US: IUserService,
{
    fn from_ref(input: &AuthAppState<US>) -> Self {
        input.user_service.clone()
    }
}

pub fn email_based_login_routes<US: IUserService, S>(
    mut base_router: Router<S>,
    user_service: US,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let auth_state = AuthAppState {
        cookie_config: Arc::new(CookieConfig::default()),
        user_service: Arc::new(user_service),
    };

    base_router = base_router.route_layer(axum::middleware::from_fn_with_state(
        auth_state.clone(),
        cookie_auth_middleware::<US>,
    ));

    let router = Router::new()
        .route("/register", post(register_user::<US>))
        .route("/login", post(login_user::<US>))
        .route("/login/validate/{auth_token}", post(validate_login::<US>));
    let router = router.with_state(auth_state);

    base_router.nest("/api", router)
}
