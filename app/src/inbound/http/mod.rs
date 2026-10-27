use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};

use anyhow::Context;
use axum::extract::DefaultBodyLimit;
use axum::extract::Request;
use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;

use axum::routing::{delete, get, patch};
use axum::{Json, Router, response::IntoResponse, routing::post};
use cookie::{Cookie, SameSite, time::OffsetDateTime};
use serde_json::json;
use tokio::net;
use tower::Service;
use tower_http::compression::CompressionLayer;
use tower_http::compression::predicate::{NotForContentType, Predicate, SizeAbove};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::config::{AppMode, BaseConfig};
use crate::domain::ports::{
    activity::IActivityService, preferences::IPreferencesService, search::ISearchService,
    training::ITrainingService,
};

use crate::inbound::auth::AuthStrategy;
use crate::inbound::auth::infra::add_auth_router;
use crate::inbound::auth::multi_user::{
    IMultiUserService, admin::IAdminService, infra::handlers::admin::admin_routes,
};
use crate::inbound::http::handlers::{
    get_default_activity_metrics, get_training_metric_templates, upload_bulk_weight_and_nutrition,
};
use crate::inbound::parser::ParseFile;
use handlers::{
    compute_training_metric_values, copy_training_metric, create_standalone_activity,
    create_training_metric, create_training_note, create_training_period, delete_activity,
    delete_hooper_index, delete_preference, delete_training_metric, delete_training_note,
    delete_training_period, delete_weight_and_nutrition, get_active_training_periods, get_activity,
    get_all_preferences, get_all_raw_activities, get_hooper_index, get_preference,
    get_raw_activity, get_training_metrics, get_training_metrics_ordering, get_training_note,
    get_training_notes, get_training_period, get_training_period_metrics,
    get_training_period_notes, get_training_periods, get_weight_and_nutrition, list_activities,
    patch_activity, save_hooper_index, save_weight_and_nutrition, search, set_preference,
    set_training_metrics_ordering, update_training_metric, update_training_note,
    update_training_period, upload_activities,
};

pub use crate::inbound::auth::multi_user::infra::mailer::smtp::SMTPEmailProvider;

pub use crate::inbound::auth::multi_user::{
    AuthMethods, DisabledUserService, MultiUserService, SessionService,
    admin::DisabledAdminService,
    infra::{mailer::DoNothingMailProvider, sqlite::SqliteUsersRepository},
    spawn_expired_sessions_cleanup,
};

mod handlers;
pub mod middlewares;
mod shared;

#[derive(Debug, Clone)]
pub struct CookieConfig {
    /// Whether cookies may be marked `Secure` at all (see [`CookieConfig::for_request`]).
    pub secure: bool,
    pub same_site: SameSite,
    pub http_only: bool,
    pub domain: Option<String>,
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            secure: true,
            same_site: SameSite::Strict,
            http_only: true,
            domain: None,
        }
    }
}

impl CookieConfig {
    /// Returns this configuration adjusted to the scheme the request was received on.
    ///
    /// A cookie is only marked `Secure` when the request reached us over HTTPS, either directly or
    /// through a TLS-terminating reverse proxy that forwarded the original scheme in
    /// `X-Forwarded-Proto`. Without this, plain-HTTP deployments (the single-user modes, the
    /// multi-user modes run over a private tunnel, and local development) would have their session
    /// cookie silently dropped by the browser, so every subsequent request would answer `401`.
    pub fn for_request(&self, headers: &HeaderMap) -> Self {
        Self {
            secure: self.secure && request_is_https(headers),
            ..self.clone()
        }
    }
}

/// Whether the request reached us over HTTPS, as reported by the reverse proxy in
/// `X-Forwarded-Proto` (a comma-separated list, of which the first value is the client-facing
/// scheme). When the header is absent we assume plain HTTP, which keeps cookie handling working in
/// direct/local deployments while still hardening it behind a TLS-terminating proxy.
fn request_is_https(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .is_some_and(|proto| proto.trim().eq_ignore_ascii_case("https"))
}

