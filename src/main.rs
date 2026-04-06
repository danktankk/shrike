// src/main.rs
mod config;
mod db;
mod models;
mod matcher;
mod scheduler;
mod sources;
mod notifier;
mod api;
mod assets;

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("discoprowl starting");
    Ok(())
}
