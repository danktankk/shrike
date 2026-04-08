// src/notifier/mod.rs
pub mod discord;
pub mod apprise;
pub mod pushover;

use std::sync::Arc;
use crate::config::Config;
use crate::models::SearchTerm;
use crate::sources::SourceItem;
use crate::steamgriddb;

pub struct Notifier {
    pub config: Arc<Config>,
    pub http: reqwest::Client,
}

impl Notifier {
    pub fn new(config: Arc<Config>, http: reqwest::Client) -> Self {
        Self { config, http }
    }

    /// Look up a box art URL for the term's query via SteamGridDB.
    /// Returns `None` when no key is configured, the lookup fails, or
    /// no match/no grid exists. Callers decide how to handle absence.
    async fn lookup_box_art(&self, query: &str) -> Option<String> {
        let key = self.config.steamgriddb_api_key.as_deref()?;
        match steamgriddb::search_game(&self.http, key, query).await {
            Ok(Some(g)) => g.grid_url.or(g.hero_url),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!("SteamGridDB lookup failed for '{query}': {e}");
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
        let image_url = self.lookup_box_art(&term.query).await;
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
        let image_url = self.lookup_box_art(&term.query).await;
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
