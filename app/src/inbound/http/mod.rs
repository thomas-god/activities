use std::sync::Arc;

use anyhow::Context;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, Method};
use axum::routing::get;
use axum::{Router, routing::post};
use tokio::net;
use tower_http::cors::CorsLayer;

use crate::config::Config;
use crate::domain::ports::ActivityService;
use crate::inbound::http::handlers::{create_activity, list_activities};

mod handlers;

#[derive(Debug, Clone)]
struct AppState<AS: ActivityService> {
    activity_service: Arc<AS>,
}

pub struct HttpServer {
    router: axum::Router,
    listener: net::TcpListener,
}

impl HttpServer {
    pub async fn new(
        activity_service: impl ActivityService,
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
                    .allow_methods([Method::GET, Method::POST]),
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

fn api_routes<AS: ActivityService>() -> Router<AppState<AS>> {
    Router::new()
        .route("/activity", post(create_activity::<AS>))
        .route("/activities", get(list_activities::<AS>))
}
