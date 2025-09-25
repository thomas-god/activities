use std::{path::PathBuf, sync::Arc};

use anyhow::Ok;
use tokio::sync::Mutex;

use crate::{
    config::{Config, load_env},
    domain::services::{activity::ActivityService, training_metrics::TrainingMetricService},
    inbound::{
        http::{
            HttpServer, MagicLinkService, SMTPEmailProvider, SessionService,
            SqliteMagicLinkRepository, SqliteSessionRepository, SqliteUserRepository, UserService,
        },
        parser::FitParser,
    },
    outbound::{
        fs::FilesystemRawDataRepository,
        sqlite::{
            activity::SqliteActivityRepository, training_metrics::SqliteTrainingMetricsRepository,
        },
    },
};

pub async fn bootsrap_multi_user() -> anyhow::Result<
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
        UserService<
            MagicLinkService<SqliteMagicLinkRepository, SMTPEmailProvider>,
            SqliteUserRepository,
            SessionService<SqliteSessionRepository>,
        >,
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

    let (activity_service, parser, training_metrics_service) = build_activity_service().await?;

    let user_service = build_user_service().await?;

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

fn build_mailer() -> anyhow::Result<SMTPEmailProvider> {
    let from = load_env("ACTIVITIES_MAILER_FROM")?;
    let username = load_env("ACTIVITIES_MAILER_USERNAME")?;
    let password = load_env("ACTIVITIES_MAILER_PASSWORD")?;
    let relay = load_env("ACTIVITIES_MAILER_RELAY")?;
    let domain = load_env("ACTIVITIES_MAILER_DOMAIN")?;

    let mailer = SMTPEmailProvider::new(&from, &username, &password, &relay, &domain)?;

    Ok(mailer)
}

async fn build_activity_service() -> anyhow::Result<(
    ActivityService<
        SqliteActivityRepository<FilesystemRawDataRepository, FitParser>,
        FilesystemRawDataRepository,
        TrainingMetricService<
            SqliteTrainingMetricsRepository,
            SqliteActivityRepository<FilesystemRawDataRepository, FitParser>,
        >,
    >,
    FitParser,
    Arc<
        TrainingMetricService<
            SqliteTrainingMetricsRepository,
            SqliteActivityRepository<FilesystemRawDataRepository, FitParser>,
        >,
    >,
)> {
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

    anyhow::Ok((activity_service, parser, training_metrics_service))
}

async fn build_user_service() -> anyhow::Result<
    UserService<
        MagicLinkService<SqliteMagicLinkRepository, SMTPEmailProvider>,
        SqliteUserRepository,
        SessionService<SqliteSessionRepository>,
    >,
> {
    let root_path = PathBuf::from(load_env("ACTIVITIES_DATA_PATH")?);
    let db_dir = root_path.clone().join("db/");
    if !db_dir.exists() {
        tokio::fs::create_dir_all(&db_dir).await?;
    }

    let magic_db = db_dir.clone().join("magic_link.db");
    let magic_link_repository = Arc::new(Mutex::new(
        SqliteMagicLinkRepository::new(&format!("sqlite:{}", magic_db.to_string_lossy())).await?,
    ));
    let mail_provider = Arc::new(build_mailer()?);
    let magic_link_service = Arc::new(Mutex::new(MagicLinkService::new(
        magic_link_repository,
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
    let user_service = UserService::new(magic_link_service, user_repository, session_service);

    Ok(user_service)
}
