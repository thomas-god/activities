//! API key lifecycle HTTP surface: creation, disabling, deletion and activity logs.

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    domain::api_key::{
        APIKey, APIKeyActivity, APIKeyError, APIKeyId, APIPermission, IAPIKeyService,
    },
    infra::http::{
        AuthenticatedUser,
        handlers::AuthAppState,
        rate_limit::{IpRateLimitLayer, RateLimitStore},
    },
};

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyBody {
    name: String,
    permission: String,
    duration_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    id: String,
    name: String,
    permission: &'static str,
    value: String,
    display: String,
    expire_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyActivityResponse {
    activity: String,
    timestamp: String,
}

fn permission_from_str(value: &str) -> Option<APIPermission> {
    match value.to_ascii_lowercase().as_str() {
        "read" => Some(APIPermission::Read),
        "write" => Some(APIPermission::Write),
        "delete" => Some(APIPermission::Delete),
        _ => None,
    }
}

fn permission_as_str(permission: &APIPermission) -> &'static str {
    match permission {
        APIPermission::Read => "read",
        APIPermission::Write => "write",
        APIPermission::Delete => "delete",
    }
}

fn api_key_error_status(err: APIKeyError) -> StatusCode {
    match err {
        APIKeyError::KeyDoesNotExist => StatusCode::NOT_FOUND,
        APIKeyError::UserDoesNotOwnKey => StatusCode::FORBIDDEN,
        APIKeyError::KeyExpiredOrDisabled => StatusCode::UNAUTHORIZED,
        APIKeyError::StorageError(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

impl From<APIKey> for CreateApiKeyResponse {
    fn from(key: APIKey) -> Self {
        Self {
            id: key.id().as_str().to_string(),
            name: key.name().to_string(),
            permission: permission_as_str(key.permission()),
            value: key.value().as_str().to_string(),
            display: key.value().as_display().as_str().to_string(),
            expire_at: *key.expire_at(),
        }
    }
}

impl From<APIKeyActivity> for ApiKeyActivityResponse {
    fn from(activity: APIKeyActivity) -> Self {
        Self {
            activity: activity.activity().to_string(),
            timestamp: activity.timestamp().to_rfc3339(),
        }
    }
}

/// Creates a new API key for the authenticated user. The raw value is returned exactly once.
#[tracing::instrument(skip_all)]
pub async fn create_api_key<AK: IAPIKeyService>(
    State(state): State<AuthAppState<AK>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(body): Json<CreateApiKeyBody>,
) -> impl IntoResponse {
    let Some(permission) = permission_from_str(&body.permission) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let duration = body.duration_seconds.map(std::time::Duration::from_secs);

    match state
        .user_service
        .create_key(user.user(), permission, body.name, duration)
        .await
    {
        Ok(key) => (StatusCode::CREATED, Json(CreateApiKeyResponse::from(key))).into_response(),
        Err(err) => api_key_error_status(err).into_response(),
    }
}

/// Disables an API key owned by the authenticated user. Idempotent.
#[tracing::instrument(skip_all)]
pub async fn disable_api_key<AK: IAPIKeyService>(
    State(state): State<AuthAppState<AK>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(key_id): Path<String>,
) -> impl IntoResponse {
    match state
        .user_service
        .disable_key(user.user(), &APIKeyId::from(key_id))
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => api_key_error_status(err).into_response(),
    }
}

/// Deletes an API key owned by the authenticated user.
#[tracing::instrument(skip_all)]
pub async fn delete_api_key<AK: IAPIKeyService>(
    State(state): State<AuthAppState<AK>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(key_id): Path<String>,
) -> impl IntoResponse {
    match state
        .user_service
        .delete_key(user.user(), &APIKeyId::from(key_id))
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => api_key_error_status(err).into_response(),
    }
}

/// Returns the activity log of an API key owned by the authenticated user, oldest first.
#[tracing::instrument(skip_all)]
pub async fn get_api_key_activity_logs<AK: IAPIKeyService>(
    State(state): State<AuthAppState<AK>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(key_id): Path<String>,
) -> impl IntoResponse {
    match state
        .user_service
        .get_key_activity_logs(user.user(), &APIKeyId::from(key_id))
        .await
    {
        Ok(activities) => Json(
            activities
                .into_iter()
                .map(ApiKeyActivityResponse::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(err) => api_key_error_status(err).into_response(),
    }
}

pub fn api_key_router<AK: IAPIKeyService>() -> Router<AuthAppState<AK>> {
    Router::new()
        .route("/account/api-keys", post(create_api_key::<AK>))
        .route("/account/api-keys/{key_id}", delete(delete_api_key::<AK>))
        .route(
            "/account/api-keys/{key_id}/disable",
            post(disable_api_key::<AK>),
        )
        .route(
            "/account/api-keys/{key_id}/activity",
            get(get_api_key_activity_logs::<AK>),
        )
        .route_layer(IpRateLimitLayer::new(
            RateLimitStore::new(),
            30,
            std::time::Duration::from_secs(60),
        ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{Router, http::StatusCode, middleware, routing::get};
    use axum_test::TestServer;
    use chrono::{TimeDelta, Utc};
    use serde_json::json;

    use activities_core::domain::shared::UserId;

    use crate::{
        auth::UserRegistration,
        domain::{
            api_key::{APIKey, APIKeyActivity, APIKeyError, APIKeyId, APIKeyValue, APIPermission},
            test_utils::MockAPIKeyService,
        },
        infra::http::CookieConfig,
    };

    use super::*;

    /// Injects an authenticated user so handler tests can focus on routing and status mapping,
    /// independently of the cookie middleware (covered by `extractor.rs`).
    async fn authed_user_middleware(
        mut request: axum::extract::Request,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request
            .extensions_mut()
            .insert(AuthenticatedUser::new(UserId::test_default()));
        next.run(request).await
    }

    fn build_server(api_key_service: MockAPIKeyService) -> TestServer {
        let state = AuthAppState {
            user_service: Arc::new(api_key_service),
            cookie_config: Arc::new(CookieConfig::default()),
            registration_mode: UserRegistration::Allowed,
        };

        let app = Router::new()
            .route(
                "/account/api-keys",
                post(create_api_key::<MockAPIKeyService>),
            )
            .route(
                "/account/api-keys/{key_id}",
                delete(delete_api_key::<MockAPIKeyService>),
            )
            .route(
                "/account/api-keys/{key_id}/disable",
                post(disable_api_key::<MockAPIKeyService>),
            )
            .route(
                "/account/api-keys/{key_id}/activity",
                get(get_api_key_activity_logs::<MockAPIKeyService>),
            )
            .layer(middleware::from_fn(authed_user_middleware))
            .with_state(state);

        TestServer::new(app)
    }

    fn key_id(value: &str) -> APIKeyId {
        APIKeyId::from(value)
    }

    fn created_key(expire_at: Option<DateTime<Utc>>) -> APIKey {
        APIKey::new(
            UserId::test_default(),
            key_id("key-1"),
            "test key".to_string(),
            APIPermission::Write,
            APIKeyValue::from("raw-key-value"),
            expire_at,
        )
    }

    fn activity(text: &str, timestamp: DateTime<Utc>) -> APIKeyActivity {
        APIKeyActivity::new(
            UserId::test_default(),
            key_id("key-1"),
            text.to_string(),
            timestamp,
        )
    }

    #[tokio::test]
    async fn test_create_api_key_returns_raw_value_once() {
        let expire_at = Utc::now() + TimeDelta::days(30);
        let mut api_key = MockAPIKeyService::new();
        api_key
            .expect_create_key()
            .times(1)
            .withf(|user, permission, name, duration| {
                user == &UserId::test_default()
                    && matches!(permission, APIPermission::Write)
                    && name == "test key"
                    && duration == &Some(std::time::Duration::from_secs(3600))
            })
            .returning(move |_, _, _, _| Ok(created_key(Some(expire_at))));
        let server = build_server(api_key);

        let response = server
            .post("/account/api-keys")
            .json(&json!({
                "name": "test key",
                "permission": "write",
                "duration_seconds": 3600
            }))
            .await;

        response.assert_status(StatusCode::CREATED);
        response.assert_json(&json!({
            "id": "key-1",
            "name": "test key",
            "permission": "write",
            "value": "raw-key-value",
            "display": "raw***lue",
            "expire_at": expire_at,
        }));
    }

    #[tokio::test]
    async fn test_create_api_key_without_duration_has_no_expiry() {
        let mut api_key = MockAPIKeyService::new();
        api_key
            .expect_create_key()
            .times(1)
            .withf(|_, _, _, duration| duration.is_none())
            .returning(|_, _, _, _| Ok(created_key(None)));
        let server = build_server(api_key);

        let response = server
            .post("/account/api-keys")
            .json(&json!({
                "name": "test key",
                "permission": "read"
            }))
            .await;

        response.assert_status(StatusCode::CREATED);
        assert!(response.json::<serde_json::Value>()["expire_at"].is_null());
    }

    #[tokio::test]
    async fn test_create_api_key_rejects_invalid_permission() {
        let mut api_key = MockAPIKeyService::new();
        api_key.expect_create_key().times(0);
        let server = build_server(api_key);

        let response = server
            .post("/account/api-keys")
            .json(&json!({ "name": "test key", "permission": "admin" }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_api_key_storage_error_is_unavailable() {
        let mut api_key = MockAPIKeyService::new();
        api_key
            .expect_create_key()
            .returning(|_, _, _, _| Err(APIKeyError::StorageError(anyhow::anyhow!("failure"))));
        let server = build_server(api_key);

        let response = server
            .post("/account/api-keys")
            .json(&json!({ "name": "test key", "permission": "read" }))
            .await;

        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_disable_api_key_returns_no_content() {
        let mut api_key = MockAPIKeyService::new();
        api_key
            .expect_disable_key()
            .times(1)
            .withf(|user, key| user == &UserId::test_default() && key == &key_id("key-1"))
            .returning(|_, _| Ok(()));
        let server = build_server(api_key);

        let response = server.post("/account/api-keys/key-1/disable").await;

        response.assert_status(StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_disable_api_key_unknown_key_is_not_found() {
        let mut api_key = MockAPIKeyService::new();
        api_key
            .expect_disable_key()
            .returning(|_, _| Err(APIKeyError::KeyDoesNotExist));
        let server = build_server(api_key);

        let response = server.post("/account/api-keys/unknown/disable").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_disable_api_key_other_user_is_forbidden() {
        let mut api_key = MockAPIKeyService::new();
        api_key
            .expect_disable_key()
            .returning(|_, _| Err(APIKeyError::UserDoesNotOwnKey));
        let server = build_server(api_key);

        let response = server.post("/account/api-keys/key-1/disable").await;

        response.assert_status(StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_delete_api_key_returns_no_content() {
        let mut api_key = MockAPIKeyService::new();
        api_key
            .expect_delete_key()
            .times(1)
            .withf(|user, key| user == &UserId::test_default() && key == &key_id("key-1"))
            .returning(|_, _| Ok(()));
        let server = build_server(api_key);

        let response = server.delete("/account/api-keys/key-1").await;

        response.assert_status(StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_delete_api_key_unknown_key_is_not_found() {
        let mut api_key = MockAPIKeyService::new();
        api_key
            .expect_delete_key()
            .returning(|_, _| Err(APIKeyError::KeyDoesNotExist));
        let server = build_server(api_key);

        let response = server.delete("/account/api-keys/unknown").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_api_key_activity_logs_returns_entries() {
        let timestamp = Utc::now();
        let mut api_key = MockAPIKeyService::new();
        api_key
            .expect_get_key_activity_logs()
            .times(1)
            .withf(|user, key| user == &UserId::test_default() && key == &key_id("key-1"))
            .returning(move |_, _| Ok(vec![activity("logged", timestamp)]));
        let server = build_server(api_key);

        let response = server.get("/account/api-keys/key-1/activity").await;

        response.assert_status(StatusCode::OK);
        response.assert_json(&json!([{
            "activity": "logged",
            "timestamp": timestamp.to_rfc3339(),
        }]));
    }

    #[tokio::test]
    async fn test_get_api_key_activity_logs_unknown_key_is_not_found() {
        let mut api_key = MockAPIKeyService::new();
        api_key
            .expect_get_key_activity_logs()
            .returning(|_, _| Err(APIKeyError::KeyDoesNotExist));
        let server = build_server(api_key);

        let response = server.get("/account/api-keys/unknown/activity").await;

        response.assert_status(StatusCode::NOT_FOUND);
    }
}
