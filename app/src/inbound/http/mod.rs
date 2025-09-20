use std::marker::PhantomData;
use std::sync::Arc;

use anyhow::Context;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, Method};
use axum::middleware::from_extractor;
use axum::routing::{delete, get, patch};
use axum::{Router, routing::post};
use tokio::net;
use tower_http::cors::CorsLayer;

use crate::config::Config;
use crate::domain::ports::{IActivityService, ITrainingMetricService};
use crate::inbound::http::auth::{DefaultUserExtractor, IUserService};
use crate::inbound::http::handlers::{
    create_training_metric, delete_activity, delete_training_metric, get_activity,
    get_training_metrics, list_activities, patch_activity, upload_activities,
};
use crate::inbound::parser::ParseFile;

pub use self::auth::infra::{
    DoNothingMailProvider, InMemoryMagicLinkRepository, InMemoryUserRepository,
};
pub use self::auth::services::{MagicLinkService, SessionService, UserService};

mod auth;
mod handlers;

#[derive(Debug, Clone)]
struct AppState<AS: IActivityService, PF: ParseFile, TMS: ITrainingMetricService, UR: IUserService>
{
    activity_service: Arc<AS>,
    file_parser: Arc<PF>,
    training_metrics_service: Arc<TMS>,
    user_service: Option<Arc<UR>>,
}

pub struct HttpServer<AS, PF, TMS, UR> {
    router: axum::Router,
    listener: net::TcpListener,
    _marker_activity: PhantomData<AS>,
    _marker_paser: PhantomData<PF>,
    _marker_training_metrics: PhantomData<TMS>,
    _marker_user_service: PhantomData<UR>,
}

impl<AS: IActivityService, PF: ParseFile, TMS: ITrainingMetricService, UR: IUserService>
    HttpServer<AS, PF, TMS, UR>
{
    pub async fn new(
        activity_service: AS,
        file_parser: PF,
        training_metric_service: Arc<TMS>,
        session_repository: Option<UR>,
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
            user_service: session_repository.map(Arc::new),
        };

        let origin = config
            .allow_origin
            .parse::<HeaderValue>()
            .with_context(|| format!("Not a valid origin {}", config.allow_origin))?;

        let router = axum::Router::new()
            .nest("/api", api_routes())
            .layer(trace_layer)
            .layer(
                CorsLayer::new()
                    .allow_headers([CONTENT_TYPE])
                    .allow_origin([origin])
                    .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH]),
            )
            .route_layer(from_extractor::<DefaultUserExtractor>())
            .with_state(state);

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

fn api_routes<
    AS: IActivityService,
    FP: ParseFile,
    TMS: ITrainingMetricService,
    UR: IUserService,
>() -> Router<AppState<AS, FP, TMS, UR>> {
    Router::new()
        .route("/activity", post(upload_activities::<AS, FP, TMS, UR>))
        .route("/activities", get(list_activities::<AS, FP, TMS, UR>))
        .route(
            "/activity/{activity_id}",
            get(get_activity::<AS, FP, TMS, UR>),
        )
        .route(
            "/activity/{activity_id}",
            patch(patch_activity::<AS, FP, TMS, UR>),
        )
        .route(
            "/activity/{activity_id}",
            delete(delete_activity::<AS, FP, TMS, UR>),
        )
        .route(
            "/training/metrics",
            get(get_training_metrics::<AS, FP, TMS, UR>),
        )
        .route(
            "/training/metric",
            post(create_training_metric::<AS, FP, TMS, UR>),
        )
        .route(
            "/training/metric/{metric_id}",
            delete(delete_training_metric::<AS, FP, TMS, UR>),
        )
}
