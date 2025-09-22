use std::{env, fs, path::Path};

use anyhow::Context;

const SERVER_PORT_KEY: &str = "SERVER_PORT";

const ALLOW_ORIGIN_KEY: &str = "ALLOW_ORIGIN";

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
