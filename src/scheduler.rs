// src/scheduler.rs
use std::sync::Arc;
use tokio::time::{interval, Duration};
use sqlx::SqlitePool;
use tracing::{info, warn, error};

use crate::models::{Source, SearchTerm};
use crate::matcher::{whole_word_match, age_ok, keywords_ok};
use crate::notifier::Notifier;
use crate::sources::build_source;

/// Starts the background poll loop. Runs indefinitely — call from a spawned task.
pub async fn run(pool: SqlitePool, notifier: Arc<Notifier>, http: reqwest::Client) {
    info!("Scheduler started — polling every 60s");
    let mut tick = interval(Duration::from_secs(60));
    loop {
        tick.tick().await;
        if let Err(e) = poll_due_sources(&pool, &notifier, &http).await {
            error!("Scheduler poll error: {e}");
        }
    }
}

async fn poll_due_sources(
    pool: &SqlitePool,
    notifier: &Arc<Notifier>,
    http: &reqwest::Client,
) -> anyhow::Result<()> {
    // Find all enabled sources that are due for a poll
    let sources: Vec<Source> = sqlx::query_as(
        r#"SELECT * FROM sources
           WHERE enabled = 1
             AND (
               last_polled_at IS NULL
               OR datetime(last_polled_at, '+' || CAST(poll_interval_mins AS TEXT) || ' minutes') <= datetime('now')
             )"#
    )
    .fetch_all(pool)
    .await?;

    if sources.is_empty() {
        return Ok(());
    }

    info!("Polling {} due source(s)", sources.len());

    // Load all enabled search terms (same set for every source in this tick)
    let terms: Vec<SearchTerm> = sqlx::query_as(
        "SELECT * FROM search_terms WHERE enabled = 1"
    )
    .fetch_all(pool)
    .await?;

    if terms.is_empty() {
        info!("No enabled search terms — skipping poll");
        return Ok(());
    }

    for source in sources {
        info!("Polling source '{}' ({})", source.name, source.source_type);

        // Mark polled immediately to prevent double-dispatch if the fetch is slow
        if let Err(e) = sqlx::query(
            "UPDATE sources SET last_polled_at = datetime('now') WHERE id = ?"
        )
        .bind(source.id)
        .execute(pool)
        .await {
            error!("Failed to mark source {} as polled: {e}", source.id);
            continue;
        }

        let source_impl = match build_source(&source, http.clone()) {
            Some(s) => s,
            None => {
                warn!("Unknown source_type '{}' for source '{}' — skipping", source.source_type, source.name);
                continue;
            }
        };

        for term in &terms {
            let items = match source_impl.fetch(term).await {
                Ok(i) => i,
                Err(e) => {
                    warn!("Fetch failed for term '{}' on source '{}': {e}", term.query, source.name);
                    continue;
                }
            };

            let max_age = term.max_age_days.unwrap_or(30);
            let disallowed = term.disallowed_list();

            for item in items {
                // Apply all three filters
                if !whole_word_match(&term.query, &item.title) { continue; }
                if !age_ok(item.pub_date, max_age) { continue; }
                if !keywords_ok(&item.title, &disallowed) { continue; }

                // Notify (returns JSON array of channels fired)
                let channels = notifier.notify(term, &item, &source.name).await;

                // Log the match
                if let Err(e) = sqlx::query(
                    r#"INSERT INTO matches
                       (search_term_id, source_id, item_title, item_url, item_guid, notification_channels)
                       VALUES (?, ?, ?, ?, ?, ?)"#
                )
                .bind(term.id)
                .bind(source.id)
                .bind(&item.title)
                .bind(&item.url)
                .bind(&item.guid)
                .bind(&channels)
                .execute(pool)
                .await {
                    error!("Failed to log match '{}': {e}", item.title);
                }

                info!("Matched: '{}' for term '{}' (channels: {})", item.title, term.query, channels);
            }
        }
    }

    Ok(())
}
