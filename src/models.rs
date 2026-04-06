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

/// For inserting new search terms (no id/created_at — DB generates them).
#[derive(Debug, Deserialize)]
pub struct NewSearchTerm {
    pub name: String,
    pub query: String,
    pub enabled: Option<bool>,
    pub max_age_days: Option<i64>,
    pub disallowed_keywords: Option<String>,
}

/// For inserting new sources (no id/last_polled_at).
#[derive(Debug, Deserialize)]
pub struct NewSource {
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
    pub poll_interval_mins: Option<i64>,
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
            created_at: chrono::Utc::now(),
        };
        assert!(term.disallowed_list().is_empty());
    }
}
