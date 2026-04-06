# DiscoProwl Rewrite — Design Spec
**Date:** 2026-04-06  
**Status:** Approved

---

## Overview

Full rewrite of DiscoProwl in Rust. The current Python single-file implementation is Prowlarr-only, has no frontend, no persistence, and re-notifies the same results every cycle with no history. The rewrite makes it source-agnostic, adds a web UI, and treats notification as a first-class feature.

**Core purpose:** Watch configured sources for items matching user-defined search terms. Notify when matches are found. Log everything.

---

## Architecture

Single Rust binary. One Docker container on staging2 (192.168.160.161).

```
┌─────────────────────────────────────────┐
│              discoprowl                 │
│                                         │
│  Axum (HTTP server)                     │
│    ├── REST API  (/api/*)               │
│    └── Static assets (embedded Svelte)  │
│                                         │
│  Scheduler (tokio-cron)                 │
│    └── per-source poll intervals        │
│                                         │
│  Source plugins (trait objects)         │
│    ├── RssSource    (CommaFeed + direct) │
│    ├── NewznabSource (Usenet indexers)  │
│    └── TorznabSource (Prowlarr/Jackett) │
│                                         │
│  SQLite (sqlx)                          │
│    ├── search_terms                     │
│    ├── sources                          │
│    └── matches (history log)            │
│                                         │
│  Notifier                               │
│    ├── Discord webhook                  │
│    ├── Apprise                          │
│    └── Pushover                         │
└─────────────────────────────────────────┘
```

**Frontend:** Svelte + Vite, compiled and embedded in the binary via `rust-embed`. No separate build container at runtime — assets baked in at compile time.

**Auth:** Not in v1 — internal network only, same pattern as all other tools on staging2. Auth layer is planned for a future pass; the API and session model should be designed to accommodate it without a full rewrite (e.g., Axum middleware layer).

---

## Data Model

SQLite, three tables. No deduplication — every poll cycle runs clean and notifies on all matches. `matches` is a history log only.

```sql
-- What to search for
CREATE TABLE search_terms (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    name                TEXT NOT NULL,
    query               TEXT NOT NULL,
    enabled             BOOLEAN NOT NULL DEFAULT true,
    max_age_days        INTEGER DEFAULT 30,
    disallowed_keywords TEXT,  -- comma-separated
    created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Where to look
CREATE TABLE sources (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    name                TEXT NOT NULL,
    source_type         TEXT NOT NULL,  -- 'rss' | 'newznab' | 'torznab'
    url                 TEXT NOT NULL,
    api_key             TEXT,
    enabled             BOOLEAN NOT NULL DEFAULT true,
    poll_interval_mins  INTEGER NOT NULL DEFAULT 720,  -- 12 hours
    last_polled_at      DATETIME
);

-- Match history — what was found, when, what was notified
CREATE TABLE matches (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    search_term_id          INTEGER NOT NULL REFERENCES search_terms(id),
    source_id               INTEGER NOT NULL REFERENCES sources(id),
    item_title              TEXT NOT NULL,
    item_url                TEXT,
    item_guid               TEXT,
    matched_at              DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    notification_channels   TEXT  -- JSON array: ["discord","pushover"]
);
```

---

## Source Plugin System

Single Rust trait, three implementations:

```rust
#[async_trait]
trait Source: Send + Sync {
    async fn fetch(&self, term: &SearchTerm) -> Result<Vec<SourceItem>>;
    fn source_type(&self) -> &'static str;
}

struct SourceItem {
    title:       String,
    url:         Option<String>,
    guid:        String,
    pub_date:    Option<DateTime<Utc>>,
    description: Option<String>,
    indexer:     Option<String>,  // for Torznab/Newznab results
    seeders:     Option<u32>,     // for Torznab results
}
```

### RssSource
Queries CommaFeed REST API (`/rest/feed/entries`) using the CC account credentials. Filters items to the last `max_age_days`. Also supports direct RSS/Atom feed URLs as a fallback for sources not managed through CommaFeed.

**CommaFeed auth:** Basic auth — `CC` / configured password.

