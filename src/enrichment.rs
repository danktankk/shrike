// src/enrichment.rs
//
// Game metadata enrichment via Steam's public Storefront + News APIs.
// Complements `steamgriddb.rs` (art only) by supplying release date,
// platforms, store URL, short description, studios, genres, metacritic,
// and recent news. No API key required.
//
// Public surface:
//   - `EnrichedGame`, `NewsItem`, `StoreLinks`, `PlatformFlags` — JSON contract
//   - `EnrichmentCache` — 6h TTL, lowercased-title keyed, negative caching
//   - `enrich(client, cache, title) -> Option<EnrichedGame>` — cached entry
//
// Network/parse failures log at warn and return None. Upstream shape drift
// degrades gracefully because every serde field is Option/default.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::Client;
use serde::{Deserialize, Serialize};

const STORE_SEARCH: &str = "https://store.steampowered.com/api/storesearch/";
const STORE_APPDETAILS: &str = "https://store.steampowered.com/api/appdetails";
const STORE_NEWS: &str = "https://api.steampowered.com/ISteamNews/GetNewsForApp/v2/";
const CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const NEWS_COUNT: &str = "3";
const NEWS_MAXLENGTH: &str = "300";

// ---- Public JSON contract ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedGame {
    pub steam_appid: Option<u64>,
    pub store_url: Option<String>,
    pub stores: StoreLinks,
    pub release_date: Option<String>,
    pub short_description: Option<String>,
    pub platforms: PlatformFlags,
    pub header_image: Option<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub genres: Vec<String>,
    pub metacritic_score: Option<u32>,
    pub news: Vec<NewsItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoreLinks {
    pub steam_url: Option<String>,
    pub gog_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformFlags {
    pub windows: bool,
    pub mac: bool,
    pub linux: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub title: String,
    pub url: String,
    pub contents: String,
    pub date: i64,
}

// ---- Cache ----

#[derive(Default)]
pub struct EnrichmentCache {
    inner: Mutex<HashMap<String, (Instant, Option<EnrichedGame>)>>,
}

impl EnrichmentCache {
    pub fn new() -> Self { Self::default() }

    fn get(&self, key: &str) -> Option<Option<EnrichedGame>> {
        let k = key.to_lowercase();
        let guard = self.inner.lock().ok()?;
        let (at, v) = guard.get(&k)?;
        if at.elapsed() > CACHE_TTL { return None; }
        Some(v.clone())
    }

    fn put(&self, key: &str, value: Option<EnrichedGame>) {
        let k = key.to_lowercase();
        if let Ok(mut g) = self.inner.lock() {
            g.insert(k, (Instant::now(), value));
        }
    }
}

// ---- Raw Steam response shapes (private) ----

#[derive(Deserialize)]
struct StoreSearchEnvelope {
    #[serde(default)]
    items: Vec<StoreSearchItem>,
}

#[derive(Deserialize)]
struct StoreSearchItem { id: u64 }

#[derive(Deserialize)]
struct AppDetailsWrapper {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    data: Option<AppDetailsData>,
}

#[derive(Deserialize)]
struct AppDetailsData {
    #[serde(default)]
    steam_appid: Option<u64>,
    #[serde(default)]
    short_description: Option<String>,
    #[serde(default)]
    header_image: Option<String>,
    #[serde(default)]
    developers: Vec<String>,
    #[serde(default)]
    publishers: Vec<String>,
    #[serde(default)]
    platforms: Option<RawPlatforms>,
    #[serde(default)]
    release_date: Option<RawReleaseDate>,
    #[serde(default)]
    metacritic: Option<RawMetacritic>,
    #[serde(default)]
    genres: Vec<RawGenre>,
}

#[derive(Deserialize)]
struct RawPlatforms {
    #[serde(default)] windows: bool,
    #[serde(default)] mac: bool,
    #[serde(default)] linux: bool,
}

#[derive(Deserialize)]
struct RawReleaseDate {
    #[serde(default)] date: String,
}

#[derive(Deserialize)]
struct RawMetacritic {
    #[serde(default)] score: Option<u32>,
}

#[derive(Deserialize)]
struct RawGenre {
    #[serde(default)] description: String,
}

#[derive(Deserialize)]
struct NewsEnvelope {
    #[serde(default)]
    appnews: Option<NewsAppNews>,
}

#[derive(Deserialize)]
struct NewsAppNews {
    #[serde(default)]
    newsitems: Vec<RawNewsItem>,
}

#[derive(Deserialize)]
struct RawNewsItem {
    #[serde(default)] title: String,
    #[serde(default)] url: String,
    #[serde(default)] contents: String,
    #[serde(default)] date: i64,
}

// ---- Public API ----

/// Enrich a free-text title via Steam's public APIs. Returns `None` when
/// Steam has no match or the lookup fails. Consults the cache first
/// (positive and negative hits, 6h TTL, lowercased-title key).
pub async fn enrich(
    client: &Client,
    cache: &EnrichmentCache,
    title: &str,
) -> Option<EnrichedGame> {
    let trimmed = title.trim();
    if trimmed.is_empty() { return None; }
    if let Some(cached) = cache.get(trimmed) { return cached; }

    let result = match fetch(client, trimmed).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Steam enrichment failed for '{trimmed}': {e}");
            None
        }
    };
    cache.put(trimmed, result.clone());
    result
}

