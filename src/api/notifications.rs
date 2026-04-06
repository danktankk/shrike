// src/api/notifications.rs
use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse};
use super::AppState;

pub async fn get_config(State(_s): State<AppState>) -> impl IntoResponse { StatusCode::NOT_IMPLEMENTED }
pub async fn put_config(State(_s): State<AppState>) -> impl IntoResponse { StatusCode::NOT_IMPLEMENTED }
pub async fn test_channel(State(_s): State<AppState>, Path(_channel): Path<String>) -> impl IntoResponse { StatusCode::NOT_IMPLEMENTED }
