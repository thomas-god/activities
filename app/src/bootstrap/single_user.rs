use std::{path::PathBuf, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    config::{Config, load_env},
    domain::services::{activity::ActivityService, training_metrics::TrainingMetricService},
    inbound::{
        http::{DisabledUserService, HttpServer},
        parser::FitParser,
    },
    outbound::{
        fs::FilesystemRawDataRepository,
        sqlite::{
            activity::SqliteActivityRepository, training_metrics::SqliteTrainingMetricsRepository,
        },
    },
};

pub async fn bootsrap_single_user() -> anyhow::Result<
    HttpServer<
        ActivityService<
            SqliteActivityRepository<FilesystemRawDataRepository, FitParser>,
            FilesystemRawDataRepository,
            TrainingMetricService<
                SqliteTrainingMetricsRepository,
                SqliteActivityRepository<FilesystemRawDataRepository, FitParser>,
            >,
        >,
        FitParser,
        TrainingMetricService<
            SqliteTrainingMetricsRepository,
            SqliteActivityRepository<FilesystemRawDataRepository, FitParser>,
        >,
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
    let root_path = PathBuf::from(load_env("ACTIVITIES_DATA_PATH")?);
    let db_dir = root_path.clone().join("db/");
    if !db_dir.exists() {
        tokio::fs::create_dir_all(&db_dir).await?;
    }
    let raw_data_dir = root_path.clone().join("activities/");
    if !raw_data_dir.exists() {
        tokio::fs::create_dir_all(&raw_data_dir).await?;
    }

    let parser = FitParser {};

    let raw_data_repository = FilesystemRawDataRepository::new(raw_data_dir);

    let activity_db = db_dir.clone().join("activities.db");
    let activity_repository = Arc::new(Mutex::new(
        SqliteActivityRepository::new(
            &format!("sqlite:{}", activity_db.to_string_lossy()),
            raw_data_repository.clone(),
            parser.clone(),
        )
        .await?,
    ));

    let trainin_metrics_db = db_dir.clone().join("training_metrics.db");
    let training_metrics_repository = SqliteTrainingMetricsRepository::new(&format!(
        "sqlite:{}",
        trainin_metrics_db.to_string_lossy()
    ))
    .await?;

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
