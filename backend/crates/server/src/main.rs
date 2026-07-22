use icthub_server::{build_router, AppState, Config};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("icthub_server=info,tower_http=info")
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let state = AppState::new(&config).await?;
    let app = build_router(state);
    let listener = TcpListener::bind(config.bind_addr).await?;

    tracing::info!(address = %config.bind_addr, "ICTHub API listening");
    axum::serve(listener, app).await?;
    Ok(())
}
