// src/api/notifications.rs
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use super::{AppError, AppState};

#[derive(Serialize)]
pub struct NotificationConfig {
    pub discord_webhook_url: Option<String>,
    pub apprise_url: Option<String>,
    pub pushover_configured: bool,
    pub steamgriddb_configured: bool,
}

pub async fn get_config(State(state): State<AppState>) -> Json<NotificationConfig> {
    let cfg = &state.config;
    Json(NotificationConfig {
        discord_webhook_url: cfg.discord_webhook_url.as_ref().map(|u| mask_url(u)),
        apprise_url: cfg.apprise_url.as_ref().map(|u| mask_url(u)),
        pushover_configured: cfg.pushover_app_token.is_some() && cfg.pushover_user_key.is_some(),
        steamgriddb_configured: cfg.steamgriddb_api_key.is_some(),
    })
}

pub async fn test_channel(
    State(state): State<AppState>,
    Path(channel): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dummy_term = crate::models::SearchTerm::test_sentinel("test");
    let dummy_item = crate::sources::SourceItem::test_sentinel();

    match channel.as_str() {
        "discord" => {
            let url = state
                .config
                .discord_webhook_url
                .as_ref()
                .ok_or_else(|| AppError::BadRequest("Discord not configured".into()))?;
            state
                .notifier
                .send_discord(url, &dummy_term, &dummy_item, "test-source")
                .await?;
        }
        "apprise" => {
            let url = state
                .config
                .apprise_url
                .as_ref()
                .ok_or_else(|| AppError::BadRequest("Apprise not configured".into()))?;
            state
                .notifier
                .send_apprise(url, &dummy_term, &dummy_item)
                .await?;
        }
        "pushover" => {
            let (token, key) = match (
                &state.config.pushover_app_token,
                &state.config.pushover_user_key,
            ) {
                (Some(t), Some(k)) => (t, k),
                _ => return Err(AppError::BadRequest("Pushover not configured".into())),
            };
            state
                .notifier
                .send_pushover(token, key, &dummy_term, &dummy_item)
                .await?;
        }
        _ => return Err(AppError::BadRequest("Unknown channel".into())),
    }

    Ok(Json(serde_json::json!({ "ok": true, "channel": channel })))
}

/// Masks a URL by keeping `scheme://host` and replacing the path/query with `***`.
/// e.g. `https://discord.com/api/webhooks/12345/secret` -> `https://discord.com/***`
fn mask_url(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let after_scheme = scheme_end + 3;
        if let Some(rel_slash) = url[after_scheme..].find('/') {
            return format!("{}/***", &url[..after_scheme + rel_slash]);
        }
        return format!("{}/***", url);
    }
    "***".to_string()
}

#[cfg(test)]
mod tests {
    use super::mask_url;

    #[test]
    fn mask_url_hides_path() {
        assert_eq!(
            mask_url("https://discord.com/api/webhooks/12345/secret"),
            "https://discord.com/***"
        );
    }

    #[test]
    fn mask_url_no_path() {
        assert_eq!(mask_url("https://example.com"), "https://example.com/***");
    }
}
