// src/db.rs
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use anyhow::Result;

/// Full consolidated schema. Every statement is idempotent (IF NOT EXISTS),
/// so running this on a fresh DB creates everything and running it on an
/// already-migrated DB is a no-op. This replaces the previous
/// `sqlx::migrate!("./migrations")` call, which required the migrations
/// directory to exist at compile time.
const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS search_terms (
        id                  INTEGER PRIMARY KEY AUTOINCREMENT,
        name                TEXT NOT NULL,
        query               TEXT NOT NULL,
        enabled             BOOLEAN NOT NULL DEFAULT 1,
        max_age_days        INTEGER DEFAULT 30,
        disallowed_keywords TEXT,
        created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
    )",
    "CREATE TABLE IF NOT EXISTS sources (
        id                  INTEGER PRIMARY KEY AUTOINCREMENT,
        name                TEXT NOT NULL,
        source_type         TEXT NOT NULL CHECK(source_type IN ('rss','newznab','torznab','prowlarr')),
        url                 TEXT NOT NULL,
        api_key             TEXT,
        enabled             BOOLEAN NOT NULL DEFAULT 1,
        poll_interval_mins  INTEGER NOT NULL DEFAULT 720,
        last_polled_at      DATETIME,
        last_error          TEXT,
        last_success_at     TIMESTAMP,
        categories          TEXT
    )",
    "CREATE TABLE IF NOT EXISTS matches (
        id                    INTEGER PRIMARY KEY AUTOINCREMENT,
        search_term_id        INTEGER NOT NULL REFERENCES search_terms(id),
        source_id             INTEGER NOT NULL REFERENCES sources(id),
        item_title            TEXT NOT NULL,
        item_url              TEXT,
        item_guid             TEXT,
        matched_at            DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        notification_channels TEXT
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_matches_dedup
        ON matches (search_term_id, source_id, item_guid)",
];

pub async fn init_pool(database_url: &str) -> Result<SqlitePool> {
    // Create parent directory for the DB file if needed
    if database_url != ":memory:" && !database_url.starts_with("file:") {
        if let Some(parent) = std::path::Path::new(database_url).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
    }

    let connect_url = if database_url == ":memory:" {
        "sqlite::memory:".to_string()
    } else {
        format!("sqlite:{}?mode=rwc", database_url)
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&connect_url)
        .await?;

    // Apply schema. Each statement is idempotent, so this is safe on both
    // fresh databases and already-populated ones.
    for stmt in SCHEMA {
        sqlx::query(stmt).execute(&pool).await?;
    }

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_applied() {
        let pool = init_pool(":memory:").await.unwrap();
        // Verify all 3 tables exist
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('search_terms','sources','matches')"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 3);
    }
}
