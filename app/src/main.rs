use std::collections::HashMap;

use app::{
    config::Config,
    domain::services::Service,
    inbound::{http::HttpServer, parser::FitParser},
    outbound::memory::{InMemoryActivityRepository, InMemoryRawDataRepository},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let config = Config::from_env()?;

    let activity_repository = InMemoryActivityRepository::new(vec![]);
    let raw_data_repository = InMemoryRawDataRepository::new(HashMap::new());

    let activity_service = Service::new(activity_repository, raw_data_repository);
    let parser = FitParser {};

    let http_server = HttpServer::new(activity_service, parser, config).await?;

    http_server.run().await
}
