#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match activities_cli::parse_env() {
        Some(Ok(command)) => activities_cli::run(command).await,
        Some(Err(err)) => {
            let _ = err.print();
            std::process::exit(2);
        }
        None => {
            anyhow::bail!("expected an `admin` or `backup` command");
        }
    }
}
