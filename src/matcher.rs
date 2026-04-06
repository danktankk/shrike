// src/matcher.rs
use chrono::{DateTime, Utc};

/// Returns true if `query` appears as a whole-word match (case-insensitive) in `title`.
/// A word boundary is a position where the adjacent character is not alphanumeric or underscore.
pub fn whole_word_match(query: &str, title: &str) -> bool {
    let title_lower = title.to_lowercase();
    let query_lower = query.to_lowercase();

    if query_lower.is_empty() {
        return false;
    }

    let mut start = 0;
    while let Some(pos) = title_lower[start..].find(query_lower.as_str()) {
        let abs_pos = start + pos;
        let end = abs_pos + query_lower.len();

        let left_ok = title_lower[..abs_pos]
            .chars()
            .next_back()
            .map_or(true, |c| !is_word_char(c));

        let right_ok = title_lower[end..]
            .chars()
            .next()
            .map_or(true, |c| !is_word_char(c));

        if left_ok && right_ok {
            return true;
        }

        // Advance by one char (not one byte) to stay on valid UTF-8 boundaries
        start = abs_pos + title_lower[abs_pos..].chars().next().map_or(1, |c| c.len_utf8());
    }
    false
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Returns true if the item is within `max_age_days` days old (or has no date).
pub fn age_ok(pub_date: Option<DateTime<Utc>>, max_age_days: i64) -> bool {
    match pub_date {
        None => true,
        Some(dt) => {
            let age = Utc::now().signed_duration_since(dt).num_days();
            age >= 0 && age <= max_age_days
        }
    }
}

/// Returns true if the title contains none of the disallowed keywords (case-insensitive).
pub fn keywords_ok(title: &str, disallowed: &[String]) -> bool {
    if disallowed.is_empty() {
        return true;
    }
    let lower = title.to_lowercase();
    !disallowed.iter().any(|kw| lower.contains(kw.to_lowercase().as_str()))
}
