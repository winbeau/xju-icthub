use icthub_server::{run_import_worker, AppState, Config, ImportWorkerOptions};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("icthub_server=info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let state = AppState::new(&config).await?;
    let options = ImportWorkerOptions::new(
        config.import_worker_poll_ms,
        config.import_worker_lease_secs,
    );
    run_import_worker(state, options).await?;
    Ok(())
}
