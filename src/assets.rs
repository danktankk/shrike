// src/assets.rs
use axum::{
    body::Body,
    http::{header, Response, StatusCode, Uri},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct FrontendAssets;

pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try the exact path first
    if let Some(file) = FrontendAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(file.data.into_owned()))
            .unwrap();
    }

    // SPA fallback: return index.html for all non-asset paths
    if let Some(index) = FrontendAssets::get("index.html") {
        return Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(index.data.into_owned()))
            .unwrap();
    }

    StatusCode::NOT_FOUND.into_response()
}
