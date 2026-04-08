// src/steamgriddb.rs
//
// SteamGridDB client. Single source of truth for SGDB lookups.
// Notifier + API handlers both consume from here — no duplicated fetch logic.
//
// All functions return `Option<...>` on empty results. There is NO fallback
// URL. Callers decide how to handle absence (omit image, show placeholder,
// 404, etc.). Network/HTTP errors surface as `Err`.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::Client;
use serde::{Deserialize, Serialize};

const SGDB_BASE: &str = "https://www.steamgriddb.com/api/v2";
const CACHE_TTL: Duration = Duration::from_secs(60 * 60); // 1h

/// Lightweight SGDB art reference. The minimal `art` slot of `GameBundle`.
/// Release date, platforms, summary, dev/pub/genre metadata are NOT here —
/// those live on the `info` slot owned by agent-game-data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRef {
    pub id: u64,
    pub name: String,
    pub hero_url: Option<String>,
    pub grid_url: Option<String>,
    pub logo_url: Option<String>,
}

/// Server-side cache for /api/art lookups. Keyed by lowercased title.
/// `None` value is a negative cache entry (lookup ran, found nothing).
#[derive(Default)]
pub struct ArtCache {
    inner: Mutex<HashMap<String, (Instant, Option<GameRef>)>>,
}

impl ArtCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<Option<GameRef>> {
        let k = key.to_lowercase();
        let guard = self.inner.lock().ok()?;
        let (at, v) = guard.get(&k)?;
        if at.elapsed() > CACHE_TTL {
            return None;
        }
        Some(v.clone())
    }

    pub fn put(&self, key: &str, value: Option<GameRef>) {
        let k = key.to_lowercase();
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(k, (Instant::now(), value));
        }
    }
}

// ---- Raw SGDB response shapes (private) ----

#[derive(Deserialize)]
struct SgdbEnvelope<T> {
    data: T,
}

#[derive(Deserialize)]
struct SgdbSearchHit {
    id: u64,
    name: String,
}

#[derive(Deserialize)]
struct SgdbAsset {
    url: String,
}

// ---- Public API ----

/// Search SGDB for a game by free-text title. Returns `Ok(None)` when no
/// match is found (NOT an error). HTTP/parse failures return `Err`.
pub async fn search_game(
    client: &Client,
    api_key: &str,
    query: &str,
) -> anyhow::Result<Option<GameRef>> {
    let url = format!(
        "{}/search/autocomplete/{}",
        SGDB_BASE,
        urlencoding::encode(query)
    );
    let resp: SgdbEnvelope<Vec<SgdbSearchHit>> = client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let Some(hit) = resp.data.into_iter().next() else {
        return Ok(None);
    };

    let (hero_url, grid_url, logo_url) = fetch_art_urls(client, api_key, hit.id).await?;

    Ok(Some(GameRef {
        id: hit.id,
        name: hit.name,
        hero_url,
        grid_url,
        logo_url,
    }))
}

/// Fetch (hero, grid, logo) URLs for a game id. Each asset is independently
/// optional — one missing kind does not fail the whole lookup.
async fn fetch_art_urls(
    client: &Client,
    api_key: &str,
    game_id: u64,
) -> anyhow::Result<(Option<String>, Option<String>, Option<String>)> {
    let hero = first_asset_url(client, api_key, "heroes", game_id).await?;
    let grid = first_asset_url(client, api_key, "grids", game_id).await?;
    let logo = first_asset_url(client, api_key, "logos", game_id).await?;
    Ok((hero, grid, logo))
}

async fn first_asset_url(
    client: &Client,
    api_key: &str,
    kind: &str,
    game_id: u64,
) -> anyhow::Result<Option<String>> {
    let url = format!("{}/{}/game/{}", SGDB_BASE, kind, game_id);
    let resp = client.get(&url).bearer_auth(api_key).send().await?;
    if !resp.status().is_success() {
        // SGDB returns 404 for "no assets of this kind" on some games.
        tracing::debug!("SGDB {} for game {} returned {}", kind, game_id, resp.status());
        return Ok(None);
    }
    let env: SgdbEnvelope<Vec<SgdbAsset>> = resp.json().await?;
    Ok(env.data.into_iter().next().map(|a| a.url))
}

