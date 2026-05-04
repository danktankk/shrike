// src/api/art.rs
//
// HTTP handlers for SteamGridDB-backed art + the match-detail aggregator.
// All SGDB lookup logic lives in `crate::steamgriddb`; all Steam
// storefront/news lookup lives in `crate::enrichment`. This file is thin
// glue plus the aggregator for GET /api/match/{id}.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use super::{AppError, AppState};
use crate::enrichment::{self, EnrichedGame, NewsItem};
use crate::models::{Match, SearchTerm, Source};
use crate::steamgriddb::{self, GameRef};

// ---- /api/art ----

#[derive(Deserialize)]
pub struct ArtQuery {
    pub q: String,
}

#[derive(Serialize)]
pub struct ArtResponse {
    /// `None` when no SGDB key is configured or nothing matched. Frontend
    /// reads `r.game?.grid_url` — the null alone is sufficient signal.
    pub game: Option<GameRef>,
}

/// GET /api/art?q=<title>
///
/// SGDB-only thumbnail lookup. Consults the server-side cache first (1h TTL,
/// keyed by lowercased title); on miss, queries SteamGridDB with the global
/// blocklist applied and caches the result (including negatives). Never
/// returns a fallback URL — callers render a placeholder when `game` is null.
pub async fn get_art(
    State(state): State<AppState>,
    Query(params): Query<ArtQuery>,
) -> Result<Json<ArtResponse>, AppError> {
    if params.q.trim().is_empty() {
        return Err(AppError::BadRequest("q must not be empty".to_string()));
    }

    if let Some(cached) = state.art_cache.get(&params.q) {
        return Ok(Json(ArtResponse { game: cached }));
    }

    let Some(key) = state.config.steamgriddb_api_key.as_deref() else {
        state.art_cache.put(&params.q, None);
        return Ok(Json(ArtResponse { game: None }));
    };

    let blocklist = crate::blocklist::load(&state.pool).await.unwrap_or_default();
    let game = match steamgriddb::search_game_filtered(
        &state.http.external_strict,
        key,
        &params.q,
        &blocklist,
    )
    .await
    {
        Ok(g) => g,
        Err(e) => {
            tracing::warn!("SteamGridDB search failed for '{}': {e}", params.q);
            None
        }
    };

    state.art_cache.put(&params.q, game.clone());
    Ok(Json(ArtResponse { game }))
}

// ---- /api/match/{id} aggregator ----

/// Flattened view of `EnrichedGame` minus `news`. Frontend reads every field
/// here directly; `platforms` is flattened from the `PlatformFlags` bitfield
/// to a display-friendly `Vec<String>` so the UI can `{#each}` over it.
#[derive(Serialize)]
pub struct InfoView {
    pub steam_appid: Option<u64>,
    pub store_url: Option<String>,
    pub release_date: Option<String>,
    pub short_description: Option<String>,
    pub platforms: Vec<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub genres: Vec<String>,
    pub metacritic_score: Option<u32>,
    pub header_image: Option<String>,
}

/// The `game` field of the match-detail response. `art` is SGDB-sourced,
/// `info` is Steam-storefront-sourced, `news` is Steam news (may be empty).
/// All three are independently optional: a title might hit SGDB but miss
/// Steam, or vice versa.
#[derive(Serialize)]
pub struct GameBundle {
    pub art: Option<GameRef>,
    pub info: Option<InfoView>,
    pub news: Vec<NewsItem>,
}

#[derive(Serialize)]
pub struct MatchDetailResponse {
    #[serde(rename = "match")]
    pub match_row: Match,
    pub search_term: SearchTerm,
    pub source: Source,
    /// `None` when BOTH SGDB and Steam returned nothing. If either succeeds
    /// this is `Some` with the absent side(s) internally null/empty.
    pub game: Option<GameBundle>,
}

