use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    config::Config,
    domain::services::{activity::ActivityService, training_metrics::TrainingMetricService},
    inbound::{
        http::{DisabledUserService, HttpServer},
        parser::FitParser,
    },
    outbound::memory::{
        InMemoryActivityRepository, InMemoryRawDataRepository, InMemoryTrainingMetricsRepository,
    },
};

pub async fn bootsrap_single_user() -> anyhow::Result<
    HttpServer<
        ActivityService<
            InMemoryActivityRepository,
            InMemoryRawDataRepository,
            TrainingMetricService<InMemoryTrainingMetricsRepository, InMemoryActivityRepository>,
        >,
        FitParser,
        TrainingMetricService<InMemoryTrainingMetricsRepository, InMemoryActivityRepository>,
        DisabledUserService,
    >,
> {
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
    let training_metrics_repository = InMemoryTrainingMetricsRepository::new(HashMap::new());

    let training_metrics_service = Arc::new(TrainingMetricService::new(
        training_metrics_repository,
        activity_repository.clone(),
    ));
    let activity_service = ActivityService::new(
        activity_repository.clone(),
        raw_data_repository,
        training_metrics_service.clone(),
    );
    let user_service = DisabledUserService {};

    let parser = FitParser {};

    let http_server = HttpServer::new(
        activity_service,
        parser,
        training_metrics_service,
        user_service,
        config,
    )
    .await?;

    Ok(http_server)
}
