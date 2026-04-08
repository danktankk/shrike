// src/main.rs
mod config;
mod db;
mod models;
mod matcher;
mod scheduler;
mod sources;
mod notifier;
mod steamgriddb;
mod enrichment;
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

    // Two HTTP clients with different TLS policies.
    // Internal: indexers (Prowlarr/Torznab/Newznab/RSS) commonly sit behind
    // self-signed certs on the LAN, so we unconditionally accept invalid certs.
    // External: third-party services (Discord/Pushover/Apprise/SteamGridDB)
    // get full TLS verification — never relaxed.
    let internal_insecure = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("discoprowl/2.0")
        .danger_accept_invalid_certs(true)
        .build()?;
    let external_strict = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("discoprowl/2.0")
        .build()?;
    let http = api::HttpClients { internal_insecure, external_strict };

    let notifier = Arc::new(notifier::Notifier::new(
        config.clone(),
        http.external_strict.clone(),
    ));

    // Spawn background scheduler — scheduler only drives source fetches,
    // so it receives the internal (insecure) client.
    let sched_pool = pool.clone();
    let sched_notifier = notifier.clone();
    let sched_http = http.internal_insecure.clone();
    let tick_secs = config.scheduler_tick_secs;
    tokio::spawn(async move {
        scheduler::run(sched_pool, sched_notifier, sched_http, tick_secs).await;
        tracing::error!("Scheduler task exited unexpectedly — terminating process so container can restart");
        std::process::exit(1);
    });

    let state = api::AppState {
        pool,
        config: config.clone(),
        notifier,
        http,
        art_cache: Arc::new(steamgriddb::ArtCache::new()),
        enrichment_cache: Arc::new(enrichment::EnrichmentCache::new()),
    };

    let app = api::router(state);
    let addr: std::net::SocketAddr = config.bind_addr.parse()
        .map_err(|e| anyhow::anyhow!("Invalid BIND_ADDR '{}': {e}", config.bind_addr))?;
    info!("Listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