/// Builds a cookie that instructs the browser to immediately drop `name`, for use by logout
/// handlers. Attributes (path/domain/etc) must match the cookie that was originally set for the
/// browser to actually overwrite it.
pub fn build_removal_cookie(name: &'static str, cookie_config: &CookieConfig) -> Cookie<'static> {
    let mut builder = Cookie::build((name, ""))
        .expires(OffsetDateTime::UNIX_EPOCH)
        .secure(cookie_config.secure)
        .http_only(cookie_config.http_only)
        .same_site(cookie_config.same_site)
        .path("/");
    if let Some(domain) = cookie_config.domain.clone() {
        builder = builder.domain(domain);
    }
    builder.build()
}

#[derive(Debug, Clone)]
struct AppState<AS: IActivityService, PF: ParseFile, TMS: ITrainingService, PS: IPreferencesService>
{
    activity_service: Arc<AS>,
    file_parser: Arc<PF>,
    training_metrics_service: Arc<TMS>,
    #[allow(dead_code)]
    preferences_service: Arc<PS>,
}

pub struct HttpServer<AS, PF, TMS, UR, PS, AD> {
    router: axum::Router,
    listener: net::TcpListener,
    _marker_activity: PhantomData<AS>,
    _marker_parser: PhantomData<PF>,
    _marker_training_metrics: PhantomData<TMS>,
    _marker_user_service: PhantomData<UR>,
    _marker_preferences_service: PhantomData<PS>,
    _marker_admin_service: PhantomData<AD>,
}

impl<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    US: IMultiUserService,
    PS: IPreferencesService,
    AD: IAdminService,
> HttpServer<AS, PF, TMS, US, PS, AD>
{
    #[allow(clippy::too_many_arguments)]
    pub async fn new<SS: ISearchService>(
        mode: &AppMode,
        activity_service: AS,
        file_parser: PF,
        training_metric_service: TMS,
        user_service: US,
        admin_service: AD,
        preferences_service: PS,
        search_service: SS,
        config: BaseConfig,
    ) -> anyhow::Result<Self> {
        let trace_layer = tower_http::trace::TraceLayer::new_for_http().make_span_with(
            |request: &axum::extract::Request<_>| {
                let uri = request.uri().to_string();
                tracing::info_span!("http_request", method = ?request.method(), uri)
            },
        );

        let state = AppState {
            activity_service: Arc::new(activity_service),
            training_metrics_service: Arc::new(training_metric_service),
            file_parser: Arc::new(file_parser),
            preferences_service: Arc::new(preferences_service),
        };

        let origin = config
            .allow_origin
            .parse::<HeaderValue>()
            .with_context(|| format!("Not a valid origin {}", config.allow_origin))?;

        let api_routes = core_routes(state.clone()).merge(search_routes(search_service));
        let router = axum::Router::new().nest("/api", api_routes);

        // Admin routes only exist in multi-user mode; the single-user modes carry a disabled admin
        // service and never mount them.
        let admin_router = matches!(mode, AppMode::MultiUser(_))
            .then(|| admin_routes(user_service.clone(), admin_service));

        let auth_strategy = AuthStrategy::from(mode);
        tracing::info!(
            "App starting with authentication strategy: {:?}",
            &auth_strategy
        );
        // Mount /health *after* the auth router: `route_layer` inside
        // `add_auth_router` only wraps the routes that already exist when it
        // is called, so mounting afterwards keeps the healthcheck outside the
        // auth middleware.
        let mut router =
            add_auth_router(auth_strategy, router, user_service).route("/health", get(health));

        if let Some(admin_router) = admin_router {
            router = router.nest("/api", admin_router);
        }

        if let Some(static_path) = &config.static_assets_path {
            router = router.fallback_service(StaticFiles::try_new(static_path)?);
        }

        router = router
            .layer(middleware::from_fn(static_cache_control))
            .layer(trace_layer)
            .layer(
                CorsLayer::new()
                    .allow_headers([CONTENT_TYPE, COOKIE, SET_COOKIE, CONTENT_DISPOSITION])
                    .expose_headers([CONTENT_DISPOSITION])
                    .allow_origin([origin])
                    .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH])
                    .allow_credentials(true),
            )
            .layer(compression_layer());

        let listener = net::TcpListener::bind(format!("0.0.0.0:{}", config.server_port))
            .await
            .with_context(|| format!("failed to listen on {}", config.server_port))?;

        Ok(Self {
            router,
            listener,
            _marker_activity: PhantomData,
            _marker_parser: PhantomData,
            _marker_training_metrics: PhantomData,
            _marker_user_service: PhantomData,
            _marker_preferences_service: PhantomData,
            _marker_admin_service: PhantomData,
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        tracing::debug!("listening on {}", self.listener.local_addr().unwrap());
        axum::serve(
            self.listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .context("received error from running server")?;
        Ok(())
    }
}

/// Dedicated health-check endpoint, used by the Docker `HEALTHCHECK`.
///
/// Mounted at the router root (not under `/api`) and *after* the auth router is
/// applied, so it stays unauthenticated and reachable by infrastructure
/// probes.
async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

/// Serves the built client as the router's fallback.
///
/// Because this is mounted as the fallback, it is also reached by `/api/...`
/// requests for which no API route matched. Those must keep returning an
/// (empty) `404` rather than being answered with the SPA shell, so API paths
/// short-circuit before delegating to `ServeDir`.
#[derive(Clone)]
struct StaticFiles {
    serve_dir: ServeDir<ServeFile>,
}

impl StaticFiles {
    fn try_new(static_path: &str) -> anyhow::Result<Self> {
        let root = PathBuf::from(static_path);
        anyhow::ensure!(
            root.is_dir(),
            "ACTIVITIES_STATIC_ASSETS_PATH points to `{}`, which is not a directory",
            root.display()
        );
        let index = root.join("index.html");
        anyhow::ensure!(
            index.is_file(),
            "ACTIVITIES_STATIC_ASSETS_PATH `{}` does not contain an `index.html`",
            root.display()
        );

        Ok(Self {
            serve_dir: ServeDir::new(&root)
                .append_index_html_on_directories(true)
                .html_as_default_extension(true)
                .fallback(ServeFile::new(index)),
        })
    }
}

impl Service<Request> for StaticFiles {
    type Response = Response;
    type Error = Infallible;
    // The two branches (API's 404 and static files) produce different future types (`ServeDir`'s
    // is opaque), so the future is boxed rather than boxing the whole service.
    type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request) -> Self::Future {
        if is_api_path(request.uri().path()) {
            return Box::pin(async { Ok(StatusCode::NOT_FOUND.into_response()) });
        }

        let mut serve_dir = self.serve_dir.clone();
        Box::pin(async move {
            let response = serve_dir
                .call(request)
                .await
                .expect("serving a file from disk is infallible");
            Ok(response.map(axum::body::Body::new))
        })
    }
}

fn is_api_path(path: &str) -> bool {
    path == "/api" || path.starts_with("/api/") || path == "/health"
}

async fn static_cache_control(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;

    let cache_control = if is_api_path(&path) {
        None
    } else if path.starts_with("/_app/immutable/") {
        // Client's immutable build artifacts
        Some("public, max-age=31536000, immutable")
    } else {
        Some("no-cache")
    };

    if let Some(value) = cache_control {
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static(value));
    }

    response
}

