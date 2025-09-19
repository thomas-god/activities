use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{self},
    sync::Arc,
};

use app::{
    config::Config,
    domain::{
        models::{
            UserId,
            activity::{ActivityStatistic, TimeseriesMetric},
            training_metrics::{
                TrainingMetricAggregate, TrainingMetricDefinition, TrainingMetricGranularity,
                TrainingMetricId, TrainingMetricSource,
            },
        },
        ports::{ActivityRepository, IActivityService, ITrainingMetricService, RawDataRepository},
        services::{activity::ActivityService, training_metrics::TrainingMetricService},
    },
    inbound::{
        http::{HttpServer, SessionRepository},
        parser::{FitParser, ParseFile},
    },
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
    let training_metrics_repository = InMemoryTrainingMetricsRepository::new(HashMap::from_iter(
        default_training_metrics_definitions(),
    ));

    let training_metrics_service = Arc::new(TrainingMetricService::new(
        training_metrics_repository,
        activity_repository.clone(),
    ));
    let activity_service = ActivityService::new(
        activity_repository.clone(),
        raw_data_repository,
        training_metrics_service.clone(),
    );
    let session_repository = SessionRepository::new();

    let parser = FitParser {};

    // Load demo activities before starting receiving request
    load_demo_activities(&activity_service, &parser).await;

    let http_server = HttpServer::new(
        activity_service,
        parser,
        training_metrics_service,
        session_repository,
        config,
    )
    .await?;

    http_server.run().await
}

fn default_training_metrics_definitions() -> Vec<(TrainingMetricId, TrainingMetricDefinition)> {
    let mut definitions = Vec::new();

    // Weekly distance
    let id = TrainingMetricId::new();
    definitions.push((
        id.clone(),
        TrainingMetricDefinition::new(
            id.clone(),
            UserId::default(),
            TrainingMetricSource::Statistic(ActivityStatistic::Distance),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Sum,
        ),
    ));

    // Weekly calories
    let id = TrainingMetricId::new();
    definitions.push((
        id.clone(),
        TrainingMetricDefinition::new(
            id.clone(),
            UserId::default(),
            TrainingMetricSource::Statistic(ActivityStatistic::Calories),
            TrainingMetricGranularity::Weekly,
            TrainingMetricAggregate::Sum,
        ),
    ));

    // Activity max heart rate
    let id = TrainingMetricId::new();
    definitions.push((
        id.clone(),
        TrainingMetricDefinition::new(
            id.clone(),
            UserId::default(),
            TrainingMetricSource::Timeseries((
                TimeseriesMetric::HeartRate,
                TrainingMetricAggregate::Max,
            )),
            TrainingMetricGranularity::Activity,
            TrainingMetricAggregate::Max,
        ),
    ));

    definitions
}

async fn load_demo_activities<AR, RDR, TMS>(
    activity_service: &ActivityService<AR, RDR, TMS>,
    parser: &FitParser,
) where
    AR: ActivityRepository,
    RDR: RawDataRepository,
    TMS: ITrainingMetricService,
{
    let Ok(dir) = fs::read_dir("app/resources") else {
        return;
    };

    for file in dir.flatten() {
        if file.path().extension().and_then(OsStr::to_str) == Some("fit") {
            let content = fs::read(file.path()).unwrap();
            let req = parser.try_bytes_into_domain(content).unwrap();
            activity_service
                .create_activity(req.into_request(&UserId::default()))
                .await
                .unwrap();
            tracing::info!("Loaded {:?}", file.path());
        }
    }
}
