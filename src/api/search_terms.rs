// src/api/search_terms.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
};
use super::AppState;
use crate::models::{SearchTerm, NewSearchTerm};

pub async fn list(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_as::<_, SearchTerm>("SELECT * FROM search_terms ORDER BY created_at DESC")
        .fetch_all(&state.pool)
        .await
    {
        Ok(terms) => Json(terms).into_response(),
        Err(e) => {
            tracing::error!("list search_terms: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewSearchTerm>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, SearchTerm>(
        "INSERT INTO search_terms (name, query, enabled, max_age_days, disallowed_keywords)
         VALUES (?, ?, ?, ?, ?)
         RETURNING *"
    )
    .bind(&body.name)
    .bind(&body.query)
    .bind(body.enabled.unwrap_or(true))
    .bind(body.max_age_days)
    .bind(&body.disallowed_keywords)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok(term) => (StatusCode::CREATED, Json(term)).into_response(),
        Err(e) => {
            tracing::error!("create search_term: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<NewSearchTerm>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, SearchTerm>(
        "UPDATE search_terms SET name=?, query=?, enabled=?, max_age_days=?, disallowed_keywords=?
         WHERE id=? RETURNING *"
    )
    .bind(&body.name)
    .bind(&body.query)
    .bind(body.enabled.unwrap_or(true))
    .bind(body.max_age_days)
    .bind(&body.disallowed_keywords)
    .bind(id)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok(term) => Json(term).into_response(),
        Err(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("update search_term: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> StatusCode {
    match sqlx::query("DELETE FROM search_terms WHERE id=?")
        .bind(id)
        .execute(&state.pool)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT,
        Ok(_) => StatusCode::NOT_FOUND,
        Err(e) => {
            tracing::error!("delete search_term: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
