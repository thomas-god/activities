use std::{
    env::{self},
    fs,
    path::Path,
};

// Base config keys
const SERVER_PORT_KEY: &str = "SERVER_PORT";
const ALLOW_ORIGIN_KEY: &str = "ALLOW_ORIGIN";
const ACTIVITIES_DATA_PATH_KEY: &str = "ACTIVITIES_DATA_PATH";

// Single user related keys
const SINGLE_USER_PASSWORD_KEY: &str = "ACTIVITIES_SINGLE_USER_PASSWORD";

// Multi user related keys
const MULTI_USER_MAILER_FROM_KEY: &str = "ACTIVITIES_MAILER_FROM";
const MULTI_USER_MAILER_USERNAME_KEY: &str = "ACTIVITIES_MAILER_USERNAME";
const MULTI_USER_MAILER_PASSWORD_KEY: &str = "ACTIVITIES_MAILER_PASSWORD";
const MULTI_USER_MAILER_RELAY_KEY: &str = "ACTIVITIES_MAILER_RELAY";
const MULTI_USER_MAILER_DOMAIN_KEY: &str = "ACTIVITIES_MAILER_DOMAIN";
const MULTI_USER_ALLOW_REGISTRATION_KEY: &str = "ACTIVITIES_ALLOW_REGISTRATION";
const MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY: &str = "ACTIVITIES_BOOTSTRAP_ADMIN_EMAIL";
const MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY: &str = "ACTIVITIES_BOOTSTRAP_ADMIN_PASSWORD";

// Telemetry related keys. Named after the standard OpenTelemetry env var conventions `OTEL_`
// (rather than prefixed with ACTIVITIES_) so they stay interoperable with collectors/dashboards
// that expect them.
const OTEL_EXPORTER_OTLP_ENDPOINT_KEY: &str = "OTEL_EXPORTER_OTLP_ENDPOINT";
const OTEL_SERVICE_NAME_KEY: &str = "OTEL_SERVICE_NAME";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseConfig {
    pub server_port: String,
    pub allow_origin: String,
    pub activities_data_path: String,
}

/// Reads only the data path from the environment.
///
/// Exposed for maintenance commands (e.g. the admin CLI) that need to locate the databases without
/// providing the full server configuration.
pub fn data_path_from_env<T: Environment>(env: &T) -> Result<String, String> {
    load_env(env, ACTIVITIES_DATA_PATH_KEY)
        .as_string()
        .ok_or_else(|| format!("Invalid or missing {ACTIVITIES_DATA_PATH_KEY}"))
}

impl BaseConfig {
    pub fn from_env<T: Environment>(env: &T) -> Result<BaseConfig, String> {
        let server_port = load_env(env, SERVER_PORT_KEY)
            .as_string()
            .ok_or_else(|| format!("Invalid or missing {SERVER_PORT_KEY}"))?;
        let allow_origin = load_env(env, ALLOW_ORIGIN_KEY)
            .as_string()
            .ok_or_else(|| format!("Invalid or missing {ALLOW_ORIGIN_KEY}"))?;
        let activities_data_path = data_path_from_env(env)?;

        Ok(BaseConfig {
            server_port,
            allow_origin,
            activities_data_path,
        })
    }
}

#[derive(Debug, Clone)]
pub enum AppMode {
    SingleUser(SingleUserConfig),
    MultiUser(MultiUserConfig),
}

impl AppMode {
    pub fn try_from_env<T: Environment>(env: &T) -> Result<AppMode, String> {
        if is_single_user_mode_selected_legacy(env) {
            // Publicly deprecated, but single-user will still be used for the demo version
            tracing::warn!(
                "[DEPRECATED] Activities will start in multi-user mode by default in a future version. You can migrate now using the guide in the readme."
            );
            return Ok(AppMode::SingleUser(SingleUserConfig::try_from_env(env)?));
        }

        Ok(AppMode::MultiUser(MultiUserConfig::try_from_env(env)?))
    }
}

