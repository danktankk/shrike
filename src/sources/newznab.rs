// src/sources/newznab.rs
use super::{Source, SourceItem};
use async_trait::async_trait;
use anyhow::Result;
use crate::models::SearchTerm;

pub struct NewznabSource {
    pub url: String,
    pub api_key: String,
    pub http: reqwest::Client,
}

impl NewznabSource {
    pub fn new(url: String, api_key: String, http: reqwest::Client) -> Self {
        Self { url, api_key, http }
    }
}

#[async_trait]
impl Source for NewznabSource {
    async fn fetch(&self, _term: &SearchTerm) -> Result<Vec<SourceItem>> {
        Ok(vec![]) // implemented in Task 8
    }
    fn source_type(&self) -> &'static str { "newznab" }
}

/// URL-encode a string (percent-encoding for query parameters).
pub(crate) fn urlencode(s: &str) -> String {
    urlencoding::encode(s).to_string()
}

/// Parse a Newznab/Torznab XML response into SourceItems.
/// Used by both NewznabSource and TorznabSource.
pub(crate) fn parse_newznab_xml(_xml: &str) -> Result<Vec<SourceItem>> {
    Ok(vec![]) // implemented in Task 8
}
