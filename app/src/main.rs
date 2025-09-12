use std::{collections::HashMap, sync::Arc};

use app::{
    config::Config,
    domain::{
        models::{
            activity::Metric,
            training_metrics::{
                TrainingMetricAggregate, TrainingMetricDefinition, TrainingMetricGranularity,
                TrainingMetricId,
            },
        },
        services::{ActivityService, TrainingMetricService},
    },
    inbound::{http::HttpServer, parser::FitParser},
    outbound::memory::{
        InMemoryActivityRepository, InMemoryRawDataRepository, InMemoryTrainingMetricsRepository,
    },
};
use tokio::sync::Mutex;

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

    let activity_repository = Arc::new(Mutex::new(InMemoryActivityRepository::new(vec![])));
    let raw_data_repository = InMemoryRawDataRepository::new(HashMap::new());
    let id = TrainingMetricId::new();
    let training_metrics_repository = InMemoryTrainingMetricsRepository::new(HashMap::from([(
        id.clone(),
        TrainingMetricDefinition::new(
            id,
            Metric::Power,
            TrainingMetricAggregate::Max,
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Max,
        ),
    )]));

    let training_metrics_service = Arc::new(TrainingMetricService::new(
        training_metrics_repository,
        activity_repository.clone(),
    ));
    let activity_service = ActivityService::new(
        activity_repository.clone(),
        raw_data_repository,
        training_metrics_service.clone(),
    );

    let parser = FitParser {};
    let http_server =
        HttpServer::new(activity_service, parser, training_metrics_service, config).await?;

    http_server.run().await
}
