use axum::{Json, Router, extract::State, response::IntoResponse, routing::get};
use serde::Serialize;

use crate::{
    auth::UserAuthStrategy,
    domain::{AuthMethods, IUserService, api_key::IAPIKeyService},
    infra::http::handlers::multi_user_login_routes,
};

/// Public information about the configured authentication strategy, exposed at `/api/auth_info`.
#[derive(Debug, Clone, Serialize)]
pub struct AuthInfoResponse {
    registration_open: bool,
    auth_methods: AuthInfoMethods,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthInfoMethods {
    email: bool,
    password: bool,
}

impl From<&AuthMethods> for AuthInfoMethods {
    fn from(value: &AuthMethods) -> Self {
        Self {
            email: value.email(),
            // Password-based authentication is always enabled in multi-user mode.
            password: true,
        }
    }
}

pub fn add_auth_router<S, US: IUserService + IAPIKeyService>(
    strategy: UserAuthStrategy,
    base_router: Router<S>,
    user_service: US,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let registration_open = bool::from(strategy.registration());
    let auth_methods = AuthInfoMethods::from(strategy.auth_methods());

    let router =
        multi_user_login_routes(base_router, user_service, strategy.registration().clone());

    let info_router = Router::new()
        .route("/auth_info", get(auth_info))
        .with_state(AuthInfoResponse {
            registration_open,
            auth_methods,
        });
    router.nest("/api", info_router)
}

pub async fn auth_info(State(state): State<AuthInfoResponse>) -> impl IntoResponse {
    Json(state)
}

#[cfg(test)]
mod tests {
    use axum::{Router, http::StatusCode, routing::get};
    use axum_test::TestServer;

    use crate::{
        auth::{UserAuthStrategy, UserRegistration},
        domain::{AuthMethods, test_utils::MockFullUserService},
    };

    use super::*;

    fn multi_user_strategy(email: bool, allow_registration: bool) -> UserAuthStrategy {
        UserAuthStrategy::new(
            AuthMethods::new(email),
            UserRegistration::from(allow_registration),
        )
    }

    fn build_server(config: UserAuthStrategy, user_service: MockFullUserService) -> TestServer {
        // `add_auth_router` adds a `route_layer`, which panics if the base router has no route
        // yet; give it one so the middleware has something to wrap.
        let base_router = Router::new().route("/", get(|| async { StatusCode::OK }));
        TestServer::new(add_auth_router(config, base_router, user_service))
    }

    #[tokio::test]
    async fn test_password_only_mode_reports_multi_user_strategy_with_password_method() {
        let server = build_server(
            multi_user_strategy(false, false),
            MockFullUserService::new(),
        );

        let response = server.get("/api/auth_info").await;
        response.assert_status(StatusCode::OK);
        let body: serde_json::Value = response.json();
        assert_eq!(body["registration_open"], false);
        assert_eq!(body["auth_methods"]["email"], false);
        assert_eq!(body["auth_methods"]["password"], true);
    }

    #[tokio::test]
    async fn test_email_and_password_mode_reports_both_methods() {
        let server = build_server(multi_user_strategy(true, true), MockFullUserService::new());

        let response = server.get("/api/auth_info").await;
        response.assert_status(StatusCode::OK);
        let body: serde_json::Value = response.json();
        assert_eq!(body["registration_open"], true);
        assert_eq!(body["auth_methods"]["email"], true);
        assert_eq!(body["auth_methods"]["password"], true);
    }
}
