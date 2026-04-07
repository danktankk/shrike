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

use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let config = Arc::new(config::Config::from_env());
    info!("Connecting to database: {}", config.database_url);

    let pool = db::init_pool(&config.database_url).await?;
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("discoprowl/2.0")
        .danger_accept_invalid_certs(true)
        .build()?;

    let notifier = Arc::new(notifier::Notifier::new(config.clone(), http.clone()));

    // Spawn background scheduler
    let sched_pool = pool.clone();
    let sched_notifier = notifier.clone();
    let sched_http = http.clone();
    tokio::spawn(async move {
        scheduler::run(sched_pool, sched_notifier, sched_http).await;
        tracing::error!("Scheduler task exited unexpectedly — terminating process so container can restart");
        std::process::exit(1);
    });

    let state = api::AppState {
        pool,
        config: config.clone(),
        notifier,
        http,
    };

    let app = api::router(state);
    let addr: std::net::SocketAddr = config.bind_addr.parse()
        .map_err(|e| anyhow::anyhow!("Invalid BIND_ADDR '{}': {e}", config.bind_addr))?;
    info!("Listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
