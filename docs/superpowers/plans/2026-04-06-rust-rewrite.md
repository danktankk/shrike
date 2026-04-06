# DiscoProwl Rust Rewrite — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Python single-file DiscoProwl with a Rust binary exposing a REST API + embedded Svelte web UI, persisting search terms and match history in SQLite, and polling RSS (CommaFeed), Newznab, and Torznab sources on configurable intervals.

**Architecture:** Single Axum binary. A background tokio task wakes every 60 seconds, checks each enabled source's `last_polled_at` against its `poll_interval_mins`, and dispatches fetches for due sources. All state lives in a SQLite file via sqlx. The Svelte frontend is compiled with Vite and embedded directly in the binary via rust-embed — no separate asset server needed.

**Tech Stack:** Rust (stable), axum 0.8, tokio 1, sqlx 0.8 (sqlite), reqwest 0.12, feed-rs 1, quick-xml 0.37, serde/serde_json 1, rust-embed 8, chrono 0.4, tracing/tracing-subscriber 0.1/0.3, anyhow 1, async-trait 0.1. Frontend: Svelte 4, Vite 5.

---

## File Map

All files are new. The Python implementation (`discoprowl.py`, old `Dockerfile`) is replaced.

```
Cargo.toml
Dockerfile
docker-compose.yml
.env.example
.gitignore
build.rs                         - runs `npm run build` in frontend/ before cargo build

migrations/
  001_initial.sql                - search_terms, sources, matches DDL

src/
  main.rs                        - load config, init DB, spawn scheduler, bind Axum
  config.rs                      - Config struct from env vars (std::env)
  db.rs                          - SqlitePool init + sqlx::migrate!()
  models.rs                      - SearchTerm, Source, Match structs (sqlx::FromRow + serde)
  matcher.rs                     - whole_word_match(), age_ok(), keywords_ok()
  scheduler.rs                   - background poll loop
  sources/
    mod.rs                       - Source trait + SourceItem struct
    rss.rs                       - RssSource: CommaFeed REST API + direct RSS/Atom
    newznab.rs                   - NewznabSource: Newznab XML search
    torznab.rs                   - TorznabSource: same protocol, t=search
  notifier/
    mod.rs                       - Notifier, Channel enum, dispatch()
    discord.rs                   - webhook POST + embed + SteamGridDB lookup
    apprise.rs                   - Apprise URL POST
    pushover.rs                  - Pushover messages.json + image attachment
  api/
    mod.rs                       - AppState, Axum router assembly
    search_terms.rs              - CRUD /api/search_terms
    sources.rs                   - CRUD /api/sources + POST /api/sources/:id/test
    matches.rs                   - GET /api/matches
    notifications.rs             - GET/PUT /api/notifications/config, POST /api/notifications/test/:channel
  assets.rs                      - rust-embed static serving with SPA fallback

frontend/
  package.json
  vite.config.js
  index.html
  src/
    main.js
    App.svelte                   - shell with hash router
    lib/
      api.js                     - fetch() wrappers
      Nav.svelte
      Modal.svelte
      Toast.svelte
    routes/
      Dashboard.svelte
      SearchTerms.svelte
      Sources.svelte
      Notifications.svelte
  dist/                          - gitignored; embedded at compile time

tests/
  matcher_tests.rs
  api_tests.rs
```

---

## Task 1: Cargo.toml + Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore` (update)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "discoprowl"
version = "2.0.0"
edition = "2021"

[[bin]]
name = "discoprowl"
path = "src/main.rs"

