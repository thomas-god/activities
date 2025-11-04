#[cfg(feature = "multi-user")]
async fn run() -> anyhow::Result<()> {
    app::bootstrap::multi_user::bootsrap_multi_user()
        .await?
        .run()
        .await
}

#[cfg(not(feature = "multi-user"))]
async fn run() -> anyhow::Result<()> {
    app::bootstrap::single_user::bootsrap_single_user()
        .await?
        .run()
        .await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run().await
}