/// Legacy way to check for single-user mode.
/// Multi-user mode was selected as soon as any email-based or password-based authentication
/// environment variable was set. When none of them were set, the app started in single-user mode.
fn is_single_user_mode_selected_legacy<T: Environment>(env: &T) -> bool {
    [
        MULTI_USER_MAILER_FROM_KEY,
        MULTI_USER_MAILER_USERNAME_KEY,
        MULTI_USER_MAILER_PASSWORD_KEY,
        MULTI_USER_MAILER_RELAY_KEY,
        MULTI_USER_MAILER_DOMAIN_KEY,
        MULTI_USER_ALLOW_REGISTRATION_KEY,
        MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY,
        MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY,
    ]
    .iter()
    .all(|key| load_env(env, key).as_string().is_none())
}

#[derive(Debug, Clone)]
pub struct SingleUserConfig {
    pub password: Option<String>,
}

impl SingleUserConfig {
    pub fn try_from_env<T: Environment>(env: &T) -> Result<SingleUserConfig, String> {
        Ok(SingleUserConfig {
            password: load_env(env, SINGLE_USER_PASSWORD_KEY).as_string(),
        })
    }
}

/// Email-based (SMTP mailer) authentication configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailAuthConfig {
    pub from: String,
    pub username: String,
    pub password: String,
    pub relay: String,
    pub domain: String,
}

impl EmailAuthConfig {
    pub fn try_from_env<T: Environment>(env: &T) -> Result<Option<EmailAuthConfig>, String> {
        let from = load_env(env, MULTI_USER_MAILER_FROM_KEY).as_string();
        let username = load_env(env, MULTI_USER_MAILER_USERNAME_KEY).as_string();
        let password = load_env(env, MULTI_USER_MAILER_PASSWORD_KEY).as_string();
        let relay = load_env(env, MULTI_USER_MAILER_RELAY_KEY).as_string();
        let domain = load_env(env, MULTI_USER_MAILER_DOMAIN_KEY).as_string();

        match [from, username, password, relay, domain] {
            [None, None, None, None, None] => Ok(None),
            [
                Some(from),
                Some(username),
                Some(password),
                Some(relay),
                Some(domain),
            ] => Ok(Some(EmailAuthConfig {
                from,
                username,
                password,
                relay,
                domain,
            })),
            [from, username, password, relay, domain] => {
                let mut errs = Vec::new();
                if from.is_none() {
                    errs.push(MULTI_USER_MAILER_FROM_KEY);
                }
                if username.is_none() {
                    errs.push(MULTI_USER_MAILER_USERNAME_KEY);
                }
                if password.is_none() {
                    errs.push(MULTI_USER_MAILER_PASSWORD_KEY);
                }
                if relay.is_none() {
                    errs.push(MULTI_USER_MAILER_RELAY_KEY);
                }
                if domain.is_none() {
                    errs.push(MULTI_USER_MAILER_DOMAIN_KEY);
                }

                Err(format!(
                    "Invalid email configuration, missing or invalid environment variables: {}",
                    errs.join(", ")
                ))
            }
        }
    }
}

/// First admin bootstrap configuration.
///
/// Email and password are configured together: requiring a password guarantees the bootstrapped
/// admin can log in immediately in every authentication configuration. The `_FILE` variant of the
/// password variable is handled by [`load_env`] (e.g. for Docker secrets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminBootstrapConfig {
    pub email: String,
    pub password: String,
}

