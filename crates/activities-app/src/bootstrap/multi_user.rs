use std::{path::PathBuf, sync::Arc};

use anyhow::{Ok, anyhow};

use crate::{
    backup::BackupService,
    clock::Clock,
    config::{AppMode, BackupConfig, BaseConfig, EmailAuthConfig, MultiUserConfig, StdEnvironment},
    domain::services::{
        activity::ActivityService, preferences::PreferencesService, search::SearchService,
        training::TrainingService,
    },
    inbound::{
        auth::multi_user::{
            MaybeMailProvider,
            admin::AdminService,
            service::{
                UserRepository, spawn_expired_auth_links_cleanup,
                spawn_expired_email_change_tokens_cleanup,
            },
        },
        http::{
            AuthMethods, DoNothingMailProvider, HttpServer, MultiUserService, SMTPEmailProvider,
            SessionService, SqliteUsersRepository, spawn_expired_sessions_cleanup,
        },
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
    Clock,
>;
type ActualUserService = MultiUserService<
    SqliteUsersRepository,
    SessionService<SqliteUsersRepository>,
    MaybeMailProvider<SMTPEmailProvider>,
>;
type ActualAdminService = AdminService<SqliteUsersRepository, Clock>;
type ActualSearchService =
    SearchService<SearchRepository<Clock>, ActualActivityService, ActualTrainingService, Clock>;
type ActualBackupService = BackupService<Clock>;

const EXPIRED_AUTH_STATE_CLEANUP_INTERVAL: std::time::Duration =
    std::time::Duration::from_secs(3600 * 24);

pub async fn bootstrap_multi_user(
    mode_config: MultiUserConfig,
    mode: AppMode,
) -> anyhow::Result<(
    HttpServer<
        ActualActivityService,
        Parser,
        ActualTrainingService,
        ActualUserService,
        PreferencesService<SqlitePreferencesRepository>,
        ActualAdminService,
    >,
    ActualSearchService,
    ActualTrainingService,
    ActualActivityService,
    ActualBackupService,
)> {
    tracing::info!("Starting multi-user app");

    let config = BaseConfig::from_env(&StdEnvironment {}).map_err(|err| anyhow!(err))?;

    let activity_notify = Arc::new(tokio::sync::Notify::new());
    let training_notify = Arc::new(tokio::sync::Notify::new());
    let duration_curve_notify = Arc::new(tokio::sync::Notify::new());

    let (activity_service, parser, training_service) = build_activity_service(
        &config,
        activity_notify.clone(),
        training_notify.clone(),
        duration_curve_notify,
    )
    .await?;

    let (user_service, admin_service) = build_user_service(&config, &mode_config).await?;

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
        &mode,
        activity_service.clone(),
        parser,
        training_service.clone(),
        user_service,
        admin_service,
        preferences_service,
        search_service.clone(),
        config,
    )
    .await?;
    Ok((
        http_server,
        search_service,
        training_service,
        activity_service,
        backup_service,
    ))
}

async fn build_mailer(email: &EmailAuthConfig) -> anyhow::Result<SMTPEmailProvider> {
    let mailer = SMTPEmailProvider::new(
        &email.from,
        &email.username,
        &email.password,
        &email.relay,
        &email.domain,
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
    activity_notify: Arc<tokio::sync::Notify>,
    training_notify: Arc<tokio::sync::Notify>,
    duration_curve_notify: Arc<tokio::sync::Notify>,
) -> anyhow::Result<(ActualActivityService, Parser, ActualTrainingService)> {
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

    let activity_service = ActivityService::new(
        activity_repository.clone(),
        raw_data_repository,
        activity_notify,
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
        training_notify,
        duration_curve_notify,
        tokio_util::sync::CancellationToken::new(),
        Clock::new(),
    );

    anyhow::Ok((activity_service, parser, training_service))
}

/// Builds the startup warning about accounts without a password, if any.
///
/// Password-based authentication is always enabled, but accounts created before it (or before a
/// password was registered) can only log in through the email-based flow. Operators changing the
/// authentication methods should know how many such accounts exist before they disable email auth.
fn passwordless_users_warning(count: u64, email_auth_enabled: bool) -> Option<String> {
    if count == 0 {
        return None;
    }

    Some(if email_auth_enabled {
        format!(
            "{count} user(s) have no password set. They can only log in through the email-based flow; disabling email-based authentication would lock them out. Ask them to set a password from their account settings while email login is still enabled."
        )
    } else {
        format!(
            "{count} user(s) have no password set and email-based authentication is disabled: these accounts currently cannot log in. Restore email login or set a password for them."
        )
    })
}

async fn build_user_service(
    config: &BaseConfig,
    mode_config: &MultiUserConfig,
) -> anyhow::Result<(ActualUserService, ActualAdminService)> {
    let root_path = PathBuf::from(config.activities_data_path.clone());
    let db_dir = root_path.clone().join("db/");
    if !db_dir.exists() {
        tokio::fs::create_dir_all(&db_dir).await?;
    }

    let user_db = db_dir.clone().join("user.db");
    let users_repository = Arc::new(
        SqliteUsersRepository::new(&format!("sqlite:{}", user_db.to_string_lossy())).await?,
    );

    if let Some(admin_bootstrap) = mode_config.admin_bootstrap.as_ref() {
        AdminService::new(users_repository.as_ref().clone(), Clock::new())
            .apply_bootstrap_config(admin_bootstrap)
            .await
            .map_err(|err| {
                anyhow!(
                    "Failed to bootstrap admin `{}` from environment variables: {err}",
                    admin_bootstrap.email
                )
            })?;
    }

    // Warn about accounts that can only log in through the email-based flow, so operators know
    // before changing the authentication methods (e.g. disabling email auth and locking them out).
    match users_repository.count_users_without_password().await {
        Result::Ok(count) => {
            if let Some(warning) = passwordless_users_warning(count, mode_config.email.is_some()) {
                tracing::warn!("{warning}");
            }
        }
        Result::Err(()) => {
            tracing::warn!(
                "Unable to count users without a password; some accounts may be at risk if email-based authentication is disabled."
            );
        }
    }

    let mailer = match mode_config.email.as_ref() {
        Some(email_config) => MaybeMailProvider::Enabled(build_mailer(email_config).await?),
        None => MaybeMailProvider::Disabled(DoNothingMailProvider::new()),
    };

    spawn_expired_sessions_cleanup(
        users_repository.clone(),
        EXPIRED_AUTH_STATE_CLEANUP_INTERVAL,
    );

    if mode_config.email.is_some() {
        spawn_expired_auth_links_cleanup(
            users_repository.clone(),
            EXPIRED_AUTH_STATE_CLEANUP_INTERVAL,
        );
        spawn_expired_email_change_tokens_cleanup(
            users_repository.clone(),
            EXPIRED_AUTH_STATE_CLEANUP_INTERVAL,
        );
    }

    let admin_service = AdminService::new(users_repository.as_ref().clone(), Clock::new());

    let session_service = SessionService::new(users_repository.clone());
    let auth_methods = AuthMethods::from(mode_config);
    let user_service = MultiUserService::new(
        users_repository,
        session_service,
        auth_methods,
        Arc::new(mailer),
    );

    Ok((user_service, admin_service))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inbound::auth::multi_user::EmailAddress;

    fn base_config(path: &std::path::Path) -> BaseConfig {
        BaseConfig {
            server_port: "0".to_string(),
            allow_origin: "http://localhost".to_string(),
            activities_data_path: path.to_string_lossy().to_string(),
            static_assets_path: None,
        }
    }

    fn password_only_config() -> MultiUserConfig {
        MultiUserConfig {
            email: None,
            allow_registration: true,
            admin_bootstrap: None,
        }
    }

    fn admin_bootstrap_config(email: &str, password: &str) -> MultiUserConfig {
        MultiUserConfig {
            email: None,
            allow_registration: true,
            admin_bootstrap: Some(crate::config::AdminBootstrapConfig {
                email: email.to_string(),
                password: password.to_string(),
            }),
        }
    }

    /// Opens the users database to inspect the admin state after [`build_user_service`] returned.
    async fn admin_repository(data_path: &std::path::Path) -> SqliteUsersRepository {
        let user_db = data_path.join("db").join("user.db");
        SqliteUsersRepository::new(&format!("sqlite:{}", user_db.to_string_lossy()))
            .await
            .expect("user db should open")
    }

    fn email(value: &str) -> EmailAddress {
        EmailAddress::try_from(value).unwrap()
    }

    #[tokio::test]
    async fn test_bootstrap_admin_from_env_creates_admin() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = base_config(tmp.path());

        let _ = build_user_service(
            &config,
            &admin_bootstrap_config("admin@example.com", "s3cret"),
        )
        .await
        .expect("bootstrap config should start");

        assert_eq!(
            admin_repository(tmp.path()).await.test_count_admins().await,
            1
        );
        assert!(
            admin_repository(tmp.path())
                .await
                .test_is_admin(&email("admin@example.com"))
                .await
        );
    }

    #[tokio::test]
    async fn test_bootstrap_admin_from_env_is_skipped_when_admin_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = base_config(tmp.path());

        let _ = build_user_service(
            &config,
            &admin_bootstrap_config("first@example.com", "s3cret"),
        )
        .await
        .unwrap();
        let _ = build_user_service(
            &config,
            &admin_bootstrap_config("second@example.com", "s3cret"),
        )
        .await
        .expect("a second bootstrap should be ignored, not fail");

        assert!(
            admin_repository(tmp.path())
                .await
                .test_is_admin(&email("first@example.com"))
                .await
        );
        assert!(
            !admin_repository(tmp.path())
                .await
                .test_is_admin(&email("second@example.com"))
                .await
        );
        assert_eq!(
            admin_repository(tmp.path()).await.test_count_admins().await,
            1
        );
    }

    #[tokio::test]
    async fn test_bootstrap_admin_from_env_invalid_email_fails_startup() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = base_config(tmp.path());

        assert!(
            build_user_service(&config, &admin_bootstrap_config("not-an-email", "s3cret"))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_no_bootstrap_config_creates_no_admin() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = base_config(tmp.path());

        let _ = build_user_service(&config, &password_only_config())
            .await
            .unwrap();

        assert_eq!(
            admin_repository(tmp.path()).await.test_count_admins().await,
            0
        );
    }

    #[tokio::test]
    async fn test_password_only_config_builds_user_service_without_mailer() {
        let tmp = tempfile::TempDir::new().expect("temp dir should be created");
        let config = base_config(tmp.path());

        let _ = build_user_service(&config, &password_only_config())
            .await
            .expect("password-only config should start without SMTP credentials");
    }

    #[test]
    fn test_passwordless_users_warning_none_when_no_passwordless_user() {
        assert!(passwordless_users_warning(0, true).is_none());
        assert!(passwordless_users_warning(0, false).is_none());
    }

    #[test]
    fn test_passwordless_users_warning_mentions_count_and_remedy_when_email_enabled() {
        let warning = passwordless_users_warning(3, true).expect("expected a warning");

        assert!(warning.contains('3'));
        assert!(warning.contains("email-based"));
        assert!(warning.contains("set a password"));
    }

    #[test]
    fn test_passwordless_users_warning_reports_locked_out_when_email_disabled() {
        let warning = passwordless_users_warning(2, false).expect("expected a warning");

        assert!(warning.contains('2'));
        assert!(warning.contains("cannot log in"));
    }
}