/// GET /api/match/{id}
///
/// Aggregator for the click-through page. Pulls the match row + its parent
/// term/source, then sequentially calls SteamGridDB (for art) and Steam
/// enrichment (for storefront info + news). Enrichment is fed the cleaned
/// SGDB name when available, falling back to the raw `item_title`. SGDB or
/// Steam failures degrade to null on their respective side of the bundle
/// — they never fail the request.
pub async fn get_match(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<MatchDetailResponse>, AppError> {
    let match_row: Match = sqlx::query_as::<_, Match>("SELECT * FROM matches WHERE id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    let search_term: SearchTerm =
        sqlx::query_as::<_, SearchTerm>("SELECT * FROM search_terms WHERE id = ?")
            .bind(match_row.search_term_id)
            .fetch_one(&state.pool)
            .await?;

    let source: Source = sqlx::query_as::<_, Source>("SELECT * FROM sources WHERE id = ?")
        .bind(match_row.source_id)
        .fetch_one(&state.pool)
        .await?;

    // 1. SGDB art. If the term has a `steamgriddb_id` override, use it
    //    directly (autocomplete is bypassed). Otherwise run autocomplete
    //    with the blocklist applied.
    let art = resolve_art(&state, &search_term).await;

    // 2. Steam enrichment. If the term has a `steam_appid` override, use it
    //    directly. Otherwise prefer SGDB's cleaned name, fall back to the
    //    raw item_title — Steam's storesearch is forgiving enough to
    //    resolve the latter most of the time.
    let enriched = if let Some(appid) = search_term.steam_appid {
        enrichment::enrich_by_appid(
            &state.http.external_strict,
            &state.enrichment_cache,
            appid as u64,
        )
        .await
    } else {
        let enrich_title = art
            .as_ref()
            .map(|g| g.name.as_str())
            .unwrap_or(&match_row.item_title);
        let blocklist = crate::blocklist::load(&state.pool).await.unwrap_or_default();
        enrichment::enrich_filtered(
            &state.http.external_strict,
            &state.enrichment_cache,
            enrich_title,
            &blocklist,
        )
        .await
    };

    let (info, news) = split_enriched(enriched);

    let game = if art.is_none() && info.is_none() && news.is_empty() {
        None
    } else {
        Some(GameBundle { art, info, news })
    };

    Ok(Json(MatchDetailResponse {
        match_row,
        search_term,
        source,
        game,
    }))
}

/// Best-effort SGDB resolution honoring per-term overrides + global blocklist.
///
/// - When `term.steamgriddb_id` is set, fetches art directly for that id and
///   caches under a synthetic `sgdb:<id>` key (separate keyspace so a future
///   query-string with the same name doesn't collide).
/// - Otherwise queries autocomplete with the blocklist applied and caches
///   under the term's `query` (preserves the prior cache layout).
///
/// Missing key, HTTP error, or empty hits all degrade to `None` with a
/// warn-level log.
async fn resolve_art(state: &AppState, term: &SearchTerm) -> Option<GameRef> {
    let key = state.config.steamgriddb_api_key.as_deref()?;

    if let Some(id) = term.steamgriddb_id {
        let cache_key = format!("sgdb:{id}");
        if let Some(cached) = state.art_cache.get(&cache_key) {
            return cached;
        }
        let game = match steamgriddb::fetch_game_by_id(
            &state.http.external_strict,
            key,
            id as u64,
        )
        .await
        {
            Ok(g) => g,
            Err(e) => {
                tracing::warn!("SGDB fetch by id={id} failed: {e}");
                None
            }
        };
        state.art_cache.put(&cache_key, game.clone());
        return game;
    }

    if let Some(cached) = state.art_cache.get(&term.query) {
        return cached;
    }
    let blocklist = crate::blocklist::load(&state.pool).await.unwrap_or_default();
    let game = match steamgriddb::search_game_filtered(
        &state.http.external_strict,
        key,
        &term.query,
        &blocklist,
    )
    .await
    {
        Ok(g) => g,
        Err(e) => {
            tracing::warn!("SGDB search failed for '{}': {e}", term.query);
            None
        }
    };
    state.art_cache.put(&term.query, game.clone());
    game
}

/// Split `EnrichedGame` into the `(info, news)` pair the frontend expects.
/// `PlatformFlags` bitfield → display-friendly `Vec<String>`.
fn split_enriched(enriched: Option<EnrichedGame>) -> (Option<InfoView>, Vec<NewsItem>) {
    let Some(e) = enriched else {
        return (None, Vec::new());
    };
    let mut platforms = Vec::new();
    if e.platforms.windows { platforms.push("Windows".to_string()); }
    if e.platforms.mac     { platforms.push("macOS".to_string()); }
    if e.platforms.linux   { platforms.push("Linux".to_string()); }

    let info = InfoView {
        steam_appid: e.steam_appid,
        store_url: e.store_url,
        release_date: e.release_date,
        short_description: e.short_description,
        platforms,
        developers: e.developers,
        publishers: e.publishers,
        genres: e.genres,
        metacritic_score: e.metacritic_score,
        header_image: e.header_image,
    };
    (Some(info), e.news)
}
