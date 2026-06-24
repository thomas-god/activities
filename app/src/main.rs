use anyhow::anyhow;
use app::config::{AppMode, StdEnvironment};

async fn run() -> anyhow::Result<()> {
    let mode = AppMode::try_from_env(&StdEnvironment {}).map_err(|err| anyhow!(err))?;
    match &mode {
        AppMode::MultiUser(config) => {
            app::bootstrap::multi_user::bootstrap_multi_user(config.clone(), mode)
                .await?
                .run()
                .await
        }
        AppMode::SingleUser(config) => {
            app::bootstrap::single_user::bootstrap_single_user(config.clone(), mode)
                .await?
                .run()
                .await
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run().await
}
