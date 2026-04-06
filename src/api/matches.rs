// src/api/matches.rs
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use super::AppState;

pub async fn list(State(_s): State<AppState>) -> impl IntoResponse { StatusCode::NOT_IMPLEMENTED }
