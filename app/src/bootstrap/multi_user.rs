use std::{collections::HashMap, path::Path, sync::Arc};

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
    outbound::memory::{
        InMemoryActivityRepository, InMemoryRawDataRepository, InMemoryTrainingMetricsRepository,
    },
};

pub async fn bootsrap_multi_user() -> anyhow::Result<
    HttpServer<
        ActivityService<
            InMemoryActivityRepository,
            InMemoryRawDataRepository,
            TrainingMetricService<InMemoryTrainingMetricsRepository, InMemoryActivityRepository>,
        >,
        FitParser,
        TrainingMetricService<InMemoryTrainingMetricsRepository, InMemoryActivityRepository>,
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

    let user_service = build_user_service().await?;

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

fn build_mailer() -> anyhow::Result<SMTPEmailProvider> {
    let from = load_env("ACTIVITIES_MAILER_FROM")?;
    let username = load_env("ACTIVITIES_MAILER_USERNAME")?;
    let password = load_env("ACTIVITIES_MAILER_PASSWORD")?;
    let relay = load_env("ACTIVITIES_MAILER_RELAY")?;
    let domain = load_env("ACTIVITIES_MAILER_DOMAIN")?;

    let mailer = SMTPEmailProvider::new(&from, &username, &password, &relay, &domain)?;

    Ok(mailer)
}

async fn build_user_service() -> anyhow::Result<
    UserService<
        MagicLinkService<SqliteMagicLinkRepository, SMTPEmailProvider>,
        SqliteUserRepository,
        SessionService<SqliteSessionRepository>,
    >,
> {
    let db_dir = load_env("ACTIVITIES_DB_PATH")?;

    let magic_db = Path::new(&db_dir).join("magic_link.db");
    let magic_link_repository = Arc::new(Mutex::new(
        SqliteMagicLinkRepository::new(&format!("sqlite:{}", magic_db.to_string_lossy())).await?,
    ));
    let mail_provider = Arc::new(build_mailer()?);
    let magic_link_service = Arc::new(Mutex::new(MagicLinkService::new(
        magic_link_repository,
        mail_provider,
    )));

    let user_db = Path::new(&db_dir).join("user.db");
    let user_repository = Arc::new(Mutex::new(
        SqliteUserRepository::new(&format!("sqlite:{}", user_db.to_string_lossy())).await?,
    ));

    let session_db = Path::new(&db_dir).join("user.db");
    let session_repository = Arc::new(Mutex::new(
        SqliteSessionRepository::new(&format!("sqlite:{}", session_db.to_string_lossy())).await?,
    ));
    let session_service = Arc::new(Mutex::new(SessionService::new(session_repository)));
    let user_service = UserService::new(magic_link_service, user_repository, session_service);

    Ok(user_service)
}
