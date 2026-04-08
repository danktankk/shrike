// src/api/search_terms.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use super::{AppError, AppState};
use crate::models::{SearchTerm, SearchTermPayload};

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<SearchTerm>>, AppError> {
    let terms = sqlx::query_as::<_, SearchTerm>(
        "SELECT * FROM search_terms ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(terms))
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<SearchTermPayload>,
) -> Result<(StatusCode, Json<SearchTerm>), AppError> {
    let term = sqlx::query_as::<_, SearchTerm>(
        "INSERT INTO search_terms (name, query, enabled, max_age_days, disallowed_keywords)
         VALUES (?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(&body.name)
    .bind(&body.query)
    .bind(body.enabled.unwrap_or(true))
    .bind(body.max_age_days)
    .bind(&body.disallowed_keywords)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(term)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<SearchTermPayload>,
) -> Result<Json<SearchTerm>, AppError> {
    let term = sqlx::query_as::<_, SearchTerm>(
        "UPDATE search_terms SET name=?, query=?,
                                 enabled=COALESCE(?, enabled),
                                 max_age_days=?, disallowed_keywords=?
         WHERE id=? RETURNING *",
    )
    .bind(&body.name)
    .bind(&body.query)
    .bind(body.enabled)
    .bind(body.max_age_days)
    .bind(&body.disallowed_keywords)
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(term))
}

pub async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let r = sqlx::query("DELETE FROM search_terms WHERE id=?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if r.rows_affected() > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
