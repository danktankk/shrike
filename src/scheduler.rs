// src/scheduler.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use sqlx::{Row, SqlitePool};
use tracing::{info, warn, error};

use crate::models::{Source, SearchTerm};
use crate::matcher::{whole_word_match, age_ok, keywords_ok, DEFAULT_MAX_AGE_DAYS};
use crate::notifier::Notifier;
use crate::sources::{build_source, SourceItem};

/// After this many consecutive fetch cycles return Ok but zero items across
/// ALL terms, the scheduler writes a warning to `sources.last_error` so the UI
/// surfaces the silent failure. A fully-silent source is almost always a
/// misconfiguration (bad category, stale API key, wrong URL) and must not fail
/// quietly just because fetches technically succeed.
pub(crate) const EMPTY_FETCH_WARN_THRESHOLD: u32 = 3;

/// Outcome of a single source poll cycle, used to decide which DB columns
/// to stamp and whether to surface a silent-indexer warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PollOutcome {
    /// Fetch(es) returned Ok and at least one item came back.
    SuccessWithItems,
    /// Fetch(es) returned Ok but zero items across all terms.
    SuccessEmpty,
    /// No fetch returned Ok; carries the last error message seen.
    Failed(String),
}

/// Pure decision function: given the current poll outcome and the running
/// per-source empty-streak counter, return the new streak value and the
/// `last_error` string that should be written (None = clear).
///
/// Extracted from `poll_sources` so scheduling decisions can be unit-tested
/// without a running DB or network.
pub(crate) fn decide_stamp(
    outcome: &PollOutcome,
    current_streak: u32,
) -> (u32, Option<String>) {
    match outcome {
        PollOutcome::SuccessWithItems => (0, None),
        PollOutcome::SuccessEmpty => {
            let new_streak = current_streak + 1;
            if new_streak >= EMPTY_FETCH_WARN_THRESHOLD {
                (
                    new_streak,
                    Some(format!(
                        "no items returned in {new_streak} consecutive polls — check source configuration"
                    )),
                )
            } else {
                (new_streak, None)
            }
        }
        PollOutcome::Failed(err) => (current_streak, Some(err.clone())),
    }
}

/// Starts the background poll loop. Runs indefinitely — call from a spawned task.
/// `tick_secs` controls how often the loop wakes to check for due sources
/// (sourced from `config.scheduler_tick_secs`).
pub async fn run(pool: SqlitePool, notifier: Arc<Notifier>, http: reqwest::Client, tick_secs: u64) {
    info!("Scheduler started — polling every {}s", tick_secs);
    let mut tick = interval(Duration::from_secs(tick_secs));
    // Counter of consecutive fetch cycles that returned Ok but no items, keyed
    // by source id. Persists for the lifetime of the scheduler task only; a
    // restart resets the warning, which is acceptable.
    let mut empty_streaks: HashMap<i64, u32> = HashMap::new();
    loop {
        tick.tick().await;
        if let Err(e) = poll_due_sources(&pool, &notifier, &http, &mut empty_streaks).await {
            error!("Scheduler poll error: {e}");
        }
    }
}

/// Force-polls all enabled sources immediately, ignoring their schedule.
/// Returns the total number of matches found and recorded.
pub async fn scan_now(
    pool: &SqlitePool,
    notifier: &Arc<Notifier>,
    http: &reqwest::Client,
) -> anyhow::Result<usize> {
    let sources: Vec<Source> = sqlx::query_as(
        "SELECT * FROM sources WHERE enabled = 1"
    )
    .fetch_all(pool)
    .await?;

    if sources.is_empty() {
        return Ok(0);
    }

    let terms: Vec<SearchTerm> = sqlx::query_as(
        "SELECT * FROM search_terms WHERE enabled = 1"
    )
    .fetch_all(pool)
    .await?;

    if terms.is_empty() {
        info!("scan_now: no enabled search terms — skipping");
        return Ok(0);
    }

    info!("Manual scan: {} source(s), {} term(s)", sources.len(), terms.len());
    // Manual scans don't carry long-lived state, so give them a throwaway
    // counter. Their results never roll into the warning threshold.
    let mut streaks = HashMap::new();
    poll_sources(pool, notifier, http, sources, &terms, &mut streaks).await
}

