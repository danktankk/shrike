// src/notifier/pushover.rs
use anyhow::Result;
use reqwest::Client;
use crate::models::SearchTerm;
use crate::sources::SourceItem;

pub async fn send(
    http: &Client,
    app_token: &str,
    user_key: &str,
    term: &SearchTerm,
    item: &SourceItem,
    image_url: Option<&str>,
) -> Result<()> {
    let message = format!(
        "{}\nSearch term: {}\n{}",
        item.title,
        term.name,
        item.url.as_deref().unwrap_or("")
    );

    // If we have an image URL, try to fetch it; if the fetch fails, send
    // without attachment rather than erroring the whole notification.
    let img_bytes = match image_url {
        Some(url) => match http.get(url).send().await {
            Ok(resp) => match resp.bytes().await {
                Ok(b) => Some(b),
                Err(e) => {
                    tracing::warn!("Pushover: failed to read box art bytes from {url}: {e}");
                    None
                }
            },
            Err(e) => {
                tracing::warn!("Pushover: failed to fetch box art from {url}: {e}");
                None
            }
        },
        None => None,
    };

    let mut form = reqwest::multipart::Form::new()
        .text("token", app_token.to_string())
        .text("user", user_key.to_string())
        .text("title", format!("DiscoProwl: {}", term.name))
        .text("message", message);

    if let Some(img) = img_bytes {
        form = form.part(
            "attachment",
            reqwest::multipart::Part::bytes(img.to_vec())
                .file_name("cover.jpg")
                .mime_str("image/jpeg")?,
        );
    }

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
