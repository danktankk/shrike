// src/api/sources.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use super::{AppError, AppState};
use crate::models::{Source, SourcePayload};
use crate::sources::build_source;

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Source>>, AppError> {
    let sources = sqlx::query_as::<_, Source>("SELECT * FROM sources ORDER BY name ASC")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(sources))
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<SourcePayload>,
) -> Result<(StatusCode, Json<Source>), AppError> {
    let s = sqlx::query_as::<_, Source>(
        "INSERT INTO sources (name, source_type, url, api_key, enabled, poll_interval_mins, categories)
         VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING *",
    )
    .bind(&body.name)
    .bind(&body.source_type)
    .bind(&body.url)
    .bind(&body.api_key)
    .bind(body.enabled.unwrap_or(true))
    .bind(body.poll_interval_mins.unwrap_or(720))
    .bind(&body.categories)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(s)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<SourcePayload>,
) -> Result<Json<Source>, AppError> {
    let s = sqlx::query_as::<_, Source>(
        "UPDATE sources SET name=?, source_type=?, url=?, api_key=?,
                            enabled=COALESCE(?, enabled),
                            poll_interval_mins=COALESCE(?, poll_interval_mins),
                            categories=?
         WHERE id=? RETURNING *",
    )
    .bind(&body.name)
    .bind(&body.source_type)
    .bind(&body.url)
    .bind(&body.api_key)
    .bind(body.enabled)
    .bind(body.poll_interval_mins)
    .bind(&body.categories)
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(s))
}

pub async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let r = sqlx::query("DELETE FROM sources WHERE id=?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if r.rows_affected() > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

/// POST /api/sources/{id}/test — runs a live fetch with a sample query and returns raw items.
pub async fn test_source(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let source = sqlx::query_as::<_, Source>("SELECT * FROM sources WHERE id=?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    let plugin = build_source(&source, state.http.internal_insecure.clone())
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    // For search-based sources (torznab/prowlarr/newznab) use "*" so they return
    // recent items. RSS sources ignore the query entirely.
    let test_term = crate::models::SearchTerm::test_sentinel(
        if plugin.is_search_based() { "*" } else { "" },
    );

    let items = plugin
        .fetch(&test_term)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "source": source.name,
        "item_count": items.len(),
        "items": items.iter().take(10).map(|i| serde_json::json!({
            "title": i.title,
            "url": i.url,
            "pub_date": i.pub_date,
            "seeders": i.seeders,
            "indexer": i.indexer,
        })).collect::<Vec<_>>()
    })))
}

/// GET /api/sources/{id}/categories — pull the flat newznab category tree from
/// a Prowlarr source so the frontend can show a picker. Only valid for
/// `source_type == "prowlarr"`; anything else returns 400.
///
/// Calls Prowlarr's own `/api/v1/indexer` and aggregates the `capabilities.categories`
/// from every enabled indexer into a de-duplicated list of `{id, name}`.
pub async fn list_categories(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let source = sqlx::query_as::<_, Source>("SELECT * FROM sources WHERE id=?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    if source.source_type != "prowlarr" {
        return Err(AppError::BadRequest(
            "categories listing is only supported for Prowlarr sources".into(),
        ));
    }

    let api_key = source.api_key.unwrap_or_default();
    let base = source.url.trim_end_matches('/').to_string();
    let url = format!("{base}/api/v1/indexer");

    let resp = state.http.internal_insecure
        .get(&url)
        .header("X-Api-Key", &api_key)
        .send()
        .await
        .map_err(|e| AppError::BadRequest(format!("prowlarr request failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::BadRequest(format!(
            "prowlarr returned HTTP {}", resp.status()
        )));
    }

    let indexers: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| AppError::BadRequest(format!("prowlarr JSON decode failed: {e}")))?;

    // Aggregate: { id -> name }. Prowlarr's category tree is nested as
    // categories[].subCategories[]; flatten both levels.
    let mut seen: std::collections::BTreeMap<i64, String> = std::collections::BTreeMap::new();
    for ix in &indexers {
        let Some(cats) = ix.pointer("/capabilities/categories").and_then(|v| v.as_array()) else { continue };
        for c in cats {
            if let (Some(id), Some(name)) = (c.get("id").and_then(|v| v.as_i64()), c.get("name").and_then(|v| v.as_str())) {
                seen.entry(id).or_insert_with(|| name.to_string());
            }
            if let Some(subs) = c.get("subCategories").and_then(|v| v.as_array()) {
                for sc in subs {
                    if let (Some(id), Some(name)) = (sc.get("id").and_then(|v| v.as_i64()), sc.get("name").and_then(|v| v.as_str())) {
                        seen.entry(id).or_insert_with(|| name.to_string());
                    }
                }
            }
        }
    }

    let list: Vec<serde_json::Value> = seen.into_iter()
        .map(|(id, name)| serde_json::json!({ "id": id, "name": name }))
        .collect();

    Ok(Json(serde_json::json!({ "categories": list })))
}
