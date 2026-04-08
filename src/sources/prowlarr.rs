// src/sources/prowlarr.rs
//
// Native Prowlarr search against `/api/v1/search`.
//
// Prowlarr is a multi-indexer proxy, NOT a single torznab server. The previous
// implementation hit `/api?t=search&...` on the base URL, which just returns
// Prowlarr's API-version discovery JSON (`{"current":"v1","deprecated":[]}`)
// and parses as zero items. This module uses the real search endpoint with
// `X-Api-Key` auth and decodes the JSON response.

use super::{Source, SourceItem};
use async_trait::async_trait;
use anyhow::{Result, Context};
use serde::Deserialize;
use crate::models::SearchTerm;

pub struct ProwlarrSource {
    pub url: String,
    pub api_key: String,
    pub http: reqwest::Client,
    /// Parsed from the DB `categories` column (CSV of newznab IDs).
    /// Empty = no category filter.
    pub categories: Vec<u32>,
}

impl ProwlarrSource {
    pub fn new(url: String, api_key: String, http: reqwest::Client, categories: Option<String>) -> Self {
        let categories = categories
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .collect();
        Self { url, api_key, http, categories }
    }
}

/// Subset of the Prowlarr /api/v1/search response we consume.
/// Prowlarr returns a JSON array; each item has many more fields than we use.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProwlarrRelease {
    #[serde(default)]
    guid: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    info_url: Option<String>,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    magnet_url: Option<String>,
    #[serde(default)]
    publish_date: Option<String>,
    #[serde(default)]
    indexer: Option<String>,
    #[serde(default)]
    seeders: Option<u32>,
}

#[async_trait]
impl Source for ProwlarrSource {
    async fn fetch(&self, term: &SearchTerm) -> Result<Vec<SourceItem>> {
        // Prowlarr's search endpoint. `type=search` is the generic/keyword type;
        // other valid values are movie/tvsearch/book/music but `search` works
        // across all indexers regardless of category and is what users expect
        // from a cross-indexer query.
        let base = self.url.trim_end_matches('/');
        let mut url = format!(
            "{}/api/v1/search?query={}&type=search&limit=100",
            base,
            urlencoding::encode(&term.query),
        );
        for cat in &self.categories {
            url.push_str(&format!("&categories={cat}"));
        }

        let resp = self.http
            .get(&url)
            .header("X-Api-Key", &self.api_key)
            .send()
            .await
            .context("prowlarr: request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("prowlarr: HTTP {} — {}", status, body.chars().take(300).collect::<String>());
        }

        let releases: Vec<ProwlarrRelease> = resp
            .json()
            .await
            .context("prowlarr: response was not valid JSON (wrong endpoint or API key?)")?;

        let items = releases.into_iter().filter_map(|r| {
            let title = r.title?;
            // Prefer infoUrl for clicking through to the release page;
            // fall back to downloadUrl, then magnetUrl.
            let url = r.info_url.or(r.download_url).or(r.magnet_url);
            // Prowlarr always returns a guid for real releases, but synthesize
            // one from the title if somehow missing so dedup still works.
            let guid = r.guid.unwrap_or_else(|| super::newznab::stable_title_hash(&title));
            let pub_date = r.publish_date
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc));
            Some(SourceItem {
                title,
                url,
                guid,
                pub_date,
                description: None,
                indexer: r.indexer,
                seeders: r.seeders,
            })
        }).collect();

        Ok(items)
    }

    fn source_type(&self) -> &'static str { "prowlarr" }
}
