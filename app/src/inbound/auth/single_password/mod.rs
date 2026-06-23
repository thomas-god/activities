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
    inbound::{auth::SinglePassword, http::CookieConfig},
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

    let router = Router::new().route("/login", post(login_user));
    let router = router.with_state(state);

    base_router.nest("/api", router)
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
    use axum::{Extension, Router, http::header::COOKIE, routing::get};
    use axum_test::TestServer;

    use super::*;

    async fn protected_route(Extension(user): Extension<AuthenticatedUser>) -> impl IntoResponse {
        user.user().to_string()
    }

    fn build_test_server(password: SinglePassword) -> TestServer {
        let app =
            single_password_login_routes(Router::new().route("/", get(protected_route)), &password);

        TestServer::new(app).expect("unable to create test server")
    }

    #[tokio::test]
    async fn test_login_user_success_sets_cookie_and_authenticates_requests() {
        let password = SinglePassword::from("secret");
        let server = build_test_server(password);

        let response = server
            .post("/api/login")
            .json(&serde_json::json!({
                "password": "secret"
            }))
            .await;

        response.assert_status(StatusCode::OK);
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
        let response = server.get("/").add_header(COOKIE, cookie_pair).await;

        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), UserId::default().to_string());
    }

    #[tokio::test]
    async fn test_login_user_rejects_wrong_password() {
        let server = build_test_server(SinglePassword::from("secret"));

        let response = server
            .post("/api/login")
            .json(&serde_json::json!({
                "password": "not-the-password"
            }))
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
        assert!(response.headers().get(SET_COOKIE).is_none());
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_rejects_missing_cookie() {
        let server = build_test_server(SinglePassword::from("secret"));

        let response = server.get("/").await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_cookie_auth_middleware_rejects_invalid_cookie() {
        let server = build_test_server(SinglePassword::from("secret"));

        let response = server
            .get("/")
            .add_cookie(Cookie::new("token", "not-a-valid-hmac"))
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}
