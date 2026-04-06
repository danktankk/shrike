// src/api/matches.rs
use axum::{
    extract::{Query, State},
    Json,
    response::IntoResponse,
    http::StatusCode,
};
use serde::Deserialize;
use super::AppState;
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
) -> impl IntoResponse {
    let limit = filter.limit.unwrap_or(100).min(500);

    let result = sqlx::query_as::<_, Match>(
        "SELECT * FROM matches
         WHERE (search_term_id = COALESCE(?, search_term_id))
           AND (source_id      = COALESCE(?, source_id))
         ORDER BY matched_at DESC
         LIMIT ?"
    )
    .bind(filter.search_term_id)
    .bind(filter.source_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await;

    match result {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => { tracing::error!("list matches: {e}"); StatusCode::INTERNAL_SERVER_ERROR.into_response() }
    }
}
