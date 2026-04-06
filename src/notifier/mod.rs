// src/notifier/mod.rs
pub mod discord;
pub mod apprise;
pub mod pushover;

use std::sync::Arc;
use crate::config::Config;
use crate::models::SearchTerm;
use crate::sources::SourceItem;

pub struct Notifier {
    pub config: Arc<Config>,
    pub http: reqwest::Client,
}

impl Notifier {
    pub fn new(config: Arc<Config>, http: reqwest::Client) -> Self {
        Self { config, http }
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
            match discord::send(
                &self.http, url, term, item, source_name,
                self.config.steamgriddb_api_key.as_deref(),
            ).await {
                Ok(_) => { fired.push("discord"); }
                Err(e) => { tracing::warn!("Discord notify failed: {e}"); }
            }
        }

        if let Some(ref url) = self.config.apprise_url {
            match apprise::send(&self.http, url, term, item).await {
                Ok(_) => { fired.push("apprise"); }
                Err(e) => { tracing::warn!("Apprise notify failed: {e}"); }
            }
        }

        if let (Some(token), Some(key)) = (
            self.config.pushover_app_token.as_deref(),
            self.config.pushover_user_key.as_deref(),
        ) {
            match pushover::send(
                &self.http, token, key, term, item,
                self.config.steamgriddb_api_key.as_deref(),
            ).await {
                Ok(_) => { fired.push("pushover"); }
                Err(e) => { tracing::warn!("Pushover notify failed: {e}"); }
            }
        }

        serde_json::to_string(&fired).unwrap_or_else(|_| "[]".to_string())
    }
}
