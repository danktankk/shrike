// src/api/scan.rs
use axum::{extract::{Path, State}, Json};
use super::{AppError, AppState};

/// POST /api/scan — force-polls all enabled sources immediately, ignoring schedule.
pub async fn scan_now(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = crate::scheduler::scan_now(
        &state.pool,
        &state.notifier,
        &state.http.internal_insecure,
    )
    .await?;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "matches_found": count,
    })))
}

/// POST /api/search_terms/:id/scan — force-polls all enabled sources for ONE
/// search term. Lets users smoke-test a freshly-added term without re-scanning
/// the whole list.
pub async fn scan_term(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = crate::scheduler::scan_now_term(
        &state.pool,
        &state.notifier,
        &state.http.internal_insecure,
        id,
    )
    .await?;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "matches_found": count,
    })))
}
