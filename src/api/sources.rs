// src/api/sources.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
};
use super::AppState;
use crate::models::{Source, NewSource};
use crate::sources::build_source;

pub async fn list(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_as::<_, Source>("SELECT * FROM sources ORDER BY name ASC")
        .fetch_all(&state.pool)
        .await
    {
        Ok(sources) => Json(sources).into_response(),
        Err(e) => { tracing::error!("list sources: {e}"); StatusCode::INTERNAL_SERVER_ERROR.into_response() }
    }
}

pub async fn create(State(state): State<AppState>, Json(body): Json<NewSource>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Source>(
        "INSERT INTO sources (name, source_type, url, api_key, enabled, poll_interval_mins)
         VALUES (?, ?, ?, ?, ?, ?) RETURNING *"
    )
    .bind(&body.name).bind(&body.source_type).bind(&body.url)
    .bind(&body.api_key).bind(body.enabled.unwrap_or(true))
    .bind(body.poll_interval_mins.unwrap_or(720))
    .fetch_one(&state.pool).await;

    match result {
        Ok(s) => (StatusCode::CREATED, Json(s)).into_response(),
        Err(e) => { tracing::error!("create source: {e}"); StatusCode::INTERNAL_SERVER_ERROR.into_response() }
    }
}

pub async fn update(State(state): State<AppState>, Path(id): Path<i64>, Json(body): Json<NewSource>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Source>(
        "UPDATE sources SET name=?, source_type=?, url=?, api_key=?, enabled=?, poll_interval_mins=?
         WHERE id=? RETURNING *"
    )
    .bind(&body.name).bind(&body.source_type).bind(&body.url)
    .bind(&body.api_key).bind(body.enabled.unwrap_or(true))
    .bind(body.poll_interval_mins.unwrap_or(720)).bind(id)
    .fetch_one(&state.pool).await;

    match result {
        Ok(s) => Json(s).into_response(),
        Err(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => { tracing::error!("update source: {e}"); StatusCode::INTERNAL_SERVER_ERROR.into_response() }
    }
}

pub async fn delete_one(State(state): State<AppState>, Path(id): Path<i64>) -> StatusCode {
    match sqlx::query("DELETE FROM sources WHERE id=?").bind(id).execute(&state.pool).await {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT,
        Ok(_) => StatusCode::NOT_FOUND,
        Err(e) => { tracing::error!("delete source: {e}"); StatusCode::INTERNAL_SERVER_ERROR }
    }
}

/// POST /api/sources/{id}/test — runs a live fetch with a sample query and returns raw items.
pub async fn test_source(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    let source = match sqlx::query_as::<_, Source>("SELECT * FROM sources WHERE id=?")
        .bind(id).fetch_one(&state.pool).await
    {
        Ok(s) => s,
        Err(sqlx::Error::RowNotFound) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => { tracing::error!("test_source fetch: {e}"); return StatusCode::INTERNAL_SERVER_ERROR.into_response(); }
    };

    let plugin = match build_source(&source, state.http.clone()) {
        Some(p) => p,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "unknown source_type"}))).into_response(),
    };

    // Use a placeholder term for the test fetch
    let test_term = crate::models::SearchTerm {
        id: 0,
        name: "test".into(),
        query: String::new(),
        enabled: true,
        max_age_days: Some(30),
        disallowed_keywords: None,
        created_at: chrono::Utc::now(),
    };

    match plugin.fetch(&test_term).await {
        Ok(items) => Json(serde_json::json!({
            "source": source.name,
            "item_count": items.len(),
            "items": items.iter().take(10).map(|i| serde_json::json!({
                "title": i.title,
                "url": i.url,
                "pub_date": i.pub_date,
                "seeders": i.seeders,
                "indexer": i.indexer,
            })).collect::<Vec<_>>()
        })).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}
