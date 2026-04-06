// src/api/notifications.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
};
use serde::Serialize;
use super::AppState;

#[derive(Serialize)]
pub struct NotificationConfig {
    pub discord_webhook_url: Option<String>,
    pub apprise_url: Option<String>,
    pub pushover_configured: bool,
    pub steamgriddb_configured: bool,
}

pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = &state.config;
    Json(NotificationConfig {
        discord_webhook_url: cfg.discord_webhook_url.as_ref().map(|u| mask_url(u)),
        apprise_url: cfg.apprise_url.as_ref().map(|u| mask_url(u)),
        pushover_configured: cfg.pushover_app_token.is_some() && cfg.pushover_user_key.is_some(),
        steamgriddb_configured: cfg.steamgriddb_api_key.is_some(),
    })
}

pub async fn put_config(State(_s): State<AppState>) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED,
     Json(serde_json::json!({"message": "Runtime config update not yet implemented. Restart with updated env vars."})))
}

pub async fn test_channel(
    State(state): State<AppState>,
    Path(channel): Path<String>,
) -> impl IntoResponse {
    let dummy_term = crate::models::SearchTerm {
        id: 0, name: "Test".into(), query: "test".into(),
        enabled: true, max_age_days: Some(30), disallowed_keywords: None,
        created_at: chrono::Utc::now(),
    };
    let dummy_item = crate::sources::SourceItem {
        title: "DiscoProwl Test Notification".into(),
        url: Some("https://github.com/danktankk/discoprowl".into()),
        guid: "test-guid".into(),
        pub_date: Some(chrono::Utc::now()),
        description: Some("This is a test notification from DiscoProwl.".into()),
        indexer: Some("test".into()),
        seeders: Some(42),
    };

    let result = match channel.as_str() {
        "discord" => {
            if let Some(ref url) = state.config.discord_webhook_url {
                crate::notifier::discord::send(&state.http, url, &dummy_term, &dummy_item, "test-source", state.config.steamgriddb_api_key.as_deref()).await
            } else {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Discord not configured"}))).into_response();
            }
        }
        "apprise" => {
            if let Some(ref url) = state.config.apprise_url {
                crate::notifier::apprise::send(&state.http, url, &dummy_term, &dummy_item).await
            } else {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Apprise not configured"}))).into_response();
            }
        }
        "pushover" => {
            if let (Some(token), Some(key)) = (&state.config.pushover_app_token, &state.config.pushover_user_key) {
                crate::notifier::pushover::send(&state.http, token, key, &dummy_term, &dummy_item, state.config.steamgriddb_api_key.as_deref()).await
            } else {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Pushover not configured"}))).into_response();
            }
        }
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Unknown channel"}))).into_response(),
    };

    match result {
        Ok(_) => Json(serde_json::json!({"ok": true, "channel": channel})).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

fn mask_url(url: &str) -> String {
    if url.len() > 20 { format!("{}***", &url[..20]) } else { url.to_string() }
}
