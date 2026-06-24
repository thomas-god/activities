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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseConfig {
    pub server_port: String,
    pub allow_origin: String,
    pub activities_data_path: String,
}

impl BaseConfig {
    pub fn from_env<T: Environment>(env: &T) -> Result<BaseConfig, String> {
        let server_port = load_env(env, SERVER_PORT_KEY)
            .as_string()
            .ok_or_else(|| format!("Invalid or missing {SERVER_PORT_KEY}"))?;
        let allow_origin = load_env(env, ALLOW_ORIGIN_KEY)
            .as_string()
            .ok_or_else(|| format!("Invalid or missing {ALLOW_ORIGIN_KEY}"))?;
        let activities_data_path = load_env(env, ACTIVITIES_DATA_PATH_KEY)
            .as_string()
            .ok_or_else(|| format!("Invalid or missing {ACTIVITIES_DATA_PATH_KEY}"))?;

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
        match MultiUserConfig::try_from_env(env) {
            Ok(Some(config)) => return Ok(AppMode::MultiUser(config)),
            Err(err) => return Err(err),
            Ok(None) => { /* Pass, multi-user mode is not selected */ }
        }

        Ok(AppMode::SingleUser(SingleUserConfig::try_from_env(env)?))
    }
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

#[derive(Debug, Clone)]
pub struct MultiUserConfig {
    pub mailer_from: String,
    pub mailer_username: String,
    pub mailer_password: String,
    pub mailer_relay: String,
    pub mailer_domain: String,
}
impl MultiUserConfig {
    pub fn try_from_env<T: Environment>(env: &T) -> Result<Option<MultiUserConfig>, String> {
        let mailer_from = load_env(env, MULTI_USER_MAILER_FROM_KEY).as_string();
        let mailer_username = load_env(env, MULTI_USER_MAILER_USERNAME_KEY).as_string();
        let mailer_password = load_env(env, MULTI_USER_MAILER_PASSWORD_KEY).as_string();
        let mailer_relay = load_env(env, MULTI_USER_MAILER_RELAY_KEY).as_string();
        let mailer_domain = load_env(env, MULTI_USER_MAILER_DOMAIN_KEY).as_string();

        match [
            mailer_from,
            mailer_username,
            mailer_password,
            mailer_relay,
            mailer_domain,
        ] {
            [None, None, None, None, None] => Ok(None),
            [
                Some(mailer_from),
                Some(mailer_username),
                Some(mailer_password),
                Some(mailer_relay),
                Some(mailer_domain),
            ] => Ok(Some(MultiUserConfig {
                mailer_from,
                mailer_username,
                mailer_password,
                mailer_relay,
                mailer_domain,
            })),
            [
                mailer_from,
                mailer_username,
                mailer_password,
                mailer_relay,
                mailer_domain,
            ] => {
                let mut errs = Vec::new();
                if mailer_from.is_none() {
                    errs.push(MULTI_USER_MAILER_FROM_KEY);
                }
                if mailer_username.is_none() {
                    errs.push(MULTI_USER_MAILER_USERNAME_KEY);
                }
                if mailer_password.is_none() {
                    errs.push(MULTI_USER_MAILER_PASSWORD_KEY);
                }
                if mailer_relay.is_none() {
                    errs.push(MULTI_USER_MAILER_RELAY_KEY);
                }
                if mailer_domain.is_none() {
                    errs.push(MULTI_USER_MAILER_DOMAIN_KEY);
                }

                Err(format!(
                    "Invalid multi-user configuration, missing or invalid environment variables: {}",
                    errs.join(", ")
                ))
            }
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
    fn test_multi_user_config_try_from_env_none_when_unset() {
        let env = MockEnvironment::default();

        assert_eq!(MultiUserConfig::try_from_env(&env).unwrap().is_none(), true);
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

        let config = MultiUserConfig::try_from_env(&env).unwrap().unwrap();

        assert_eq!(config.mailer_from, "noreply@example.com");
        assert_eq!(config.mailer_username, "mailer-user");
        assert_eq!(config.mailer_password, "mailer-password");
        assert_eq!(config.mailer_relay, "smtp.example.com");
        assert_eq!(config.mailer_domain, "example.com");
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
    fn test_app_mode_try_from_env_prefers_multi_user() {
        let mut env = MockEnvironment::default();
        env.set_var(
            SINGLE_USER_PASSWORD_KEY,
            EnvironmentVariable::Set("single-user-secret".to_string()),
        );
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

        let mode = AppMode::try_from_env(&env).unwrap();

        assert!(matches!(mode, AppMode::MultiUser(_)));
    }

    #[test]
    fn test_app_mode_try_from_env_falls_back_to_single_user() {
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
}
