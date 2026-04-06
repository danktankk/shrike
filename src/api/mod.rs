// src/api/mod.rs
pub mod search_terms;
pub mod sources;
pub mod matches;
pub mod notifications;

use std::sync::Arc;
use axum::{
    Router,
    routing::{get, post, put},
};
use sqlx::SqlitePool;
use crate::config::Config;
use crate::notifier::Notifier;
use crate::assets::static_handler;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub notifier: Arc<Notifier>,
    pub http: reqwest::Client,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        // Search terms
        .route("/api/search_terms",     get(search_terms::list).post(search_terms::create))
        .route("/api/search_terms/{id}", put(search_terms::update).delete(search_terms::delete_one))
        // Sources
        .route("/api/sources",          get(sources::list).post(sources::create))
        .route("/api/sources/{id}",      put(sources::update).delete(sources::delete_one))
        .route("/api/sources/{id}/test", post(sources::test_source))
        // Matches
        .route("/api/matches",          get(matches::list))
        // Notifications
        .route("/api/notifications/config",      get(notifications::get_config).put(notifications::put_config))
        .route("/api/notifications/test/{channel}", post(notifications::test_channel))
        // Static / SPA fallback
        .fallback(static_handler)
        .with_state(state)
}
