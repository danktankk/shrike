// src/sources/torznab.rs
use super::{Source, SourceItem};
use super::newznab::{parse_newznab_xml, urlencode};
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
    async fn fetch(&self, term: &SearchTerm) -> Result<Vec<SourceItem>> {
        let query_url = format!(
            "{}/api?t=search&q={}&apikey={}",
            self.url.trim_end_matches('/'),
            urlencode(&term.query),
            self.api_key
        );
        let body = self.http.get(&query_url).send().await?.text().await?;
        parse_newznab_xml(&body)
    }

    fn source_type(&self) -> &'static str { "torznab" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    const TORZNAB_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <item>
      <title>Hollow Knight v1.5 PC</title>
      <link>https://prowlarr.example.com/dl/hollow-knight.torrent</link>
      <guid>torrent-guid-abc</guid>
      <pubDate>Sat, 04 Apr 2026 15:00:00 +0000</pubDate>
      <torznab:attr name="seeders" value="99"/>
    </item>
  </channel>
</rss>"#;

    #[tokio::test]
    async fn fetches_torznab_and_parses_items() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(TORZNAB_XML, "application/rss+xml"))
            .mount(&server)
            .await;

        let source = TorznabSource::new(server.uri(), "key123".into(), reqwest::Client::new());
        let term = crate::models::SearchTerm {
            id: 1, name: "HK".into(), query: "Hollow Knight".into(),
            enabled: true, max_age_days: None, disallowed_keywords: None,
            created_at: chrono::Utc::now(),
        };

        let items = source.fetch(&term).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Hollow Knight v1.5 PC");
        assert_eq!(items[0].guid, "torrent-guid-abc");
        assert_eq!(items[0].seeders, Some(99));
    }
}
