use std::{env, fs, path::Path};

use anyhow::Context;

// Base config keys
const SERVER_PORT_KEY: &str = "SERVER_PORT";
const ALLOW_ORIGIN_KEY: &str = "ALLOW_ORIGIN";

// Single user related keys
const SINGLE_USER_PASSWORD_KEY: &str = "ACTIVITIES_SINGLE_USER_PASSWORD";

// Multi user related keys
const MULTI_USER_MAILER_FROM_KEY: &str = "ACTIVITIES_MAILER_FROM";
const MULTI_USER_MAILER_USERNAME_KEY: &str = "ACTIVITIES_MAILER_USERNAME";
const MULTI_USER_MAILER_PASSWORD_KEY: &str = "ACTIVITIES_MAILER_PASSWORD";
const MULTI_USER_MAILER_RELAY_KEY: &str = "ACTIVITIES_MAILER_RELAY";
const MULTI_USER_MAILER_DOMAIN_KEY: &str = "ACTIVITIES_MAILER_DOMAIN";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub server_port: String,
    pub allow_origin: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Config> {
        let server_port = load_env(SERVER_PORT_KEY)?;
        let allow_origin = load_env(ALLOW_ORIGIN_KEY)?;

        Ok(Config {
            server_port,
            allow_origin,
        })
    }
}

#[derive(Debug, Clone)]
pub enum AppMode {
    SingleUser(SingleUserConfig),
    MultiUser(MultiUserConfig),
}

impl AppMode {
    pub fn try_from_env() -> Result<AppMode, String> {
        match MultiUserConfig::try_from_env() {
            Ok(Some(config)) => return Ok(AppMode::MultiUser(config)),
            Err(err) => return Err(err),
            Ok(None) => { /* Pass, multi-user mode is not selected */ }
        }

        Ok(AppMode::SingleUser(SingleUserConfig::try_from_env()?))
    }
}

#[derive(Debug, Clone)]
pub struct SingleUserConfig {
    pub password: Option<String>,
}

impl SingleUserConfig {
    pub fn try_from_env() -> Result<SingleUserConfig, String> {
        Ok(SingleUserConfig {
            password: load_optional_env(&SINGLE_USER_PASSWORD_KEY).map_err(|err| {
                format!("Failed to load env {}: {}", SINGLE_USER_PASSWORD_KEY, err)
            })?,
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
    pub fn try_from_env() -> Result<Option<MultiUserConfig>, String> {
        Ok(Some(MultiUserConfig {
            mailer_from: load_env(&MULTI_USER_MAILER_FROM_KEY).map_err(|err| {
                format!("Failed to load env {}: {}", MULTI_USER_MAILER_FROM_KEY, err)
            })?,
            mailer_username: load_env(&MULTI_USER_MAILER_USERNAME_KEY).map_err(|err| {
                format!(
                    "Failed to load env {}: {}",
                    MULTI_USER_MAILER_USERNAME_KEY, err
                )
            })?,
            mailer_password: load_env(&MULTI_USER_MAILER_PASSWORD_KEY).map_err(|err| {
                format!(
                    "Failed to load env {}: {}",
                    MULTI_USER_MAILER_PASSWORD_KEY, err
                )
            })?,
            mailer_relay: load_env(&MULTI_USER_MAILER_RELAY_KEY).map_err(|err| {
                format!(
                    "Failed to load env {}: {}",
                    MULTI_USER_MAILER_RELAY_KEY, err
                )
            })?,
            mailer_domain: load_env(&MULTI_USER_MAILER_DOMAIN_KEY).map_err(|err| {
                format!(
                    "Failed to load env {}: {}",
                    MULTI_USER_MAILER_DOMAIN_KEY, err
                )
            })?,
        }))
    }
}

pub fn key_is_set(key: &str) -> bool {
    // First check the env as a path to a file containing the env value
    if let Ok(path) = env::var(format!("{key}_FILE")) {
        return Path::new(&path).exists();
    };

    // Else check if the key is set
    env::var(key).is_ok()
}
// TODO: introduce a GetFromEnv trait to remove the ddep on env::var to allow testing in isolation
/// Optionnaly load a value from environnemnt variable. See [load_env] for more details.
pub fn load_optional_env(key: &str) -> Result<Option<String>, String> {
    // First check the env as a path to a file containing the env value
    if let Ok(path) = env::var(format!("{key}_FILE")) {
        let path = Path::new(&path);

        if let Ok(content) = fs::read(path) {
            return match String::from_utf8(content) {
                Ok(value) => Ok(Some(value)),
                _ => Err(format!("File content of {key}_FILE is invalid")),
            };
        };
    };

    // Else try to load the content directly from the env
    match env::var(key) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        _err => Err(format!("failed to load environment variable {}", key)),
    }
}

/// Load a value from environment variable. First checks if the environment variable `key_FILE`
/// points to a file that can be loaded into a String (e.g. for Docker secrets), else tries to read
/// the content of environment variable `key` directly. If `key_FILE` points to a valid file, but
/// the file content is not a valid UTF8 string, the function returns an err.
pub fn load_env(key: &str) -> anyhow::Result<String> {
    // First check the env as a path to a file containing the env value
    if let Ok(path) = env::var(format!("{key}_FILE")) {
        let path = Path::new(&path);

        if let Ok(content) = fs::read(path) {
            return String::from_utf8(content)
                .with_context(|| format!("File content of {key}_FILE is invalid"));
        };
    };

    // Else try to load the content directly from the env
    env::var(key).with_context(|| format!("failed to load environment variable {}", key))
}