async fn fetch(client: &Client, title: &str) -> anyhow::Result<Option<EnrichedGame>> {
    let Some(appid) = search_first_appid(client, title).await? else {
        return Ok(None);
    };
    let Some(d) = fetch_app_details(client, appid).await? else {
        return Ok(None);
    };

    // News is best-effort — a failure here should not drop the rest.
    let news = fetch_news(client, appid).await.unwrap_or_else(|e| {
        tracing::debug!("Steam news fetch failed for appid {appid}: {e}");
        Vec::new()
    });

    let steam_url = format!("https://store.steampowered.com/app/{appid}/");
    let platforms = d.platforms.map(|p| PlatformFlags {
        windows: p.windows, mac: p.mac, linux: p.linux,
    }).unwrap_or_default();
    let release_date = d.release_date.map(|r| r.date).filter(|s| !s.is_empty());
    let metacritic_score = d.metacritic.and_then(|m| m.score);
    let genres = d.genres.into_iter()
        .map(|g| g.description)
        .filter(|s| !s.is_empty())
        .collect();

    Ok(Some(EnrichedGame {
        steam_appid: Some(d.steam_appid.unwrap_or(appid)),
        store_url: Some(steam_url.clone()),
        stores: StoreLinks { steam_url: Some(steam_url), gog_url: None },
        release_date,
        short_description: d.short_description,
        platforms,
        header_image: d.header_image,
        developers: d.developers,
        publishers: d.publishers,
        genres,
        metacritic_score,
        news,
    }))
}

async fn search_first_appid(client: &Client, title: &str) -> anyhow::Result<Option<u64>> {
    let env: StoreSearchEnvelope = client
        .get(STORE_SEARCH)
        .query(&[("term", title), ("l", "en"), ("cc", "US")])
        .send().await?
        .error_for_status()?
        .json().await?;
    Ok(env.items.into_iter().next().map(|i| i.id))
}

async fn fetch_app_details(
    client: &Client,
    appid: u64,
) -> anyhow::Result<Option<AppDetailsData>> {
    // Shape: { "<appid>": { "success": bool, "data": {...} } }
    let appid_s = appid.to_string();
    let raw: HashMap<String, AppDetailsWrapper> = client
        .get(STORE_APPDETAILS)
        .query(&[("appids", appid_s.as_str()), ("l", "en"), ("cc", "US")])
        .send().await?
        .error_for_status()?
        .json().await?;
    let Some(w) = raw.into_iter().next().map(|(_, v)| v) else {
        return Ok(None);
    };
    if !w.success { return Ok(None); }
    Ok(w.data)
}

async fn fetch_news(client: &Client, appid: u64) -> anyhow::Result<Vec<NewsItem>> {
    let appid_s = appid.to_string();
    let env: NewsEnvelope = client
        .get(STORE_NEWS)
        .query(&[
            ("appid", appid_s.as_str()),
            ("count", NEWS_COUNT),
            ("maxlength", NEWS_MAXLENGTH),
            ("format", "json"),
        ])
        .send().await?
        .error_for_status()?
        .json().await?;
    Ok(env.appnews
        .map(|a| a.newsitems)
        .unwrap_or_default()
        .into_iter()
        .map(|n| NewsItem {
            title: n.title, url: n.url, contents: n.contents, date: n.date,
        })
        .collect())
}
