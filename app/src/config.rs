use std::env;

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

fn load_env(key: &str) -> anyhow::Result<String> {
    env::var(key).with_context(|| format!("failed to load environment variable {}", key))
}
