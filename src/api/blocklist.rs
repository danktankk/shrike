// src/api/blocklist.rs
//
// CRUD for the SGDB/Steam title blocklist (`sgdb_blocklist` table). The
// matcher itself lives in `crate::blocklist`; this is just the HTTP layer.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use super::{AppError, AppState};
use crate::models::{BlocklistEntry, BlocklistPayload};

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<BlocklistEntry>>, AppError> {
    let rows = sqlx::query_as::<_, BlocklistEntry>(
        "SELECT * FROM sgdb_blocklist ORDER BY id",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<BlocklistPayload>,
) -> Result<(StatusCode, Json<BlocklistEntry>), AppError> {
    let pat = body.pattern.trim();
    if pat.is_empty() {
        return Err(AppError::BadRequest(
            "pattern must not be empty".to_string(),
        ));
    }
    let entry = sqlx::query_as::<_, BlocklistEntry>(
        "INSERT INTO sgdb_blocklist (pattern) VALUES (?) RETURNING *",
    )
    .bind(pat)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

pub async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let r = sqlx::query("DELETE FROM sgdb_blocklist WHERE id=?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if r.rows_affected() > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
