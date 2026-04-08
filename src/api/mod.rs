// src/api/mod.rs
pub mod error;
pub mod search_terms;
pub mod sources;
pub mod matches;
pub mod notifications;
pub mod scan;
pub mod art;

pub use error::AppError;

use std::sync::Arc;
use axum::{
    Router,
    routing::{get, post, put},
};
use sqlx::SqlitePool;
use crate::config::Config;
use crate::notifier::Notifier;
use crate::assets::static_handler;
use crate::steamgriddb::ArtCache;
use crate::enrichment::EnrichmentCache;

/// Typed HTTP clients with separated TLS policies.
///
/// `internal_insecure` is used for indexer traffic (Prowlarr/Torznab/Newznab/RSS),
/// which typically sits on the homelab LAN behind self-signed certs. It
/// unconditionally accepts invalid certs.
///
/// `external_strict` is used for third-party notification + metadata services
/// (Discord, Pushover, Apprise, SteamGridDB) and performs full TLS verification.
#[derive(Clone)]
pub struct HttpClients {
    pub internal_insecure: reqwest::Client,
    pub external_strict: reqwest::Client,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub notifier: Arc<Notifier>,
    pub http: HttpClients,
    pub art_cache: Arc<ArtCache>,
    pub enrichment_cache: Arc<EnrichmentCache>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        // Search terms
        .route("/api/search_terms",     get(search_terms::list).post(search_terms::create))
        .route("/api/search_terms/{id}", put(search_terms::update).delete(search_terms::delete_one))
        .route("/api/search_terms/{id}/scan", post(scan::scan_term))
        // Sources
        .route("/api/sources",          get(sources::list).post(sources::create))
        .route("/api/sources/{id}",      put(sources::update).delete(sources::delete_one))
        .route("/api/sources/{id}/test", post(sources::test_source))
        .route("/api/sources/{id}/categories", get(sources::list_categories))
        // Matches
        .route("/api/matches",          get(matches::list).delete(matches::delete_all))
        .route("/api/matches/{id}",     axum::routing::delete(matches::delete_one))
        .route("/api/match/{id}",       get(art::get_match))
        // Art (SteamGridDB proxy)
        .route("/api/art",              get(art::get_art))
        // Scan
        .route("/api/scan", post(scan::scan_now))
        // Notifications
        .route("/api/notifications/config",      get(notifications::get_config))
        .route("/api/notifications/test/{channel}", post(notifications::test_channel))
        // Static / SPA fallback
        .fallback(static_handler)
        .with_state(state)
}
