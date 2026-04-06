// src/sources/rss.rs
use super::{Source, SourceItem};
use async_trait::async_trait;
use anyhow::Result;
use crate::models::SearchTerm;

pub struct RssSource {
    pub url: String,
    pub api_key: Option<String>,
}

impl RssSource {
    pub fn new(url: String, api_key: Option<String>) -> Self {
        Self { url, api_key }
    }
}

#[async_trait]
impl Source for RssSource {
    async fn fetch(&self, _term: &SearchTerm) -> Result<Vec<SourceItem>> {
        Ok(vec![]) // implemented in Task 7
    }
    fn source_type(&self) -> &'static str { "rss" }
}
