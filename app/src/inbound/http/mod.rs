use std::marker::PhantomData;
use std::sync::Arc;

use anyhow::Context;
use axum::extract::DefaultBodyLimit;
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderValue, Method};

use axum::routing::{delete, get, patch};
use axum::{Router, routing::post};
use cookie::SameSite;
use tokio::net;
use tower_http::cors::CorsLayer;

use crate::config::Config;
use crate::domain::ports::{IActivityService, IPreferencesService, ITrainingService};

use crate::inbound::parser::ParseFile;
use handlers::{
    compute_training_metric_values, create_training_metric, create_training_note,
    create_training_period, delete_activity, delete_preference, delete_training_metric,
    delete_training_note, delete_training_period, get_activity, get_all_preferences,
    get_preference, get_training_metric_values, get_training_metrics, get_training_note,
    get_training_notes, get_training_period, get_training_periods, list_activities, patch_activity,
    set_preference, update_training_note, update_training_period, upload_activities,
};

#[cfg(feature = "multi-user")]
pub use self::auth::infra::mailer::smtp::SMTPEmailProvider;

pub use self::auth::infra::{
    mailer::DoNothingMailProvider,
    sqlite::{
        magic_link::SqliteMagicLinkRepository, session::SqliteSessionRepository,
        user::SqliteUserRepository,
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
            secure: true,
            same_site: SameSite::Strict,
            http_only: true,
            domain: None,
        }
    }
}

#[derive(Debug, Clone)]
struct AppState<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    UR: IUserService,
    PS: IPreferencesService,
> {
    activity_service: Arc<AS>,
    file_parser: Arc<PF>,
    training_metrics_service: Arc<TMS>,
    user_service: Arc<UR>,
    #[allow(dead_code)]
    preferences_service: Arc<PS>,
    cookie_config: Arc<CookieConfig>,
}

pub struct HttpServer<AS, PF, TMS, UR, PS> {
    router: axum::Router,
    listener: net::TcpListener,
    _marker_activity: PhantomData<AS>,
    _marker_paser: PhantomData<PF>,
    _marker_training_metrics: PhantomData<TMS>,
    _marker_user_service: PhantomData<UR>,
    _marker_preferences_service: PhantomData<PS>,
}

impl<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    US: IUserService,
    PS: IPreferencesService,
> HttpServer<AS, PF, TMS, US, PS>
{
    pub async fn new(
        activity_service: AS,
        file_parser: PF,
        training_metric_service: Arc<TMS>,
        session_repository: US,
        preferences_service: PS,
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
            preferences_service: Arc::new(preferences_service),
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
            _marker_preferences_service: PhantomData,
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
    TS: ITrainingService,
    US: IUserService,
    PS: IPreferencesService,
>(
    state: AppState<AS, PF, TS, US, PS>,
) -> Router<AppState<AS, PF, TS, US, PS>> {
    let mut router = Router::new()
        .route(
            "/activity",
            post(upload_activities::<AS, PF, TS, US, PS>)
                .route_layer(DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .route("/activities", get(list_activities::<AS, PF, TS, US, PS>))
        .route(
            "/activity/{activity_id}",
            get(get_activity::<AS, PF, TS, US, PS>),
        )
        .route(
            "/activity/{activity_id}",
            patch(patch_activity::<AS, PF, TS, US, PS>),
        )
        .route(
            "/activity/{activity_id}",
            delete(delete_activity::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/metrics",
            get(get_training_metrics::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/metric",
            post(create_training_metric::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/metric/{metric_id}",
            delete(delete_training_metric::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/metric/{metric_id}/values",
            get(get_training_metric_values::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/metric/values",
            get(compute_training_metric_values::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/period",
            post(create_training_period::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/note",
            post(create_training_note::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/notes",
            get(get_training_notes::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/note/{note_id}",
            get(get_training_note::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/note/{note_id}",
            patch(update_training_note::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/note/{note_id}",
            delete(delete_training_note::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/period/{period_id}",
            get(get_training_period::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/period/{period_id}",
            delete(delete_training_period::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/period/{period_id}",
            patch(update_training_period::<AS, PF, TS, US, PS>),
        )
        .route(
            "/training/periods",
            get(get_training_periods::<AS, PF, TS, US, PS>),
        )
        .route(
            "/preferences",
            get(get_all_preferences::<AS, PF, TS, US, PS>),
        )
        .route("/preferences", post(set_preference::<AS, PF, TS, US, PS>))
        .route(
            "/preferences/{key}",
            get(get_preference::<AS, PF, TS, US, PS>),
        )
        .route(
            "/preferences/{key}",
            delete(delete_preference::<AS, PF, TS, US, PS>),
        );

    if cfg!(feature = "single-user") {
        router = router.route_layer(axum::middleware::from_extractor::<
            crate::inbound::http::auth::DefaultUserExtractor,
        >());
    } else {
        router = router.route_layer(axum::middleware::from_extractor_with_state::<
            crate::inbound::http::auth::CookieUserExtractor<US>,
            AppState<AS, PF, TS, US, PS>,
        >(state.clone()));
    }

    router
}

fn login_routes<
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingService,
    US: IUserService,
    PS: IPreferencesService,
>() -> Router<AppState<AS, PF, TMS, US, PS>> {
    Router::new()
        .route(
            "/register",
            post(crate::inbound::http::handlers::register_user::<AS, PF, TMS, US, PS>),
        )
        .route(
            "/login",
            post(crate::inbound::http::handlers::login_user::<AS, PF, TMS, US, PS>),
        )
        .route(
            "/login/validate/{magic_token}",
            post(crate::inbound::http::handlers::validate_login::<AS, PF, TMS, US, PS>),
        )
}
