// src/api/matches.rs
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use super::{AppError, AppState};
use crate::models::Match;

#[derive(Deserialize)]
pub struct MatchFilter {
    pub search_term_id: Option<i64>,
    pub source_id: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(filter): Query<MatchFilter>,
) -> Result<Json<Vec<Match>>, AppError> {
    let limit = filter.limit.unwrap_or(100).min(500);

    let rows = sqlx::query_as::<_, Match>(
        "SELECT * FROM matches
         WHERE (search_term_id = COALESCE(?, search_term_id))
           AND (source_id      = COALESCE(?, source_id))
         ORDER BY matched_at DESC
         LIMIT ?",
    )
    .bind(filter.search_term_id)
    .bind(filter.source_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

/// DELETE /api/matches/{id} — remove a single match (e.g. a false-positive / bad hit).
pub async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let r = sqlx::query("DELETE FROM matches WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if r.rows_affected() > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

/// DELETE /api/matches — wipe all recorded matches. Use with care.
pub async fn delete_all(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let r = sqlx::query("DELETE FROM matches")
        .execute(&state.pool)
        .await?;
    Ok(Json(serde_json::json!({ "deleted": r.rows_affected() })))
}
