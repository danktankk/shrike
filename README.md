<p align="center">
  <img src="https://raw.githubusercontent.com/danktankk/shrike/main/assets/logo-circular.png" alt="Shrike Icon" height="100" style="vertical-align: middle;"/>
  <img src="https://raw.githubusercontent.com/danktankk/shrike/main/assets/logo-namer.png" alt="Shrike Text" height="65" style="vertical-align: middle; margin-left: 10px;"/>
</p>

<p align="center">
  <strong>v2.0.0 — Rust rewrite</strong>
</p>

---

## What Is Shrike?

Have you ever found yourself watching upcoming AAA game title videos and made a mental list of games you *had* to have — only to completely forget about them until someone brought them up in conversation? Yeah, me neither. But just in case, Shrike has you covered.

**Shrike** watches configured sources for game titles (or anything, really) you care about and notifies you the moment they appear. Set it and forget it.

**v2.0 is a full rewrite in Rust.** It ships as a single binary with an embedded web UI — no more managing search terms via env vars. Add sources, define search terms, and monitor match history all from the browser.

---

## What's New in v2.0

- **Web UI** — Manage search terms, sources, and notifications from a clean browser interface (port `3079`)
- **Multiple source types** — RSS feeds (via CommaFeed), Newznab indexers, and Torznab (Prowlarr/Jackett) — all in one app
- **SQLite persistence** — Match history is stored and browsable; no more re-notifying without context
- **Per-source poll intervals** — Each source can have its own schedule (default 12h)
- **Background scheduler** — Polls automatically based on each source's interval; no manual restarts needed
- **Single binary** — Svelte frontend embedded at compile time via `rust-embed`
- **No dedup by design** — Every poll cycle re-notifies on matches, so if you miss a notification it comes back around

---

## Quick Start

```bash
docker pull ghcr.io/danktankk/shrike:latest
```

Multi-arch image — supports `linux/amd64` and `linux/arm64`.

---

## Docker Compose

```yaml
services:
  shrike:
    image: ghcr.io/danktankk/shrike:latest
    ports:
      - "3079:3079"
    volumes:
      - ./data:/data
    environment:
      DATABASE_URL: /data/shrike.db
      BIND_ADDR: 0.0.0.0:3079
      COMMAFEED_URL: http://your-commafeed:8882
      COMMAFEED_USER: CC
      COMMAFEED_PASS: ${COMMAFEED_PASS}
      DISCORD_WEBHOOK_URL: ${DISCORD_WEBHOOK_URL}
      APPRISE_URL: ${APPRISE_URL:-}
      PUSHOVER_APP_TOKEN: ${PUSHOVER_APP_TOKEN:-}
      PUSHOVER_USER_KEY: ${PUSHOVER_USER_KEY:-}
      STEAMGRIDDB_API_KEY: ${STEAMGRIDDB_API_KEY:-}
      RUST_LOG: info
    restart: unless-stopped
```

The web UI is available at `http://your-host:3079` once the container is running.

---

## Notification Channels

At least one must be configured:

| Channel | Method |
|---------|--------|
| Discord | Webhook with rich embed + optional SteamGridDB box art |
| Apprise | Single URL covering Telegram, Slack, ntfy, and more |
| Pushover | Direct API with image attachment |

---

## Environment Variables

### Required

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | Path to SQLite file — e.g. `/data/shrike.db` |

### Optional

| Variable | Description |
|----------|-------------|
| `BIND_ADDR` | Listen address (default: `0.0.0.0:3079`) |
| `COMMAFEED_URL` | CommaFeed instance URL for RSS sources |
| `COMMAFEED_USER` | CommaFeed username (default: `CC`) |
| `COMMAFEED_PASS` | CommaFeed password |
| `DISCORD_WEBHOOK_URL` | Discord webhook URL |
| `APPRISE_URL` | Apprise-compatible URL |
| `PUSHOVER_APP_TOKEN` | Pushover App Token |
| `PUSHOVER_USER_KEY` | Pushover User Key |
| `STEAMGRIDDB_API_KEY` | API key for box art (optional — falls back to placeholder) |
| `RUST_LOG` | Log level — e.g. `info`, `debug` (default: `info`) |

At least one notification channel (`DISCORD_WEBHOOK_URL`, `APPRISE_URL`, or both Pushover vars) must be set at startup.

---

## Source Types

| Type | Description |
|------|-------------|
| `rss` | RSS/Atom feed — direct URL or via CommaFeed REST API |
| `newznab` | Newznab-compatible Usenet indexer |
| `torznab` | Torznab-compatible tracker (Prowlarr, Jackett) |

Sources are added and managed through the UI. Each source has its own poll interval, API key, and enable/disable toggle.

---

## Matching & Filtering

Per search term:

- **Whole-word match** on item title (regex boundary, case-insensitive)
- **max_age_days** — skip items older than N days
- **disallowed_keywords** — skip items whose title contains any of these (comma-separated, case-insensitive)

---

## Web UI

| View | Purpose |
|------|---------|
| Dashboard | Recent match history across all terms and sources, filterable |
| Search Terms | Add / edit / delete search terms |
| Sources | Add / edit / delete sources; Test button for live fetch |
| Notifications | Channel status and per-channel test button |

---

## Contributor

- [danktankk](https://github.com/danktankk)

---

## License

MIT
