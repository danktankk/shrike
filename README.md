<p align="center">
  <img src="assets/shrike-banner.png/>
</p>

<p align="center">
  <strong>Shrike</strong> — a self-hosted release watcher.<br/>
  Watches RSS, Newznab, Torznab, and Prowlarr. Notifies Discord, Apprise, and Pushover. Rust binary with an embedded Svelte web UI.
</p>

<p align="center">
  <code>ghcr.io/danktankk/shrike:latest</code> &nbsp;·&nbsp; <code>:latest-alpine</code>
</p>

---

## Origin

Shrike started life as **discoprowl**, a 200-line Python single-file script. It had exactly one job: query a Prowlarr instance every few hours for a handful of hard-coded search terms, post any new hits to a Discord webhook, and fall asleep until the next tick. No web UI, no persistence, no history — if you missed the Discord ping, the release was gone. If you wanted to change the watchlist, you edited the `.env`, restarted the container, and hoped you got the quoting right.

It worked. It was also the kind of thing that only the person who wrote it could love.

## Why It Bloomed

The pain point that drove this project is embarrassingly universal: you watch a trailer for an upcoming game, you tell yourself *"I have to play that the day it comes out,"* and then three weeks later someone mentions it and you realize you forgot it exists. Shrike is the thing that refuses to let you forget. You tell it the name of a game (or a movie, or an album, or anything Prowlarr can index), and when a release shows up in any source you have configured, it pings you.

Once that basic loop was working reliably, the wishlist started growing:

- *It should support more than just Prowlarr.* Plain RSS feeds. Newznab. Torznab trackers without the Prowlarr wrapper.
- *It should have a proper UI.* Typing JSON into env vars is a bad user experience even for the author.
- *It should remember what it has already notified me about.* Getting paged three times for the same release at 2am gets old.
- *It should show me box art and release info, not just a filename.* If I'm going to spend half a second deciding whether this is the right game, I want the cover, not a torrent title.
- *It should group releases.* Four repacks of the same game should be one card with four entries in a drawer, not four separate notifications clogging the dashboard.
- *It should run on my phone.* The browser interface should be usable from the couch without pinch-zooming.

At that point the Python script was no longer the right shape for the job.

## The Rewrite

The whole thing was rewritten from scratch in Rust, with a Svelte frontend embedded into the binary at compile time via `rust-embed`. One binary, one container, zero runtime dependencies beyond `ca-certificates`. Persistent state lives in SQLite. A background scheduler polls each configured source on its own interval. Every new match is deduplicated by `(search_term_id, source_id, item_guid)` — re-notifications for the same release are structurally impossible. The web UI is served by the same binary on port `3079`.

## Why "Shrike"

"discoprowl" was tied to Prowlarr and to the moment the script was written. It no longer fit. The tool hunts, pins what it catches, and waits for you to come look — so it got named after the bird that does exactly that. A shrike is a small predatory songbird that hunts prey and impales its kills on thorns for later retrieval. It is the larder that watches.

The name also gave the project a reason for a clean break: a new repo name (`danktankk/shrike`), a new container image (`ghcr.io/danktankk/shrike`), a new version line starting at `v0.9.1`, and a README that tells the actual story instead of being the original Python README with a search-and-replace run over it.

## What It Is Now

- **Sources.** Pulls from RSS feeds (direct URL or via CommaFeed), Newznab indexers, Torznab trackers, and Prowlarr. Sources are defined in the UI — name, type, URL, API key, poll interval, enable flag, optional category filter.
- **Search terms.** User-defined watchlist entries with whole-word matching, `max_age_days`, and a `disallowed_keywords` blocklist. Both a global scan button and a per-term scan button for smoke-testing a freshly added term without re-polling the whole list.
- **Category filter.** For Prowlarr and other indexers that advertise categories, you can load the category tree from the indexer and tick which buckets a source should return. No more drowning in unrelated categories.
- **Matches dedup.** A unique index on `(search_term_id, source_id, item_guid)` guarantees a given release-on-an-indexer is notified exactly once. Re-scanning is idempotent.
- **Notifications.** At least one of Discord, Apprise, or Pushover must be configured. Discord posts a rich embed with optional SteamGridDB box art. Apprise gives you one URL that fans out to Telegram, Slack, ntfy, and anything else Apprise supports. Pushover includes an image attachment.
- **Metadata enrichment.** If a SteamGridDB API key is configured, Shrike fetches hero / grid / logo art per match and, where available, Steam metadata — short description, developers, publishers, genres, Metacritic score, release date, platforms, store URL. The match detail page renders it all.
- **Dashboard bundling.** Matches are grouped by search term: every hit for "Borderlands 4" collapses into a single card regardless of release-group, repack, or version suffix on the torrent name. The expandable drawer on each card lists every individual release with its raw filename, indexer, matched timestamp, and release link — so you can see exactly which repacks or versions are available without losing the per-release detail.
- **Match detail page.** Each match has its own `/match/:id` route with a hero banner, metadata pills, developer / publisher / genre breakdown, and the latest Steam news items.
- **Source health.** The scheduler tracks `last_polled_at`, `last_success_at`, and `last_error` per source and surfaces a silent-indexer warning when a source keeps returning zero items for too long.
- **Responsive UI.** The sidebar collapses to a sticky top bar at 720px and below. The modal becomes near-fullscreen on phones (with a `100dvh` fallback for iOS Safari). Data tables transform into a card layout on mobile via `data-label` attributes. Tap targets are at least 44px tall. Works on a phone without pinch-zooming.
- **Two runtime images.** `:latest` is the Debian-bookworm-slim variant. `:latest-alpine` is built against `rust:1.86-alpine3.20` with `OPENSSL_STATIC=1` and ships on `alpine:3.20` with `tini` as PID 1 — the binary has no runtime libssl/libcrypto dependency.

