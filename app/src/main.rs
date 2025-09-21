#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let http_server = if cfg!(feature = "single-user") {
        app::bootstrap::single_user::bootsrap_single_user().await?
    } else if cfg!(feature = "multi-user") {
        app::bootstrap::multi_user::bootsrap_multi_user().await?
    } else {
        app::bootstrap::demo::bootsrap_demo().await?
    };

    http_server.run().await
}
