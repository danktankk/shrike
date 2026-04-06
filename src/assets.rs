// src/assets.rs
use axum::{http::StatusCode, response::IntoResponse};

pub async fn static_handler() -> impl IntoResponse {
    StatusCode::NOT_FOUND
}
