use app::bootstrap::demo::bootsrap_demo;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let http_server = bootsrap_demo().await?;

    http_server.run().await
}
