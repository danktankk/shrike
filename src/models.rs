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
    /// Optional pinned SteamGridDB game id. When set, art lookups skip
    /// autocomplete entirely and fetch directly for this id — disambiguates
    /// year-suffixed sequels (e.g. "PGA Tour 2025" vs "PGA Tour 2026") that
    /// share a name root and would otherwise collapse onto the same
    /// first-hit autocomplete result.
    pub steamgriddb_id: Option<i64>,
    /// Optional pinned Steam appid. Same purpose as `steamgriddb_id` but
    /// for the Steam storefront/news enrichment path.
    pub steam_appid: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl SearchTerm {
    /// Builds a transient SearchTerm used by test handlers (test_source, test_channel).
    /// Not persisted; id=0 and only `query` is meaningful.
    pub fn test_sentinel(query: impl Into<String>) -> Self {
        SearchTerm {
            id: 0,
            name: "Test".into(),
            query: query.into(),
            enabled: true,
            max_age_days: Some(30),
            disallowed_keywords: None,
            steamgriddb_id: None,
            steam_appid: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Returns disallowed keywords as a Vec<String> (lowercase, trimmed).
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
    pub last_error: Option<String>,
    pub last_success_at: Option<DateTime<Utc>>,
    /// Comma-separated newznab category IDs (e.g. "1000,4050"). NULL or empty = no filter.
    /// Only honored by search-based sources (Prowlarr/Torznab/Newznab).
    pub categories: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Match {
    pub id: i64,
    pub search_term_id: i64,
    pub source_id: i64,
    pub item_title: String,
    pub item_url: Option<String>,
    pub item_guid: String,
    pub matched_at: DateTime<Utc>,
    pub notification_channels: Option<String>,
}

/// Create/update payload for search terms (no id/created_at — DB generates them).
/// Used for both POST and PUT; update handlers COALESCE optional fields.
#[derive(Debug, Deserialize)]
pub struct SearchTermPayload {
    pub name: String,
    pub query: String,
    pub enabled: Option<bool>,
    pub max_age_days: Option<i64>,
    pub disallowed_keywords: Option<String>,
    pub steamgriddb_id: Option<i64>,
    pub steam_appid: Option<i64>,
}

/// Row from `sgdb_blocklist`. `pattern` is matched as a case-insensitive
/// whole-word substring against SGDB and Steam storefront hit names; if it
/// matches, the hit is suppressed unless a sequel/version marker follows
/// (digits ≥ 2, roman numerals II+, `v\d+`).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BlocklistEntry {
    pub id: i64,
    pub pattern: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BlocklistPayload {
    pub pattern: String,
}

/// Create/update payload for sources (no id/last_polled_at).
/// Used for both POST and PUT; update handlers COALESCE optional fields.
#[derive(Debug, Deserialize)]
pub struct SourcePayload {
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
    pub poll_interval_mins: Option<i64>,
    pub categories: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disallowed_list_parses_csv() {
        let term = SearchTerm {
            id: 1,
            name: "test".into(),
            query: "test".into(),
            enabled: true,
            max_age_days: Some(30),
            disallowed_keywords: Some("Trainer, CRACK , repack".into()),
            steamgriddb_id: None,
            steam_appid: None,
            created_at: chrono::Utc::now(),
        };
        let list = term.disallowed_list();
        assert_eq!(list, vec!["trainer", "crack", "repack"]);
    }

    #[test]
    fn disallowed_list_empty_when_none() {
        let term = SearchTerm {
            id: 1,
            name: "test".into(),
            query: "test".into(),
            enabled: true,
            max_age_days: None,
            disallowed_keywords: None,
            steamgriddb_id: None,
            steam_appid: None,
            created_at: chrono::Utc::now(),
        };
        assert!(term.disallowed_list().is_empty());
    }
}
