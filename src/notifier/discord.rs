// src/notifier/discord.rs
use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use crate::models::SearchTerm;
use crate::sources::SourceItem;

const FALLBACK_IMAGE: &str =
    "https://raw.githubusercontent.com/danktankk/discoprowl/main/assets/no-image.jpg";

pub async fn send(
    http: &Client,
    webhook_url: &str,
    term: &SearchTerm,
    item: &SourceItem,
    source_name: &str,
    steamgriddb_key: Option<&str>,
) -> Result<()> {
    let image_url = if let Some(key) = steamgriddb_key {
        fetch_box_art(http, key, &term.query).await
            .unwrap_or_else(|_| FALLBACK_IMAGE.to_string())
    } else {
        FALLBACK_IMAGE.to_string()
    };

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
        "thumbnail": { "url": image_url },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

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

/// Fetch a box art image URL from SteamGridDB for the given query.
/// Returns the URL of the first grid image found, or an error.
pub(crate) async fn fetch_box_art(http: &Client, api_key: &str, query: &str) -> Result<String> {
    let search_url = format!(
        "https://www.steamgriddb.com/api/v2/search/autocomplete/{}",
        urlencoding::encode(query)
    );
    let resp: Value = http
        .get(&search_url)
        .bearer_auth(api_key)
        .send().await?
        .json().await?;

    let game_id = resp["data"][0]["id"]
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("SteamGridDB: no game found for '{}'", query))?;

    let grids_url = format!("https://www.steamgriddb.com/api/v2/grids/game/{}", game_id);
    let grids: Value = http
        .get(&grids_url)
        .bearer_auth(api_key)
        .send().await?
        .json().await?;

    grids["data"][0]["url"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("SteamGridDB: no grid image found for game {}", game_id))
}
