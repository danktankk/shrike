// src/matcher.rs
use chrono::{DateTime, Utc};

/// Returns true if `query` appears as a whole-word match (case-insensitive) in `title`.
/// A word boundary is defined as: position is at start/end of string, or
/// the adjacent character is not alphanumeric and not underscore.
pub fn whole_word_match(query: &str, title: &str) -> bool {
    let title_lower = title.to_lowercase();
    let query_lower = query.to_lowercase();

    if query_lower.is_empty() {
        return false;
    }

    let title_bytes = title_lower.as_bytes();
    let query_bytes = query_lower.as_bytes();
    let qlen = query_bytes.len();
    let tlen = title_bytes.len();

    if qlen > tlen {
        return false;
    }

    let mut i = 0;
    while i <= tlen - qlen {
        if title_bytes[i..i + qlen] == *query_bytes {
            // Check left boundary
            let left_ok = i == 0 || !is_word_char(title_bytes[i - 1] as char);
            // Check right boundary
            let right_ok = i + qlen >= tlen || !is_word_char(title_bytes[i + qlen] as char);
            if left_ok && right_ok {
                return true;
            }
        }
        i += 1;
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
            age <= max_age_days
        }
    }
}

/// Returns true if the title contains none of the disallowed keywords (case-insensitive).
pub fn keywords_ok(title: &str, disallowed: &[String]) -> bool {
    if disallowed.is_empty() {
        return true;
    }
    let lower = title.to_lowercase();
    !disallowed.iter().any(|kw| lower.contains(kw.as_str()))
}