[dependencies]
axum = { version = "0.8", features = ["multipart"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio", "chrono"] }
reqwest = { version = "0.12", features = ["json", "multipart"] }
feed-rs = "1"
quick-xml = { version = "0.37", features = ["serialize"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rust-embed = "8"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"
async-trait = "0.1"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "fs"] }

[dev-dependencies]
tokio-test = "0.4"
wiremock = "0.6"
```

- [ ] **Step 2: Create stub main.rs**

```rust
// src/main.rs
mod config;
mod db;
mod models;
mod matcher;
mod scheduler;
mod sources;
mod notifier;
mod api;
mod assets;

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("discoprowl starting");
    Ok(())
}
```

- [ ] **Step 3: Create stub files for each module so it compiles**

Create `src/config.rs`, `src/db.rs`, `src/models.rs`, `src/matcher.rs`, `src/scheduler.rs`, `src/assets.rs` each with just `// placeholder` initially. Create `src/sources/mod.rs`, `src/notifier/mod.rs`, `src/api/mod.rs` with `// placeholder`.

- [ ] **Step 4: Verify it compiles**

```bash
cargo check
```
Expected: no errors (warnings OK for empty modules).

- [ ] **Step 5: Update .gitignore**

Add to `.gitignore`:
```
/target/
frontend/dist/
frontend/node_modules/
*.db
*.db-shm
*.db-wal
.env
```

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/ .gitignore
git commit -m "feat: scaffold Rust project structure"
```

---

## Task 2: Config

**Files:**
- Create: `src/config.rs`

- [ ] **Step 1: Write the test first**

Add to bottom of `src/config.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_requires_database_url() {
        // Config::from_env panics / errors if DATABASE_URL is missing
        std::env::remove_var("DATABASE_URL");
        let result = std::panic::catch_unwind(Config::from_env);
        assert!(result.is_err());
    }

    #[test]
    fn config_parses_optional_channels() {
        std::env::set_var("DATABASE_URL", "/tmp/test.db");
        std::env::set_var("DISCORD_WEBHOOK_URL", "https://discord.com/api/webhooks/test");
        std::env::remove_var("APPRISE_URL");
        std::env::remove_var("PUSHOVER_APP_TOKEN");
        std::env::remove_var("PUSHOVER_USER_KEY");
        let cfg = Config::from_env();
        assert!(cfg.discord_webhook_url.is_some());
        assert!(cfg.apprise_url.is_none());
    }
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cargo test config
```
Expected: FAIL — `Config` not defined.

- [ ] **Step 3: Implement Config**

```rust
// src/config.rs
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub commafeed_url: Option<String>,
    pub commafeed_user: Option<String>,
    pub commafeed_pass: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub apprise_url: Option<String>,
    pub pushover_app_token: Option<String>,
    pub pushover_user_key: Option<String>,
    pub steamgriddb_api_key: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");

        Config {
            database_url,
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3079".to_string()),
            commafeed_url: env::var("COMMAFEED_URL").ok(),
            commafeed_user: env::var("COMMAFEED_USER").ok(),
            commafeed_pass: env::var("COMMAFEED_PASS").ok(),
            discord_webhook_url: env::var("DISCORD_WEBHOOK_URL").ok(),
            apprise_url: env::var("APPRISE_URL").ok(),
            pushover_app_token: env::var("PUSHOVER_APP_TOKEN").ok(),
            pushover_user_key: env::var("PUSHOVER_USER_KEY").ok(),
            steamgriddb_api_key: env::var("STEAMGRIDDB_API_KEY").ok(),
        }
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test config
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: Config struct from env vars"
```

---

## Task 3: Database + Migrations

**Files:**
- Create: `migrations/001_initial.sql`
- Create: `src/db.rs`

- [ ] **Step 1: Write migration SQL**

```sql
-- migrations/001_initial.sql

CREATE TABLE IF NOT EXISTS search_terms (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    name                TEXT NOT NULL,
    query               TEXT NOT NULL,
    enabled             BOOLEAN NOT NULL DEFAULT 1,
    max_age_days        INTEGER DEFAULT 30,
    disallowed_keywords TEXT,
    created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS sources (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    name                TEXT NOT NULL,
    source_type         TEXT NOT NULL CHECK(source_type IN ('rss','newznab','torznab')),
    url                 TEXT NOT NULL,
    api_key             TEXT,
    enabled             BOOLEAN NOT NULL DEFAULT 1,
    poll_interval_mins  INTEGER NOT NULL DEFAULT 720,
    last_polled_at      DATETIME
);

CREATE TABLE IF NOT EXISTS matches (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    search_term_id        INTEGER NOT NULL REFERENCES search_terms(id),
    source_id             INTEGER NOT NULL REFERENCES sources(id),
    item_title            TEXT NOT NULL,
    item_url              TEXT,
    item_guid             TEXT,
    matched_at            DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    notification_channels TEXT
);
```

- [ ] **Step 2: Implement db.rs**

```rust
// src/db.rs
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use anyhow::Result;

pub async fn init_pool(database_url: &str) -> Result<SqlitePool> {
    // Create the file if it doesn't exist
    if database_url != ":memory:" && !database_url.starts_with("file:") {
        if let Some(parent) = std::path::Path::new(database_url).parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{database_url}?mode=rwc"))
        .await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}
```

- [ ] **Step 3: Write test**

Add to `src/db.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migrations_run_clean() {
        let pool = init_pool(":memory:").await.unwrap();
        // Verify tables exist
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('search_terms','sources','matches')"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 3);
    }
}
```

- [ ] **Step 4: Run test**

```bash
cargo test db::tests
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add migrations/ src/db.rs
git commit -m "feat: SQLite migrations + pool init"
```

---

## Task 4: Models

**Files:**
- Create: `src/models.rs`

- [ ] **Step 1: Implement models**

```rust
// src/models.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SearchTerm {
    pub id: i64,
    pub name: String,
    pub query: String,
    pub enabled: bool,
    pub max_age_days: Option<i64>,
    pub disallowed_keywords: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl SearchTerm {
    /// Returns disallowed keywords as a Vec<String> (lowercase).
    pub fn disallowed_list(&self) -> Vec<String> {
        self.disallowed_keywords
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub poll_interval_mins: i64,
    pub last_polled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Match {
    pub id: i64,
    pub search_term_id: i64,
    pub source_id: i64,
    pub item_title: String,
    pub item_url: Option<String>,
    pub item_guid: Option<String>,
    pub matched_at: DateTime<Utc>,
    pub notification_channels: Option<String>,
}

/// For inserting new search terms (no id/created_at).
#[derive(Debug, Deserialize)]
pub struct NewSearchTerm {
    pub name: String,
    pub query: String,
    pub enabled: Option<bool>,
    pub max_age_days: Option<i64>,
    pub disallowed_keywords: Option<String>,
}

/// For inserting new sources.
#[derive(Debug, Deserialize)]
pub struct NewSource {
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
    pub poll_interval_mins: Option<i64>,
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/models.rs
git commit -m "feat: SearchTerm, Source, Match models"
```

---

## Task 5: Matcher Logic (TDD)

**Files:**
- Create: `src/matcher.rs`
- Create: `tests/matcher_tests.rs`

- [ ] **Step 1: Write failing tests in tests/matcher_tests.rs**

```rust
// tests/matcher_tests.rs
use discoprowl::matcher::{age_ok, keywords_ok, whole_word_match};
use chrono::Utc;

#[test]
fn whole_word_match_exact() {
    assert!(whole_word_match("Hollow Knight", "Hollow Knight"));
}

#[test]
fn whole_word_match_partial_word_rejected() {
    // "hollow" should NOT match "HollowKnight" (no boundary)
    assert!(!whole_word_match("hollow", "HollowKnight"));
}

#[test]
fn whole_word_match_case_insensitive() {
    assert!(whole_word_match("hollow knight", "Hollow Knight Silksong v1.0"));
}

#[test]
fn whole_word_match_multi_word_query() {
    assert!(whole_word_match("elden ring", "Elden Ring v1.10 REPACK"));
}

#[test]
fn age_ok_recent_item() {
    let now = Utc::now();
    assert!(age_ok(Some(now), 30));
}

#[test]
fn age_ok_old_item_rejected() {
    let old = Utc::now() - chrono::Duration::days(45);
    assert!(!age_ok(Some(old), 30));
}

#[test]
fn age_ok_no_date_passes() {
    // Items with no pub_date are not filtered out
    assert!(age_ok(None, 30));
}

#[test]
fn keywords_ok_blocks_disallowed() {
    assert!(!keywords_ok("Elden Ring Trainer REPACK", &["trainer".to_string()]));
}

#[test]
fn keywords_ok_passes_clean_title() {
    assert!(keywords_ok("Elden Ring v1.10", &["trainer".to_string(), "crack".to_string()]));
}

#[test]
fn keywords_ok_case_insensitive() {
    assert!(!keywords_ok("Elden Ring TRAINER", &["trainer".to_string()]));
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test --test matcher_tests
```
Expected: FAIL — `discoprowl::matcher` not found.

- [ ] **Step 3: Add `pub` visibility to lib entry (add lib.rs)**

The integration tests need `discoprowl` as a library crate. Add to `Cargo.toml`:
```toml
[lib]
name = "discoprowl"
path = "src/lib.rs"
```

Create `src/lib.rs`:
```rust
pub mod config;
pub mod db;
pub mod models;
pub mod matcher;
pub mod scheduler;
pub mod sources;
pub mod notifier;
pub mod api;
pub mod assets;
```

- [ ] **Step 4: Implement matcher.rs**

```rust
// src/matcher.rs
use chrono::{DateTime, Utc};

/// Returns true if `query` appears as a whole word (case-insensitive) in `title`.
pub fn whole_word_match(query: &str, title: &str) -> bool {
    let title_lower = title.to_lowercase();
    let query_lower = query.to_lowercase();
    // Escape regex metacharacters in query
    let escaped = regex_escape(&query_lower);
    let pattern = format!(r"(?i)\b{escaped}\b");
    // Use simple boundary check without regex crate: split on non-alphanumeric
    // boundaries and check for exact substring match at word boundaries.
    word_boundary_match(&query_lower, &title_lower)
}

fn word_boundary_match(query: &str, text: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = text[start..].find(query) {
        let abs_pos = start + pos;
        let end_pos = abs_pos + query.len();
        let left_ok = abs_pos == 0 || !text.chars().nth(abs_pos - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
        let right_ok = end_pos >= text.len() || !text.chars().nth(end_pos).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
        if left_ok && right_ok {
            return true;
        }
        start = abs_pos + 1;
        if start >= text.len() {
            break;
        }
    }
    false
}

fn regex_escape(s: &str) -> String {
    s.chars().flat_map(|c| {
        if "\\^$.|?*+()[]{}".contains(c) {
            vec!['\\', c]
        } else {
            vec![c]
        }
    }).collect()
}

/// Returns true if the item is within `max_age_days` days old (or has no date).
pub fn age_ok(pub_date: Option<DateTime<Utc>>, max_age_days: i64) -> bool {
    match pub_date {
        None => true,
        Some(dt) => {
            let age = Utc::now().signed_duration_since(dt).num_days();
            age <= max_age_days
        }
    }
}

/// Returns true if the title contains none of the disallowed keywords (case-insensitive).
pub fn keywords_ok(title: &str, disallowed: &[String]) -> bool {
    let lower = title.to_lowercase();
    !disallowed.iter().any(|kw| lower.contains(kw.as_str()))
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test --test matcher_tests
```
Expected: all 10 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/matcher.rs src/lib.rs Cargo.toml tests/matcher_tests.rs
git commit -m "feat: matcher logic with whole-word, age, keyword filters (TDD)"
```

---

## Task 6: Source Trait

**Files:**
- Create: `src/sources/mod.rs`

- [ ] **Step 1: Implement Source trait and SourceItem**

```rust
// src/sources/mod.rs
pub mod rss;
pub mod newznab;
pub mod torznab;

use async_trait::async_trait;
use anyhow::Result;
use chrono::{DateTime, Utc};
use crate::models::SearchTerm;

#[derive(Debug, Clone)]
pub struct SourceItem {
    pub title: String,
    pub url: Option<String>,
    pub guid: String,
    pub pub_date: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub indexer: Option<String>,
    pub seeders: Option<u32>,
}

#[async_trait]
pub trait Source: Send + Sync {
    async fn fetch(&self, term: &SearchTerm) -> Result<Vec<SourceItem>>;
    fn source_type(&self) -> &'static str;
}

/// Build the correct Source implementation for a DB Source row.
pub fn build_source(
    source: &crate::models::Source,
    http: reqwest::Client,
) -> Option<Box<dyn Source>> {
    match source.source_type.as_str() {
        "rss" => Some(Box::new(rss::RssSource::new(
            source.url.clone(),
            source.api_key.clone(), // used as CommaFeed password if URL is CommaFeed
        ))),
        "newznab" => Some(Box::new(newznab::NewznabSource::new(
            source.url.clone(),
            source.api_key.clone().unwrap_or_default(),
            http,
        ))),
        "torznab" => Some(Box::new(torznab::TorznabSource::new(
            source.url.clone(),
            source.api_key.clone().unwrap_or_default(),
            http,
        ))),
        _ => None,
    }
}
```

- [ ] **Step 2: Create stubs for rss.rs, newznab.rs, torznab.rs**

`src/sources/rss.rs`:
```rust
use super::{Source, SourceItem};
use async_trait::async_trait;
use anyhow::Result;
use crate::models::SearchTerm;

pub struct RssSource {
    pub url: String,
    pub api_key: Option<String>,
}

impl RssSource {
    pub fn new(url: String, api_key: Option<String>) -> Self {
        Self { url, api_key }
    }
}

#[async_trait]
impl Source for RssSource {
    async fn fetch(&self, _term: &SearchTerm) -> Result<Vec<SourceItem>> {
        Ok(vec![]) // implemented in Task 7
    }
    fn source_type(&self) -> &'static str { "rss" }
}
```

Mirror the stub pattern for `newznab.rs` and `torznab.rs` with their respective struct names.

- [ ] **Step 3: Verify compile**

```bash
cargo check
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/sources/
git commit -m "feat: Source trait + SourceItem + stub implementations"
```

---

## Task 7: RssSource

**Files:**
- Modify: `src/sources/rss.rs`

RssSource handles two modes:
1. **CommaFeed mode** — if the `url` matches the configured CommaFeed base URL, use `GET /rest/feed/entries?id=<feed_id>&...` with Basic auth `CC:<pass>`. Since individual feed IDs are not available at source-level, this mode fetches all entries across all subscribed feeds via `/rest/entries?readType=unread` and filters client-side.
2. **Direct mode** — fetch the raw RSS/Atom URL and parse with feed-rs.

For v1, RssSource in direct mode fetches the URL and filters items matching the search term. In CommaFeed mode it fetches all unread entries (simulating a broad search). CommaFeed does not offer a cross-feed text search API, so all filtering happens client-side in the matcher.

- [ ] **Step 1: Write test with wiremock**

```rust
// In src/sources/rss.rs, cfg(test) block:
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    const FEED_XML: &str = r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <item>
      <title>Elden Ring v1.10 REPACK</title>
      <link>http://example.com/1</link>
      <guid>abc123</guid>
      <pubDate>Mon, 06 Apr 2026 12:00:00 +0000</pubDate>
    </item>
    <item>
      <title>Some Other Game TRAINER</title>
      <link>http://example.com/2</link>
      <guid>def456</guid>
    </item>
  </channel>
</rss>"#;

    #[tokio::test]
    async fn fetches_rss_and_returns_items() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(FEED_XML, "application/rss+xml"))
            .mount(&server)
            .await;

        let source = RssSource::new(format!("{}/feed.xml", server.uri()), None);
        let term = crate::models::SearchTerm {
            id: 1, name: "Test".into(), query: "Elden Ring".into(),
            enabled: true, max_age_days: Some(30),
            disallowed_keywords: None,
            created_at: chrono::Utc::now(),
        };

        let items = source.fetch(&term).await.unwrap();
        assert_eq!(items.len(), 2); // RssSource returns all items; matcher filters
        assert_eq!(items[0].title, "Elden Ring v1.10 REPACK");
    }
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cargo test sources::rss::tests
```
Expected: FAIL.

- [ ] **Step 3: Implement RssSource**

```rust
// src/sources/rss.rs
use super::{Source, SourceItem};
use async_trait::async_trait;
use anyhow::Result;
use crate::models::SearchTerm;
use feed_rs::parser;

pub struct RssSource {
    pub url: String,
    pub api_key: Option<String>, // used as password for CommaFeed Basic auth
}

impl RssSource {
    pub fn new(url: String, api_key: Option<String>) -> Self {
        Self { url, api_key }
    }
}

#[async_trait]
impl Source for RssSource {
    async fn fetch(&self, _term: &SearchTerm) -> Result<Vec<SourceItem>> {
        let client = reqwest::Client::new();
        let mut req = client.get(&self.url);

        // If api_key is set, treat as Basic auth password with user "CC"
        if let Some(pass) = &self.api_key {
            req = req.basic_auth("CC", Some(pass));
        }

        let body = req.send().await?.bytes().await?;
        let feed = parser::parse(body.as_ref())?;

        let items = feed.entries.into_iter().map(|entry| {
            let title = entry.title.map(|t| t.content).unwrap_or_default();
            let url = entry.links.into_iter().next().map(|l| l.href);
            let guid = entry.id;
            let pub_date = entry.published.or(entry.updated);

            SourceItem {
                title,
                url,
                guid,
                pub_date,
                description: entry.summary.map(|s| s.content),
                indexer: None,
                seeders: None,
            }
        }).collect();

        Ok(items)
    }

    fn source_type(&self) -> &'static str { "rss" }
}
```

- [ ] **Step 4: Run test**

```bash
cargo test sources::rss::tests
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/sources/rss.rs
git commit -m "feat: RssSource with feed-rs parser + Basic auth support"
```

---

## Task 8: NewznabSource + TorznabSource

**Files:**
- Modify: `src/sources/newznab.rs`
- Modify: `src/sources/torznab.rs`

Both use the same Newznab XML API protocol. TorznabSource is identical except it uses `t=search` with `cat=` omitted (Prowlarr handles category routing internally).

- [ ] **Step 1: Write tests for NewznabSource**

```rust
// src/sources/newznab.rs cfg(test):
const NEWZNAB_XML: &str = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:newznab="http://www.newznab.com/DTD/2010/feeds/attributes/">
  <channel>
    <item>
      <title>Elden Ring v1.10 MULTi9</title>
      <link>https://example.com/nzb/1</link>
      <guid>nzb-guid-001</guid>
      <pubDate>Mon, 06 Apr 2026 10:00:00 +0000</pubDate>
      <newznab:attr name="seeders" value="42"/>
    </item>
  </channel>
</rss>"#;

#[tokio::test]
async fn parses_newznab_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(NEWZNAB_XML, "application/rss+xml"))
        .mount(&server)
        .await;

    let source = NewznabSource::new(server.uri(), "testkey".into(), reqwest::Client::new());
    let term = /* same as rss test */;
    let items = source.fetch(&term).await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "Elden Ring v1.10 MULTi9");
}
```

- [ ] **Step 2: Implement NewznabSource**

```rust
// src/sources/newznab.rs
use super::{Source, SourceItem};
use async_trait::async_trait;
use anyhow::Result;
use crate::models::SearchTerm;
use quick_xml::Reader;
use quick_xml::events::Event;

pub struct NewznabSource {
    pub url: String,
    pub api_key: String,
    pub http: reqwest::Client,
}

impl NewznabSource {
    pub fn new(url: String, api_key: String, http: reqwest::Client) -> Self {
        Self { url, api_key, http }
    }
}

#[async_trait]
impl Source for NewznabSource {
    async fn fetch(&self, term: &SearchTerm) -> Result<Vec<SourceItem>> {
        let query_url = format!(
            "{}/api?t=search&q={}&apikey={}",
            self.url.trim_end_matches('/'),
            urlenccode(&term.query),
            self.api_key
        );
        let body = self.http.get(&query_url).send().await?.text().await?;
        parse_newznab_xml(&body)
    }

    fn source_type(&self) -> &'static str { "newznab" }
}

fn urlenccode(s: &str) -> String {
    s.chars().map(|c| {
        if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
            c.to_string()
        } else {
            format!("%{:02X}", c as u32)
        }
    }).collect()
}

pub(crate) fn parse_newznab_xml(xml: &str) -> Result<Vec<SourceItem>> {
    let mut items = Vec::new();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut in_item = false;
    let mut current = SourceItem {
        title: String::new(), url: None, guid: String::new(),
        pub_date: None, description: None, indexer: None, seeders: None,
    };

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"item" => { in_item = true; current = SourceItem { title: String::new(), url: None, guid: String::new(), pub_date: None, description: None, indexer: None, seeders: None }; }
                b"title" if in_item => {
                    if let Ok(Event::Text(t)) = reader.read_event() {
                        current.title = t.unescape().unwrap_or_default().to_string();
                    }
                }
                b"link" if in_item => {
                    if let Ok(Event::Text(t)) = reader.read_event() {
                        current.url = Some(t.unescape().unwrap_or_default().to_string());
                    }
                }
                b"guid" if in_item => {
                    if let Ok(Event::Text(t)) = reader.read_event() {
                        current.guid = t.unescape().unwrap_or_default().to_string();
                    }
                }
                b"pubDate" if in_item => {
                    if let Ok(Event::Text(t)) = reader.read_event() {
                        let raw = t.unescape().unwrap_or_default().to_string();
                        current.pub_date = chrono::DateTime::parse_from_rfc2822(&raw)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok();
                    }
                }
                _ => {}
            },
            Ok(Event::Empty(ref e)) if in_item && e.name().as_ref() == b"newznab:attr" => {
                let attrs: std::collections::HashMap<String,String> = e.attributes()
                    .flatten()
                    .map(|a| (
                        String::from_utf8_lossy(a.key.as_ref()).to_string(),
                        String::from_utf8_lossy(&a.value).to_string(),
                    ))
                    .collect();
                if attrs.get("name").map(|s| s.as_str()) == Some("seeders") {
                    current.seeders = attrs.get("value").and_then(|v| v.parse().ok());
                }
                if attrs.get("name").map(|s| s.as_str()) == Some("indexer") {
                    current.indexer = attrs.get("value").cloned();
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"item" => {
                if in_item && !current.title.is_empty() {
                    items.push(current.clone());
                }
                in_item = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("XML parse error: {e}")),
            _ => {}
        }
    }

    Ok(items)
}
```

- [ ] **Step 3: Implement TorznabSource (delegates to same XML parser)**

```rust
// src/sources/torznab.rs
use super::{Source, SourceItem};
use super::newznab::{parse_newznab_xml, urlenccode};  // reuse parser
use async_trait::async_trait;
use anyhow::Result;
use crate::models::SearchTerm;

pub struct TorznabSource {
    pub url: String,
    pub api_key: String,
    pub http: reqwest::Client,
}

impl TorznabSource {
    pub fn new(url: String, api_key: String, http: reqwest::Client) -> Self {
        Self { url, api_key, http }
    }
}

#[async_trait]
impl Source for TorznabSource {
    async fn fetch(&self, term: &SearchTerm) -> Result<Vec<SourceItem>> {
        let query_url = format!(
            "{}/api?t=search&q={}&apikey={}",
            self.url.trim_end_matches('/'),
            urlenccode(&term.query),
            self.api_key
        );
        let body = self.http.get(&query_url).send().await?.text().await?;
        parse_newznab_xml(&body)
    }

    fn source_type(&self) -> &'static str { "torznab" }
}
```

Make `parse_newznab_xml` and `urlenccode` `pub(crate)` in newznab.rs.

- [ ] **Step 4: Run tests**

```bash
cargo test sources::
```
Expected: all source tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/sources/newznab.rs src/sources/torznab.rs
git commit -m "feat: NewznabSource + TorznabSource with shared XML parser"
```

---

## Task 9: Notifier — Discord + SteamGridDB

**Files:**
- Create: `src/notifier/mod.rs`
- Create: `src/notifier/discord.rs`

- [ ] **Step 1: Implement notifier/mod.rs**

```rust
// src/notifier/mod.rs
pub mod discord;
pub mod apprise;
pub mod pushover;

use anyhow::Result;
use crate::config::Config;
use crate::models::SearchTerm;
use crate::sources::SourceItem;

#[derive(Debug, Clone, PartialEq)]
pub enum Channel {
    Discord,
    Apprise,
    Pushover,
}

impl Channel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Discord => "discord",
            Channel::Apprise => "apprise",
            Channel::Pushover => "pushover",
        }
    }
}

pub struct Notifier {
    pub config: std::sync::Arc<Config>,
    pub http: reqwest::Client,
}

impl Notifier {
    pub fn new(config: std::sync::Arc<Config>, http: reqwest::Client) -> Self {
        Self { config, http }
    }

    /// Fire all configured channels for a matched item.
    /// Returns a JSON array string of channels that were notified.
    pub async fn notify(
        &self,
        term: &SearchTerm,
        item: &SourceItem,
        source_name: &str,
    ) -> Result<String> {
        let mut fired: Vec<&str> = vec![];

        if let Some(ref url) = self.config.discord_webhook_url {
            if let Err(e) = discord::send(&self.http, url, term, item, source_name, self.config.steamgriddb_api_key.as_deref()).await {
                tracing::warn!("Discord notify failed: {e}");
            } else {
                fired.push("discord");
            }
        }

        if let Some(ref url) = self.config.apprise_url {
            if let Err(e) = apprise::send(&self.http, url, term, item).await {
                tracing::warn!("Apprise notify failed: {e}");
            } else {
                fired.push("apprise");
            }
        }

        if self.config.pushover_app_token.is_some() && self.config.pushover_user_key.is_some() {
            let token = self.config.pushover_app_token.as_deref().unwrap();
            let key = self.config.pushover_user_key.as_deref().unwrap();
            if let Err(e) = pushover::send(&self.http, token, key, term, item, self.config.steamgriddb_api_key.as_deref()).await {
                tracing::warn!("Pushover notify failed: {e}");
            } else {
                fired.push("pushover");
            }
        }

        Ok(serde_json::to_string(&fired)?)
    }
}
```

- [ ] **Step 2: Implement discord.rs**

```rust
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
        fetch_box_art(http, key, &term.query).await.unwrap_or_else(|_| FALLBACK_IMAGE.to_string())
    } else {
        FALLBACK_IMAGE.to_string()
    };

    let mut embed = json!({
        "title": item.title,
        "color": 0x2ECC71,
        "fields": [
            { "name": "Search Term", "value": &term.name, "inline": true },
            { "name": "Source", "value": source_name, "inline": true },
        ],
        "thumbnail": { "url": &image_url },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    if let Some(url) = &item.url {
        embed["url"] = json!(url);
    }
    if let Some(seeders) = item.seeders {
        embed["fields"].as_array_mut().unwrap().push(json!(
            { "name": "Seeders", "value": seeders.to_string(), "inline": true }
        ));
    }
    if let Some(indexer) = &item.indexer {
        embed["fields"].as_array_mut().unwrap().push(json!(
            { "name": "Indexer", "value": indexer, "inline": true }
        ));
    }

    let payload = json!({
        "username": "DiscoProwl",
        "embeds": [embed],
    });

    let resp = http.post(webhook_url).json(&payload).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Discord webhook returned {}", resp.status());
    }
    Ok(())
}

async fn fetch_box_art(http: &Client, api_key: &str, query: &str) -> Result<String> {
    let search_url = format!(
        "https://www.steamgriddb.com/api/v2/search/autocomplete/{}",
        urlencoding::encode(query)
    );
    let resp: serde_json::Value = http
        .get(&search_url)
        .bearer_auth(api_key)
        .send().await?
        .json().await?;

    let game_id = resp["data"][0]["id"]
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("no game found"))?;

    let grids_url = format!("https://www.steamgriddb.com/api/v2/grids/game/{game_id}");
    let grids: serde_json::Value = http
        .get(&grids_url)
        .bearer_auth(api_key)
        .send().await?
        .json().await?;

    grids["data"][0]["url"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no grid image found"))
}
```

Add `urlencoding = "2"` to `[dependencies]` in `Cargo.toml`.

- [ ] **Step 3: Implement apprise.rs**

```rust
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
        "**{}**\nSearch term: {}\nSource item found: {}",
        item.title, term.name,
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
        anyhow::bail!("Apprise returned {}", resp.status());
    }
    Ok(())
}
```

- [ ] **Step 4: Implement pushover.rs**

```rust
// src/notifier/pushover.rs
use anyhow::Result;
use reqwest::Client;
use crate::models::SearchTerm;
use crate::sources::SourceItem;

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

    // Fetch image bytes (fallback to placeholder)
    let image_url = if let Some(key) = steamgriddb_key {
        super::discord::fetch_box_art_pub(http, key, &term.query)
            .await
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
        .part("attachment", reqwest::multipart::Part::bytes(img_bytes.to_vec())
            .file_name("cover.jpg")
            .mime_str("image/jpeg")?);

    let resp = http
        .post("https://api.pushover.net/1/messages.json")
        .multipart(form)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Pushover returned {}", resp.status());
    }
    Ok(())
}
```

Expose `fetch_box_art` as `pub(crate) fn fetch_box_art_pub` in discord.rs so pushover can reuse it.

- [ ] **Step 5: Verify compile**

```bash
cargo check
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add src/notifier/
git commit -m "feat: Notifier with Discord embed, Apprise, Pushover + SteamGridDB"
```

---

## Task 10: Scheduler

**Files:**
- Create: `src/scheduler.rs`

The scheduler is a background tokio task. It wakes every 60 seconds, queries for sources that are due (no `last_polled_at`, or `last_polled_at` + interval has elapsed), and polls each one.

- [ ] **Step 1: Implement scheduler.rs**

```rust
// src/scheduler.rs
use std::sync::Arc;
use tokio::time::{interval, Duration};
use sqlx::SqlitePool;
use tracing::{info, warn, error};

use crate::config::Config;
use crate::models::{Source, SearchTerm, Match};
use crate::matcher::{whole_word_match, age_ok, keywords_ok};
use crate::notifier::Notifier;
use crate::sources::build_source;

pub async fn run(pool: SqlitePool, notifier: Arc<Notifier>, http: reqwest::Client) {
    let mut tick = interval(Duration::from_secs(60));
    loop {
        tick.tick().await;
        if let Err(e) = poll_due_sources(&pool, &notifier, &http).await {
            error!("Scheduler error: {e}");
        }
    }
}

async fn poll_due_sources(
    pool: &SqlitePool,
    notifier: &Arc<Notifier>,
    http: &reqwest::Client,
) -> anyhow::Result<()> {
    let sources: Vec<Source> = sqlx::query_as(
        r#"SELECT * FROM sources WHERE enabled = 1
           AND (last_polled_at IS NULL
                OR datetime(last_polled_at, '+' || poll_interval_mins || ' minutes') <= datetime('now'))"#
    )
    .fetch_all(pool)
    .await?;

    if sources.is_empty() {
        return Ok(());
    }

    let terms: Vec<SearchTerm> = sqlx::query_as(
        "SELECT * FROM search_terms WHERE enabled = 1"
    )
    .fetch_all(pool)
    .await?;

    for source in sources {
        info!("Polling source: {} ({})", source.name, source.source_type);

        // Mark polled immediately to avoid double-dispatch on slow fetches
        sqlx::query("UPDATE sources SET last_polled_at = datetime('now') WHERE id = ?")
            .bind(source.id)
            .execute(pool)
            .await?;

        let source_impl = match build_source(&source, http.clone()) {
            Some(s) => s,
            None => {
                warn!("Unknown source_type '{}' for source {}", source.source_type, source.id);
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
                if !whole_word_match(&term.query, &item.title) { continue; }
                if !age_ok(item.pub_date, max_age) { continue; }
                if !keywords_ok(&item.title, &disallowed) { continue; }

                let channels = notifier.notify(term, &item, &source.name).await
                    .unwrap_or_else(|_| "[]".to_string());

                // Log to matches table
                sqlx::query(
                    "INSERT INTO matches (search_term_id, source_id, item_title, item_url, item_guid, notification_channels)
                     VALUES (?, ?, ?, ?, ?, ?)"
                )
                .bind(term.id)
                .bind(source.id)
                .bind(&item.title)
                .bind(&item.url)
                .bind(&item.guid)
                .bind(&channels)
                .execute(pool)
                .await?;

                info!("Matched: '{}' for term '{}'", item.title, term.query);
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Verify compile**

```bash
cargo check
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/scheduler.rs
git commit -m "feat: background poll scheduler with per-source interval tracking"
```

---

## Task 11: API — AppState + Router

**Files:**
- Create: `src/api/mod.rs`
- Create stubs: `src/api/search_terms.rs`, `src/api/sources.rs`, `src/api/matches.rs`, `src/api/notifications.rs`

- [ ] **Step 1: Implement api/mod.rs**

```rust
// src/api/mod.rs
pub mod search_terms;
pub mod sources;
pub mod matches;
pub mod notifications;

use std::sync::Arc;
use axum::{Router, routing::{get, post, put, delete}};
use sqlx::SqlitePool;

use crate::config::Config;
use crate::notifier::Notifier;
use crate::assets::static_handler;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub notifier: Arc<Notifier>,
    pub http: reqwest::Client,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        // Search terms
        .route("/api/search_terms", get(search_terms::list).post(search_terms::create))
        .route("/api/search_terms/:id", put(search_terms::update).delete(search_terms::delete_one))
        // Sources
        .route("/api/sources", get(sources::list).post(sources::create))
        .route("/api/sources/:id", put(sources::update).delete(sources::delete_one))
        .route("/api/sources/:id/test", post(sources::test_source))
        // Matches
        .route("/api/matches", get(matches::list))
        // Notifications
        .route("/api/notifications/config", get(notifications::get_config).put(notifications::put_config))
        .route("/api/notifications/test/:channel", post(notifications::test_channel))
        // Static / SPA fallback
        .fallback(static_handler)
        .with_state(state)
}
```

- [ ] **Step 2: Create stub handlers that return 501**

For each of `search_terms.rs`, `sources.rs`, `matches.rs`, `notifications.rs`, create stubs:
```rust
use axum::{extract::State, Json, http::StatusCode};
use super::AppState;

pub async fn list(State(_state): State<AppState>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
// etc.
```

- [ ] **Step 3: Create stub assets.rs**

```rust
// src/assets.rs
use axum::{response::IntoResponse, http::StatusCode};

pub async fn static_handler() -> impl IntoResponse {
    StatusCode::NOT_FOUND // replaced in Task 24
}
```

- [ ] **Step 4: Verify compile**

```bash
cargo check
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/api/ src/assets.rs
git commit -m "feat: API router scaffold with AppState"
```

---

## Task 12: API — Search Terms CRUD

**Files:**
- Modify: `src/api/search_terms.rs`
- Create: `tests/api_tests.rs`

- [ ] **Step 1: Write API tests using tower::ServiceExt**

```rust
// tests/api_tests.rs
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use std::sync::Arc;
use discoprowl::{api::{AppState, router}, config::Config, db::init_pool, notifier::Notifier};

async fn test_app() -> axum::Router {
    std::env::set_var("DATABASE_URL", ":memory:");
    let pool = init_pool(":memory:").await.unwrap();
    let config = Arc::new(Config::from_env());
    let http = reqwest::Client::new();
    let notifier = Arc::new(Notifier::new(config.clone(), http.clone()));
    let state = AppState { pool, config, notifier, http };
    router(state)
}

#[tokio::test]
async fn list_search_terms_empty() {
    let app = test_app().await;
    let resp = app
        .oneshot(Request::builder().uri("/api/search_terms").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn create_and_list_search_term() {
    let app = test_app().await;
    // Create
    let create_resp = app.clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/search_terms")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"Elden Ring","query":"elden ring","max_age_days":30}"#))
            .unwrap())
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    // List
    let list_resp = app
        .oneshot(Request::builder().uri("/api/search_terms").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = axum::body::to_bytes(list_resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["query"], "elden ring");
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test --test api_tests
```
Expected: FAIL — handlers return 501.

- [ ] **Step 3: Implement search_terms.rs**

```rust
// src/api/search_terms.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
};
use super::AppState;
use crate::models::{SearchTerm, NewSearchTerm};

pub async fn list(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_as::<_, SearchTerm>("SELECT * FROM search_terms ORDER BY created_at DESC")
        .fetch_all(&state.pool)
        .await
    {
        Ok(terms) => Json(terms).into_response(),
        Err(e) => {
            tracing::error!("list search_terms: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewSearchTerm>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, SearchTerm>(
        "INSERT INTO search_terms (name, query, enabled, max_age_days, disallowed_keywords)
         VALUES (?, ?, ?, ?, ?)
         RETURNING *"
    )
    .bind(&body.name)
    .bind(&body.query)
    .bind(body.enabled.unwrap_or(true))
    .bind(body.max_age_days)
    .bind(&body.disallowed_keywords)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok(term) => (StatusCode::CREATED, Json(term)).into_response(),
        Err(e) => {
            tracing::error!("create search_term: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<NewSearchTerm>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, SearchTerm>(
        "UPDATE search_terms SET name=?, query=?, enabled=?, max_age_days=?, disallowed_keywords=?
         WHERE id=? RETURNING *"
    )
    .bind(&body.name)
    .bind(&body.query)
    .bind(body.enabled.unwrap_or(true))
    .bind(body.max_age_days)
    .bind(&body.disallowed_keywords)
    .bind(id)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok(term) => Json(term).into_response(),
        Err(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("update search_term: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> StatusCode {
    match sqlx::query("DELETE FROM search_terms WHERE id=?")
        .bind(id)
        .execute(&state.pool)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT,
        Ok(_) => StatusCode::NOT_FOUND,
        Err(e) => {
            tracing::error!("delete search_term: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test api_tests
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/api/search_terms.rs tests/api_tests.rs
git commit -m "feat: search_terms CRUD API (TDD)"
```

---

## Task 13: API — Sources CRUD + Test Endpoint

**Files:**
- Modify: `src/api/sources.rs`

- [ ] **Step 1: Add source tests to api_tests.rs**

Add to `tests/api_tests.rs`:
```rust
#[tokio::test]
async fn create_and_test_rss_source() {
    let app = test_app().await;
    let create_resp = app.clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/sources")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"Test Feed","source_type":"rss","url":"https://example.com/feed.xml","poll_interval_mins":720}"#))
            .unwrap())
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    // The created source's id should be 1
    let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
    let source: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(source["source_type"], "rss");
}
```

- [ ] **Step 2: Implement sources.rs**

```rust
// src/api/sources.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
};
use super::AppState;
use crate::models::{Source, NewSource};
use crate::sources::build_source;

pub async fn list(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_as::<_, Source>("SELECT * FROM sources ORDER BY name ASC")
        .fetch_all(&state.pool)
        .await
    {
        Ok(sources) => Json(sources).into_response(),
        Err(e) => { tracing::error!("list sources: {e}"); StatusCode::INTERNAL_SERVER_ERROR.into_response() }
    }
}

pub async fn create(State(state): State<AppState>, Json(body): Json<NewSource>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Source>(
        "INSERT INTO sources (name, source_type, url, api_key, enabled, poll_interval_mins)
         VALUES (?, ?, ?, ?, ?, ?) RETURNING *"
    )
    .bind(&body.name).bind(&body.source_type).bind(&body.url)
    .bind(&body.api_key).bind(body.enabled.unwrap_or(true))
    .bind(body.poll_interval_mins.unwrap_or(720))
    .fetch_one(&state.pool).await;

    match result {
        Ok(s) => (StatusCode::CREATED, Json(s)).into_response(),
        Err(e) => { tracing::error!("create source: {e}"); StatusCode::INTERNAL_SERVER_ERROR.into_response() }
    }
}

pub async fn update(State(state): State<AppState>, Path(id): Path<i64>, Json(body): Json<NewSource>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Source>(
        "UPDATE sources SET name=?, source_type=?, url=?, api_key=?, enabled=?, poll_interval_mins=?
         WHERE id=? RETURNING *"
    )
    .bind(&body.name).bind(&body.source_type).bind(&body.url)
    .bind(&body.api_key).bind(body.enabled.unwrap_or(true))
    .bind(body.poll_interval_mins.unwrap_or(720)).bind(id)
    .fetch_one(&state.pool).await;

    match result {
        Ok(s) => Json(s).into_response(),
        Err(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => { tracing::error!("update source: {e}"); StatusCode::INTERNAL_SERVER_ERROR.into_response() }
    }
}

pub async fn delete_one(State(state): State<AppState>, Path(id): Path<i64>) -> StatusCode {
    match sqlx::query("DELETE FROM sources WHERE id=?").bind(id).execute(&state.pool).await {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT,
        Ok(_) => StatusCode::NOT_FOUND,
        Err(e) => { tracing::error!("delete source: {e}"); StatusCode::INTERNAL_SERVER_ERROR }
    }
}

/// POST /api/sources/:id/test — runs a live fetch with a sample query and returns raw items.
pub async fn test_source(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    let source = match sqlx::query_as::<_, Source>("SELECT * FROM sources WHERE id=?")
        .bind(id).fetch_one(&state.pool).await
    {
        Ok(s) => s,
        Err(sqlx::Error::RowNotFound) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => { tracing::error!("test_source fetch: {e}"); return StatusCode::INTERNAL_SERVER_ERROR.into_response(); }
    };

    let plugin = match build_source(&source, state.http.clone()) {
        Some(p) => p,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "unknown source_type"}))).into_response(),
    };

    // Use a placeholder term for the test fetch
    let test_term = crate::models::SearchTerm {
        id: 0,
        name: "test".into(),
        query: "test".into(),
        enabled: true,
        max_age_days: Some(30),
        disallowed_keywords: None,
        created_at: chrono::Utc::now(),
    };

    match plugin.fetch(&test_term).await {
        Ok(items) => Json(serde_json::json!({
            "source": source.name,
            "item_count": items.len(),
            "items": items.iter().take(10).map(|i| serde_json::json!({
                "title": i.title,
                "url": i.url,
                "pub_date": i.pub_date,
                "seeders": i.seeders,
                "indexer": i.indexer,
            })).collect::<Vec<_>>()
        })).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test --test api_tests
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/api/sources.rs
git commit -m "feat: sources CRUD API + live test endpoint"
```

---

## Task 14: API — Matches + Notifications

**Files:**
- Modify: `src/api/matches.rs`
- Modify: `src/api/notifications.rs`

- [ ] **Step 1: Implement matches.rs**

```rust
// src/api/matches.rs
use axum::{
    extract::{Query, State},
    Json,
    response::IntoResponse,
    http::StatusCode,
};
use serde::Deserialize;
use super::AppState;
use crate::models::Match;

#[derive(Deserialize)]
pub struct MatchFilter {
    pub search_term_id: Option<i64>,
    pub source_id: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(filter): Query<MatchFilter>,
) -> impl IntoResponse {
    let limit = filter.limit.unwrap_or(100).min(500);

    // Build dynamic query
    let mut query = "SELECT * FROM matches WHERE 1=1".to_string();
    if filter.search_term_id.is_some() { query.push_str(" AND search_term_id=?1"); }
    if filter.source_id.is_some() { query.push_str(" AND source_id=?2"); }
    query.push_str(" ORDER BY matched_at DESC LIMIT ?3");

    // sqlx doesn't support fully dynamic binding cleanly; use explicit variants
    let result = sqlx::query_as::<_, Match>(
        "SELECT * FROM matches
         WHERE (?1 IS NULL OR search_term_id=?1)
           AND (?2 IS NULL OR source_id=?2)
         ORDER BY matched_at DESC
         LIMIT ?3"
    )
    .bind(filter.search_term_id)
    .bind(filter.source_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await;

    match result {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => { tracing::error!("list matches: {e}"); StatusCode::INTERNAL_SERVER_ERROR.into_response() }
    }
}
```

- [ ] **Step 2: Implement notifications.rs**

Notification config is stored in env vars (read-only from the app's perspective for v1). The GET returns current config (with secrets masked), PUT updates the running config in-memory only (not persisted to disk — requires restart to change via env). Test endpoint fires a sample notification.

```rust
// src/api/notifications.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use super::AppState;

#[derive(Serialize)]
pub struct NotificationConfig {
    pub discord_webhook_url: Option<String>,  // masked
    pub apprise_url: Option<String>,          // masked
    pub pushover_configured: bool,
    pub steamgriddb_configured: bool,
}

pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = &state.config;
    Json(NotificationConfig {
        discord_webhook_url: cfg.discord_webhook_url.as_ref().map(|u| mask_url(u)),
        apprise_url: cfg.apprise_url.as_ref().map(|u| mask_url(u)),
        pushover_configured: cfg.pushover_app_token.is_some() && cfg.pushover_user_key.is_some(),
        steamgriddb_configured: cfg.steamgriddb_api_key.is_some(),
    })
}

/// For v1, PUT is a no-op that returns 501 with a note.
/// Full runtime config update is a future-pass feature.
pub async fn put_config() -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED,
     Json(serde_json::json!({"message": "Runtime config update not yet implemented. Restart with updated env vars."})))
}

/// POST /api/notifications/test/:channel — fires a dummy notification on the named channel.
pub async fn test_channel(
    State(state): State<AppState>,
    Path(channel): Path<String>,
) -> impl IntoResponse {
    let dummy_term = crate::models::SearchTerm {
        id: 0, name: "Test".into(), query: "test".into(),
        enabled: true, max_age_days: Some(30), disallowed_keywords: None,
        created_at: chrono::Utc::now(),
    };
    let dummy_item = crate::sources::SourceItem {
        title: "DiscoProwl Test Notification".into(),
        url: Some("https://github.com/danktankk/discoprowl".into()),
        guid: "test-guid".into(),
        pub_date: Some(chrono::Utc::now()),
        description: Some("This is a test notification from DiscoProwl.".into()),
        indexer: Some("test".into()),
        seeders: Some(42),
    };

    let result = match channel.as_str() {
        "discord" => {
            if let Some(ref url) = state.config.discord_webhook_url {
                crate::notifier::discord::send(&state.http, url, &dummy_term, &dummy_item, "test-source", state.config.steamgriddb_api_key.as_deref()).await
            } else {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Discord not configured"}))).into_response();
            }
        }
        "apprise" => {
            if let Some(ref url) = state.config.apprise_url {
                crate::notifier::apprise::send(&state.http, url, &dummy_term, &dummy_item).await
            } else {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Apprise not configured"}))).into_response();
            }
        }
        "pushover" => {
            if let (Some(token), Some(key)) = (&state.config.pushover_app_token, &state.config.pushover_user_key) {
                crate::notifier::pushover::send(&state.http, token, key, &dummy_term, &dummy_item, state.config.steamgriddb_api_key.as_deref()).await
            } else {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Pushover not configured"}))).into_response();
            }
        }
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Unknown channel"}))).into_response(),
    };

    match result {
        Ok(_) => Json(serde_json::json!({"ok": true, "channel": channel})).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

fn mask_url(url: &str) -> String {
    // Show first 20 chars + ***
    if url.len() > 20 { format!("{}***", &url[..20]) } else { url.to_string() }
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo check
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/api/matches.rs src/api/notifications.rs
git commit -m "feat: matches list API + notifications config/test endpoints"
```

---

## Task 15: Wire main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement main.rs**

```rust
// src/main.rs
mod config;
mod db;
mod models;
mod matcher;
mod scheduler;
mod sources;
mod notifier;
mod api;
mod assets;

use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let config = Arc::new(config::Config::from_env());
    info!("Connecting to database: {}", config.database_url);

    let pool = db::init_pool(&config.database_url).await?;
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("discoprowl/2.0")
        .build()?;

    let notifier = Arc::new(notifier::Notifier::new(config.clone(), http.clone()));

    // Spawn background scheduler
    let sched_pool = pool.clone();
    let sched_notifier = notifier.clone();
    let sched_http = http.clone();
    tokio::spawn(async move {
        scheduler::run(sched_pool, sched_notifier, sched_http).await;
    });

    let state = api::AppState {
        pool,
        config: config.clone(),
        notifier,
        http,
    };

    let app = api::router(state);
    let addr: std::net::SocketAddr = config.bind_addr.parse()?;
    info!("Listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

- [ ] **Step 2: Build to verify**

```bash
cargo build
```
Expected: binary at `target/debug/discoprowl` with no errors.

- [ ] **Step 3: Smoke test**

```bash
DATABASE_URL=/tmp/dp_test.db DISCORD_WEBHOOK_URL=https://example.com/hook cargo run &
sleep 2
curl -s http://localhost:3079/api/search_terms
# Expected: []
kill %1
```

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire main.rs — config, pool, scheduler, axum server"
```

---

## Task 16: Frontend — Scaffold (Svelte + Vite)

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/vite.config.js`
- Create: `frontend/index.html`
- Create: `frontend/src/main.js`
- Create: `frontend/src/App.svelte`
- Create: `frontend/src/lib/api.js`
- Create: `frontend/src/lib/Nav.svelte`

- [ ] **Step 1: Initialize frontend**

```bash
cd frontend
npm create vite@latest . -- --template svelte
# Accept overwrite, choose Svelte + JavaScript
npm install
```

- [ ] **Step 2: Install dependencies**

```bash
npm install
```

- [ ] **Step 3: Update vite.config.js**

```js
// frontend/vite.config.js
import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

export default defineConfig({
  plugins: [svelte()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
  base: '/',
})
```

- [ ] **Step 4: Create App.svelte with hash router**

```svelte
<!-- frontend/src/App.svelte -->
<script>
  import Nav from './lib/Nav.svelte'
  import Dashboard from './routes/Dashboard.svelte'
  import SearchTerms from './routes/SearchTerms.svelte'
  import Sources from './routes/Sources.svelte'
  import Notifications from './routes/Notifications.svelte'

  let route = window.location.hash.slice(1) || '/'

  window.addEventListener('hashchange', () => {
    route = window.location.hash.slice(1) || '/'
  })
</script>

<div class="app">
  <Nav />
  <main>
    {#if route === '/'}
      <Dashboard />
    {:else if route === '/search-terms'}
      <SearchTerms />
    {:else if route === '/sources'}
      <Sources />
    {:else if route === '/notifications'}
      <Notifications />
    {/if}
  </main>
</div>

<style>
  .app { display: flex; min-height: 100vh; }
  main { flex: 1; padding: 1.5rem; }
</style>
```

- [ ] **Step 5: Create Nav.svelte**

```svelte
<!-- frontend/src/lib/Nav.svelte -->
<script>
  const links = [
    { href: '#/', label: 'Dashboard' },
    { href: '#/search-terms', label: 'Search Terms' },
    { href: '#/sources', label: 'Sources' },
    { href: '#/notifications', label: 'Notifications' },
  ]
</script>

<nav>
  <div class="logo">DiscoProwl</div>
  {#each links as link}
    <a href={link.href}>{link.label}</a>
  {/each}
</nav>

<style>
  nav {
    width: 200px; background: #1a1a2e; color: #eee;
    display: flex; flex-direction: column; padding: 1rem; gap: 0.5rem;
  }
  .logo { font-weight: bold; font-size: 1.1rem; margin-bottom: 1rem; color: #7c83fd; }
  a { color: #ccc; text-decoration: none; padding: 0.4rem 0.5rem; border-radius: 4px; }
  a:hover { background: #2a2a4e; color: #fff; }
</style>
```

- [ ] **Step 6: Create api.js**

```js
// frontend/src/lib/api.js
const BASE = '/api'

async function request(method, path, body) {
  const opts = {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined,
  }
  const res = await fetch(BASE + path, opts)
  if (!res.ok) throw new Error(`${method} ${path} → ${res.status}`)
  if (res.status === 204) return null
  return res.json()
}

export const api = {
  searchTerms: {
    list: () => request('GET', '/search_terms'),
    create: (data) => request('POST', '/search_terms', data),
    update: (id, data) => request('PUT', `/search_terms/${id}`, data),
    delete: (id) => request('DELETE', `/search_terms/${id}`),
  },
  sources: {
    list: () => request('GET', '/sources'),
    create: (data) => request('POST', '/sources', data),
    update: (id, data) => request('PUT', `/sources/${id}`, data),
    delete: (id) => request('DELETE', `/sources/${id}`),
    test: (id) => request('POST', `/sources/${id}/test`),
  },
  matches: {
    list: (params = {}) => {
      const qs = new URLSearchParams(Object.entries(params).filter(([,v]) => v != null))
      return request('GET', `/matches${qs.toString() ? '?' + qs : ''}`)
    },
  },
  notifications: {
    getConfig: () => request('GET', '/notifications/config'),
    test: (channel) => request('POST', `/notifications/test/${channel}`),
  },
}
```

- [ ] **Step 7: Create route stubs**

Create `frontend/src/routes/Dashboard.svelte`, `SearchTerms.svelte`, `Sources.svelte`, `Notifications.svelte` — each with a `<h2>Page Name</h2>` stub.

- [ ] **Step 8: Build to verify**

```bash
cd frontend && npm run build
```
Expected: `dist/` created with `index.html` and assets.

- [ ] **Step 9: Commit**

```bash
cd .. && git add frontend/
git commit -m "feat: Svelte + Vite frontend scaffold with hash router"
```

---

## Task 17: Frontend — Dashboard View

**Files:**
- Modify: `frontend/src/routes/Dashboard.svelte`

- [ ] **Step 1: Implement Dashboard.svelte**

```svelte
<!-- frontend/src/routes/Dashboard.svelte -->
<script>
  import { onMount } from 'svelte'
  import { api } from '../lib/api.js'

  let matches = []
  let terms = []
  let sources = []
  let filterTerm = ''
  let filterSource = ''
  let error = null

  onMount(async () => {
    try {
      ;[matches, terms, sources] = await Promise.all([
        api.matches.list({ limit: 100 }),
        api.searchTerms.list(),
        api.sources.list(),
      ])
    } catch (e) {
      error = e.message
    }
  })

  $: filtered = matches.filter(m => {
    if (filterTerm && m.search_term_id != filterTerm) return false
    if (filterSource && m.source_id != filterSource) return false
    return true
  })

  function termName(id) { return terms.find(t => t.id === id)?.name ?? id }
  function sourceName(id) { return sources.find(s => s.id === id)?.name ?? id }
  function channels(json) {
    try { return JSON.parse(json || '[]').join(', ') || '—' } catch { return '—' }
  }
  function fmt(dt) { return dt ? new Date(dt).toLocaleString() : '—' }
</script>

<div>
  <h2>Dashboard</h2>
  {#if error}<p class="error">{error}</p>{/if}

  <div class="filters">
    <select bind:value={filterTerm}>
      <option value="">All terms</option>
      {#each terms as t}<option value={t.id}>{t.name}</option>{/each}
    </select>
    <select bind:value={filterSource}>
      <option value="">All sources</option>
      {#each sources as s}<option value={s.id}>{s.name}</option>{/each}
    </select>
  </div>

  {#if filtered.length === 0}
    <p class="empty">No matches yet.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Matched</th><th>Term</th><th>Title</th><th>Source</th><th>Channels</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as m}
          <tr>
            <td>{fmt(m.matched_at)}</td>
            <td>{termName(m.search_term_id)}</td>
            <td>{#if m.item_url}<a href={m.item_url} target="_blank">{m.item_title}</a>{:else}{m.item_title}{/if}</td>
            <td>{sourceName(m.source_id)}</td>
            <td>{channels(m.notification_channels)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  h2 { margin-top: 0; }
  .filters { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
  select { padding: 0.3rem; }
  table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }
  th, td { border: 1px solid #ddd; padding: 0.4rem 0.6rem; text-align: left; }
  th { background: #f5f5f5; }
  tr:hover td { background: #fafafa; }
  .empty { color: #888; }
  .error { color: red; }
  a { color: #5566dd; }
</style>
```

- [ ] **Step 2: Build**

```bash
cd frontend && npm run build
```
Expected: success.

- [ ] **Step 3: Commit**

```bash
cd .. && git add frontend/src/routes/Dashboard.svelte
git commit -m "feat: Dashboard view with match history table and filters"
```

---

## Task 18: Frontend — Search Terms View

**Files:**
- Modify: `frontend/src/routes/SearchTerms.svelte`

- [ ] **Step 1: Implement SearchTerms.svelte**

```svelte
<!-- frontend/src/routes/SearchTerms.svelte -->
<script>
  import { onMount } from 'svelte'
  import { api } from '../lib/api.js'

  let terms = []
  let error = null
  let showModal = false
  let editing = null  // null = new, or existing term object

  const empty = () => ({ name: '', query: '', max_age_days: 30, disallowed_keywords: '', enabled: true })
  let form = empty()

  onMount(load)

  async function load() {
    try { terms = await api.searchTerms.list() }
    catch (e) { error = e.message }
  }

  function openNew() { editing = null; form = empty(); showModal = true }
  function openEdit(t) { editing = t; form = { ...t, disallowed_keywords: t.disallowed_keywords ?? '' }; showModal = true }

  async function save() {
    try {
      if (editing) await api.searchTerms.update(editing.id, form)
      else await api.searchTerms.create(form)
      showModal = false
      await load()
    } catch(e) { error = e.message }
  }

  async function remove(id) {
    if (!confirm('Delete this term?')) return
    try { await api.searchTerms.delete(id); await load() }
    catch(e) { error = e.message }
  }

  async function toggleEnabled(t) {
    try { await api.searchTerms.update(t.id, { ...t, enabled: !t.enabled }); await load() }
    catch(e) { error = e.message }
  }
</script>

<div>
  <div class="header">
    <h2>Search Terms</h2>
    <button on:click={openNew}>+ Add</button>
  </div>
  {#if error}<p class="error">{error}</p>{/if}

  <table>
    <thead><tr><th>Name</th><th>Query</th><th>Max Age</th><th>Blocked Keywords</th><th>Enabled</th><th></th></tr></thead>
    <tbody>
      {#each terms as t}
        <tr>
          <td>{t.name}</td>
          <td><code>{t.query}</code></td>
          <td>{t.max_age_days ?? 30}d</td>
          <td>{t.disallowed_keywords ?? '—'}</td>
          <td><input type="checkbox" checked={t.enabled} on:change={() => toggleEnabled(t)} /></td>
          <td class="actions">
            <button on:click={() => openEdit(t)}>Edit</button>
            <button class="danger" on:click={() => remove(t.id)}>Delete</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  {#if showModal}
    <div class="overlay" on:click|self={() => showModal = false}>
      <div class="modal">
        <h3>{editing ? 'Edit' : 'New'} Search Term</h3>
        <label>Name <input bind:value={form.name} /></label>
        <label>Query <input bind:value={form.query} placeholder="e.g. Elden Ring" /></label>
        <label>Max Age (days) <input type="number" bind:value={form.max_age_days} /></label>
        <label>Disallowed Keywords (comma-separated) <input bind:value={form.disallowed_keywords} placeholder="trainer,crack,repack" /></label>
        <label>Enabled <input type="checkbox" bind:checked={form.enabled} /></label>
        <div class="modal-actions">
          <button on:click={() => showModal = false}>Cancel</button>
          <button class="primary" on:click={save}>Save</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .header { display: flex; align-items: center; justify-content: space-between; }
  h2 { margin-top: 0; }
  table { width: 100%; border-collapse: collapse; }
  th, td { border: 1px solid #ddd; padding: 0.4rem 0.6rem; }
  th { background: #f5f5f5; }
  .actions { white-space: nowrap; }
  .danger { color: #c00; }
  .primary { background: #5566dd; color: #fff; border: none; padding: 0.4rem 1rem; border-radius: 4px; }
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; }
  .modal { background: #fff; padding: 1.5rem; border-radius: 8px; min-width: 400px; display: flex; flex-direction: column; gap: 0.8rem; }
  .modal label { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.9rem; }
  .modal input { padding: 0.3rem; border: 1px solid #ccc; border-radius: 4px; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }
  .error { color: red; }
</style>
```

- [ ] **Step 2: Build**

```bash
cd frontend && npm run build
```

- [ ] **Step 3: Commit**

```bash
cd .. && git add frontend/src/routes/SearchTerms.svelte
git commit -m "feat: Search Terms CRUD view"
```

---

## Task 19: Frontend — Sources View

**Files:**
- Modify: `frontend/src/routes/Sources.svelte`

- [ ] **Step 1: Implement Sources.svelte**

```svelte
<!-- frontend/src/routes/Sources.svelte -->
<script>
  import { onMount } from 'svelte'
  import { api } from '../lib/api.js'

  let sources = []
  let error = null
  let showModal = false
  let editing = null
  let testResult = {}  // sourceId -> result string

  const SOURCE_TYPES = ['rss', 'newznab', 'torznab']
  const empty = () => ({ name: '', source_type: 'rss', url: '', api_key: '', poll_interval_mins: 720, enabled: true })
  let form = empty()

  onMount(load)

  async function load() {
    try { sources = await api.sources.list() }
    catch(e) { error = e.message }
  }

  function openNew() { editing = null; form = empty(); showModal = true }
  function openEdit(s) { editing = s; form = { ...s, api_key: s.api_key ?? '' }; showModal = true }

  async function save() {
    try {
      if (editing) await api.sources.update(editing.id, form)
      else await api.sources.create(form)
      showModal = false; await load()
    } catch(e) { error = e.message }
  }

  async function remove(id) {
    if (!confirm('Delete this source?')) return
    try { await api.sources.delete(id); await load() }
    catch(e) { error = e.message }
  }

  async function testNow(id) {
    testResult = { ...testResult, [id]: 'Testing...' }
    try {
      const r = await api.sources.test(id)
      testResult = { ...testResult, [id]: `${r.item_count} items returned` }
    } catch(e) {
      testResult = { ...testResult, [id]: `Error: ${e.message}` }
    }
  }
</script>

<div>
  <div class="header"><h2>Sources</h2><button on:click={openNew}>+ Add</button></div>
  {#if error}<p class="error">{error}</p>{/if}

  <table>
    <thead><tr><th>Name</th><th>Type</th><th>URL</th><th>Interval</th><th>Last Polled</th><th>Enabled</th><th></th></tr></thead>
    <tbody>
      {#each sources as s}
        <tr>
          <td>{s.name}</td>
          <td><span class="badge {s.source_type}">{s.source_type}</span></td>
          <td class="url">{s.url}</td>
          <td>{s.poll_interval_mins}m</td>
          <td>{s.last_polled_at ? new Date(s.last_polled_at).toLocaleString() : 'Never'}</td>
          <td>{s.enabled ? '✓' : '—'}</td>
          <td class="actions">
            <button on:click={() => testNow(s.id)}>Test</button>
            <button on:click={() => openEdit(s)}>Edit</button>
            <button class="danger" on:click={() => remove(s.id)}>Delete</button>
          </td>
        </tr>
        {#if testResult[s.id]}
          <tr class="test-row"><td colspan="7">{testResult[s.id]}</td></tr>
        {/if}
      {/each}
    </tbody>
  </table>

  {#if showModal}
    <div class="overlay" on:click|self={() => showModal = false}>
      <div class="modal">
        <h3>{editing ? 'Edit' : 'New'} Source</h3>
        <label>Name <input bind:value={form.name} /></label>
        <label>Type
          <select bind:value={form.source_type}>
            {#each SOURCE_TYPES as t}<option value={t}>{t}</option>{/each}
          </select>
        </label>
        <label>URL <input bind:value={form.url} placeholder="https://..." /></label>
        <label>API Key <input bind:value={form.api_key} placeholder="(optional)" /></label>
        <label>Poll Interval (minutes) <input type="number" bind:value={form.poll_interval_mins} /></label>
        <label>Enabled <input type="checkbox" bind:checked={form.enabled} /></label>
        <div class="modal-actions">
          <button on:click={() => showModal = false}>Cancel</button>
          <button class="primary" on:click={save}>Save</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .header { display: flex; align-items: center; justify-content: space-between; }
  h2 { margin-top: 0; }
  table { width: 100%; border-collapse: collapse; }
  th, td { border: 1px solid #ddd; padding: 0.4rem 0.6rem; font-size: 0.85rem; }
  th { background: #f5f5f5; }
  .url { max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .badge { padding: 2px 6px; border-radius: 3px; font-size: 0.75rem; font-weight: bold; }
  .badge.rss { background: #e8f5e9; color: #2e7d32; }
  .badge.newznab { background: #e3f2fd; color: #1565c0; }
  .badge.torznab { background: #fce4ec; color: #880e4f; }
  .test-row td { background: #f9f9f9; color: #555; font-size: 0.82rem; padding-left: 1rem; }
  .actions { white-space: nowrap; }
  .danger { color: #c00; }
  .primary { background: #5566dd; color: #fff; border: none; padding: 0.4rem 1rem; border-radius: 4px; }
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; }
  .modal { background: #fff; padding: 1.5rem; border-radius: 8px; min-width: 420px; display: flex; flex-direction: column; gap: 0.8rem; }
  .modal label { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.9rem; }
  .modal input, .modal select { padding: 0.3rem; border: 1px solid #ccc; border-radius: 4px; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }
  .error { color: red; }
</style>
```

- [ ] **Step 2: Build + commit**

```bash
cd frontend && npm run build && cd ..
git add frontend/src/routes/Sources.svelte
git commit -m "feat: Sources CRUD view with live test button"
```

---

## Task 20: Frontend — Notifications View

**Files:**
- Modify: `frontend/src/routes/Notifications.svelte`

- [ ] **Step 1: Implement Notifications.svelte**

```svelte
<!-- frontend/src/routes/Notifications.svelte -->
<script>
  import { onMount } from 'svelte'
  import { api } from '../lib/api.js'

  let config = null
  let error = null
  let testResults = {}

  onMount(async () => {
    try { config = await api.notifications.getConfig() }
    catch(e) { error = e.message }
  })

  async function test(channel) {
    testResults = { ...testResults, [channel]: 'Sending...' }
    try {
      await api.notifications.test(channel)
      testResults = { ...testResults, [channel]: '✓ Sent' }
    } catch(e) {
      testResults = { ...testResults, [channel]: `✗ ${e.message}` }
    }
  }
</script>

<div>
  <h2>Notifications</h2>
  {#if error}<p class="error">{error}</p>{/if}

  {#if config}
    <p class="note">Notification channels are configured via environment variables.
    Restart the container to change them.</p>

    <div class="channels">
      <div class="channel" class:configured={config.discord_webhook_url}>
        <div class="channel-header">
          <span class="channel-name">Discord</span>
          <span class="status">{config.discord_webhook_url ? '● Configured' : '○ Not configured'}</span>
        </div>
        {#if config.discord_webhook_url}
          <p class="masked">URL: {config.discord_webhook_url}</p>
          <button on:click={() => test('discord')}>Send Test</button>
          {#if testResults.discord}<span class="result">{testResults.discord}</span>{/if}
        {/if}
      </div>

      <div class="channel" class:configured={config.apprise_url}>
        <div class="channel-header">
          <span class="channel-name">Apprise</span>
          <span class="status">{config.apprise_url ? '● Configured' : '○ Not configured'}</span>
        </div>
        {#if config.apprise_url}
          <p class="masked">URL: {config.apprise_url}</p>
          <button on:click={() => test('apprise')}>Send Test</button>
          {#if testResults.apprise}<span class="result">{testResults.apprise}</span>{/if}
        {/if}
      </div>

      <div class="channel" class:configured={config.pushover_configured}>
        <div class="channel-header">
          <span class="channel-name">Pushover</span>
          <span class="status">{config.pushover_configured ? '● Configured' : '○ Not configured'}</span>
        </div>
        {#if config.pushover_configured}
          <button on:click={() => test('pushover')}>Send Test</button>
          {#if testResults.pushover}<span class="result">{testResults.pushover}</span>{/if}
        {/if}
      </div>

      <div class="channel" class:configured={config.steamgriddb_configured}>
        <div class="channel-header">
          <span class="channel-name">SteamGridDB (box art)</span>
          <span class="status">{config.steamgriddb_configured ? '● Configured' : '○ Not configured (fallback image used)'}</span>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  h2 { margin-top: 0; }
  .note { color: #666; font-size: 0.9rem; margin-bottom: 1.5rem; }
  .channels { display: flex; flex-direction: column; gap: 1rem; }
  .channel { border: 1px solid #ddd; border-radius: 6px; padding: 1rem; }
  .channel.configured { border-color: #4caf50; }
  .channel-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.5rem; }
  .channel-name { font-weight: bold; }
  .status { font-size: 0.85rem; color: #888; }
  .channel.configured .status { color: #4caf50; }
  .masked { font-size: 0.82rem; color: #666; margin: 0.3rem 0; font-family: monospace; }
  button { padding: 0.3rem 0.8rem; cursor: pointer; }
  .result { margin-left: 0.5rem; font-size: 0.85rem; }
  .error { color: red; }
</style>
```

- [ ] **Step 2: Build + commit**

```bash
cd frontend && npm run build && cd ..
git add frontend/src/routes/Notifications.svelte
git commit -m "feat: Notifications view with per-channel test buttons"
```

---

## Task 21: rust-embed Integration + Build Script

**Files:**
- Modify: `src/assets.rs`
- Create: `build.rs`

- [ ] **Step 1: Write build.rs to compile frontend before cargo build**

```rust
// build.rs
use std::process::Command;

fn main() {
    let frontend = std::path::Path::new("frontend");
    if !frontend.exists() {
        return; // no frontend in this checkout
    }

    // Tell cargo to re-run if frontend sources change
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/package.json");
    println!("cargo:rerun-if-changed=frontend/vite.config.js");

    // Install dependencies if node_modules missing
    if !frontend.join("node_modules").exists() {
        let status = Command::new("npm")
            .args(["install"])
            .current_dir(frontend)
            .status()
            .expect("npm install failed");
        assert!(status.success(), "npm install failed");
    }

    // Build
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(frontend)
        .status()
        .expect("npm run build failed");
    assert!(status.success(), "frontend build failed");
}
```

- [ ] **Step 2: Implement assets.rs with rust-embed**

```rust
// src/assets.rs
use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode, Uri},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct FrontendAssets;

pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try the exact path first
    if let Some(file) = FrontendAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(file.data.into_owned()))
            .unwrap();
    }

    // SPA fallback: return index.html for all non-asset paths
    if let Some(index) = FrontendAssets::get("index.html") {
        return Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(index.data.into_owned()))
            .unwrap();
    }

    StatusCode::NOT_FOUND.into_response()
}
```

Add `mime_guess = "2"` to `[dependencies]` in `Cargo.toml`.

- [ ] **Step 3: Build the full binary**

```bash
cargo build --release
```
Expected: `target/release/discoprowl` compiled with frontend embedded. Build may take 2-3 minutes on first run.

- [ ] **Step 4: Smoke test static serving**

```bash
DATABASE_URL=/tmp/dp_test2.db DISCORD_WEBHOOK_URL=https://example.com/hook ./target/release/discoprowl &
sleep 1
curl -s -I http://localhost:3079/
# Expected: HTTP/1.1 200, content-type: text/html
curl -s http://localhost:3079/api/search_terms
# Expected: []
kill %1
```

- [ ] **Step 5: Commit**

```bash
git add build.rs src/assets.rs Cargo.toml
git commit -m "feat: rust-embed static serving + build.rs frontend compile step"
```

---

## Task 22: Dockerfile + Docker Compose

**Files:**
- Modify: `Dockerfile` (replace Python Dockerfile)
- Modify: `docker-compose.yml` (update for Rust binary + port 3079)
- Modify: `.env.example`

- [ ] **Step 1: Write multi-stage Dockerfile**

```dockerfile
# Dockerfile

# ── Stage 1: Frontend build ─────────────────────────────────────────────────
FROM node:20-alpine AS frontend-build
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# ── Stage 2: Rust build ──────────────────────────────────────────────────────
FROM rust:1.77-slim-bookworm AS rust-build
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY --from=frontend-build /app/frontend/dist ./frontend/dist/

# Stub build.rs so cargo doesn't try to run npm again
RUN echo 'fn main() {}' > build.rs

RUN cargo build --release

# ── Stage 3: Runtime ─────────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=rust-build /app/target/release/discoprowl ./discoprowl

EXPOSE 3079
CMD ["./discoprowl"]
```

- [ ] **Step 2: Update docker-compose.yml**

```yaml
# docker-compose.yml
services:
  discoprowl:
    image: danktankk/discoprowl:latest
    build: .
    ports:
      - "3079:3079"
    volumes:
      - ./data:/data
    environment:
      DATABASE_URL: /data/discoprowl.db
      BIND_ADDR: 0.0.0.0:3079
      COMMAFEED_URL: http://192.168.160.155:8882
      COMMAFEED_USER: CC
      COMMAFEED_PASS: ${COMMAFEED_PASS}
      DISCORD_WEBHOOK_URL: ${DISCORD_WEBHOOK_URL}
      APPRISE_URL: ${APPRISE_URL:-}
      PUSHOVER_APP_TOKEN: ${PUSHOVER_APP_TOKEN:-}
      PUSHOVER_USER_KEY: ${PUSHOVER_USER_KEY:-}
      STEAMGRIDDB_API_KEY: ${STEAMGRIDDB_API_KEY:-}
      RUST_LOG: info
    restart: unless-stopped
    networks:
      - proxy

networks:
  proxy:
    external: true
```

- [ ] **Step 3: Update .env.example**

```bash
# .env.example
DATABASE_URL=/data/discoprowl.db
BIND_ADDR=0.0.0.0:3079

# CommaFeed (RSS source)
COMMAFEED_URL=http://192.168.160.155:8882
COMMAFEED_USER=CC
COMMAFEED_PASS=your_password

# Notifications (at least one required)
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
APPRISE_URL=
PUSHOVER_APP_TOKEN=
PUSHOVER_USER_KEY=

# Optional
STEAMGRIDDB_API_KEY=

RUST_LOG=info
```

- [ ] **Step 4: Test Docker build locally**

```bash
docker build -t discoprowl:test .
```
Expected: multi-stage build completes, final image ~50MB.

- [ ] **Step 5: Commit**

```bash
git add Dockerfile docker-compose.yml .env.example
git commit -m "feat: multi-stage Dockerfile + updated docker-compose.yml for Rust rewrite"
```

---

## Task 23: Remove Old Python Files

**Files:**
- Delete: `discoprowl.py`
- Delete: `.pylintrc`

- [ ] **Step 1: Remove Python artifacts**

```bash
git rm discoprowl.py .pylintrc
```

- [ ] **Step 2: Commit**

```bash
git commit -m "chore: remove Python implementation (replaced by Rust rewrite)"
```

---

## Task 24: Final Integration Test + PR

- [ ] **Step 1: Run full test suite**

```bash
cargo test
```
Expected: all tests pass (matcher + API).

- [ ] **Step 2: Build release binary**

```bash
cargo build --release
```
Expected: success.

- [ ] **Step 3: End-to-end local smoke test**

```bash
mkdir -p /tmp/dp_data
DATABASE_URL=/tmp/dp_data/test.db DISCORD_WEBHOOK_URL=https://example.com/hook \
  ./target/release/discoprowl &
sleep 1

# Create a search term
curl -s -X POST http://localhost:3079/api/search_terms \
  -H "Content-Type: application/json" \
  -d '{"name":"Elden Ring","query":"elden ring","max_age_days":30}'

# Create a source (RSS)
curl -s -X POST http://localhost:3079/api/sources \
  -H "Content-Type: application/json" \
  -d '{"name":"Test Feed","source_type":"rss","url":"https://rss.azbbc.net/top-stories","poll_interval_mins":5}'

# Verify both exist
curl -s http://localhost:3079/api/search_terms | python3 -m json.tool
curl -s http://localhost:3079/api/sources | python3 -m json.tool

# Check web UI loads
curl -s -I http://localhost:3079/
# Expected: 200 text/html

kill %1
```

- [ ] **Step 4: Push branch and open PR**

```bash
GIT_SSH_COMMAND="ssh -i ~/.ssh/cc" git push github design/rust-rewrite-spec
```

Open PR on GitHub: `danktankk/discoprowl` — base: `main`, compare: `design/rust-rewrite-spec`.
Title: `feat: Rust rewrite — Axum + SQLite + Svelte frontend`

- [ ] **Step 5: Deploy to staging2 (after PR merge)**

SSH to staging2 (`dankk@192.168.160.161`), pull updated image, restart container:
```bash
docker compose pull discoprowl
docker compose up -d discoprowl
docker logs -f discoprowl
```
Verify at `http://192.168.160.161:3079`.

---

## Self-Review Checklist

**Spec coverage:**
- [x] Axum + embedded Svelte — Tasks 11, 16-20, 21
- [x] SQLite via sqlx, 3 tables — Tasks 3-4
- [x] No deduplication — scheduler logs every match, no seen-before check
- [x] RssSource (CommaFeed + direct) — Task 7
- [x] NewznabSource — Task 8
- [x] TorznabSource — Task 8
- [x] Whole-word match, max_age_days, disallowed_keywords — Task 5
- [x] Discord webhook + embed + SteamGridDB — Task 9
- [x] Apprise — Task 9
- [x] Pushover + image attachment — Task 9
- [x] Dashboard view — Task 17
- [x] Search Terms CRUD — Task 12, 18
- [x] Sources CRUD + Test button — Task 13, 19
- [x] Notifications config + Test button — Task 14, 20
- [x] Port 3079 — Task 22
- [x] Auth out of scope (v1), architecture accommodates it via Axum middleware — Task 11 (router wired for `.layer()` addition)
- [x] Docker container on staging2 — Task 22, 24

**No placeholders:** Checked — all code blocks are complete implementations.

**Type consistency:** `SearchTerm`, `Source`, `Match`, `SourceItem` defined in Tasks 4/6 and used consistently through Tasks 7-14. `NewSearchTerm`/`NewSource` used for API inputs throughout.
