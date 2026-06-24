use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;

use crate::{
    config::{AppMode, BaseConfig, SingleUserConfig, StdEnvironment},
    domain::services::{
        activity::ActivityService, preferences::PreferencesService, training::TrainingService,
    },
    inbound::{
        http::{DisabledUserService, HttpServer},
        parser::Parser,
    },
    outbound::{
        fs::FilesystemRawDataRepository,
        sqlite::{
            activity::SqliteActivityRepository, preferences::SqlitePreferencesRepository,
            training::SqliteTrainingRepository,
        },
    },
};

pub async fn bootstrap_single_user(
    _mode_config: SingleUserConfig,
    mode: AppMode,
) -> anyhow::Result<
    HttpServer<
        ActivityService<
            SqliteActivityRepository<FilesystemRawDataRepository, Parser>,
            FilesystemRawDataRepository,
        >,
        Parser,
        TrainingService<
            SqliteTrainingRepository,
            ActivityService<
                SqliteActivityRepository<FilesystemRawDataRepository, Parser>,
                FilesystemRawDataRepository,
            >,
        >,
        DisabledUserService,
        PreferencesService<SqlitePreferencesRepository>,
    >,
> {
    tracing::info!("Starting multi-user app");
    // start tracing
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_max_level(tracing::Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .finish();
    if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
        tracing::error!("Error while setting up tracing subscriber: {err:?}");
    };

    let config = BaseConfig::from_env(&StdEnvironment {}).map_err(|err| anyhow!(err))?;
    let root_path = PathBuf::from(config.activities_data_path.clone());
    let db_dir = root_path.clone().join("db/");
    if !db_dir.exists() {
        tokio::fs::create_dir_all(&db_dir).await?;
    }
    let raw_data_dir = root_path.clone().join("activities/");
    if !raw_data_dir.exists() {
        tokio::fs::create_dir_all(&raw_data_dir).await?;
    }

    let parser = Parser {};

    let raw_data_repository = FilesystemRawDataRepository::new(raw_data_dir);

    let activity_db = db_dir.clone().join("activities.db");
    let activity_repository = SqliteActivityRepository::new(
        &format!("sqlite:{}", activity_db.to_string_lossy()),
        raw_data_repository.clone(),
        parser.clone(),
    )
    .await?;
    let activity_service = ActivityService::new(activity_repository.clone(), raw_data_repository);

    let trainin_metrics_db = db_dir.clone().join("training_metrics.db");
    let training_metrics_repository =
        SqliteTrainingRepository::new(&format!("sqlite:{}", trainin_metrics_db.to_string_lossy()))
            .await?;

    let training_metrics_service = Arc::new(TrainingService::new(
        training_metrics_repository,
        activity_service.clone(),
    ));

    let user_service = DisabledUserService {};
    let preferences_service = build_preferences_service(&config).await?;

    let http_server = HttpServer::new(
        &mode,
        activity_service,
        parser,
        training_metrics_service,
        user_service,
        preferences_service,
        config,
    )
    .await?;

    Ok(http_server)
}

async fn build_preferences_service(
    config: &BaseConfig,
) -> anyhow::Result<PreferencesService<SqlitePreferencesRepository>> {
    let root_path = PathBuf::from(config.activities_data_path.clone());
    let db_dir = root_path.clone().join("db/");
    if !db_dir.exists() {
        tokio::fs::create_dir_all(&db_dir).await?;
    }

    let preferences_db = db_dir.clone().join("preferences.db");
    let preferences_repository =
        SqlitePreferencesRepository::new(&format!("sqlite:{}", preferences_db.to_string_lossy()))
            .await?;

    let preference_service = PreferencesService::new(preferences_repository);

    anyhow::Ok(preference_service)
}
