// src/notifier/apprise.rs
use anyhow::Result;
use reqwest::Client;
use crate::models::SearchTerm;
use crate::sources::SourceItem;

pub async fn send(
    http: &Client,
    apprise_url: &str,
    term: &SearchTerm,
    item: &SourceItem,
) -> Result<()> {
    let body = format!(
        "{}\nSearch term: {}\n{}",
        item.title,
        term.name,
        item.url.as_deref().unwrap_or("(no URL)")
    );

    let resp = http
        .post(apprise_url)
        .json(&serde_json::json!({
            "title": format!("DiscoProwl: {}", term.name),
            "body": body,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Apprise returned {}: {}", status, body);
    }
    Ok(())
}
