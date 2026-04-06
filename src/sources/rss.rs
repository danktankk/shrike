// src/sources/rss.rs
use super::{Source, SourceItem};
use async_trait::async_trait;
use anyhow::Result;
use chrono::Utc;
use crate::models::SearchTerm;
use feed_rs::parser;

pub struct RssSource {
    pub url: String,
    pub api_key: Option<String>,
    pub http: reqwest::Client,
}

impl RssSource {
    pub fn new(url: String, api_key: Option<String>, http: reqwest::Client) -> Self {
        Self { url, api_key, http }
    }
}

#[async_trait]
impl Source for RssSource {
    async fn fetch(&self, _term: &SearchTerm) -> Result<Vec<SourceItem>> {
        let mut req = self.http.get(&self.url);

        // If api_key is set, use it as Basic auth password with username "CC"
        if let Some(pass) = &self.api_key {
            req = req.basic_auth("CC", Some(pass));
        }

        let body = req.send().await?.bytes().await?;
        let feed = parser::parse(body.as_ref())?;

        let items = feed.entries.into_iter().map(|entry| {
            let title = entry.title
                .map(|t| t.content)
                .unwrap_or_default();
            let url = entry.links.into_iter().next().map(|l| l.href);
            let guid = entry.id;
            let pub_date = entry.published
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|| entry.updated.map(|dt| dt.with_timezone(&Utc)));

            SourceItem {
                title,
                url,
                guid,
                pub_date,
                description: entry.summary.map(|s| s.content),
                indexer: None,
                seeders: None,
            }
        }).collect();

        Ok(items)
    }

    fn source_type(&self) -> &'static str { "rss" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    const FEED_XML: &str = r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <item>
      <title>Elden Ring v1.10 REPACK</title>
      <link>http://example.com/1</link>
      <guid>abc123</guid>
      <pubDate>Mon, 06 Apr 2026 12:00:00 +0000</pubDate>
    </item>
    <item>
      <title>Some Other Game TRAINER</title>
      <link>http://example.com/2</link>
      <guid>def456</guid>
    </item>
  </channel>
</rss>"#;

    #[tokio::test]
    async fn fetches_rss_and_returns_all_items() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(FEED_XML, "application/rss+xml")
            )
            .mount(&server)
            .await;

        let source = RssSource::new(
            format!("{}/feed.xml", server.uri()),
            None, // no auth
            reqwest::Client::new(),
        );

        let term = crate::models::SearchTerm {
            id: 1,
            name: "Test".into(),
            query: "Elden Ring".into(),
            enabled: true,
            max_age_days: Some(30),
            disallowed_keywords: None,
            created_at: chrono::Utc::now(),
        };

        let items = source.fetch(&term).await.unwrap();
        // RssSource returns ALL items — caller does the filtering
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Elden Ring v1.10 REPACK");
        assert_eq!(items[0].guid, "abc123");
        assert!(items[0].pub_date.is_some());
        assert_eq!(items[1].title, "Some Other Game TRAINER");
    }

    #[tokio::test]
    async fn empty_feed_returns_empty_vec() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/empty.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(
                        r#"<?xml version="1.0"?><rss version="2.0"><channel></channel></rss>"#,
                        "application/rss+xml"
                    )
            )
            .mount(&server)
            .await;

        let source = RssSource::new(format!("{}/empty.xml", server.uri()), None, reqwest::Client::new());
        let term = crate::models::SearchTerm {
            id: 1, name: "T".into(), query: "t".into(), enabled: true,
            max_age_days: None, disallowed_keywords: None, created_at: chrono::Utc::now(),
        };

        let items = source.fetch(&term).await.unwrap();
        assert!(items.is_empty());
    }
}
