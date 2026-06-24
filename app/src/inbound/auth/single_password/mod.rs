use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{StatusCode, header::SET_COOKIE},
    middleware::Next,
    response::{AppendHeaders, IntoResponse, Response},
    routing::post,
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, TimeDelta, Utc};
use cookie::{Cookie, time::OffsetDateTime};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::{
    domain::models::UserId,
    inbound::{
        auth::SinglePassword,
        http::{
            CookieConfig,
            middlewares::rate_limit::{IpRateLimitLayer, RateLimitStore},
        },
    },
};

use crate::inbound::auth::AuthenticatedUser;

const SESSION_DURATION: i64 = 30;

#[derive(Clone)]
pub struct SinglePasswordAuthState {
    password: SinglePassword,
    cookie_config: CookieConfig,
}

pub async fn cookie_auth_middleware(
    State(state): State<SinglePasswordAuthState>,
    mut request: Request,
    next: Next,
) -> Response {
    let jar = CookieJar::from_headers(request.headers());
    let Some(cookie) = jar.get("token") else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    if verify_cookie(cookie, &state.password).is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    request
        .extensions_mut()
        .insert(AuthenticatedUser::new(UserId::default()));
    next.run(request).await
}

type HmacSha256 = Hmac<Sha256>;

fn build_cookie<'a>(
    pwd: &'a SinglePassword,
    expiry: &DateTime<Utc>,
    cookie_config: &CookieConfig,
) -> Result<Cookie<'a>, String> {
    let mut mac = HmacSha256::new_from_slice(pwd.as_bytes())
        .map_err(|_| "Error while build the HMAC instance")?;
    mac.update(UserId::default().as_bytes());
    let results = mac.finalize().into_bytes();

    let expire_at = OffsetDateTime::from_unix_timestamp(expiry.timestamp())
        .map_err(|_| format!("Cannot build datetime offset form expiry {expiry:?}"))?;
    let mut builder = Cookie::build(("token", const_hex::encode(results)))
        .expires(expire_at)
        .http_only(cookie_config.http_only)
        .same_site(cookie_config.same_site)
        .path("/");
    if let Some(domain) = cookie_config.domain.clone() {
        builder = builder.domain(domain);
    }
    let cookie = builder.build();
    Ok(cookie)
}

fn verify_cookie(cookie: &Cookie<'_>, pwd: &SinglePassword) -> Option<()> {
    let bytes = const_hex::decode(cookie.value()).ok()?;
    let mut verifier = HmacSha256::new_from_slice(pwd.as_bytes()).ok()?;
    verifier.update(UserId::default().as_bytes());
    verifier.verify_slice(&bytes).ok()
}

pub fn single_password_login_routes<S>(
    mut base_router: Router<S>,
    password: &SinglePassword,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let state = SinglePasswordAuthState {
        password: password.clone(),
        cookie_config: CookieConfig::default(),
    };

    base_router = base_router.route_layer(axum::middleware::from_fn_with_state(
        state.clone(),
        cookie_auth_middleware,
    ));

    let login_router = Router::new()
        .route("/login", post(login_user))
        .route_layer(IpRateLimitLayer::new(
            RateLimitStore::new(),
            100,
            Duration::from_secs(60),
        ))
        .with_state(state);

    base_router.nest("/api", login_router)
}

#[derive(Debug, Deserialize)]
pub struct LoginUserQuery {
    password: String,
}

pub async fn login_user(
    State(state): State<SinglePasswordAuthState>,
    Json(body): Json<LoginUserQuery>,
) -> impl IntoResponse {
    let pwd = SinglePassword::from(body.password);

    if state.password != pwd {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let expiry = Utc::now() + TimeDelta::days(SESSION_DURATION);
    let Ok(cookie) = build_cookie(&state.password, &expiry, &state.cookie_config) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let headers = AppendHeaders([(SET_COOKIE, cookie.encoded().to_string())]);
    (headers, StatusCode::OK).into_response()
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::{Extension, Router, http::header::COOKIE, routing::get};
    use reqwest::header::CONTENT_TYPE;
    use tokio::task::JoinHandle;
    use url::Url;

    use super::*;

    pub struct TestApp {
        pub base_url: Url,
        pub client: reqwest::Client,
        server: JoinHandle<()>,
    }

    // Adapted from https://github.com/tokio-rs/axum/discussions/748
    impl TestApp {
        pub async fn new(password: &SinglePassword) -> TestApp {
            let app = single_password_login_routes(
                Router::new().route("/", get(protected_route)),
                password,
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

        pub fn get(&self, path: &str) -> reqwest::RequestBuilder {
            let base_url = Some(&self.base_url);
            let base = Url::options().base_url(base_url);
            let url = base.parse(path).unwrap();
            self.client.get(url)
        }

        pub fn post(&self, path: &str) -> reqwest::RequestBuilder {
            let base_url = Some(&self.base_url);
            let base = Url::options().base_url(base_url);
            let url = base.parse(path).unwrap();
            self.client.post(url)
        }
    }

    impl Drop for TestApp {
        fn drop(&mut self) {
            tracing::debug!("Dropping test server at {}", self.base_url.as_str());
            self.server.abort()
        }
    }

    async fn protected_route(Extension(user): Extension<AuthenticatedUser>) -> impl IntoResponse {
        user.user().to_string()
    }

    #[tokio::test]
    async fn test_login_user_success_sets_cookie_and_authenticates_requests() {
        let password = SinglePassword::from("secret");
        let app = TestApp::new(&password).await;

        let response = app
            .post("/api/login")
            .body(
                serde_json::json!({
                    "password": "secret"
                })
                .to_string(),
            )
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .expect("Should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .expect("expected Set-Cookie header on login")
            .to_str()
            .expect("Set-Cookie header should be valid ascii");
        assert!(set_cookie.contains("token="));
        assert!(set_cookie.contains("Path=/"));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=Strict"));

        let cookie_pair = set_cookie
            .split(';')
            .next()
            .expect("expected cookie name/value pair");
        let response = app
            .get("/")
            .header(COOKIE, cookie_pair)
            .send()
            .await
            .expect("Should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.text().await.unwrap(),
            UserId::default().to_string()
        );
    }

    #[tokio::test]
    async fn test_login_user_rejects_wrong_password() {
        let password = SinglePassword::from("secret");
        let app = TestApp::new(&password).await;

        let response = app
            .post("/api/login")
            .body(
                serde_json::json!({
                    "password": "not-the-password"
                })
                .to_string(),
            )
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .expect("Should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().get(SET_COOKIE).is_none());
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_rejects_missing_cookie() {
        let password = SinglePassword::from("secret");
        let app = TestApp::new(&password).await;

        let response = app.get("/").send().await.expect("Should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_rejects_invalid_cookie() {
        let password = SinglePassword::from("secret");
        let app = TestApp::new(&password).await;

        let response = app
            .get("/")
            .header(COOKIE, Cookie::new("token", "not-a-valid-hmac").value())
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
