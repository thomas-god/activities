use std::env;

use axum::{
    Router,
    body::Bytes,
    http::{
        HeaderValue, Method, StatusCode,
        header::{CONTENT_TYPE, COOKIE},
    },
    routing::{get, post},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[tokio::main]
async fn main() {
    // start tracing
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_max_level(tracing::Level::DEBUG)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .finish();
    if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
        tracing::error!("Error while setting up tracing subscriber: {err:?}");
    };

    let origin = env::var("ALLOW_ORIGIN")
        .unwrap_or("http://127.0.0.1:5173".to_string())
        .parse::<HeaderValue>()
        .expect("ALLOW_ORIGIN is not a valid origin");
    tracing::info!("allowed origin: {origin:?}");

    let app = Router::new()
        .route(
            "/",
            get(|| async {
                tracing::info!("Hello world");
                "Hello, World!"
            }),
        )
        .route("/activity", post(post_activity))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_headers([CONTENT_TYPE, COOKIE])
                .allow_origin([origin])
                .allow_methods([Method::GET, Method::POST]),
        );

    // run our app with hyper, listening globally on port 3001
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3001));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("App started");
    axum::serve(listener, app).await.unwrap();
}

async fn post_activity(bytes: Bytes) -> StatusCode {
    tracing::info!("Hello from post_activity");
    StatusCode::CREATED
}
