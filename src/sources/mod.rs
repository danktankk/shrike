// src/sources/mod.rs
pub mod rss;
pub mod newznab;
pub mod prowlarr;

use async_trait::async_trait;
use anyhow::Result;
use chrono::{DateTime, Utc};
use crate::models::SearchTerm;

#[derive(Debug, Clone)]
pub struct SourceItem {
    pub title: String,
    pub url: Option<String>,
    pub guid: String,
    pub pub_date: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub indexer: Option<String>,
    pub seeders: Option<u32>,
}

impl SourceItem {
    /// Builds a synthetic SourceItem used only by notification test handlers.
    /// Not persisted, never returned from a real fetch.
    pub fn test_sentinel() -> Self {
        SourceItem {
            title: "DiscoProwl Test Notification".into(),
            url: Some("https://github.com/danktankk/discoprowl".into()),
            guid: "test-guid".into(),
            pub_date: Some(Utc::now()),
            description: Some("This is a test notification from DiscoProwl.".into()),
            indexer: Some("test".into()),
            seeders: Some(42u32),
        }
    }
}

#[async_trait]
pub trait Source: Send + Sync {
    async fn fetch(&self, term: &SearchTerm) -> Result<Vec<SourceItem>>;
    fn source_type(&self) -> &'static str;
    /// Returns false for sources like RSS that return all items regardless of query.
    /// The scheduler uses this to fetch once and filter all terms client-side.
    fn is_search_based(&self) -> bool { true }
}

/// Build the correct Source implementation for a DB Source row.
/// Returns Err with the unknown source_type string if it isn't recognized.
pub fn build_source(
    source: &crate::models::Source,
    http: reqwest::Client,
) -> Result<Box<dyn Source>> {
    match source.source_type.as_str() {
        "rss" => Ok(Box::new(rss::RssSource::new(
            source.url.clone(),
            source.api_key.clone(),
            http,
        ))),
        "prowlarr" => Ok(Box::new(prowlarr::ProwlarrSource::new(
            source.url.clone(),
            source.api_key.clone().unwrap_or_default(),
            http,
            source.categories.clone(),
        ))),
        "newznab" | "torznab" => {
            let kind: &'static str = if source.source_type == "newznab" { "newznab" } else { "torznab" };
            Ok(Box::new(newznab::NzbSource::new(
                source.url.clone(),
                source.api_key.clone().unwrap_or_default(),
                http,
                kind,
            )))
        }
        other => Err(anyhow::anyhow!("unknown source_type: {other}")),
    }
}
