use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;

use crate::{
    clock::Clock,
    config::{AppMode, BaseConfig, SingleUserConfig, StdEnvironment},
    domain::services::{
        activity::ActivityService, preferences::PreferencesService, search::SearchService,
        training::TrainingService,
    },
    inbound::{
        http::{DisabledUserService, HttpServer},
        parser::Parser,
    },
    outbound::{
        fs::FilesystemRawDataRepository,
        sqlite::{
            activity::SqliteActivityRepository, preferences::SqlitePreferencesRepository,
            search::SearchRepository, training::SqliteTrainingRepository,
        },
    },
};

type ActualActivityService = ActivityService<
    SqliteActivityRepository<FilesystemRawDataRepository, Parser, Clock>,
    FilesystemRawDataRepository,
>;

type ActualTrainingService = TrainingService<
    SqliteTrainingRepository<Clock>,
    ActivityService<
        SqliteActivityRepository<FilesystemRawDataRepository, Parser, Clock>,
        FilesystemRawDataRepository,
    >,
>;
type ActualSearchService =
    SearchService<SearchRepository<Clock>, ActualActivityService, ActualTrainingService, Clock>;

pub async fn bootstrap_single_user(
    _mode_config: SingleUserConfig,
    mode: AppMode,
) -> anyhow::Result<(
    HttpServer<
        ActualActivityService,
        Parser,
        ActualTrainingService,
        DisabledUserService,
        PreferencesService<SqlitePreferencesRepository>,
    >,
    ActualSearchService,
)> {
    tracing::info!("Starting single-user app");

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

    let activity_notify = Arc::new(tokio::sync::Notify::new());
    let training_notify = Arc::new(tokio::sync::Notify::new());

    let activity_db = db_dir.clone().join("activities.db");
    let activity_repository = SqliteActivityRepository::new(
        &format!("sqlite:{}", activity_db.to_string_lossy()),
        raw_data_repository.clone(),
        parser.clone(),
        Clock::new(),
    )
    .await?;
    let activity_service = ActivityService::new(
        activity_repository.clone(),
        raw_data_repository,
        activity_notify.clone(),
    );

    let trainin_metrics_db = db_dir.clone().join("training_metrics.db");
    let training_metrics_repository = SqliteTrainingRepository::new(
        &format!("sqlite:{}", trainin_metrics_db.to_string_lossy()),
        Clock::new(),
    )
    .await?;

    let training_metrics_service = Arc::new(TrainingService::new(
        training_metrics_repository,
        activity_service.clone(),
        training_notify.clone(),
    ));

    let user_service = DisabledUserService {};
    let preferences_service = build_preferences_service(&config).await?;

    let search_service = build_search_service(
        &config,
        activity_service.clone(),
        activity_notify,
        training_metrics_service.as_ref().clone(),
        training_notify,
    )
    .await?;

    let http_server = HttpServer::new(
        &mode,
        activity_service,
        parser,
        training_metrics_service,
        user_service,
        preferences_service,
        search_service.clone(),
        config,
    )
    .await?;

    Ok((http_server, search_service))
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

async fn build_search_service(
    config: &BaseConfig,
    activity_service: ActualActivityService,
    activity_notify: Arc<tokio::sync::Notify>,
    training_service: ActualTrainingService,
    training_notify: Arc<tokio::sync::Notify>,
) -> anyhow::Result<
    SearchService<SearchRepository<Clock>, ActualActivityService, ActualTrainingService, Clock>,
> {
    let search_db = PathBuf::from(config.activities_data_path.clone())
        .join("db/")
        .join("search.db");
    let search_repository = SearchRepository::new(
        &format!("sqlite:{}", search_db.to_string_lossy()),
        Clock::new(),
    )
    .await?;

    anyhow::Ok(SearchService::new(
        search_repository,
        activity_notify,
        activity_service,
        training_notify,
        training_service,
        tokio_util::sync::CancellationToken::new(),
        Clock::new(),
    ))
}
