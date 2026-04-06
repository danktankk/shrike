// src/db.rs
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use anyhow::Result;

pub async fn init_pool(database_url: &str) -> Result<SqlitePool> {
    // Create parent directory for the DB file if needed
    if database_url != ":memory:" && !database_url.starts_with("file:") {
        if let Some(parent) = std::path::Path::new(database_url).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await.ok();
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

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migrations_run_clean() {
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
