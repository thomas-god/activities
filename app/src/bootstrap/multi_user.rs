use std::{path::PathBuf, sync::Arc};

use anyhow::{Ok, anyhow};
use tokio::sync::Mutex;

use crate::{
    clock::Clock,
    config::{AppMode, BaseConfig, MultiUserConfig, StdEnvironment},
    domain::services::{
        activity::ActivityService, preferences::PreferencesService, training::TrainingService,
    },
    inbound::{
        http::{
            AuthLinkService, HttpServer, SMTPEmailProvider, SessionService,
            SqliteAuthLinkRepository, SqliteSessionRepository, SqliteUserRepository, UserService,
            spawn_expired_auth_links_cleanup, spawn_expired_sessions_cleanup,
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

const EXPIRED_AUTH_STATE_CLEANUP_INTERVAL: std::time::Duration =
    std::time::Duration::from_secs(3600 * 24);

pub async fn bootstrap_multi_user(
    mode_config: MultiUserConfig,
    mode: AppMode,
) -> anyhow::Result<
    HttpServer<
        ActivityService<
            SqliteActivityRepository<FilesystemRawDataRepository, Parser, Clock>,
            FilesystemRawDataRepository,
        >,
        Parser,
        TrainingService<
            SqliteTrainingRepository<Clock>,
            ActivityService<
                SqliteActivityRepository<FilesystemRawDataRepository, Parser, Clock>,
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

async fn build_mailer(config: &MultiUserConfig) -> anyhow::Result<SMTPEmailProvider> {
    let mailer = SMTPEmailProvider::new(
        &config.mailer_from,
        &config.mailer_username,
        &config.mailer_password,
        &config.mailer_relay,
        &config.mailer_domain,
    )?;

    if let Err(err) = mailer.test_connection().await {
        tracing::error!(
            "Failed to connect to the configured SMTP relay: {err}. Application will start but won't be able to authenticate new users"
        );
    }

    Ok(mailer)
}

async fn build_activity_service(
    config: &BaseConfig,
) -> anyhow::Result<(
    ActivityService<
        SqliteActivityRepository<FilesystemRawDataRepository, Parser, Clock>,
        FilesystemRawDataRepository,
    >,
    Parser,
    Arc<
        TrainingService<
            SqliteTrainingRepository<Clock>,
            ActivityService<
                SqliteActivityRepository<FilesystemRawDataRepository, Parser, Clock>,
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
        Clock::new(),
    )
    .await?;

    let activity_service = ActivityService::new(activity_repository.clone(), raw_data_repository);

    let trainin_metrics_db = db_dir.clone().join("training_metrics.db");
    let training_metrics_repository = SqliteTrainingRepository::new(
        &format!("sqlite:{}", trainin_metrics_db.to_string_lossy()),
        Clock::new(),
    )
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
    spawn_expired_auth_links_cleanup(
        auth_link_repository.clone(),
        EXPIRED_AUTH_STATE_CLEANUP_INTERVAL,
    );
    let mail_provider = Arc::new(build_mailer(mode_config).await?);
    let auth_link_service = Arc::new(Mutex::new(AuthLinkService::new(
        auth_link_repository,
        mail_provider,
    )));

    let user_db = db_dir.clone().join("user.db");
    let user_repository = Arc::new(Mutex::new(
        SqliteUserRepository::new(&format!("sqlite:{}", user_db.to_string_lossy())).await?,
    ));

    let session_db = db_dir.clone().join("session.db");
    let session_repository =
        SqliteSessionRepository::new(&format!("sqlite:{}", session_db.to_string_lossy())).await?;
    spawn_expired_sessions_cleanup(
        session_repository.clone(),
        EXPIRED_AUTH_STATE_CLEANUP_INTERVAL,
    );
    let session_service = SessionService::new(session_repository);
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