### NewznabSource
Hits `<url>/api?t=search&q=<term>&apikey=<key>`, parses Newznab XML response. Returns results filtered by age.

### TorznabSource
Identical protocol to Newznab with `t=movie` or `t=search`. One implementation covers Prowlarr and Jackett — no separate adapter needed. Optional source; app works fully without it.

### Poll loop
Scheduler fires per source based on `poll_interval_mins`. For each poll: fetch all enabled search terms, call `source.fetch(term)` for each, run results through matching/filtering, fire notifications, log to `matches`.

---

## Matching & Filtering

Per search term:
- **Whole-word match** on item title (same regex boundary approach as current implementation)
- **max_age_days** — skip items older than N days (uses `pub_date` from feed item)
- **disallowed_keywords** — skip items whose title contains any of these (case-insensitive)

No category filtering at the source level — RSS feeds in CommaFeed are user-curated, so category is implicit.

---

## Notification System

Three channels, all optional. At least one must be configured.

| Channel | Method |
|---------|--------|
| Discord | Webhook — rich embed with title, source, item URL, optional box art from SteamGridDB |
| Apprise | Apprise URL (covers Telegram, Slack, ntfy, etc.) |
| Pushover | Direct API with image attachment |

**SteamGridDB** remains optional — if API key is set, box art is fetched and attached to Discord embeds. Falls back to placeholder image if not set or if no match found.

All channel config stored in the `sources` table concept is extended — notification config lives in a dedicated settings table or environment variables (TBD at implementation time based on preference).

---

## Web UI

Svelte frontend, four views:

### Dashboard
- Recent matches across all terms, sorted by date
- Shows: term name, item title, source, matched_at, notification channels fired
- Filterable by term or source

### Search Terms
- Table with add / edit / delete
- Per-term fields: name, query, max_age_days, disallowed_keywords, enabled toggle

### Sources
- Table with add / edit / delete
- Per-source fields: name, type, URL, API key, poll interval, enabled toggle
- **Test button** — fires a live fetch against a sample query and shows raw results inline

### Notifications
- Configure Discord webhook URL, Apprise URL, Pushover tokens
- **Test button** per channel — sends a sample notification

---

## Deployment

Single container on staging2 (192.168.160.161). Replaces the existing `discoprowl` container in-place.

```yaml
services:
  discoprowl:
    image: danktankk/discoprowl:latest
    ports:
      - "3079:3079"
    volumes:
      - ./data:/data  # SQLite lives here
    environment:
      DATABASE_URL: /data/discoprowl.db
      COMMAFEEED_URL: http://192.168.160.155:8882
      COMMAFEED_USER: CC
      COMMAFEED_PASS: ${COMMAFEED_PASS}
      # Notification channels (at least one required)
      DISCORD_WEBHOOK_URL: ${DISCORD_WEBHOOK_URL}
      APPRISE_URL: ${APPRISE_URL}
      PUSHOVER_APP_TOKEN: ${PUSHOVER_APP_TOKEN}
      PUSHOVER_USER_KEY: ${PUSHOVER_USER_KEY}
      # Optional
      STEAMGRIDDB_API_KEY: ${STEAMGRIDDB_API}
    restart: unless-stopped
    networks:
      - proxy
```

Port `3079` — adjacent to DiscoRelay at `3080`, keeps the Discord tooling ports grouped.

---

## Rust Crate Stack

| Crate | Purpose |
|-------|---------|
| `axum` | Web server + REST API |
| `tokio` | Async runtime |
| `sqlx` | SQLite async ORM |
| `reqwest` | HTTP client for sources + notifications |
| `feed-rs` | RSS/Atom parsing |
| `quick-xml` | Newznab/Torznab XML parsing |
| `serde` / `serde_json` | Serialization |
| `rust-embed` | Embed compiled Svelte assets in binary |
| `tokio-cron-scheduler` | Poll scheduling |
| `chrono` | Date/time handling |
| `tracing` | Structured logging |

---

## Out of Scope (v1)

- Authentication / multi-user support — planned for future pass, architecture must not preclude it
- Mobile app
- Auto-download integration (no grab/snatch — notify only)
- Feed management UI (feeds managed directly in CommaFeed)
