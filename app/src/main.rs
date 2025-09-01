use std::env;

use axum::{
    Json, Router,
    body::Bytes,
    http::{
        HeaderValue, Method, StatusCode,
        header::{CONTENT_TYPE, COOKIE},
    },
    response::IntoResponse,
    routing::{get, post},
};
use fit_parser::{parse_fit_messages, utils::find_field_value_as_uint};
use serde::Serialize;
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

#[derive(Debug, Serialize)]
struct NewActivityResponse {
    calories: Option<usize>,
}

async fn post_activity(bytes: Bytes) -> Result<impl IntoResponse, StatusCode> {
    let content = bytes.to_vec().into_iter();
    let Ok(messages) = parse_fit_messages(content) else {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    };
    let calories = find_field_value_as_uint(
        &messages,
        &fit_parser::FitField::Session(fit_parser::SessionField::TotalCalories),
    );
    tracing::info!("Hello from post_activity");

    Ok((StatusCode::CREATED, Json(NewActivityResponse { calories })))
}
