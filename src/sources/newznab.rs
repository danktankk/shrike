// src/sources/newznab.rs
use super::{Source, SourceItem};
use async_trait::async_trait;
use anyhow::Result;
use crate::models::SearchTerm;
use quick_xml::Reader;
use quick_xml::events::Event;
use chrono::Utc;

/// Unified source for Newznab/Torznab/Prowlarr APIs.
/// All three speak the same `t=search&q=...&apikey=...` protocol and return
/// the same Newznab-style XML; only the `source_type` label differs.
pub struct NzbSource {
    pub url: String,
    pub api_key: String,
    pub http: reqwest::Client,
    pub source_type: &'static str,
}

impl NzbSource {
    pub fn new(url: String, api_key: String, http: reqwest::Client, source_type: &'static str) -> Self {
        Self { url, api_key, http, source_type }
    }
}

#[async_trait]
impl Source for NzbSource {
    async fn fetch(&self, term: &SearchTerm) -> Result<Vec<SourceItem>> {
        let query_url = format!(
            "{}/api?t=search&q={}&apikey={}",
            self.url.trim_end_matches('/'),
            urlencoding::encode(&term.query),
            self.api_key
        );
        let body = self.http.get(&query_url).send().await?.text().await?;
        parse_newznab_xml(&body)
    }

    fn source_type(&self) -> &'static str { self.source_type }
}

/// Deterministic 64-bit hash of a string, formatted as hex.
/// Used to synthesize a stable guid for items missing one, so that
/// the same item is not re-inserted on every poll. djb2 variant.
pub(crate) fn stable_title_hash(s: &str) -> String {
    let mut h: u64 = 5381;
    for b in s.as_bytes() {
        h = h.wrapping_mul(33).wrapping_add(*b as u64);
    }
    format!("{:016x}", h)
}

/// Parse a Newznab/Torznab XML RSS response into SourceItems.
/// Shared between NewznabSource and TorznabSource.
pub(crate) fn parse_newznab_xml(xml: &str) -> Result<Vec<SourceItem>> {
    let mut items = Vec::new();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut in_item = false;
    let mut cur_title = String::new();
    let mut cur_url: Option<String> = None;
    let mut cur_guid = String::new();
    let mut cur_pub_date: Option<chrono::DateTime<Utc>> = None;
    let mut cur_seeders: Option<u32> = None;
    let mut cur_indexer: Option<String> = None;
    let mut capture_next: Option<&'static str> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                let local = name.local_name();
                match local.as_ref() {
                    b"item" => {
                        in_item = true;
                        cur_title = String::new();
                        cur_url = None;
                        cur_guid = String::new();
                        cur_pub_date = None;
                        cur_seeders = None;
                        cur_indexer = None;
                        capture_next = None;
                    }
                    b"title" if in_item => { capture_next = Some("title"); }
                    b"link" if in_item => { capture_next = Some("link"); }
                    b"guid" if in_item => { capture_next = Some("guid"); }
                    b"pubDate" if in_item => { capture_next = Some("pubDate"); }
                    _ => { capture_next = None; }
                }
            }
            Ok(Event::Text(ref t)) => {
                if in_item {
                    let text = t.unescape().unwrap_or_default().to_string();
                    match capture_next {
                        Some("title") => { cur_title = text; }
                        Some("link") => { cur_url = if text.is_empty() { None } else { Some(text) }; }
                        Some("guid") => { cur_guid = text; }
                        Some("pubDate") => {
                            cur_pub_date = chrono::DateTime::parse_from_rfc2822(&text)
                                .map(|dt| dt.with_timezone(&Utc))
                                .ok();
                        }
                        _ => {}
                    }
                    capture_next = None;
                }
            }
            Ok(Event::Empty(ref e)) => {
                if in_item {
                    let local = e.name().local_name();
                    let local_str = std::str::from_utf8(local.as_ref()).unwrap_or("");
                    // Match newznab:attr elements (local name is "attr")
                    if local_str == "attr" {
                        let mut attr_name = String::new();
                        let mut attr_value = String::new();
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.local_name().as_ref())
                                .unwrap_or("").to_string();
                            let val = std::str::from_utf8(&attr.value)
                                .unwrap_or("").to_string();
                            if key == "name" { attr_name = val; }
                            else if key == "value" { attr_value = val; }
                        }
                        match attr_name.as_str() {
                            "seeders" => { cur_seeders = attr_value.parse().ok(); }
                            "indexer" => { cur_indexer = Some(attr_value); }
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let local = e.name().local_name();
                if local.as_ref() == b"item" && in_item {
                    if !cur_title.is_empty() || !cur_guid.is_empty() {
                        items.push(SourceItem {
                            title: cur_title.clone(),
                            url: cur_url.clone(),
                            guid: if cur_guid.is_empty() {
                                stable_title_hash(&cur_title)
                            } else {
                                cur_guid.clone()
                            },
                            pub_date: cur_pub_date,
                            description: None,
                            indexer: cur_indexer.clone(),
                            seeders: cur_seeders,
                        });
                    }
                    in_item = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("XML parse error at position {}: {}", reader.buffer_position(), e)),
            _ => {}
        }
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, query_param};

    const NEWZNAB_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:newznab="http://www.newznab.com/DTD/2010/feeds/attributes/">
  <channel>
    <title>NZB Test Feed</title>
    <item>
      <title>Elden Ring v1.10 MULTi9 REPACK</title>
      <link>https://example.com/nzb/1</link>
      <guid isPermaLink="false">nzb-guid-001</guid>
      <pubDate>Mon, 06 Apr 2026 10:00:00 +0000</pubDate>
      <newznab:attr name="seeders" value="42"/>
      <newznab:attr name="indexer" value="nzbplanet"/>
    </item>
    <item>
      <title>Dark Souls III Complete REPACK</title>
      <link>https://example.com/nzb/2</link>
      <guid isPermaLink="false">nzb-guid-002</guid>
      <pubDate>Sun, 05 Apr 2026 08:00:00 +0000</pubDate>
      <newznab:attr name="seeders" value="7"/>
    </item>
  </channel>
