// src/notifier/mod.rs
pub mod discord;
pub mod apprise;
pub mod pushover;

use std::sync::Arc;
use sqlx::SqlitePool;
use crate::config::Config;
use crate::models::SearchTerm;
use crate::sources::SourceItem;
use crate::steamgriddb;

pub struct Notifier {
    pub config: Arc<Config>,
    pub http: reqwest::Client,
    pub pool: SqlitePool,
}

impl Notifier {
    pub fn new(config: Arc<Config>, http: reqwest::Client, pool: SqlitePool) -> Self {
        Self { config, http, pool }
    }

    /// Look up a box art URL for the term via SteamGridDB.
    ///
    /// Resolution order:
    /// 1. If `term.steamgriddb_id` is set, fetch art directly for that id —
    ///    autocomplete is bypassed (the override is the whole point of the
    ///    field: it disambiguates year-suffix sequels).
    /// 2. Otherwise run autocomplete with the current SGDB blocklist
    ///    applied — first non-blocked hit wins.
    ///
    /// Returns `None` when no key is configured, the lookup fails, no hit
    /// matched, or the matched game has no grid/hero asset.
    async fn lookup_box_art(&self, term: &SearchTerm) -> Option<String> {
        let key = self.config.steamgriddb_api_key.as_deref()?;

        if let Some(id) = term.steamgriddb_id {
            return match steamgriddb::fetch_game_by_id(&self.http, key, id as u64).await {
                Ok(Some(g)) => g.grid_url.or(g.hero_url),
                Ok(None) => None,
                Err(e) => {
                    tracing::warn!("SteamGridDB id={id} fetch failed: {e}");
                    None
                }
            };
        }

        let blocklist = crate::blocklist::load(&self.pool).await.unwrap_or_default();
        match steamgriddb::search_game_filtered(&self.http, key, &term.query, &blocklist).await {
            Ok(Some(g)) => g.grid_url.or(g.hero_url),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!("SteamGridDB lookup failed for '{}': {e}", term.query);
                None
            }
        }
    }

    /// Send to Discord. Thin wrapper so callers (notify + test_channel) share one path.
    pub async fn send_discord(
        &self,
        webhook_url: &str,
        term: &SearchTerm,
        item: &SourceItem,
        source_name: &str,
    ) -> anyhow::Result<()> {
        let image_url = self.lookup_box_art(term).await;
        discord::send(&self.http, webhook_url, term, item, source_name, image_url.as_deref()).await
    }

    /// Send to Pushover. Thin wrapper so callers (notify + test_channel) share one path.
    pub async fn send_pushover(
        &self,
        app_token: &str,
        user_key: &str,
        term: &SearchTerm,
        item: &SourceItem,
    ) -> anyhow::Result<()> {
        let image_url = self.lookup_box_art(term).await;
        pushover::send(&self.http, app_token, user_key, term, item, image_url.as_deref()).await
    }

    /// Send to Apprise. Thin wrapper so callers (notify + test_channel) share one path.
    pub async fn send_apprise(
        &self,
        apprise_url: &str,
        term: &SearchTerm,
        item: &SourceItem,
    ) -> anyhow::Result<()> {
        apprise::send(&self.http, apprise_url, term, item).await
    }

    /// Fire all configured channels for a matched item.
    /// Returns a JSON array string of channels that were notified (e.g. `["discord","pushover"]`).
    /// Channel failures are logged as warnings but do not propagate — partial success is OK.
    pub async fn notify(
        &self,
        term: &SearchTerm,
        item: &SourceItem,
        source_name: &str,
    ) -> String {
        let mut fired: Vec<&str> = vec![];

        if let Some(ref url) = self.config.discord_webhook_url {
            match self.send_discord(url, term, item, source_name).await {
                Ok(_) => { fired.push("discord"); }
                Err(e) => { tracing::warn!("Discord notify failed: {e}"); }
            }
        }

        if let Some(ref url) = self.config.apprise_url {
            match self.send_apprise(url, term, item).await {
                Ok(_) => { fired.push("apprise"); }
                Err(e) => { tracing::warn!("Apprise notify failed: {e}"); }
            }
        }

        if let (Some(token), Some(key)) = (
            self.config.pushover_app_token.as_deref(),
            self.config.pushover_user_key.as_deref(),
        ) {
            match self.send_pushover(token, key, term, item).await {
                Ok(_) => { fired.push("pushover"); }
                Err(e) => { tracing::warn!("Pushover notify failed: {e}"); }
            }
        }

        serde_json::to_string(&fired).unwrap_or_else(|_| "[]".to_string())
    }
}