/// Force-polls all enabled sources for a SINGLE search term, ignoring its
/// enabled flag so users can smoke-test freshly-added terms. Reuses the same
/// `poll_sources` pipeline as full scans, so match dedup, notifications, and
/// source health stamping all behave identically.
///
/// Side effect: search-based sources (Prowlarr/Newznab/Torznab) will have
/// their `last_polled_at` bumped, which defers the next scheduled poll by one
/// interval. This is intentional — per-term scans share the indexer budget.
pub async fn scan_now_term(
    pool: &SqlitePool,
    notifier: &Arc<Notifier>,
    http: &reqwest::Client,
    term_id: i64,
) -> anyhow::Result<usize> {
    let term: Option<SearchTerm> = sqlx::query_as(
        "SELECT * FROM search_terms WHERE id = ?"
    )
    .bind(term_id)
    .fetch_optional(pool)
    .await?;

    let term = match term {
        Some(t) => t,
        None => return Err(anyhow::anyhow!("search term {term_id} not found")),
    };

    let sources: Vec<Source> = sqlx::query_as(
        "SELECT * FROM sources WHERE enabled = 1"
    )
    .fetch_all(pool)
    .await?;

    if sources.is_empty() {
        return Ok(0);
    }

    info!("Per-term scan: term '{}' across {} source(s)", term.name, sources.len());
    let mut streaks = HashMap::new();
    poll_sources(pool, notifier, http, sources, std::slice::from_ref(&term), &mut streaks).await
}

async fn poll_due_sources(
    pool: &SqlitePool,
    notifier: &Arc<Notifier>,
    http: &reqwest::Client,
    empty_streaks: &mut HashMap<i64, u32>,
) -> anyhow::Result<()> {
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

    let terms: Vec<SearchTerm> = sqlx::query_as(
        "SELECT * FROM search_terms WHERE enabled = 1"
    )
    .fetch_all(pool)
    .await?;

    if terms.is_empty() {
        info!("No enabled search terms — skipping poll");
        return Ok(());
    }

    info!("Polling {} due source(s)", sources.len());
    poll_sources(pool, notifier, http, sources, &terms, empty_streaks).await?;
    Ok(())
}

async fn poll_sources(
    pool: &SqlitePool,
    notifier: &Arc<Notifier>,
    http: &reqwest::Client,
    sources: Vec<Source>,
    terms: &[SearchTerm],
    empty_streaks: &mut HashMap<i64, u32>,
) -> anyhow::Result<usize> {
    let mut total_matches = 0usize;

    for source in sources {
        info!("Polling source '{}' ({})", source.name, source.source_type);

        let source_impl = match build_source(&source, http.clone()) {
            Ok(s) => s,
            Err(e) => {
                warn!("Cannot build source '{}': {e} — skipping", source.name);
                continue;
            }
        };

        // Per-source poll outcome tracking:
        //   fetch_succeeded  — at least one fetch call returned Ok(_)
        //   items_seen       — at least one fetch returned a non-empty list
        //   last_fetch_error — most recent fetch error message (if any)
        let mut fetch_succeeded = false;
        let mut items_seen = false;
        let mut last_fetch_error: Option<String> = None;

        if source_impl.is_search_based() {
            // Search-based (Newznab, Torznab, Prowlarr): query per term
            for term in terms {
                let items = match source_impl.fetch(term).await {
                    Ok(i) => i,
                    Err(e) => {
                        let msg = format!("term '{}': {e}", term.query);
                        warn!("Fetch failed for term '{}' on source '{}': {e}", term.query, source.name);
                        last_fetch_error = Some(msg);
                        continue;
                    }
                };
                fetch_succeeded = true;
                if !items.is_empty() { items_seen = true; }
                total_matches += process_matches(pool, notifier, &source, term, items).await;
            }
        } else {
            // Feed-based (RSS): fetch once, filter all terms client-side
            // Feed-based fetches don't use the term's query (the RSS source
            // ignores it and returns all items), so a transient sentinel is fine.
            let dummy_term = SearchTerm::test_sentinel("");
            match source_impl.fetch(&dummy_term).await {
                Ok(items) => {
                    fetch_succeeded = true;
                    if !items.is_empty() { items_seen = true; }
                    info!("Source '{}' returned {} item(s)", source.name, items.len());
                    for term in terms {
                        total_matches += process_matches(pool, notifier, &source, term, items.clone()).await;
                    }
                }
                Err(e) => {
                    warn!("Fetch failed for source '{}': {e}", source.name);
                    last_fetch_error = Some(e.to_string());
                }
            }
        }

        // Classify the poll outcome, then hand off to the pure `decide_stamp`
        // helper to update the empty-streak counter and derive the last_error
        // message. This keeps the scheduling policy unit-testable.
        let outcome = if fetch_succeeded {
            if items_seen { PollOutcome::SuccessWithItems } else { PollOutcome::SuccessEmpty }
        } else {
            PollOutcome::Failed(
                last_fetch_error.unwrap_or_else(|| "unknown fetch error".to_string()),
            )
        };

        let current_streak = empty_streaks.get(&source.id).copied().unwrap_or(0);
        let (new_streak, last_error_msg) = decide_stamp(&outcome, current_streak);
        if new_streak == 0 {
            empty_streaks.remove(&source.id);
        } else {
            empty_streaks.insert(source.id, new_streak);
        }

        // Writes diverge on outcome:
        // - Success (with or without items): bump last_polled_at + last_success_at
        // - Failure: record last_error only; leave last_polled_at alone so a
        //   fully-broken source doesn't silently wait out its poll interval.
        let stamp_res = match &outcome {
            PollOutcome::SuccessWithItems | PollOutcome::SuccessEmpty => {
                sqlx::query(
                    "UPDATE sources
                        SET last_polled_at  = datetime('now'),
                            last_success_at = datetime('now'),
                            last_error      = ?
                      WHERE id = ?",
                )
                .bind(last_error_msg.as_deref())
                .bind(source.id)
                .execute(pool)
                .await
            }
            PollOutcome::Failed(_) => {
                sqlx::query("UPDATE sources SET last_error = ? WHERE id = ?")
                    .bind(last_error_msg.as_deref())
                    .bind(source.id)
                    .execute(pool)
                    .await
            }
        };
        if let Err(e) = stamp_res {
            error!("Failed to update source {} status: {e}", source.id);
        }
    }

    Ok(total_matches)
}