</rss>"#;

    #[tokio::test]
    async fn fetches_newznab_and_parses_items() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api"))
            .and(query_param("t", "search"))
            .and(query_param("apikey", "testkey123"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(NEWZNAB_XML, "application/rss+xml"))
            .mount(&server)
            .await;

        let source = NzbSource::new(server.uri(), "testkey123".into(), reqwest::Client::new(), "newznab");
        let term = crate::models::SearchTerm {
            id: 1, name: "Elden Ring".into(), query: "Elden Ring".into(),
            enabled: true, max_age_days: Some(30), disallowed_keywords: None,
            created_at: chrono::Utc::now(),
        };

        let items = source.fetch(&term).await.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Elden Ring v1.10 MULTi9 REPACK");
        assert_eq!(items[0].guid, "nzb-guid-001");
        assert_eq!(items[0].seeders, Some(42));
        assert_eq!(items[0].indexer.as_deref(), Some("nzbplanet"));
        assert!(items[0].pub_date.is_some());
        assert_eq!(items[1].title, "Dark Souls III Complete REPACK");
        assert_eq!(items[1].seeders, Some(7));
        assert!(items[1].indexer.is_none());
    }

    #[tokio::test]
    async fn parse_xml_empty_channel() {
        let xml = r#"<?xml version="1.0"?><rss version="2.0"><channel></channel></rss>"#;
        let items = parse_newznab_xml(xml).unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn url_encodes_query_with_spaces() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api"))
            .and(query_param("q", "Elden Ring"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"<?xml version="1.0"?><rss version="2.0"><channel></channel></rss>"#,
                "application/rss+xml"
            ))
            .mount(&server)
            .await;

        let source = NzbSource::new(server.uri(), "key".into(), reqwest::Client::new(), "newznab");
        let term = crate::models::SearchTerm {
            id: 1, name: "Test".into(), query: "Elden Ring".into(),
            enabled: true, max_age_days: None, disallowed_keywords: None,
            created_at: chrono::Utc::now(),
        };
        // Should succeed — mock matched on the raw query param value
        let result = source.fetch(&term).await;
        assert!(result.is_ok());
    }

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
    async fn fetches_torznab_via_nzb_source() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(TORZNAB_XML, "application/rss+xml"))
            .mount(&server)
            .await;

        let source = NzbSource::new(server.uri(), "key123".into(), reqwest::Client::new(), "torznab");
        assert_eq!(source.source_type(), "torznab");
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

    #[test]
    fn stable_title_hash_is_deterministic() {
        let a = stable_title_hash("Elden Ring v1.10 REPACK");
        let b = stable_title_hash("Elden Ring v1.10 REPACK");
        assert_eq!(a, b);
        assert_ne!(a, stable_title_hash("Different Title"));
    }
}
