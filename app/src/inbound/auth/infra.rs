use axum::{Json, Router, extract::State, response::IntoResponse, routing::get};
use serde::Serialize;

use crate::inbound::auth::{
    AuthStrategy,
    multi_user::{AuthMethods, IUserService, infra::handlers::multi_user_login_routes},
    no_auth::no_auth_login_routes,
    single_password::single_password_login_routes,
};

/// Public information about the configured authentication strategy, exposed at `/api/auth_info`.
#[derive(Debug, Clone, Serialize)]
pub struct AuthInfoResponse {
    /// Strategy name: `"NoAuth"`, `"SinglePassword"` or `"MultiUser"`.
    strategy: String,
    /// Whether new users can register; only set for the multi-user (`MultiUser`) strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    registration_open: Option<bool>,
    /// Multi-user only: which authentication methods are active on this instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    auth_methods: Option<AuthInfoMethods>,
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
            password: value.password(),
        }
    }
}

pub fn add_auth_router<S, US: IUserService>(
    strategy: AuthStrategy,
    base_router: Router<S>,
    user_service: US,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let (registration_open, auth_methods) = match &strategy {
        AuthStrategy::MultiUser(multi_user) => (
            Some(bool::from(multi_user.registration())),
            Some(AuthInfoMethods::from(multi_user.auth_methods())),
        ),
        AuthStrategy::NoAuth | AuthStrategy::SinglePassword(_) => (None, None),
    };

    let router = match &strategy {
        AuthStrategy::NoAuth => no_auth_login_routes(base_router),
        AuthStrategy::SinglePassword(pwd) => single_password_login_routes(base_router, pwd),
        // Every multi-user combination mounts the same email-based routes: `/api/login` handles
        // both password and mail flows, and `/api/register` accepts an optional password. Which
        // methods are active is advertised through `/api/auth_info`, not through the route set.
        AuthStrategy::MultiUser(multi_user) => {
            multi_user_login_routes(base_router, user_service, multi_user.registration().clone())
        }
    };

    let info_router = Router::new()
        .route("/auth_info", get(auth_info))
        .with_state(AuthInfoResponse {
            strategy: format!("{strategy}"),
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
    use axum::Router;
    use axum::http::StatusCode;
    use axum::routing::get;
    use axum_test::TestServer;
    use serde_json::json;

    use crate::config::{AppMode, EmailAuthConfig, MultiUserConfig, PasswordAuthConfig};
    use crate::inbound::auth::multi_user::test_utils::MockUserService;

    use super::*;

    fn multi_user_mode(email: bool, password: bool, allow_registration: bool) -> AppMode {
        AppMode::MultiUser(MultiUserConfig {
            email: email.then(|| EmailAuthConfig {
                from: "noreply@example.com".to_string(),
                username: "mailer-user".to_string(),
                password: "mailer-password".to_string(),
                relay: "smtp.example.com".to_string(),
                domain: "https://example.com".to_string(),
            }),
            password: password.then(PasswordAuthConfig::default),
            allow_registration,
        })
    }

    fn build_server(mode: &AppMode, user_service: MockUserService) -> TestServer {
        let strategy = AuthStrategy::from(mode);
        // `add_auth_router` adds a `route_layer`, which panics if the base router has no route
        // yet; give it one so the middleware has something to wrap.
        let base_router = Router::new().route("/", get(|| async { StatusCode::OK }));
        TestServer::new(add_auth_router(strategy, base_router, user_service))
    }

    #[tokio::test]
    async fn test_password_only_mode_reports_multi_user_strategy_with_password_method() {
        let server = build_server(&multi_user_mode(false, true, false), MockUserService::new());

        let response = server.get("/api/auth_info").await;
        response.assert_status(StatusCode::OK);
        let body: serde_json::Value = response.json();
        assert_eq!(body["strategy"], "MultiUser");
        assert_eq!(body["registration_open"], false);
        assert_eq!(body["auth_methods"]["email"], false);
        assert_eq!(body["auth_methods"]["password"], true);

        // Password-only mode still mounts the (shared) email-based login routes. The test server
        // has no `ConnectInfo`, so the rate limiter short-circuits with 429; what matters here is
        // that the route is mounted rather than answering 404.
        let response = server
            .post("/api/login")
            .json(&json!({"email": "test@mail.test", "password": "secret"}))
            .await;
        assert_ne!(response.status_code(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_email_and_password_mode_reports_both_methods() {
        let server = build_server(&multi_user_mode(true, true, true), MockUserService::new());

        let response = server.get("/api/auth_info").await;
        response.assert_status(StatusCode::OK);
        let body: serde_json::Value = response.json();
        assert_eq!(body["strategy"], "MultiUser");
        assert_eq!(body["registration_open"], true);
        assert_eq!(body["auth_methods"]["email"], true);
        assert_eq!(body["auth_methods"]["password"], true);
    }
}
