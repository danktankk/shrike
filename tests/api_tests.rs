use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use std::sync::Arc;
use discoprowl::{api::{AppState, router}, config::Config, db::init_pool, notifier::Notifier};

async fn test_app() -> axum::Router {
    std::env::set_var("DATABASE_URL", "sqlite::memory:");
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let config = Arc::new(Config::from_env());
    let http = reqwest::Client::new();
    let notifier = Arc::new(Notifier::new(config.clone(), http.clone()));
    let state = AppState { pool, config, notifier, http };
    router(state)
}

#[tokio::test]
#[serial_test::serial]
async fn list_search_terms_empty() {
    let app = test_app().await;
    let resp = app
        .oneshot(Request::builder().uri("/api/search_terms").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test]
#[serial_test::serial]
async fn create_and_list_search_term() {
    let app = test_app().await;
    // Create
    let create_resp = app.clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/search_terms")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"Elden Ring","query":"elden ring","max_age_days":30}"#))
            .unwrap())
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    // List
    let list_resp = app
        .oneshot(Request::builder().uri("/api/search_terms").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = axum::body::to_bytes(list_resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["query"], "elden ring");
}

#[tokio::test]
#[serial_test::serial]
async fn create_and_list_source() {
    let app = test_app().await;
    let create_resp = app.clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/sources")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"Test Feed","source_type":"rss","url":"https://example.com/feed.xml","poll_interval_mins":720}"#))
            .unwrap())
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
    let source: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(source["source_type"], "rss");

    // List
    let list_resp = app
        .oneshot(Request::builder().uri("/api/sources").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let list_body = axum::body::to_bytes(list_resp.into_body(), usize::MAX).await.unwrap();
    let sources: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(sources.as_array().unwrap().len(), 1);
}
