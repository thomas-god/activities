use std::marker::PhantomData;
use std::sync::Arc;

use anyhow::Context;
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderValue, Method};

use axum::routing::{delete, get, patch};
use axum::{Router, routing::post};
use cookie::SameSite;
use tokio::net;
use tower_http::cors::CorsLayer;

use crate::config::Config;
use crate::domain::ports::{IActivityService, ITrainingMetricService};

use crate::inbound::http::handlers::{
    create_training_metric, delete_activity, delete_training_metric, get_activity,
    get_training_metrics, list_activities, patch_activity, upload_activities,
};
use crate::inbound::parser::ParseFile;

pub use self::auth::infra::{
    mailer::SMTPEmailProvider,
    memory::{
        DoNothingMailProvider, InMemoryMagicLinkRepository, InMemorySessionRepository,
        InMemoryUserRepository,
    },
};
pub use self::auth::services::{
    DisabledUserService, MagicLinkService, SessionService, UserService,
};
pub use self::auth::{IUserService, MagicLinkValidationResult, UserLoginResult};

mod auth;
mod handlers;

#[derive(Debug, Clone)]
pub struct CookieConfig {
    pub secure: bool,
    pub same_site: SameSite,
    pub http_only: bool,
    pub domain: Option<String>,
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            secure: false,
            same_site: SameSite::Lax,
            http_only: true,
            domain: None,
        }
    }
}

#[derive(Debug, Clone)]
struct AppState<AS: IActivityService, PF: ParseFile, TMS: ITrainingMetricService, UR: IUserService>
{
    activity_service: Arc<AS>,
    file_parser: Arc<PF>,
    training_metrics_service: Arc<TMS>,
    user_service: Arc<UR>,
    cookie_config: Arc<CookieConfig>,
}

pub struct HttpServer<AS, PF, TMS, UR> {
    router: axum::Router,
    listener: net::TcpListener,
    _marker_activity: PhantomData<AS>,
    _marker_paser: PhantomData<PF>,
    _marker_training_metrics: PhantomData<TMS>,
    _marker_user_service: PhantomData<UR>,
}

impl<AS: IActivityService, PF: ParseFile, TMS: ITrainingMetricService, US: IUserService>
    HttpServer<AS, PF, TMS, US>
{
    pub async fn new(
        activity_service: AS,
        file_parser: PF,
        training_metric_service: Arc<TMS>,
        session_repository: US,
        config: Config,
    ) -> anyhow::Result<Self> {
        let trace_layer = tower_http::trace::TraceLayer::new_for_http().make_span_with(
            |request: &axum::extract::Request<_>| {
                let uri = request.uri().to_string();
                tracing::info_span!("http_request", method = ?request.method(), uri)
            },
        );

        let state = AppState {
            activity_service: Arc::new(activity_service),
            training_metrics_service: training_metric_service,
            file_parser: Arc::new(file_parser),
            user_service: Arc::new(session_repository),
            cookie_config: Arc::new(CookieConfig::default()),
        };

        let origin = config
            .allow_origin
            .parse::<HeaderValue>()
            .with_context(|| format!("Not a valid origin {}", config.allow_origin))?;

        let mut router = axum::Router::new().nest("/api", core_routes(state.clone()));

        if cfg!(feature = "multi-user") {
            router = router.nest("/api", login_routes());
        }

        router = router.layer(trace_layer).layer(
            CorsLayer::new()
                .allow_headers([CONTENT_TYPE, COOKIE, SET_COOKIE])
                .allow_origin([origin])
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH])
                .allow_credentials(true),
        );

        let router = router.with_state(state);

        let listener = net::TcpListener::bind(format!("0.0.0.0:{}", config.server_port))
            .await
            .with_context(|| format!("failed to listen on {}", config.server_port))?;

        Ok(Self {
            router,
            listener,
            _marker_activity: PhantomData,
            _marker_paser: PhantomData,
            _marker_training_metrics: PhantomData,
            _marker_user_service: PhantomData,
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        tracing::debug!("listening on {}", self.listener.local_addr().unwrap());
        axum::serve(self.listener, self.router)
            .await
            .context("received error from running server")?;
        Ok(())
    }
}

fn core_routes<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    US: IUserService,
>(
    state: AppState<AS, PF, TMS, US>,
) -> Router<AppState<AS, PF, TMS, US>> {
    let mut router = Router::new()
        .route("/activity", post(upload_activities::<AS, PF, TMS, US>))
        .route("/activities", get(list_activities::<AS, PF, TMS, US>))
        .route(
            "/activity/{activity_id}",
            get(get_activity::<AS, PF, TMS, US>),
        )
        .route(
            "/activity/{activity_id}",
            patch(patch_activity::<AS, PF, TMS, US>),
        )
        .route(
            "/activity/{activity_id}",
            delete(delete_activity::<AS, PF, TMS, US>),
        )
        .route(
            "/training/metrics",
            get(get_training_metrics::<AS, PF, TMS, US>),
        )
        .route(
            "/training/metric",
            post(create_training_metric::<AS, PF, TMS, US>),
        )
        .route(
            "/training/metric/{metric_id}",
            delete(delete_training_metric::<AS, PF, TMS, US>),
        );

    if cfg!(any(feature = "demo", feature = "single-user")) {
        router = router.route_layer(axum::middleware::from_extractor::<
            crate::inbound::http::auth::DefaultUserExtractor,
        >());
    } else {
        router = router.route_layer(axum::middleware::from_extractor_with_state::<
            crate::inbound::http::auth::CookieUserExtractor<US>,
            AppState<AS, PF, TMS, US>,
        >(state.clone()));
    }

    router
}

fn login_routes<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    US: IUserService,
>() -> Router<AppState<AS, PF, TMS, US>> {
    Router::new()
        .route(
            "/register",
            post(crate::inbound::http::handlers::register_user::<AS, PF, TMS, US>),
        )
        .route(
            "/login",
            post(crate::inbound::http::handlers::login_user::<AS, PF, TMS, US>),
        )
        .route(
            "/login/validate/{magic_token}",
            post(crate::inbound::http::handlers::validate_login::<AS, PF, TMS, US>),
        )
}