async fn process_matches(
    pool: &SqlitePool,
    notifier: &Arc<Notifier>,
    source: &Source,
    term: &SearchTerm,
    items: Vec<SourceItem>,
) -> usize {
    let max_age = term.max_age_days.unwrap_or(DEFAULT_MAX_AGE_DAYS);
    let disallowed = term.disallowed_list();
    let mut count = 0usize;

    for item in items {
        if !whole_word_match(&term.query, &item.title) { continue; }
        if !age_ok(item.pub_date, max_age) { continue; }
        if !keywords_ok(&item.title, &disallowed) { continue; }

        // Reserve the row in a single statement via the unique dedup index.
        // RETURNING id yields Some(id) for a fresh insert and None for a
        // duplicate (ON CONFLICT DO NOTHING suppresses the row). This replaces
        // the old INSERT-OR-IGNORE + composite-key UPDATE pair and guarantees
        // the post-notify update targets exactly the row we just reserved.
        let insert_res: Result<Option<i64>, sqlx::Error> = sqlx::query(
            r#"INSERT INTO matches
               (search_term_id, source_id, item_title, item_url, item_guid, notification_channels)
               VALUES (?, ?, ?, ?, ?, '')
               ON CONFLICT (search_term_id, source_id, item_guid) DO NOTHING
               RETURNING id"#
        )
        .bind(term.id)
        .bind(source.id)
        .bind(&item.title)
        .bind(&item.url)
        .bind(&item.guid)
        .fetch_optional(pool)
        .await
        .and_then(|row| row.map(|r| r.try_get::<i64, _>("id")).transpose());

        match insert_res {
            Ok(Some(match_id)) => {
                let channels = notifier.notify(term, &item, &source.name).await;
                if let Err(e) = sqlx::query(
                    "UPDATE matches SET notification_channels = ? WHERE id = ?"
                )
                .bind(&channels)
                .bind(match_id)
                .execute(pool)
                .await {
                    error!("Failed to record notification channels for '{}': {e}", item.title);
                }
                count += 1;
                info!("Matched: '{}' for term '{}' (channels: {})", item.title, term.query, channels);
            }
            Ok(None) => {
                // Duplicate — already recorded in a previous poll, skip notify.
            }
            Err(e) => {
                error!("Failed to log match '{}': {e}", item.title);
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::sources::SourceItem;

    async fn test_pool() -> SqlitePool {
        let pool = crate::db::init_pool("sqlite::memory:").await.unwrap();
        sqlx::query(
            "INSERT INTO search_terms (id, name, query, enabled) VALUES (1, 'halo', 'halo', 1)"
        ).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO sources (id, name, source_type, url, enabled, poll_interval_mins)
             VALUES (1, 'rss', 'rss', 'http://example.invalid/feed', 1, 60)"
        ).execute(&pool).await.unwrap();
        pool
    }

    fn test_notifier() -> Arc<Notifier> {
        // Build a Config directly with all channels disabled. notify() then
        // returns "[]" and never touches the network — critical for hermetic
        // tests that can't depend on DATABASE_URL / webhook env vars.
        let config = Arc::new(Config {
            database_url: ":memory:".into(),
            bind_addr: "127.0.0.1:0".into(),
            discord_webhook_url: None,
            apprise_url: None,
            pushover_app_token: None,
            pushover_user_key: None,
            steamgriddb_api_key: None,
            scheduler_tick_secs: 60,
        });
        Arc::new(Notifier::new(config, reqwest::Client::new()))
    }

    fn term() -> SearchTerm {
        SearchTerm {
            id: 1,
            name: "halo".into(),
            query: "halo".into(),
            enabled: true,
            max_age_days: Some(3650),
            disallowed_keywords: None,
            created_at: chrono::Utc::now(),
        }
    }

    fn source() -> Source {
        Source {
            id: 1,
            name: "rss".into(),
            source_type: "rss".into(),
            url: "http://example.invalid/feed".into(),
            api_key: None,
            enabled: true,
            poll_interval_mins: 60,
            last_polled_at: None,
            last_error: None,
            last_success_at: None,
        }
    }

    fn item(guid: &str, title: &str) -> SourceItem {
        SourceItem {
            title: title.into(),
            url: Some("http://example.invalid/item".into()),
            guid: guid.into(),
            pub_date: Some(chrono::Utc::now()),
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn process_matches_dedups_across_calls() {
        let pool = test_pool().await;
        let notifier = test_notifier();
        let t = term();
        let s = source();

        let items = vec![item("guid-a", "Halo Infinite")];

        let first = process_matches(&pool, &notifier, &s, &t, items.clone()).await;
        assert_eq!(first, 1, "first insert should match once");

        let second = process_matches(&pool, &notifier, &s, &t, items).await;
        assert_eq!(second, 0, "same guid must not match again");

        let row_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM matches")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row_count.0, 1, "dedup index must prevent duplicate rows");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn process_matches_distinct_guids_both_insert() {
        let pool = test_pool().await;
        let notifier = test_notifier();
        let t = term();
        let s = source();

        let n = process_matches(
            &pool, &notifier, &s, &t,
            vec![item("guid-a", "Halo One"), item("guid-b", "Halo Two")],
        ).await;
        assert_eq!(n, 2);
    }

    #[test]
    fn decide_stamp_clears_on_success_with_items() {
        let (streak, err) = decide_stamp(&PollOutcome::SuccessWithItems, 7);
        assert_eq!(streak, 0, "any real success must reset the streak");
        assert!(err.is_none(), "real success must clear last_error");
    }

    #[test]
    fn decide_stamp_increments_empty_streak_silently_below_threshold() {
        // Below the threshold we stay quiet — a single empty cycle isn't yet
        // a misconfiguration signal.
        let (streak, err) = decide_stamp(&PollOutcome::SuccessEmpty, 0);
        assert_eq!(streak, 1);
        assert!(err.is_none());
    }

    #[test]
    fn decide_stamp_surfaces_warning_at_threshold() {
        // At the threshold boundary the helper must write a warning to
        // last_error — this is the whole point of the empty-streak counter.
        let (streak, err) = decide_stamp(
            &PollOutcome::SuccessEmpty,
            EMPTY_FETCH_WARN_THRESHOLD - 1,
        );
        assert_eq!(streak, EMPTY_FETCH_WARN_THRESHOLD);
        let msg = err.expect("threshold hit must produce a warning");
        assert!(msg.contains("no items returned"));
        assert!(msg.contains(&EMPTY_FETCH_WARN_THRESHOLD.to_string()));
    }

    #[test]
    fn decide_stamp_failure_preserves_streak_and_records_error() {
        // A hard failure is a different signal class — don't touch the
        // empty-streak counter, do record the error verbatim.
        let (streak, err) =
            decide_stamp(&PollOutcome::Failed("connection refused".into()), 2);
        assert_eq!(streak, 2);
        assert_eq!(err.as_deref(), Some("connection refused"));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn last_polled_at_only_stamped_on_success() {
        let pool = test_pool().await;

        // Simulate a failure cycle: scheduler writes last_error only, leaves
        // last_polled_at NULL.
        sqlx::query("UPDATE sources SET last_error = 'boom' WHERE id = 1")
            .execute(&pool).await.unwrap();

        let row: (Option<chrono::DateTime<chrono::Utc>>, Option<String>) =
            sqlx::query_as("SELECT last_polled_at, last_error FROM sources WHERE id = 1")
                .fetch_one(&pool).await.unwrap();
        assert!(row.0.is_none(), "last_polled_at must stay NULL on failure");
        assert_eq!(row.1.as_deref(), Some("boom"));

        // Simulate a success cycle.
        sqlx::query(
            "UPDATE sources
                SET last_polled_at  = datetime('now'),
                    last_success_at = datetime('now'),
                    last_error      = NULL
              WHERE id = 1"
        ).execute(&pool).await.unwrap();

        let row: (Option<chrono::DateTime<chrono::Utc>>, Option<String>) =
            sqlx::query_as("SELECT last_polled_at, last_error FROM sources WHERE id = 1")
                .fetch_one(&pool).await.unwrap();
        assert!(row.0.is_some(), "last_polled_at must be stamped on success");
        assert!(row.1.is_none(), "last_error must clear on success");
    }
}