impl AdminBootstrapConfig {
    pub fn try_from_env<T: Environment>(env: &T) -> Result<Option<AdminBootstrapConfig>, String> {
        let email = load_env(env, MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY).as_string();
        let password = load_env(env, MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY).as_string();

        // An empty environment variable is reported as set: treat it as absent.
        let email = email.filter(|value| !value.is_empty());
        let password = password.filter(|value| !value.is_empty());

        match (email, password) {
            (None, None) => Ok(None),
            (Some(email), Some(password)) => Ok(Some(AdminBootstrapConfig { email, password })),
            (email, password) => {
                let mut errs = Vec::new();
                if email.is_none() {
                    errs.push(MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY);
                }
                if password.is_none() {
                    errs.push(MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY);
                }
                Err(format!(
                    "Invalid admin bootstrap configuration, missing or empty environment variables: {}",
                    errs.join(", ")
                ))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultiUserConfig {
    pub email: Option<EmailAuthConfig>,
    pub allow_registration: bool,
    pub admin_bootstrap: Option<AdminBootstrapConfig>,
}
impl MultiUserConfig {
    pub fn try_from_env<T: Environment>(env: &T) -> Result<MultiUserConfig, String> {
        let allow_registration = load_env(env, MULTI_USER_ALLOW_REGISTRATION_KEY).as_string();

        let allow_registration = match allow_registration {
            None => true,
            Some(value) => match value.to_ascii_lowercase().as_str() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => {
                    return Err(format!(
                        "Invalid value for {MULTI_USER_ALLOW_REGISTRATION_KEY}: expected `true` or `false`, got `{value}`"
                    ));
                }
            },
        };

        let email = EmailAuthConfig::try_from_env(env)?;
        let admin_bootstrap = AdminBootstrapConfig::try_from_env(env)?;

        Ok(MultiUserConfig {
            email,
            allow_registration,
            admin_bootstrap,
        })
    }
}

/// Telemetry (tracing/OpenTelemetry) configuration. Every field is optional: with nothing set,
/// the app just logs to stdout and no OTLP exporter is started (see `crate::telemetry`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TelemetryConfig {
    pub otlp_endpoint: Option<String>,
    pub service_name: Option<String>,
}

impl TelemetryConfig {
    pub fn from_env<T: Environment>(env: &T) -> TelemetryConfig {
        TelemetryConfig {
            otlp_endpoint: load_env(env, OTEL_EXPORTER_OTLP_ENDPOINT_KEY).as_string(),
            service_name: load_env(env, OTEL_SERVICE_NAME_KEY).as_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum EnvironmentVariable {
    #[default]
    NotSetOrInvalid,
    Set(String),
}

impl EnvironmentVariable {
    pub fn as_string(self) -> Option<String> {
        match self {
            Self::NotSetOrInvalid => None,
            Self::Set(value) => Some(value),
        }
    }
}

pub trait Environment {
    fn get_var(&self, key: &str) -> EnvironmentVariable;
}

#[derive(Debug, Clone)]
pub struct StdEnvironment {}
impl Environment for StdEnvironment {
    fn get_var(&self, key: &str) -> EnvironmentVariable {
        match env::var(key) {
            Ok(value) => EnvironmentVariable::Set(value),
            Err(_) => EnvironmentVariable::NotSetOrInvalid,
        }
    }
}

/// Load a value from environment variable. First checks if the environment variable `key_FILE`
/// points to a file that can be parsed into a String (e.g. for Docker secrets), else tries to read
/// the content of environment variable `key` directly. If `key_FILE` points to a valid file, but
/// the file content is not a valid UTF8 string, the function returns an [EnvironmentVariable::NotSetOrInvalid].
fn load_env<T: Environment>(env: &T, key: &str) -> EnvironmentVariable {
    if let EnvironmentVariable::Set(path) = env.get_var(&format!("{key}_FILE")) {
        let path = Path::new(&path);

        let Ok(content) = fs::read(path) else {
            return EnvironmentVariable::NotSetOrInvalid;
        };

        return match String::from_utf8(content) {
            Ok(value) => EnvironmentVariable::Set(value),
            _ => EnvironmentVariable::NotSetOrInvalid,
        };
    };

    env.get_var(key)
}

#[cfg(test)]
mod test_config {
    use std::{collections::HashMap, io::Write};

    use super::*;

    #[test]
    fn test_std_environment() {
        let env = StdEnvironment {};

        std::assert_matches!(
            env.get_var("unset-key"),
            EnvironmentVariable::NotSetOrInvalid
        );

        unsafe {
            std::env::set_var("test-key", "test-value");
        }
        std::assert_matches!(env.get_var("test-key"), EnvironmentVariable::Set(_));
    }

    #[derive(Debug, Clone, Default)]
    pub struct MockEnvironment {
        env: HashMap<String, EnvironmentVariable>,
    }

    impl Environment for MockEnvironment {
        fn get_var(&self, key: &str) -> EnvironmentVariable {
            self.env.get(key).cloned().unwrap_or_default()
        }
    }

    impl MockEnvironment {
        fn set_var(&mut self, key: &str, value: EnvironmentVariable) {
            let _ = self.env.insert(key.to_string(), value);
        }
    }

    #[test]
    fn test_load_env_from_file_ok() {
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        write!(tmpfile, "test-value-in-file").unwrap();
        let mut env = MockEnvironment::default();
        env.set_var(
            "test-key_FILE",
            EnvironmentVariable::Set(tmpfile.path().to_str().unwrap().to_string()),
        );

        assert_eq!(
            load_env(&env, "test-key"),
            EnvironmentVariable::Set("test-value-in-file".to_string())
        )
    }

    #[test]
    fn test_load_env_from_file_invalid_content() {
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        tmpfile.write_all(&[0x80]).unwrap();
        let mut env = MockEnvironment::default();
        env.set_var(
            "test-key_FILE",
            EnvironmentVariable::Set(tmpfile.path().to_str().unwrap().to_string()),
        );

        assert_eq!(
            load_env(&env, "test-key"),
            EnvironmentVariable::NotSetOrInvalid
        )
    }

    #[test]
    fn test_load_env_directly_ok() {
        let mut env = MockEnvironment::default();
        env.set_var("test-key", EnvironmentVariable::Set("value".to_string()));

        assert_eq!(
            load_env(&env, "test-key"),
            EnvironmentVariable::Set("value".to_string())
        )
    }

    #[test]
    fn test_load_env_not_set() {
        let env = MockEnvironment::default();

        assert_eq!(
            load_env(&env, "test-key"),
            EnvironmentVariable::NotSetOrInvalid
        )
    }

    #[test]
    fn test_base_config_from_env_ok() {
        let mut env = MockEnvironment::default();
        env.set_var(
            SERVER_PORT_KEY,
            EnvironmentVariable::Set("3000".to_string()),
        );
        env.set_var(
            ALLOW_ORIGIN_KEY,
            EnvironmentVariable::Set("http://localhost:5173".to_string()),
        );
        env.set_var(
            ACTIVITIES_DATA_PATH_KEY,
            EnvironmentVariable::Set("/tmp/activities".to_string()),
        );

        assert_eq!(
            BaseConfig::from_env(&env).unwrap(),
            BaseConfig {
                server_port: "3000".to_string(),
                allow_origin: "http://localhost:5173".to_string(),
                activities_data_path: "/tmp/activities".to_string(),
            }
        );
    }

    #[test]
    fn test_base_config_from_env_missing_required_value() {
        let mut env = MockEnvironment::default();
        env.set_var(
            SERVER_PORT_KEY,
            EnvironmentVariable::Set("3000".to_string()),
        );
        env.set_var(
            ACTIVITIES_DATA_PATH_KEY,
            EnvironmentVariable::Set("/tmp/activities".to_string()),
        );

        assert_eq!(
            BaseConfig::from_env(&env),
            Err(format!("Invalid or missing {ALLOW_ORIGIN_KEY}"))
        );
    }

    #[test]
    fn test_single_user_config_try_from_env_with_password() {
        let mut env = MockEnvironment::default();
        env.set_var(
            SINGLE_USER_PASSWORD_KEY,
            EnvironmentVariable::Set("secret".to_string()),
        );

        assert_eq!(
            SingleUserConfig::try_from_env(&env).unwrap().password,
            Some("secret".to_string())
        );
    }

    #[test]
    fn test_single_user_config_try_from_env_without_password() {
        let env = MockEnvironment::default();

        assert_eq!(SingleUserConfig::try_from_env(&env).unwrap().password, None);
    }

    #[test]
    fn test_is_single_user_mode_selected_legacy_when_no_selector_set() {
        let env = MockEnvironment::default();

        assert!(is_single_user_mode_selected_legacy(&env));
    }

    #[test]
    fn test_is_single_user_mode_selected_legacy_when_any_selector_set() {
        for key in [
            MULTI_USER_MAILER_FROM_KEY,
            MULTI_USER_MAILER_USERNAME_KEY,
            MULTI_USER_MAILER_PASSWORD_KEY,
            MULTI_USER_MAILER_RELAY_KEY,
            MULTI_USER_MAILER_DOMAIN_KEY,
            MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY,
            MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY,
        ] {
            let mut env = MockEnvironment::default();
            env.set_var(key, EnvironmentVariable::Set("value".to_string()));

            assert!(
                !is_single_user_mode_selected_legacy(&env),
                "expected {key} to select multi-user mode"
            );
        }
    }

    #[test]
    fn test_is_single_user_mode_selected_legacy_when_all_selectors_set() {
        let mut env = MockEnvironment::default();
        set_full_multi_user_env(&mut env);
        env.set_var(
            MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY,
            EnvironmentVariable::Set("admin@example.com".to_string()),
        );
        env.set_var(
            MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY,
            EnvironmentVariable::Set("s3cret".to_string()),
        );

        assert!(!is_single_user_mode_selected_legacy(&env));
    }

    #[test]
    fn test_is_single_user_mode_selected_legacy_empty_value_selects_multi_user() {
        let mut env = MockEnvironment::default();
        env.set_var(
            MULTI_USER_ALLOW_REGISTRATION_KEY,
            EnvironmentVariable::Set(String::new()),
        );

        assert!(!is_single_user_mode_selected_legacy(&env));
    }

    #[test]
    fn test_is_single_user_mode_selected_legacy_reads_selector_from_file() {
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        write!(tmpfile, "true").unwrap();
        let mut env = MockEnvironment::default();
        env.set_var(
            &format!("{MULTI_USER_ALLOW_REGISTRATION_KEY}_FILE"),
            EnvironmentVariable::Set(tmpfile.path().to_str().unwrap().to_string()),
        );

        assert!(!is_single_user_mode_selected_legacy(&env));
    }

    #[test]
    fn test_multi_user_config_try_from_env_without_email_is_password_only() {
        let env = MockEnvironment::default();

        let config = MultiUserConfig::try_from_env(&env).unwrap();

        assert!(config.email.is_none());
        assert!(config.allow_registration);
    }

    #[test]
    fn test_app_mode_try_from_env_multi_user_password_only() {
        let mut env = MockEnvironment::default();
        env.set_var(
            MULTI_USER_ALLOW_REGISTRATION_KEY,
            EnvironmentVariable::Set("true".to_string()),
        );

        match AppMode::try_from_env(&env).unwrap() {
            AppMode::MultiUser(config) => assert!(config.email.is_none()),
            AppMode::SingleUser(_) => panic!("expected multi-user mode"),
        }
    }

    #[test]
    fn test_multi_user_config_try_from_env_ok_when_all_values_set() {
        let mut env = MockEnvironment::default();
        env.set_var(
            MULTI_USER_MAILER_FROM_KEY,
            EnvironmentVariable::Set("noreply@example.com".to_string()),
        );
        env.set_var(
            MULTI_USER_MAILER_USERNAME_KEY,
            EnvironmentVariable::Set("mailer-user".to_string()),
        );
        env.set_var(
            MULTI_USER_MAILER_PASSWORD_KEY,
            EnvironmentVariable::Set("mailer-password".to_string()),
        );
        env.set_var(
            MULTI_USER_MAILER_RELAY_KEY,
            EnvironmentVariable::Set("smtp.example.com".to_string()),
        );
        env.set_var(
            MULTI_USER_MAILER_DOMAIN_KEY,
            EnvironmentVariable::Set("example.com".to_string()),
        );

        let config = MultiUserConfig::try_from_env(&env).unwrap();

        let email = config.email.as_ref().unwrap();
        assert_eq!(email.from, "noreply@example.com");
        assert_eq!(email.username, "mailer-user");
        assert_eq!(email.password, "mailer-password");
        assert_eq!(email.relay, "smtp.example.com");
        assert_eq!(email.domain, "example.com");
        assert!(config.allow_registration);
    }

    fn set_full_multi_user_env(env: &mut MockEnvironment) {
        env.set_var(
            MULTI_USER_MAILER_FROM_KEY,
            EnvironmentVariable::Set("noreply@example.com".to_string()),
        );
        env.set_var(
            MULTI_USER_MAILER_USERNAME_KEY,
            EnvironmentVariable::Set("mailer-user".to_string()),
        );
        env.set_var(
            MULTI_USER_MAILER_PASSWORD_KEY,
            EnvironmentVariable::Set("mailer-password".to_string()),
        );
        env.set_var(
            MULTI_USER_MAILER_RELAY_KEY,
            EnvironmentVariable::Set("smtp.example.com".to_string()),
        );
        env.set_var(
            MULTI_USER_MAILER_DOMAIN_KEY,
            EnvironmentVariable::Set("example.com".to_string()),
        );
    }

    #[test]
    fn test_multi_user_allow_registration_defaults_to_true() {
        let mut env = MockEnvironment::default();
        set_full_multi_user_env(&mut env);

        let config = MultiUserConfig::try_from_env(&env).unwrap();

        assert!(config.allow_registration);
    }

    #[test]
    fn test_multi_user_allow_registration_false() {
        let mut env = MockEnvironment::default();
        set_full_multi_user_env(&mut env);
        env.set_var(
            MULTI_USER_ALLOW_REGISTRATION_KEY,
            EnvironmentVariable::Set("false".to_string()),
        );

        let config = MultiUserConfig::try_from_env(&env).unwrap();

        assert!(!config.allow_registration);
    }

    #[test]
    fn test_multi_user_allow_registration_accepts_boolean_variants() {
        for (value, expected) in [
            ("true", true),
            ("TRUE", true),
            ("1", true),
            ("false", false),
            ("False", false),
            ("0", false),
        ] {
            let mut env = MockEnvironment::default();
            set_full_multi_user_env(&mut env);
            env.set_var(
                MULTI_USER_ALLOW_REGISTRATION_KEY,
                EnvironmentVariable::Set(value.to_string()),
            );

            let config = MultiUserConfig::try_from_env(&env).unwrap();

            assert_eq!(
                config.allow_registration, expected,
                "unexpected value for `{value}`"
            );
        }
    }

    #[test]
    fn test_multi_user_allow_registration_invalid_value_errors() {
        let mut env = MockEnvironment::default();
        set_full_multi_user_env(&mut env);
        env.set_var(
            MULTI_USER_ALLOW_REGISTRATION_KEY,
            EnvironmentVariable::Set("banana".to_string()),
        );

        let err = MultiUserConfig::try_from_env(&env).unwrap_err();

        assert!(err.contains(MULTI_USER_ALLOW_REGISTRATION_KEY));
        assert!(err.contains("banana"));
    }

    #[test]
    fn test_multi_user_config_try_from_env_err_when_partial_values_set() {
        let mut env = MockEnvironment::default();
        env.set_var(
            MULTI_USER_MAILER_FROM_KEY,
            EnvironmentVariable::Set("noreply@example.com".to_string()),
        );

        assert!(MultiUserConfig::try_from_env(&env).is_err());
    }

    #[test]
    fn test_admin_bootstrap_defaults_to_none() {
        let env = MockEnvironment::default();

        assert!(
            MultiUserConfig::try_from_env(&env)
                .unwrap()
                .admin_bootstrap
                .is_none()
        );
    }

    #[test]
    fn test_admin_bootstrap_reads_email_and_password() {
        let mut env = MockEnvironment::default();
        env.set_var(
            MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY,
            EnvironmentVariable::Set("admin@example.com".to_string()),
        );
        env.set_var(
            MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY,
            EnvironmentVariable::Set("s3cret".to_string()),
        );

        let config = MultiUserConfig::try_from_env(&env)
            .unwrap()
            .admin_bootstrap
            .unwrap();

        assert_eq!(config.email, "admin@example.com");
        assert_eq!(config.password, "s3cret");
    }

    #[test]
    fn test_admin_bootstrap_reads_password_from_file() {
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        write!(tmpfile, "secret-from-file").unwrap();
        let mut env = MockEnvironment::default();
        env.set_var(
            MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY,
            EnvironmentVariable::Set("admin@example.com".to_string()),
        );
        env.set_var(
            &format!("{MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY}_FILE"),
            EnvironmentVariable::Set(tmpfile.path().to_str().unwrap().to_string()),
        );

        let config = MultiUserConfig::try_from_env(&env)
            .unwrap()
            .admin_bootstrap
            .unwrap();

        assert_eq!(config.password, "secret-from-file");
    }

    #[test]
    fn test_admin_bootstrap_email_without_password_errors() {
        let mut env = MockEnvironment::default();
        env.set_var(
            MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY,
            EnvironmentVariable::Set("admin@example.com".to_string()),
        );

        let err = MultiUserConfig::try_from_env(&env).unwrap_err();

        assert!(err.contains(MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY));
    }

    #[test]
    fn test_admin_bootstrap_password_without_email_errors() {
        let mut env = MockEnvironment::default();
        env.set_var(
            MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY,
            EnvironmentVariable::Set("s3cret".to_string()),
        );

        let err = MultiUserConfig::try_from_env(&env).unwrap_err();

        assert!(err.contains(MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY));
    }

    #[test]
    fn test_admin_bootstrap_empty_password_errors() {
        let mut env = MockEnvironment::default();
        env.set_var(
            MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY,
            EnvironmentVariable::Set("admin@example.com".to_string()),
        );
        env.set_var(
            MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY,
            EnvironmentVariable::Set(String::new()),
        );

        let err = MultiUserConfig::try_from_env(&env).unwrap_err();

        assert!(err.contains(MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY));
    }

    #[test]
    fn test_admin_bootstrap_selects_multi_user_mode() {
        let mut env = MockEnvironment::default();
        env.set_var(
            MULTI_USER_BOOTSTRAP_ADMIN_EMAIL_KEY,
            EnvironmentVariable::Set("admin@example.com".to_string()),
        );
        env.set_var(
            MULTI_USER_BOOTSTRAP_ADMIN_PASSWORD_KEY,
            EnvironmentVariable::Set("s3cret".to_string()),
        );

        match AppMode::try_from_env(&env).unwrap() {
            AppMode::MultiUser(config) => assert!(config.admin_bootstrap.is_some()),
            AppMode::SingleUser(_) => panic!("expected multi-user mode"),
        }
    }

    #[test]
    fn test_app_mode_try_from_env_multi_user_when_selectors_set() {
        let mut env = MockEnvironment::default();
        env.set_var(
            SINGLE_USER_PASSWORD_KEY,
            EnvironmentVariable::Set("single-user-secret".to_string()),
        );
        set_full_multi_user_env(&mut env);

        let mode = AppMode::try_from_env(&env).unwrap();

        assert!(matches!(mode, AppMode::MultiUser(_)));
    }

    #[test]
    fn test_app_mode_try_from_env_legacy_falls_back_to_single_user() {
        let mut env = MockEnvironment::default();
        env.set_var(
            SINGLE_USER_PASSWORD_KEY,
            EnvironmentVariable::Set("single-user-secret".to_string()),
        );

        let mode = AppMode::try_from_env(&env).unwrap();

        match mode {
            AppMode::SingleUser(config) => {
                assert_eq!(config.password, Some("single-user-secret".to_string()));
            }
            AppMode::MultiUser(_) => panic!("expected single-user mode"),
        }
    }

    #[test]
    fn test_app_mode_try_from_env_returns_multi_user_error() {
        let mut env = MockEnvironment::default();
        env.set_var(
            MULTI_USER_MAILER_FROM_KEY,
            EnvironmentVariable::Set("noreply@example.com".to_string()),
        );

        assert!(AppMode::try_from_env(&env).is_err());
    }

    #[test]
    fn test_telemetry_config_from_env_with_values() {
        let mut env = MockEnvironment::default();
        env.set_var(
            OTEL_EXPORTER_OTLP_ENDPOINT_KEY,
            EnvironmentVariable::Set("http://otel-collector:4317".to_string()),
        );
        env.set_var(
            OTEL_SERVICE_NAME_KEY,
            EnvironmentVariable::Set("activities-app".to_string()),
        );

        assert_eq!(
            TelemetryConfig::from_env(&env),
            TelemetryConfig {
                otlp_endpoint: Some("http://otel-collector:4317".to_string()),
                service_name: Some("activities-app".to_string()),
            }
        );
    }

    #[test]
    fn test_telemetry_config_from_env_defaults_to_none() {
        let env = MockEnvironment::default();

        assert_eq!(
            TelemetryConfig::from_env(&env),
            TelemetryConfig {
                otlp_endpoint: None,
                service_name: None,
            }
        );
    }
}