fn compression_layer() -> CompressionLayer<impl Predicate + Clone + Send + Sync + 'static> {
    CompressionLayer::new().compress_when(
        SizeAbove::new(1024)
            .and(NotForContentType::GRPC)
            .and(NotForContentType::IMAGES)
            .and(NotForContentType::SSE)
            .and(NotForContentType::new("application/zip"))
            .and(NotForContentType::new("application/octet-stream")),
    )
}

fn search_routes<SS: ISearchService>(search_service: SS) -> Router<()> {
    Router::new()
        .route("/search", get(search::<SS>))
        .with_state(Arc::new(search_service))
}

fn core_routes<
    AS: IActivityService,
    PF: ParseFile,
    TS: ITrainingService,
    PS: IPreferencesService,
    S,
>(
    state: AppState<AS, PF, TS, PS>,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let router = Router::new()
        .route(
            "/activity",
            post(upload_activities::<AS, PF, TS, PS>)
                .route_layer(DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .route(
            "/activity/standalone",
            post(create_standalone_activity::<AS, PF, TS, PS>),
        )
        .route("/activities", get(list_activities::<AS, PF, TS, PS>))
        .route(
            "/activities/download",
            get(get_all_raw_activities::<AS, PF, TS, PS>),
        )
        .route(
            "/activity/{activity_id}/download",
            get(get_raw_activity::<AS, PF, TS, PS>),
        )
        .route(
            "/activity/{activity_id}",
            get(get_activity::<AS, PF, TS, PS>),
        )
        .route(
            "/activity/{activity_id}",
            patch(patch_activity::<AS, PF, TS, PS>),
        )
        .route(
            "/activity/{activity_id}",
            delete(delete_activity::<AS, PF, TS, PS>),
        )
        .route(
            "/activity/default_metrics",
            get(get_default_activity_metrics),
        )
        .route(
            "/training/metrics",
            get(get_training_metrics::<AS, PF, TS, PS>),
        )
        .route(
            "/training/metrics/ordering",
            get(get_training_metrics_ordering::<AS, PF, TS, PS>)
                .post(set_training_metrics_ordering::<AS, PF, TS, PS>),
        )
        .route(
            "/training/metrics/templates",
            get(get_training_metric_templates),
        )
        .route(
            "/training/metric",
            post(create_training_metric::<AS, PF, TS, PS>),
        )
        .route(
            "/training/metric/{metric_id}",
            delete(delete_training_metric::<AS, PF, TS, PS>)
                .patch(update_training_metric::<AS, PF, TS, PS>),
        )
        .route(
            "/training/metric/{metric_id}/copy",
            post(copy_training_metric::<AS, PF, TS, PS>),
        )
        .route(
            "/training/metric/values",
            post(compute_training_metric_values::<AS, PF, TS, PS>),
        )
        .route(
            "/training/hooper-index/{date}",
            get(get_hooper_index::<AS, PF, TS, PS>)
                .patch(save_hooper_index::<AS, PF, TS, PS>)
                .delete(delete_hooper_index::<AS, PF, TS, PS>),
        )
        .route(
            "/training/weight-and-nutrition/{date}",
            get(get_weight_and_nutrition::<AS, PF, TS, PS>)
                .patch(save_weight_and_nutrition::<AS, PF, TS, PS>)
                .delete(delete_weight_and_nutrition::<AS, PF, TS, PS>),
        )
        .route(
            "/training/weight-and-nutrition/history",
            post(upload_bulk_weight_and_nutrition::<AS, PF, TS, PS>),
        )
        .route(
            "/training/period",
            post(create_training_period::<AS, PF, TS, PS>),
        )
        .route(
            "/training/note",
            post(create_training_note::<AS, PF, TS, PS>),
        )
        .route("/training/notes", get(get_training_notes::<AS, PF, TS, PS>))
        .route(
            "/training/note/{note_id}",
            get(get_training_note::<AS, PF, TS, PS>),
        )
        .route(
            "/training/note/{note_id}",
            patch(update_training_note::<AS, PF, TS, PS>),
        )
        .route(
            "/training/note/{note_id}",
            delete(delete_training_note::<AS, PF, TS, PS>),
        )
        .route(
            "/training/period/{period_id}",
            get(get_training_period::<AS, PF, TS, PS>),
        )
        .route(
            "/training/period/{period_id}",
            delete(delete_training_period::<AS, PF, TS, PS>),
        )
        .route(
            "/training/period/{period_id}",
            patch(update_training_period::<AS, PF, TS, PS>),
        )
        .route(
            "/training/period/{period_id}/notes",
            get(get_training_period_notes::<AS, PF, TS, PS>),
        )
        .route(
            "/training/period/{period_id}/metrics",
            get(get_training_period_metrics::<AS, PF, TS, PS>),
        )
        .route(
            "/training/periods",
            get(get_training_periods::<AS, PF, TS, PS>),
        )
        .route(
            "/training/periods/active",
            get(get_active_training_periods::<AS, PF, TS, PS>),
        )
        .route("/preferences", get(get_all_preferences::<AS, PF, TS, PS>))
        .route("/preferences", post(set_preference::<AS, PF, TS, PS>))
        .route("/preferences/{key}", get(get_preference::<AS, PF, TS, PS>))
        .route(
            "/preferences/{key}",
            delete(delete_preference::<AS, PF, TS, PS>),
        );

    router.with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    use crate::domain::{
        models::{UserId, activity::ActivityId},
        ports::search::{SearchResult, search_test_utils::MockSearchService},
        services::{
            activity::test_utils::MockActivityService,
            preferences::tests_utils::MockPreferencesService,
            training::test_utils::MockTrainingService,
        },
    };
    use crate::inbound::{
        auth::{SinglePassword, multi_user::test_utils::MockUserService},
        parser::test_utils::MockFileParser,
    };

    #[test]
    fn cookie_config_is_secure_only_when_request_is_https() {
        let config = CookieConfig::default();

        assert!(!config.for_request(&HeaderMap::new()).secure);

        let mut headers = HeaderMap::new();
        for proto in ["https", "HTTPS", " https ", "https, http"] {
            headers.insert("x-forwarded-proto", HeaderValue::from_static(proto));
            assert!(
                config.for_request(&headers).secure,
                "`{proto}` should be treated as HTTPS"
            );
        }

        for proto in ["http", "http, https", ""] {
            headers.insert("x-forwarded-proto", HeaderValue::from_static(proto));
            assert!(
                !config.for_request(&headers).secure,
                "`{proto}` should not be treated as HTTPS"
            );
        }
    }

    #[test]
    fn cookie_config_never_marks_cookies_secure_when_disabled() {
        let config = CookieConfig {
            secure: false,
            ..CookieConfig::default()
        };
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));

        assert!(!config.for_request(&headers).secure);
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = Router::new().route("/health", get(health));
        let server = TestServer::new(app);

        let response = server.get("/health").await;
        response.assert_status(StatusCode::OK);
        assert_eq!(response.json::<serde_json::Value>()["status"], "ok");
    }

    #[tokio::test]
    async fn search_route_is_mounted_and_protected() {
        let state = AppState {
            activity_service: Arc::new(MockActivityService::test_default()),
            file_parser: Arc::new(MockFileParser::test_default()),
            training_metrics_service: Arc::new(MockTrainingService::test_default()),
            preferences_service: Arc::new(MockPreferencesService::new()),
        };

        let router = add_auth_router(
            AuthStrategy::SinglePassword(SinglePassword::from("secret")),
            Router::new().nest(
                "/api",
                core_routes(state).merge(search_routes(MockSearchService::new())),
            ),
            MockUserService::new(),
        );
        let app = router.route("/health", get(health));
        let server = TestServer::new(app);

        server
            .get("/api/search?pattern=ride")
            .await
            .assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn search_route_returns_results() {
        let state = AppState {
            activity_service: Arc::new(MockActivityService::test_default()),
            file_parser: Arc::new(MockFileParser::test_default()),
            training_metrics_service: Arc::new(MockTrainingService::test_default()),
            preferences_service: Arc::new(MockPreferencesService::new()),
        };

        let mut search_service = MockSearchService::new();
        search_service
            .expect_search()
            .times(1)
            .withf(|user, pattern| user == &UserId::default() && pattern == "ride")
            .returning(|_, _| Ok(vec![SearchResult::Activity(ActivityId::from("activity-1"))]));

        let router = add_auth_router(
            AuthStrategy::NoAuth,
            Router::new().nest(
                "/api",
                core_routes(state).merge(search_routes(search_service)),
            ),
            MockUserService::new(),
        );
        let app = router.route("/health", get(health));
        let server = TestServer::new(app);

        let response = server.get("/api/search?pattern=ride").await;

        response.assert_status(StatusCode::OK);
        assert_eq!(
            response.json::<serde_json::Value>(),
            serde_json::json!([{ "kind": "activity", "id": "activity-1" }])
        );
    }

    /// Regression test: /health must be mounted *after* the auth router so the
    /// auth middleware (added via `route_layer` inside `add_auth_router`) does
    /// not wrap it, even with an auth strategy enabled.
    #[tokio::test]
    async fn health_bypasses_auth_middleware() {
        let state = AppState {
            activity_service: Arc::new(MockActivityService::test_default()),
            file_parser: Arc::new(MockFileParser::test_default()),
            training_metrics_service: Arc::new(MockTrainingService::test_default()),
            preferences_service: Arc::new(MockPreferencesService::new()),
        };

        let router = add_auth_router(
            AuthStrategy::SinglePassword(SinglePassword::from("secret")),
            Router::new().nest("/api", core_routes(state)),
            MockUserService::new(),
        );
        let app = router.route("/health", get(health));
        let server = TestServer::new(app);

        // The healthcheck must work without any auth cookie...
        let response = server.get("/health").await;
        response.assert_status(StatusCode::OK);
        assert_eq!(response.json::<serde_json::Value>()["status"], "ok");

        // ...while the API itself is still protected.
        server
            .get("/api/activities")
            .await
            .assert_status(StatusCode::UNAUTHORIZED);
    }

    fn build_test_app(training_service: MockTrainingService) -> Router {
        let state = AppState {
            activity_service: Arc::new(MockActivityService::test_default()),
            file_parser: Arc::new(MockFileParser::test_default()),
            training_metrics_service: Arc::new(training_service),
            preferences_service: Arc::new(MockPreferencesService::new()),
        };

        add_auth_router(
            AuthStrategy::NoAuth,
            Router::new().nest("/api", core_routes(state)),
            MockUserService::new(),
        )
    }

    fn test_date() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()
    }

    #[tokio::test]
    async fn create_hooper_index_route_calls_service() {
        use crate::domain::models::training::SubjectiveScale;

        let mut training_service = MockTrainingService::new();
        training_service
            .expect_save_hooper_index()
            .times(1)
            .withf(|req| {
                req.user() == &UserId::default()
                    && req.date() == &test_date()
                    && req.patch().fatigue() == &Some(Some(SubjectiveScale::try_from(5).unwrap()))
                    && req.patch().sleep() == &None
                    && req.patch().pain() == &None
            })
            .returning(|_| Ok(()));

        let server = TestServer::new(build_test_app(training_service));

        let response = server
            .patch("/api/training/hooper-index/2026-01-15")
            .json(&serde_json::json!({ "fatigue": 5 }))
            .await;

        response.assert_status(StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn create_hooper_index_route_rejects_out_of_range_value() {
        let mut training_service = MockTrainingService::new();
        training_service.expect_save_hooper_index().times(0);

        let server = TestServer::new(build_test_app(training_service));

        let response = server
            .patch("/api/training/hooper-index/2026-01-15")
            .json(&serde_json::json!({ "fatigue": 11 }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn update_hooper_index_route_calls_service() {
        let mut training_service = MockTrainingService::new();
        training_service
            .expect_save_hooper_index()
            .times(1)
            .withf(|req| req.user() == &UserId::default() && req.date() == &test_date())
            .returning(|_| Ok(()));

        let server = TestServer::new(build_test_app(training_service));

        let response = server
            .patch("/api/training/hooper-index/2026-01-15")
            .json(&serde_json::json!({ "fatigue": 5, "sleep": null }))
            .await;

        response.assert_status(StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn update_hooper_index_route_rejects_invalid_date() {
        let mut training_service = MockTrainingService::new();
        training_service.expect_save_hooper_index().times(0);

        let server = TestServer::new(build_test_app(training_service));

        let response = server
            .patch("/api/training/hooper-index/not-a-date")
            .json(&serde_json::json!({ "fatigue": 5 }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn delete_hooper_index_route_calls_service() {
        let mut training_service = MockTrainingService::new();
        training_service
            .expect_delete_hooper_index()
            .times(1)
            .withf(|req| req.user() == &UserId::default() && req.date() == &test_date())
            .returning(|_| Ok(()));

        let server = TestServer::new(build_test_app(training_service));

        let response = server.delete("/api/training/hooper-index/2026-01-15").await;

        response.assert_status(StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_hooper_index_route_rejects_invalid_date() {
        let mut training_service = MockTrainingService::new();
        training_service.expect_delete_hooper_index().times(0);

        let server = TestServer::new(build_test_app(training_service));

        let response = server.delete("/api/training/hooper-index/not-a-date").await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_hooper_index_route_returns_values() {
        use crate::domain::models::training::{HooperIndex, SubjectiveScale};

        let mut training_service = MockTrainingService::new();
        training_service
            .expect_get_hooper_index()
            .times(1)
            .withf(|user, date| user == &UserId::default() && date == &test_date())
            .returning(|_, _| {
                Ok(Some(HooperIndex::new(
                    Some(SubjectiveScale::try_from(5).unwrap()),
                    None,
                    None,
                    None,
                    Some(SubjectiveScale::try_from(8).unwrap()),
                )))
            });

        let server = TestServer::new(build_test_app(training_service));

        let response = server.get("/api/training/hooper-index/2026-01-15").await;

        response.assert_status(StatusCode::OK);
        assert_eq!(
            response.json::<serde_json::Value>(),
            serde_json::json!({
                "fatigue": 5,
                "sleep": null,
                "pain": null,
                "stress": null,
                "mood": 8
            })
        );
    }

    #[tokio::test]
    async fn get_hooper_index_route_returns_empty_when_missing() {
        let mut training_service = MockTrainingService::new();
        training_service
            .expect_get_hooper_index()
            .times(1)
            .returning(|_, _| Ok(None));

        let server = TestServer::new(build_test_app(training_service));

        let response = server.get("/api/training/hooper-index/2026-01-15").await;

        response.assert_status(StatusCode::OK);
        assert_eq!(
            response.json::<serde_json::Value>(),
            serde_json::json!({
                "fatigue": null,
                "sleep": null,
                "pain": null,
                "stress": null,
                "mood": null
            })
        );
    }

    #[tokio::test]
    async fn get_hooper_index_route_rejects_invalid_date() {
        let mut training_service = MockTrainingService::new();
        training_service.expect_get_hooper_index().times(0);

        let server = TestServer::new(build_test_app(training_service));

        let response = server.get("/api/training/hooper-index/not-a-date").await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn save_weight_and_nutrition_route_calls_service() {
        let mut training_service = MockTrainingService::new();
        training_service
            .expect_save_weight_and_nutrition()
            .times(1)
            .withf(|req| {
                req.user() == &UserId::default()
                    && req.date() == &test_date()
                    && req.patch().weight == Some(Some(70.5))
                    && req.patch().fat == None
                    && req.patch().muscle == Some(None)
            })
            .returning(|_| Ok(()));

        let server = TestServer::new(build_test_app(training_service));

        let response = server
            .patch("/api/training/weight-and-nutrition/2026-01-15")
            .json(&serde_json::json!({ "weight": 70.5, "muscle": null }))
            .await;

        response.assert_status(StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn save_weight_and_nutrition_route_rejects_invalid_date() {
        let mut training_service = MockTrainingService::new();
        training_service.expect_save_weight_and_nutrition().times(0);

        let server = TestServer::new(build_test_app(training_service));

        let response = server
            .patch("/api/training/weight-and-nutrition/not-a-date")
            .json(&serde_json::json!({ "weight": 70.5 }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn delete_weight_and_nutrition_route_calls_service() {
        let mut training_service = MockTrainingService::new();
        training_service
            .expect_delete_weight_and_nutrition()
            .times(1)
            .withf(|req| req.user() == &UserId::default() && req.date() == &test_date())
            .returning(|_| Ok(()));

        let server = TestServer::new(build_test_app(training_service));

        let response = server
            .delete("/api/training/weight-and-nutrition/2026-01-15")
            .await;

        response.assert_status(StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_weight_and_nutrition_route_rejects_invalid_date() {
        let mut training_service = MockTrainingService::new();
        training_service
            .expect_delete_weight_and_nutrition()
            .times(0);

        let server = TestServer::new(build_test_app(training_service));

        let response = server
            .delete("/api/training/weight-and-nutrition/not-a-date")
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_weight_and_nutrition_route_returns_values() {
        use crate::domain::models::training::WeightAndNutrition;

        let mut training_service = MockTrainingService::new();
        training_service
            .expect_get_weight_and_nutrition()
            .times(1)
            .withf(|user, date| user == &UserId::default() && date == &test_date())
            .returning(|_, _| {
                Ok(Some(WeightAndNutrition::new(
                    Some(70.5),
                    None,
                    None,
                    Some(2000.0),
                    None,
                    None,
                    None,
                    None,
                    Some(0.0),
                )))
            });

        let server = TestServer::new(build_test_app(training_service));

        let response = server
            .get("/api/training/weight-and-nutrition/2026-01-15")
            .await;

        response.assert_status(StatusCode::OK);
        assert_eq!(
            response.json::<serde_json::Value>(),
            serde_json::json!({
                "weight": 70.5,
                "fat": null,
                "muscle": null,
                "calories": 2000.0,
                "lipid": null,
                "carbs": null,
                "protein": null,
                "water": null,
                "alcohol": 0.0
            })
        );
    }

    #[tokio::test]
    async fn get_weight_and_nutrition_route_returns_empty_when_missing() {
        let mut training_service = MockTrainingService::new();
        training_service
            .expect_get_weight_and_nutrition()
            .times(1)
            .returning(|_, _| Ok(None));

        let server = TestServer::new(build_test_app(training_service));

        let response = server
            .get("/api/training/weight-and-nutrition/2026-01-15")
            .await;

        response.assert_status(StatusCode::OK);
        assert_eq!(
            response.json::<serde_json::Value>(),
            serde_json::json!({
                "weight": null,
                "fat": null,
                "muscle": null,
                "calories": null,
                "lipid": null,
                "carbs": null,
                "protein": null,
                "water": null,
                "alcohol": null
            })
        );
    }

    #[tokio::test]
    async fn get_weight_and_nutrition_route_rejects_invalid_date() {
        let mut training_service = MockTrainingService::new();
        training_service.expect_get_weight_and_nutrition().times(0);

        let server = TestServer::new(build_test_app(training_service));

        let response = server
            .get("/api/training/weight-and-nutrition/not-a-date")
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    /// Tests for serving the built client from the API process, replacing the
    /// `try_files` behavior once delegated to nginx.
    mod static_files {
        use super::*;
        use axum::http::header::{CONTENT_ENCODING, ETAG};
        use std::path::Path;

        fn write_file(root: &Path, relative: &str, content: &[u8]) {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }

        fn build_dir() -> tempfile::TempDir {
            let dir = tempfile::tempdir().unwrap();
            write_file(dir.path(), "index.html", b"<html>shell</html>");
            write_file(dir.path(), "training/periods.html", b"<html>periods</html>");
            write_file(dir.path(), "service-worker.js", b"self.skipWaiting();");
            write_file(dir.path(), "_app/immutable/app.js", &vec![b'x'; 2048]);
            dir
        }

        fn app(root: &Path) -> Router {
            Router::new()
                .route("/health", get(health))
                .fallback_service(StaticFiles::try_new(root.to_str().unwrap()).unwrap())
                .layer(axum::middleware::from_fn(static_cache_control))
                .layer(compression_layer())
        }

        fn app_with_api(root: &Path) -> Router {
            Router::new()
                .nest(
                    "/api",
                    Router::new().route("/ping", get(|| async { "pong" })),
                )
                .fallback_service(StaticFiles::try_new(root.to_str().unwrap()).unwrap())
                .layer(axum::middleware::from_fn(static_cache_control))
                .layer(compression_layer())
        }

        fn cache_control(response: &axum_test::TestResponse) -> Option<&str> {
            response
                .headers()
                .get(CACHE_CONTROL)
                .map(|v| v.to_str().unwrap())
        }

        #[tokio::test]
        async fn serves_the_index_at_root() {
            let dir = build_dir();
            let server = TestServer::new(app(dir.path()));

            let response = server.get("/").await;

            response.assert_status_ok();
            assert_eq!(response.text(), "<html>shell</html>");
            assert_eq!(cache_control(&response), Some("no-cache"));
            assert_eq!(
                response.headers()[CONTENT_TYPE].to_str().unwrap(),
                "text/html"
            );
        }

        #[tokio::test]
        async fn serves_prerendered_pages_for_extensionless_paths() {
            let dir = build_dir();
            let server = TestServer::new(app(dir.path()));

            let response = server.get("/training/periods").await;

            response.assert_status_ok();
            assert_eq!(response.text(), "<html>periods</html>");
        }

        #[tokio::test]
        async fn serves_the_spa_shell_for_client_side_routes() {
            let dir = build_dir();
            let server = TestServer::new(app(dir.path()));

            let response = server.get("/activity/123").await;

            response.assert_status_ok();
            assert_eq!(response.text(), "<html>shell</html>");
            assert_eq!(cache_control(&response), Some("no-cache"));
        }

        #[tokio::test]
        async fn content_hashed_assets_are_immutable() {
            let dir = build_dir();
            let server = TestServer::new(app(dir.path()));

            let response = server.get("/_app/immutable/app.js").await;

            response.assert_status_ok();
            assert_eq!(
                cache_control(&response),
                Some("public, max-age=31536000, immutable")
            );
        }

        #[tokio::test]
        async fn the_service_worker_is_not_immutable() {
            let dir = build_dir();
            let server = TestServer::new(app(dir.path()));

            let response = server.get("/service-worker.js").await;

            response.assert_status_ok();
            assert_eq!(cache_control(&response), Some("no-cache"));
        }

        #[tokio::test]
        async fn api_routes_take_precedence_over_the_fallback() {
            let dir = build_dir();
            let server = TestServer::new(app_with_api(dir.path()));

            server.get("/api/ping").await.assert_text("pong");
            server
                .get("/api/does-not-exist")
                .await
                .assert_status_not_found();
        }

        #[tokio::test]
        async fn health_is_not_given_a_cache_control_header() {
            let dir = build_dir();
            let server = TestServer::new(app(dir.path()));

            let response = server.get("/health").await;

            response.assert_status_ok();
            assert_eq!(cache_control(&response), None);
        }

        #[tokio::test]
        async fn conditional_requests_are_answered_with_not_modified() {
            let dir = build_dir();
            let server = TestServer::new(app(dir.path()));

            let first = server.get("/").await;
            let etag = first.headers()[ETAG].clone();

            let second = server.get("/").add_header("if-none-match", etag).await;

            second.assert_status(StatusCode::NOT_MODIFIED);
        }

        #[tokio::test]
        async fn large_text_assets_are_compressed() {
            let dir = build_dir();
            let server = TestServer::new(app(dir.path()));

            let response = server
                .get("/_app/immutable/app.js")
                .add_header("accept-encoding", "gzip")
                .await;

            response.assert_status_ok();
            assert_eq!(
                response.headers()[CONTENT_ENCODING].to_str().unwrap(),
                "gzip"
            );
        }

        #[test]
        fn missing_static_root_fails_to_build_the_service() {
            let dir = tempfile::tempdir().unwrap();
            let missing = dir.path().join("nope");

            assert!(StaticFiles::try_new(missing.to_str().unwrap()).is_err());
        }

        #[test]
        fn static_root_without_index_html_fails_to_build_the_service() {
            let dir = tempfile::tempdir().unwrap();

            assert!(StaticFiles::try_new(dir.path().to_str().unwrap()).is_err());
        }
    }
}