---

## Quick Start

```bash
docker pull ghcr.io/danktankk/shrike:latest
```

amd64 only at this release.

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
      DISCORD_WEBHOOK_URL: ${DISCORD_WEBHOOK_URL}
      APPRISE_URL: ${APPRISE_URL:-}
      PUSHOVER_APP_TOKEN: ${PUSHOVER_APP_TOKEN:-}
      PUSHOVER_USER_KEY: ${PUSHOVER_USER_KEY:-}
      STEAMGRIDDB_API_KEY: ${STEAMGRIDDB_API_KEY:-}
      RUST_LOG: info
    restart: unless-stopped
```

The web UI is at `http://your-host:3079` once the container is running. Add sources and search terms from the UI — no env-var gymnastics required.

## Environment Variables

### Required

| Variable | Description |
|---|---|
| `DATABASE_URL` | Path to the SQLite file — e.g. `/data/shrike.db` |

### At least one notification channel required

| Variable | Description |
|---|---|
| `DISCORD_WEBHOOK_URL` | Discord webhook URL |
| `APPRISE_URL` | Apprise-compatible URL (fans out to Telegram, Slack, ntfy, etc.) |
| `PUSHOVER_APP_TOKEN` + `PUSHOVER_USER_KEY` | Pushover credentials |

### Optional

| Variable | Description |
|---|---|
| `BIND_ADDR` | Listen address (default `0.0.0.0:3079`) |
| `COMMAFEED_URL` | CommaFeed instance URL for RSS sources routed through CommaFeed |
| `COMMAFEED_USER` | CommaFeed username |
| `COMMAFEED_PASS` | CommaFeed password |
| `STEAMGRIDDB_API_KEY` | Enables hero / grid / logo art and Steam metadata enrichment |
| `SHRIKE_SCHEDULER_TICK_SECS` | Scheduler tick interval in seconds (default `60`). The old name `DISCOPROWL_SCHEDULER_TICK_SECS` is still accepted as a fallback. |
| `RUST_LOG` | Log level — `info`, `debug`, etc. (default `info`) |

## Source Types

| Type | Description |
|---|---|
| `rss` | RSS / Atom feed — direct URL or via CommaFeed |
| `newznab` | Newznab-compatible Usenet indexer |
| `torznab` | Torznab-compatible tracker |
| `prowlarr` | Prowlarr instance (unified Torznab / Newznab proxy) |

## Web UI

| View | Purpose |
|---|---|
| Dashboard | Matches grouped by search term; expandable drawer per card shows each release. Global Scan Now button. |
| Search Terms | Add, edit, delete, and per-row Scan for single-term smoke tests. |
| Sources | Add, edit, delete, test live fetch, and pick category filters. |
| Notifications | Channel status and per-channel test button. |
| Match detail | `/match/:id` — hero art, metadata, release info, news. |

---

## Migrating from discoprowl

If you were running the old `ghcr.io/danktankk/discoprowl` image:

- **Image:** change `ghcr.io/danktankk/discoprowl:latest` → `ghcr.io/danktankk/shrike:latest` (or `:latest-alpine`).
- **Stack / service name:** rename `discoprowl` → `shrike` in your compose or Komodo stack (cosmetic).
- **Database file:** either rename your data volume file from `discoprowl.db` to `shrike.db`, or keep `DATABASE_URL` pointing at the old filename. Both work.
- **Env vars:** `DISCOPROWL_SCHEDULER_TICK_SECS` is still accepted as a fallback for `SHRIKE_SCHEDULER_TICK_SECS`. No action required for one minor version.
- **Old repo URL:** `github.com/danktankk/discoprowl` auto-redirects to `github.com/danktankk/shrike`. Existing clones still work but should update their remote to the new URL.

---

## Maintainer

- [danktankk](https://github.com/danktankk)

## License

MIT
