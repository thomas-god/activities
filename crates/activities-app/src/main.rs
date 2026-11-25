use activities_app::config::{StdEnvironment, TelemetryConfig};

async fn run() -> anyhow::Result<()> {
    let (server, search_service, training_service, activity_service, backup_service) =
        activities_app::bootstrap::bootstrap_app().await?;
    let _ = tokio::join!(
        server.run(),
        search_service.run(),
        training_service.run(),
        activity_service.compute_missing_duration_curves(),
        backup_service.run()
    );
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let telemetry_config = TelemetryConfig::from_env(&StdEnvironment {});
    let telemetry = activities_app::telemetry::init(&telemetry_config)?;
    let result = run().await;
    telemetry.shutdown();
    result
}
