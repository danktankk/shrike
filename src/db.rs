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
    "CREATE TABLE IF NOT EXISTS sgdb_blocklist (
        id                  INTEGER PRIMARY KEY AUTOINCREMENT,
        pattern             TEXT NOT NULL UNIQUE COLLATE NOCASE,
        created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
    )",
];

/// Idempotent ALTER TABLE additions. SQLite has no `ADD COLUMN IF NOT EXISTS`,
/// so each migration is wrapped: we swallow the "duplicate column" error and
/// surface anything else.
const COLUMN_ADDS: &[&str] = &[
    "ALTER TABLE search_terms ADD COLUMN steamgriddb_id INTEGER",
    "ALTER TABLE search_terms ADD COLUMN steam_appid INTEGER",
];

/// Default blocklist seeds inserted with `INSERT OR IGNORE` — first-run
/// boot of an existing DB picks these up; later edits via the API are
/// preserved (re-adding a removed default is a manual step).
const BLOCKLIST_SEEDS: &[&str] = &["hypervisor"];

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

    for stmt in COLUMN_ADDS {
        if let Err(e) = sqlx::query(stmt).execute(&pool).await {
            // SQLite reports "duplicate column name" when the column is
            // already present — that is the success path on an
            // already-migrated DB. Anything else is a real error.
            let msg = e.to_string();
            if !msg.contains("duplicate column name") {
                return Err(e.into());
            }
        }
    }

    for pat in BLOCKLIST_SEEDS {
        sqlx::query("INSERT OR IGNORE INTO sgdb_blocklist (pattern) VALUES (?)")
            .bind(pat)
            .execute(&pool)
            .await?;
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
        // Verify all 4 tables exist
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'
             AND name IN ('search_terms','sources','matches','sgdb_blocklist')"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 4);
    }

    #[tokio::test]
    async fn test_search_terms_has_override_columns() {
        let pool = init_pool(":memory:").await.unwrap();
        let cols: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM pragma_table_info('search_terms')",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let names: Vec<String> = cols.into_iter().map(|(n,)| n).collect();
        assert!(names.contains(&"steamgriddb_id".to_string()));
        assert!(names.contains(&"steam_appid".to_string()));
    }

    #[tokio::test]
    async fn test_blocklist_seed_present() {
        let pool = init_pool(":memory:").await.unwrap();
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sgdb_blocklist WHERE pattern = 'hypervisor'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 1);
    }

    #[tokio::test]
    async fn test_init_pool_idempotent_on_existing_db() {
        let pool = init_pool(":memory:").await.unwrap();
        // Second init via the same connect URL should no-op without error.
        // (We can't re-init the SAME :memory: pool, but the logic that
        // matters is COLUMN_ADDS — exercise it directly here.)
        for stmt in COLUMN_ADDS {
            let res = sqlx::query(stmt).execute(&pool).await;
            assert!(res.is_err(), "second ALTER should report duplicate column");
            let msg = res.unwrap_err().to_string();
            assert!(
                msg.contains("duplicate column name"),
                "unexpected error on idempotent ALTER: {msg}"
            );
        }
    }
}
