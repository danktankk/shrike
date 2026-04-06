// src/sources/mod.rs
pub mod rss;
pub mod newznab;
pub mod torznab;

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

#[async_trait]
pub trait Source: Send + Sync {
    async fn fetch(&self, term: &SearchTerm) -> Result<Vec<SourceItem>>;
    fn source_type(&self) -> &'static str;
}

/// Build the correct Source implementation for a DB Source row.
/// Returns None if the source_type is unknown.
pub fn build_source(
    source: &crate::models::Source,
    http: reqwest::Client,
) -> Option<Box<dyn Source>> {
    match source.source_type.as_str() {
        "rss" => Some(Box::new(rss::RssSource::new(
            source.url.clone(),
            source.api_key.clone(),
            http.clone(),
        ))),
        "newznab" => Some(Box::new(newznab::NewznabSource::new(
            source.url.clone(),
            source.api_key.clone().unwrap_or_default(),
            http,
        ))),
        "torznab" => Some(Box::new(torznab::TorznabSource::new(
            source.url.clone(),
            source.api_key.clone().unwrap_or_default(),
            http,
        ))),
        _ => None,
    }
}
