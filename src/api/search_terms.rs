// src/api/search_terms.rs
use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse};
use super::AppState;

pub async fn list(State(_s): State<AppState>) -> impl IntoResponse { StatusCode::NOT_IMPLEMENTED }
pub async fn create(State(_s): State<AppState>) -> impl IntoResponse { StatusCode::NOT_IMPLEMENTED }
pub async fn update(State(_s): State<AppState>, Path(_id): Path<i64>) -> impl IntoResponse { StatusCode::NOT_IMPLEMENTED }
pub async fn delete_one(State(_s): State<AppState>, Path(_id): Path<i64>) -> impl IntoResponse { StatusCode::NOT_IMPLEMENTED }
