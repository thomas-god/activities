use std::{path::PathBuf, sync::Arc};

use anyhow::{Ok, anyhow};
use tokio::sync::Mutex;

use crate::{
    config::{AppMode, BaseConfig, MultiUserConfig, StdEnvironment},
    domain::services::{
        activity::ActivityService, preferences::PreferencesService, training::TrainingService,
    },
    inbound::{
        http::{
            AuthLinkService, HttpServer, SMTPEmailProvider, SessionService,
            SqliteAuthLinkRepository, SqliteSessionRepository, SqliteUserRepository, UserService,
        },
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

pub async fn bootstrap_multi_user(
    mode_config: MultiUserConfig,
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
        UserService<
            AuthLinkService<SqliteAuthLinkRepository, SMTPEmailProvider>,
            SqliteUserRepository,
            SessionService<SqliteSessionRepository>,
        >,
        PreferencesService<SqlitePreferencesRepository>,
    >,
> {
    tracing::info!("Starting multi-user app");
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

    let config = BaseConfig::from_env(&StdEnvironment {}).map_err(|err| anyhow!(err))?;

    let (activity_service, parser, training_metrics_service) =
        build_activity_service(&config).await?;

    let user_service = build_user_service(&config, &mode_config).await?;

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

fn build_mailer(config: &MultiUserConfig) -> anyhow::Result<SMTPEmailProvider> {
    let mailer = SMTPEmailProvider::new(
        &config.mailer_from,
        &config.mailer_username,
        &config.mailer_password,
        &config.mailer_relay,
        &config.mailer_domain,
    )?;

    Ok(mailer)
}

async fn build_activity_service(
    config: &BaseConfig,
) -> anyhow::Result<(
    ActivityService<
        SqliteActivityRepository<FilesystemRawDataRepository, Parser>,
        FilesystemRawDataRepository,
    >,
    Parser,
    Arc<
        TrainingService<
            SqliteTrainingRepository,
            ActivityService<
                SqliteActivityRepository<FilesystemRawDataRepository, Parser>,
                FilesystemRawDataRepository,
            >,
        >,
    >,
)> {
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

    anyhow::Ok((activity_service, parser, training_metrics_service))
}

async fn build_user_service(
    config: &BaseConfig,
    mode_config: &MultiUserConfig,
) -> anyhow::Result<
    UserService<
        AuthLinkService<SqliteAuthLinkRepository, SMTPEmailProvider>,
        SqliteUserRepository,
        SessionService<SqliteSessionRepository>,
    >,
> {
    let root_path = PathBuf::from(config.activities_data_path.clone());
    let db_dir = root_path.clone().join("db/");
    if !db_dir.exists() {
        tokio::fs::create_dir_all(&db_dir).await?;
    }

    let auth_db = db_dir.clone().join("auth_link.db");
    let auth_link_repository = Arc::new(Mutex::new(
        SqliteAuthLinkRepository::new(&format!("sqlite:{}", auth_db.to_string_lossy())).await?,
    ));
    let mail_provider = Arc::new(build_mailer(mode_config)?);
    let auth_link_service = Arc::new(Mutex::new(AuthLinkService::new(
        auth_link_repository,
        mail_provider,
    )));

    let user_db = db_dir.clone().join("user.db");
    let user_repository = Arc::new(Mutex::new(
        SqliteUserRepository::new(&format!("sqlite:{}", user_db.to_string_lossy())).await?,
    ));

    let session_db = db_dir.clone().join("session.db");
    let session_repository = Arc::new(Mutex::new(
        SqliteSessionRepository::new(&format!("sqlite:{}", session_db.to_string_lossy())).await?,
    ));
    let session_service = Arc::new(Mutex::new(SessionService::new(session_repository)));
    let user_service = UserService::new(auth_link_service, user_repository, session_service);

    Ok(user_service)
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
