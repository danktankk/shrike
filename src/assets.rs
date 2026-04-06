// src/assets.rs
use axum::http::Uri;
use axum::{http::StatusCode, response::IntoResponse};

pub async fn static_handler(_uri: Uri) -> impl IntoResponse {
    StatusCode::NOT_FOUND
}
