use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use axum::extract::DefaultBodyLimit;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderValue, Method};

use axum::routing::{delete, get, patch};
use axum::{Router, routing::post};
use cookie::SameSite;
use tokio::net;
use tower_http::cors::CorsLayer;

use crate::config::Config;
use crate::domain::ports::{
    activity::IActivityService, preferences::IPreferencesService, training::ITrainingService,
};

use crate::inbound::auth::email_based::IUserService;
use crate::inbound::auth::infra::{add_auth_router, select_auth_strategy};
use crate::inbound::http::handlers::get_training_metric_templates;
use crate::inbound::parser::ParseFile;
use handlers::{
    compute_training_metric_values, copy_training_metric, create_standalone_activity,
    create_training_metric, create_training_note, create_training_period, delete_activity,
    delete_preference, delete_training_metric, delete_training_note, delete_training_period,
    get_active_training_periods, get_activity, get_all_preferences, get_all_raw_activities,
    get_preference, get_raw_activity, get_training_metrics, get_training_metrics_ordering,
    get_training_note, get_training_notes, get_training_period, get_training_period_metrics,
    get_training_period_notes, get_training_periods, list_activities, patch_activity,
    set_preference, set_training_metrics_ordering, update_training_metric, update_training_note,
    update_training_period, upload_activities,
};

#[cfg(feature = "multi-user")]
pub use crate::inbound::auth::email_based::infra::mailer::smtp::SMTPEmailProvider;

pub use crate::inbound::auth::email_based::{
    AuthLinkService, DisabledUserService, SessionService, UserService,
    infra::{
        mailer::DoNothingMailProvider,
        sqlite::{
            auth_link::SqliteAuthLinkRepository, session::SqliteSessionRepository,
            user::SqliteUserRepository,
        },
    },
};

mod handlers;
pub mod middlewares;

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
struct AppState<AS: IActivityService, PF: ParseFile, TMS: ITrainingService, PS: IPreferencesService>
{
    activity_service: Arc<AS>,
    file_parser: Arc<PF>,
    training_metrics_service: Arc<TMS>,
    #[allow(dead_code)]
    preferences_service: Arc<PS>,
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
        user_service: US,
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
            preferences_service: Arc::new(preferences_service),
        };

        let origin = config
            .allow_origin
            .parse::<HeaderValue>()
            .with_context(|| format!("Not a valid origin {}", config.allow_origin))?;

        let router = axum::Router::new().nest("/api", core_routes(state.clone()));

        let Ok(auth_strategy) = select_auth_strategy() else {
            tracing::error!("Unable to load a valid authentication strategy");
            panic!();
        };
        tracing::info!(
            "App starting with authentication strategy: {:?}",
            &auth_strategy
        );
        let mut router = add_auth_router(auth_strategy, router, user_service);

        router = router.layer(trace_layer).layer(
            CorsLayer::new()
                .allow_headers([CONTENT_TYPE, COOKIE, SET_COOKIE, CONTENT_DISPOSITION])
                .expose_headers([CONTENT_DISPOSITION])
                .allow_origin([origin])
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH])
                .allow_credentials(true),
        );

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
