/// Dedicated binary to launch the app in demo mode, i.e. with no authentication layer, so that
/// anyone can test it without needing to register first. Seeded daily with activities and training
/// data by a dedicated external cron.
use std::{marker::PhantomData, net::SocketAddr, path::PathBuf, sync::Arc};

use activities_auth::infra::http::AuthenticatedUser;
use anyhow::{Context, anyhow};
use axum::{
    Json, Router,
    extract::{FromRequestParts, State},
    http::{
        HeaderValue, Method, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE, COOKIE, SET_COOKIE},
        request::Parts,
    },
    middleware::{self, from_extractor},
    response::IntoResponse,
    routing::get,
};
use derive_more::Constructor;
use serde::Serialize;
use tower_http::cors::CorsLayer;

use activities_app::{
    domain::{
        activity::ports::IActivityService, preferences::ports::IPreferencesService,
        search::ports::ISearchService, shared::UserId, training::ports::ITrainingService,
    },
    http::{
        AppState, StaticFiles, compression_layer, core_routes, health, search_routes,
        static_cache_control,
    },
    parser::ParseFile,
};
use activities_core::{
    domain::{
        activity::services::ActivityService, preferences::services::PreferencesService,
        search::services::SearchService, training::services::TrainingService,
    },
    infra::{
        backup::BackupService,
        clock::Clock,
        config::{BackupConfig, BaseConfig, StdEnvironment},
        fs::FilesystemRawDataRepository,
        parser::Parser,
        sqlite::{
            activity::SqliteActivityRepository, preferences::SqlitePreferencesRepository,
            search::SearchRepository, training::SqliteTrainingRepository,
        },
    },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let telemetry_config = activities_app::config::TelemetryConfig::from_env(&StdEnvironment {});
    let telemetry = activities_app::telemetry::init(&telemetry_config)?;
    let result = run().await;
    telemetry.shutdown();
    result
}

async fn run() -> anyhow::Result<()> {
    let (server, search_service, training_service, activity_service, backup_service) =
        bootstrap_demo().await?;
    let _ = tokio::join!(
        server.run(),
        search_service.run(),
        training_service.run(),
        activity_service.compute_missing_duration_curves(),
        backup_service.run()
    );
    Ok(())
}

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
    Clock,
>;
type ActualSearchService =
    SearchService<SearchRepository<Clock>, ActualActivityService, ActualTrainingService, Clock>;
type ActualBackupService = BackupService<Clock>;

pub async fn bootstrap_demo() -> anyhow::Result<(
    HttpServer<
        ActualActivityService,
        Parser,
        ActualTrainingService,
        PreferencesService<SqlitePreferencesRepository>,
    >,
    ActualSearchService,
    ActualTrainingService,
    ActualActivityService,
    ActualBackupService,
)> {
    tracing::info!("Starting demo version");

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
    let duration_curve_notify = Arc::new(tokio::sync::Notify::new());

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
        duration_curve_notify.clone(),
    );

    let training_metrics_db = db_dir.clone().join("training_metrics.db");
    let training_metrics_repository = SqliteTrainingRepository::new(
        &format!("sqlite:{}", training_metrics_db.to_string_lossy()),
        Clock::new(),
    )
    .await?;

    let training_service = TrainingService::new(
        training_metrics_repository,
        activity_service.clone(),
        training_notify.clone(),
        duration_curve_notify,
        tokio_util::sync::CancellationToken::new(),
        Clock::new(),
    );

    let preferences_service = build_preferences_service(&config).await?;

    let search_service = build_search_service(
        &config,
        activity_service.clone(),
        activity_notify,
        training_service.clone(),
        training_notify,
    )
    .await?;

    let backup_service = BackupService::new(
        BackupConfig::from_env(&StdEnvironment {}).map_err(|err| anyhow!(err))?,
        Clock::new(),
        tokio_util::sync::CancellationToken::new(),
    );

    let http_server = HttpServer::new(
        activity_service.clone(),
        parser,
        training_service.clone(),
        preferences_service,
        search_service.clone(),
        config,
    )
    .await?;

    Ok((
        http_server,
        search_service,
        training_service,
        activity_service.clone(),
        backup_service,
    ))
}

