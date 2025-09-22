use std::{collections::HashMap, sync::Arc};

use anyhow::Ok;
use tokio::sync::Mutex;

use crate::{
    config::{Config, load_env},
    domain::services::{activity::ActivityService, training_metrics::TrainingMetricService},
    inbound::{
        http::{
            HttpServer, InMemoryMagicLinkRepository, InMemorySessionRepository,
            InMemoryUserRepository, MagicLinkService, SMTPEmailProvider, SessionService,
            UserService,
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
            MagicLinkService<InMemoryMagicLinkRepository, SMTPEmailProvider>,
            InMemoryUserRepository,
            SessionService<InMemorySessionRepository>,
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
    let magic_link_repository = Arc::new(Mutex::new(InMemoryMagicLinkRepository::new(Arc::new(
        Mutex::new(Vec::new()),
    ))));
    let mail_provider = Arc::new(build_mailer()?);
    let magic_link_service = Arc::new(Mutex::new(MagicLinkService::new(
        magic_link_repository,
        mail_provider,
    )));
    let user_repository = Arc::new(Mutex::new(InMemoryUserRepository::new(Arc::new(
        Mutex::new(HashMap::new()),
    ))));
    let session_repository = Arc::new(Mutex::new(InMemorySessionRepository::new(Arc::new(
        Mutex::new(Vec::new()),
    ))));
    let session_service = Arc::new(Mutex::new(SessionService::new(session_repository)));
    let user_service = UserService::new(magic_link_service, user_repository, session_service);

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
