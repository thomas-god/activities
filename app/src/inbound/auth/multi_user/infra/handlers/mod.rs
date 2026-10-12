use axum::Router;
use axum::extract::FromRef;
use axum::routing::{get, post};

use std::sync::Arc;
use std::time::Duration;

use crate::inbound::auth::multi_user::IMultiUserService;
use crate::inbound::http::CookieConfig;
use crate::inbound::http::middlewares::rate_limit::{IpRateLimitLayer, RateLimitStore};

pub use extractor::cookie_auth_middleware;
pub use login_user::login_user;
pub use logout::logout;
pub use me::get_me;
pub use register_user::register_user;
pub use set_password::{register_password, update_password};
pub use update_email::{update_email, validate_email_change};
pub use validate_login::validate_login;

pub mod extractor;
pub mod login_user;
pub mod logout;
pub mod me;
pub mod register_user;
pub mod set_password;
pub mod update_email;
pub mod validate_login;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserRegistration {
    Allowed,
    Closed,
}

impl From<&bool> for UserRegistration {
    fn from(value: &bool) -> Self {
        if *value { Self::Allowed } else { Self::Closed }
    }
}

impl From<bool> for UserRegistration {
    fn from(value: bool) -> Self {
        Self::from(&value)
    }
}

impl From<&UserRegistration> for bool {
    fn from(value: &UserRegistration) -> Self {
        match value {
            UserRegistration::Allowed => true,
            UserRegistration::Closed => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthAppState<UR: IMultiUserService> {
    user_service: Arc<UR>,
    cookie_config: Arc<CookieConfig>,
    registration_mode: UserRegistration,
}

impl<US> FromRef<AuthAppState<US>> for Arc<US>
where
    US: IMultiUserService,
{
    fn from_ref(input: &AuthAppState<US>) -> Self {
        input.user_service.clone()
    }
}

pub fn multi_user_login_routes<US: IMultiUserService, S>(
    mut base_router: Router<S>,
    user_service: US,
    registration_mode: UserRegistration,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let auth_state = AuthAppState {
        cookie_config: Arc::new(CookieConfig::default()),
        user_service: Arc::new(user_service),
        registration_mode,
    };

    base_router = base_router.route_layer(axum::middleware::from_fn_with_state(
        auth_state.clone(),
        cookie_auth_middleware::<US>,
    ));

    // Tighter limit for the mail-sending endpoints: each hit sends an email, so this also
    // guards against using the app to email-bomb an arbitrary address.
    let mail_router = Router::new()
        .route("/register", post(register_user::<US>))
        .route("/login", post(login_user::<US>))
        .route_layer(IpRateLimitLayer::new(
            RateLimitStore::new(),
            5,
            Duration::from_secs(60),
        ));

    // Looser limit for token validation: the token space is large enough that brute-forcing
    // isn't realistic, but this still caps the cost of the Argon2 verification per IP.
    let validate_router = Router::new()
        .route("/login/validate", post(validate_login::<US>))
        .route("/email/change/validate", post(validate_email_change::<US>))
        .route("/logout", post(logout::<US>))
        .route_layer(IpRateLimitLayer::new(
            RateLimitStore::new(),
            30,
            Duration::from_secs(60),
        ));

    // Account routes are session-authenticated, unlike the login/register ones: they mutate an
    // existing account. Both hash a password (Argon2), so they share the validation bucket. The
    // cookie-auth layer is added first so the rate limit (added last, hence outermost) runs before
    // the session lookup.
    let account_router = Router::new()
        .route("/password", post(register_password::<US>))
        .route("/password/change", post(update_password::<US>))
        .route("/account/email", post(update_email::<US>))
        .route("/account/me", get(get_me::<US>))
        .route_layer(axum::middleware::from_fn_with_state(
            auth_state.clone(),
            cookie_auth_middleware::<US>,
        ))
        .route_layer(IpRateLimitLayer::new(
            RateLimitStore::new(),
            30,
            Duration::from_secs(60),
        ));

    let router = mail_router
        .merge(validate_router)
        .merge(account_router)
        .with_state(auth_state);

    base_router.nest("/api", router)
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::http::header::SET_COOKIE;
    use axum::{http::StatusCode, response::IntoResponse, routing::get};
    use chrono::{TimeDelta, Utc};
    use reqwest::header::CONTENT_TYPE;
    use tokio::task::JoinHandle;
    use url::Url;

    use crate::inbound::auth::multi_user::{
        AuthLinkValidationResult, GenerateSessionTokenResult, SessionToken, UserLoginResult,
        UserRegistrationResult, test_utils::MockUserService,
    };

    use super::*;

    struct TestApp {
        base_url: Url,
        client: reqwest::Client,
        server: JoinHandle<()>,
    }

    async fn protected_route() -> impl IntoResponse {
        StatusCode::OK
    }

    // Adapted from https://github.com/tokio-rs/axum/discussions/748
    impl TestApp {
        async fn new(
            user_service: MockUserService,
            registration_mode: UserRegistration,
        ) -> TestApp {
            let app = multi_user_login_routes(
                Router::new().route("/", get(protected_route)),
                user_service,
                registration_mode,
            );

            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("Could not bind ephemeral socket");
            let addr = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                axum::serve(
                    listener,
                    app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await
                .unwrap();
            });

            TestApp {
                base_url: Url::parse(&format!("http://{addr}")).unwrap(),
                client: reqwest::Client::new(),
                server,
            }
        }

        fn post(&self, path: &str) -> reqwest::RequestBuilder {
            let base_url = Some(&self.base_url);
            let base = Url::options().base_url(base_url);
            let url = base.parse(path).unwrap();
            self.client.post(url)
        }

        fn post_json(&self, path: &str, body: serde_json::Value) -> reqwest::RequestBuilder {
            self.post(path)
                .header(CONTENT_TYPE, "application/json")
                .body(body.to_string())
        }
    }

    impl Drop for TestApp {
        fn drop(&mut self) {
            self.server.abort()
        }
    }

    #[tokio::test]
    async fn test_register_is_rejected_when_registration_is_disabled() {
        let mut user_service = MockUserService::new();
        user_service.expect_register_user().times(0);
        let registration_mode = UserRegistration::Closed;
        let app = TestApp::new(user_service, registration_mode).await;

        let response = app
            .post_json(
                "/api/register",
                serde_json::json!({"email": "test@mail.test"}),
            )
            .send()
            .await
            .expect("Should succeed");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_register_is_rate_limited_after_5_requests_per_minute() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_register_user()
            .returning(|_| UserRegistrationResult::Success);
        let registration_mode = UserRegistration::Allowed;
        let app = TestApp::new(user_service, registration_mode).await;

        for i in 0..5 {
            let response = app
                .post_json(
                    "/api/register",
                    serde_json::json!({"email": "test@mail.test"}),
                )
                .send()
                .await
                .expect("Should succeed");
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "request {i} should be allowed"
            );
        }

        let response = app
            .post_json(
                "/api/register",
                serde_json::json!({"email": "test@mail.test"}),
            )
            .send()
            .await
            .expect("Should succeed");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_register_and_login_share_the_same_rate_limit_bucket() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_register_user()
            .returning(|_| UserRegistrationResult::Success);
        user_service
            .expect_login_user()
            .returning(|_, _| UserLoginResult::EmailSent);
        let registration_mode = UserRegistration::Allowed;
        let app = TestApp::new(user_service, registration_mode).await;

        for _ in 0..5 {
            app.post_json(
                "/api/register",
                serde_json::json!({"email": "test@mail.test"}),
            )
            .send()
            .await
            .unwrap();
        }

        let response = app
            .post_json("/api/login", serde_json::json!({"email": "test@mail.test"}))
            .send()
            .await
            .expect("Should succeed");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_validate_login_has_its_own_higher_rate_limit_bucket() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_register_user()
            .returning(|_| UserRegistrationResult::Success);
        user_service
            .expect_validate_auth_link()
            .returning(|_| Ok(AuthLinkValidationResult::Invalid));
        let registration_mode = UserRegistration::Allowed;
        let app = TestApp::new(user_service, registration_mode).await;

        // Exhaust the mail-sending endpoints' bucket...
        for _ in 0..5 {
            app.post_json(
                "/api/register",
                serde_json::json!({"email": "test@mail.test"}),
            )
            .send()
            .await
            .unwrap();
        }

        // ...validate is unaffected since it uses a separate bucket.
        let response = app
            .post_json(
                "/api/login/validate",
                serde_json::json!({"token": "some-token"}),
            )
            .send()
            .await
            .expect("Should succeed");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_validate_login_is_rate_limited_after_30_requests_per_minute() {
        let mut user_service = MockUserService::new();
        user_service
            .expect_validate_auth_link()
            .returning(|_| Ok(AuthLinkValidationResult::Invalid));
        let registration_mode = UserRegistration::Allowed;
        let app = TestApp::new(user_service, registration_mode).await;

        for i in 0..30 {
            let response = app
                .post_json(
                    "/api/login/validate",
                    serde_json::json!({"token": "some-token"}),
                )
                .send()
                .await
                .expect("Should succeed");
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "request {i} should be allowed"
            );
        }

        let response = app
            .post_json(
                "/api/login/validate",
                serde_json::json!({"token": "some-token"}),
            )
            .send()
            .await
            .expect("Should succeed");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_login_does_not_mark_cookie_secure_over_plain_http() {
        let mut user_service = MockUserService::new();
        user_service.expect_login_user().returning(|_, _| {
            UserLoginResult::Session(GenerateSessionTokenResult::new(
                SessionToken::from("session-token"),
                Utc::now() + TimeDelta::days(30),
            ))
        });
        let app = TestApp::new(user_service, UserRegistration::Allowed).await;

        let response = app
            .post_json(
                "/api/login",
                serde_json::json!({"email": "test@mail.test", "password": "secret"}),
            )
            .send()
            .await
            .expect("Should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .expect("expected a session cookie")
            .to_str()
            .unwrap();
        assert!(!set_cookie.contains("Secure"), "got: {set_cookie}");
    }

    #[tokio::test]
    async fn test_login_marks_cookie_secure_when_proxied_over_https() {
        let mut user_service = MockUserService::new();
        user_service.expect_login_user().returning(|_, _| {
            UserLoginResult::Session(GenerateSessionTokenResult::new(
                SessionToken::from("session-token"),
                Utc::now() + TimeDelta::days(30),
            ))
        });
        let app = TestApp::new(user_service, UserRegistration::Allowed).await;

        let response = app
            .post_json(
                "/api/login",
                serde_json::json!({"email": "test@mail.test", "password": "secret"}),
            )
            .header("X-Forwarded-Proto", "https")
            .send()
            .await
            .expect("Should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .expect("expected a session cookie")
            .to_str()
            .unwrap();
        assert!(set_cookie.contains("Secure"), "got: {set_cookie}");
    }
}
