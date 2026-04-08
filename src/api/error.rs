// src/api/error.rs
//
// Unified error type for axum handlers. Handlers return `Result<T, AppError>`
// and use `?` instead of the old `match … Ok/Err → tracing::error! → 500`
// boilerplate. `sqlx::Error::RowNotFound` is mapped to 404 automatically so
// callers do not have to special-case it.
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

pub enum AppError {
    NotFound,
    BadRequest(String),
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND.into_response(),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": msg })),
            )
                .into_response(),
            AppError::Internal(e) => {
                tracing::error!("internal error: {e:#}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
                    .into_response()
            }
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::NotFound,
            other => AppError::Internal(other.into()),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e)
    }
}
