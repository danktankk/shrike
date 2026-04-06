// src/sources/torznab.rs
use super::{Source, SourceItem};
use async_trait::async_trait;
use anyhow::Result;
use crate::models::SearchTerm;

pub struct TorznabSource {
    pub url: String,
    pub api_key: String,
    pub http: reqwest::Client,
}

impl TorznabSource {
    pub fn new(url: String, api_key: String, http: reqwest::Client) -> Self {
        Self { url, api_key, http }
    }
}

#[async_trait]
impl Source for TorznabSource {
    async fn fetch(&self, _term: &SearchTerm) -> Result<Vec<SourceItem>> {
        Ok(vec![]) // implemented in Task 8
    }
    fn source_type(&self) -> &'static str { "torznab" }
}
