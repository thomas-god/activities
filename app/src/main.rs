#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if cfg!(feature = "single-user") {
        app::bootstrap::single_user::bootsrap_single_user()
            .await?
            .run()
            .await
    } else if cfg!(feature = "multi-user") {
        app::bootstrap::multi_user::bootsrap_multi_user()
            .await?
            .run()
            .await
    } else {
        app::bootstrap::demo::bootsrap_demo().await?.run().await
    }
}
