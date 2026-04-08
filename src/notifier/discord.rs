// src/notifier/discord.rs
use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use crate::models::SearchTerm;
use crate::sources::SourceItem;

pub async fn send(
    http: &Client,
    webhook_url: &str,
    term: &SearchTerm,
    item: &SourceItem,
    source_name: &str,
    image_url: Option<&str>,
) -> Result<()> {
    let mut fields = vec![
        json!({ "name": "Search Term", "value": &term.name, "inline": true }),
        json!({ "name": "Source",      "value": source_name,  "inline": true }),
    ];

    if let Some(seeders) = item.seeders {
        fields.push(json!({ "name": "Seeders", "value": seeders.to_string(), "inline": true }));
    }
    if let Some(ref indexer) = item.indexer {
        fields.push(json!({ "name": "Indexer", "value": indexer, "inline": true }));
    }

    let mut embed = json!({
        "title": item.title,
        "color": 0x2ECC71u32,
        "fields": fields,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    if let Some(url) = image_url {
        embed["thumbnail"] = json!({ "url": url });
    }

    if let Some(ref url) = item.url {
        embed["url"] = json!(url);
    }

    let payload = json!({
        "username": "DiscoProwl",
        "embeds": [embed],
    });

    let resp = http.post(webhook_url).json(&payload).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Discord webhook returned {}: {}", status, body);
    }
    Ok(())
}
