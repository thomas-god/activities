use std::sync::Arc;

use anyhow::Context;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, Method};
use axum::routing::{delete, get};
use axum::{Router, routing::post};
use tokio::net;
use tower_http::cors::CorsLayer;

use crate::config::Config;
use crate::domain::ports::{IActivityService, ITrainingMetricService};
use crate::inbound::http::handlers::{
    create_activity, create_training_metric, delete_activity, delete_training_metric, get_activity,
    get_training_metrics, list_activities,
};
use crate::inbound::parser::ParseFile;

mod handlers;

#[derive(Debug, Clone)]
struct AppState<AS: IActivityService, PF: ParseFile, TMS: ITrainingMetricService> {
    activity_service: Arc<AS>,
    file_parser: Arc<PF>,
    training_metrics_service: Arc<TMS>,
}

pub struct HttpServer {
    router: axum::Router,
    listener: net::TcpListener,
}

impl HttpServer {
    pub async fn new(
        activity_service: impl IActivityService,
        file_parser: impl ParseFile,
        training_metric_service: Arc<impl ITrainingMetricService>,
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
                    .allow_methods([Method::GET, Method::POST, Method::DELETE]),
            )
            .with_state(state);

        let listener = net::TcpListener::bind(format!("0.0.0.0:{}", config.server_port))
            .await
            .with_context(|| format!("failed to listen on {}", config.server_port))?;

        Ok(Self { router, listener })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        tracing::debug!("listening on {}", self.listener.local_addr().unwrap());
        axum::serve(self.listener, self.router)
            .await
            .context("received error from running server")?;
        Ok(())
    }
}

fn api_routes<AS: IActivityService, FP: ParseFile, TMS: ITrainingMetricService>()
-> Router<AppState<AS, FP, TMS>> {
    Router::new()
        .route("/activity", post(create_activity::<AS, FP, TMS>))
        .route("/activities", get(list_activities::<AS, FP, TMS>))
        .route("/activity/{activity_id}", get(get_activity::<AS, FP, TMS>))
        .route(
            "/activity/{activity_id}",
            delete(delete_activity::<AS, FP, TMS>),
        )
        .route(
            "/training/metrics",
            get(get_training_metrics::<AS, FP, TMS>),
        )
        .route(
            "/training/metric",
            post(create_training_metric::<AS, FP, TMS>),
        )
        .route(
            "/training/metric/{metric_id}",
            delete(delete_training_metric::<AS, FP, TMS>),
        )
}
