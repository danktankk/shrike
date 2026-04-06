// src/notifier/pushover.rs
use anyhow::Result;
use reqwest::Client;
use crate::models::SearchTerm;
use crate::sources::SourceItem;
use super::discord::fetch_box_art;

const FALLBACK_IMAGE: &str =
    "https://raw.githubusercontent.com/danktankk/discoprowl/main/assets/no-image.jpg";

pub async fn send(
    http: &Client,
    app_token: &str,
    user_key: &str,
    term: &SearchTerm,
    item: &SourceItem,
    steamgriddb_key: Option<&str>,
) -> Result<()> {
    let message = format!(
        "{}\nSearch term: {}\n{}",
        item.title,
        term.name,
        item.url.as_deref().unwrap_or("")
    );

    // Fetch image bytes (fall back to placeholder URL if no key or on error)
    let image_url = if let Some(key) = steamgriddb_key {
        fetch_box_art(http, key, &term.query).await
            .unwrap_or_else(|_| FALLBACK_IMAGE.to_string())
    } else {
        FALLBACK_IMAGE.to_string()
    };

    let img_bytes = http.get(&image_url).send().await?.bytes().await?;

    let form = reqwest::multipart::Form::new()
        .text("token", app_token.to_string())
        .text("user", user_key.to_string())
        .text("title", format!("DiscoProwl: {}", term.name))
        .text("message", message)
        .part(
            "attachment",
            reqwest::multipart::Part::bytes(img_bytes.to_vec())
                .file_name("cover.jpg")
                .mime_str("image/jpeg")?,
        );

    let resp = http
        .post("https://api.pushover.net/1/messages.json")
        .multipart(form)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        anyhow::bail!("Pushover returned {}", status);
    }
    Ok(())
}