pub struct HttpServer<AS, PF, TMS, PS> {
    router: axum::Router,
    listener: tokio::net::TcpListener,
    _marker_activity: PhantomData<AS>,
    _marker_parser: PhantomData<PF>,
    _marker_training_metrics: PhantomData<TMS>,
    _marker_preferences_service: PhantomData<PS>,
}

impl<AS: IActivityService, PF: ParseFile, TMS: ITrainingService, PS: IPreferencesService>
    HttpServer<AS, PF, TMS, PS>
{
    #[allow(clippy::too_many_arguments)]
    pub async fn new<SS: ISearchService>(
        activity_service: AS,
        file_parser: PF,
        training_metric_service: TMS,
        preferences_service: PS,
        search_service: SS,
        config: BaseConfig,
    ) -> anyhow::Result<Self> {
        let trace_layer = tower_http::trace::TraceLayer::new_for_http().make_span_with(
            |request: &axum::extract::Request<_>| {
                let uri = request.uri().to_string();
                tracing::info_span!("http_request", method = ?request.method(), uri)
            },
        );

        let state = AppState {
            activity_service: Arc::new(activity_service),
            training_metrics_service: Arc::new(training_metric_service),
            file_parser: Arc::new(file_parser),
            preferences_service: Arc::new(preferences_service),
        };

        let origin = config
            .allow_origin
            .parse::<HeaderValue>()
            .with_context(|| format!("Not a valid origin {}", config.allow_origin))?;

        let api_routes = core_routes(state.clone()).merge(search_routes(search_service));
        let mut router = axum::Router::new().nest("/api", api_routes);

        let info_router = Router::new()
            .route("/auth_info", get(auth_info))
            .with_state(DemoAuthInfoResponse::new("NoAuth".to_string()));

        router = router.nest("/api", info_router);

        let mut router = router.route("/health", get(health));

        if let Some(static_path) = &config.static_assets_path {
            router = router.fallback_service(StaticFiles::try_new(static_path)?);
        }

        router = router
            .layer(from_extractor::<DefaultUserExtractor>())
            .layer(middleware::from_fn(static_cache_control))
            .layer(trace_layer)
            .layer(
                CorsLayer::new()
                    .allow_headers([CONTENT_TYPE, COOKIE, SET_COOKIE, CONTENT_DISPOSITION])
                    .expose_headers([CONTENT_DISPOSITION])
                    .allow_origin([origin])
                    .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH])
                    .allow_credentials(true),
            )
            .layer(compression_layer());

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.server_port))
            .await
            .with_context(|| format!("failed to listen on {}", config.server_port))?;

        Ok(Self {
            router,
            listener,
            _marker_activity: PhantomData,
            _marker_parser: PhantomData,
            _marker_training_metrics: PhantomData,
            _marker_preferences_service: PhantomData,
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        tracing::debug!("listening on {}", self.listener.local_addr().unwrap());
        axum::serve(
            self.listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .context("received error from running server")?;
        Ok(())
    }
}
/// Public information about the configured authentication strategy, exposed at `/api/auth_info`.
#[derive(Debug, Clone, Serialize, Constructor)]
pub struct DemoAuthInfoResponse {
    strategy: String,
}

pub async fn auth_info(State(state): State<DemoAuthInfoResponse>) -> impl IntoResponse {
    Json(state)
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

/// Dummy extractor that always returns the default [UserId], regardless of the request.
pub struct DefaultUserExtractor;

impl<S> FromRequestParts<S> for DefaultUserExtractor
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::new(UserId::default());

        parts.extensions.insert(user);

        Ok(Self)
    }
}
