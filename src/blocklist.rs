// src/blocklist.rs
//
// Title blocklist applied to SGDB autocomplete and Steam storesearch hits
// before they're chosen as the "winning" match. Patterns are matched
// case-insensitively at word boundaries. A match is suppressed UNLESS the
// matched title carries a sequel/version marker after the pattern — that
// is, the user wants to block "Hypervisor" but pass "Hypervisor 2",
// "Hypervisor v3", "Hypervisor: II", etc. through.
//
// Storage lives in the `sgdb_blocklist` table; this module is the
// in-memory matcher + DB loader. Patterns load on each SGDB/Steam call
// (the table is tiny — typically <50 rows — and a fresh read keeps the
// behavior trivially correct after edits via the API).

use anyhow::Result;
use sqlx::SqlitePool;

/// Load all blocklist patterns from the DB. Returns lowercased patterns
/// for direct comparison with lowercased titles.
pub async fn load(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT pattern FROM sgdb_blocklist ORDER BY id")
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(|(p,)| p.to_lowercase()).collect())
}

/// Returns `true` if `title` should be filtered out by this `pattern`.
///
/// Logic:
/// 1. Pattern must appear as a whole word (word boundaries) inside `title`.
/// 2. If a sequel/version marker immediately follows the pattern (after
///    optional separator characters), the title is **allowed through**
///    (returns `false`).
/// 3. Otherwise the title is **blocked** (returns `true`).
pub fn is_blocked_by(title: &str, pattern: &str) -> bool {
    let lower_title = title.to_lowercase();
    let lower_pat = pattern.trim().to_lowercase();
    if lower_pat.is_empty() {
        return false;
    }

    let Some(after) = find_after_whole_word(&lower_title, &lower_pat) else {
        return false;
    };

    !has_sequel_marker(after)
}

/// Returns `true` if ANY pattern in `patterns` blocks this `title`.
pub fn is_blocked(title: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| is_blocked_by(title, p))
}

/// Locate `pat` as a whole-word substring of `text` and return the slice
/// that follows. Both inputs must already be lowercase.
///
/// Whole-word means the byte preceding the match (if any) is NOT
/// alphanumeric or underscore, and same for the byte following — exactly
/// the contract of `\b<pat>\b`.
fn find_after_whole_word<'a>(text: &'a str, pat: &str) -> Option<&'a str> {
    let bytes = text.as_bytes();
    let pat_bytes = pat.as_bytes();
    let mut start = 0;
    while let Some(idx) = text[start..].find(pat) {
        let abs = start + idx;
        let end = abs + pat_bytes.len();
        let left_word = abs > 0 && is_word_byte(bytes[abs - 1]);
        let right_word = end < bytes.len() && is_word_byte(bytes[end]);
        if !left_word && !right_word {
            return Some(&text[end..]);
        }
        start = abs + 1;
    }
    None
}

#[inline]
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Returns `true` if the slice starts with a sequel/version marker after
/// optional separator characters (whitespace, `:`, `-`, `—`, `.`, `,`).
///
/// Recognized markers:
/// - `2`–`9` followed by additional digits then a non-word char or end.
/// - `v` then digits ≥ 2 (e.g. `v2`, `V10`).
/// - Roman numerals `II` … `X` (case-insensitive).
fn has_sequel_marker(after: &str) -> bool {
    let trimmed = after.trim_start_matches(|c: char| {
        c.is_whitespace() || matches!(c, ':' | '-' | '—' | '–' | '.' | ',')
    });
    let lower = trimmed.to_lowercase();
    let bytes = lower.as_bytes();

    // Plain digits — parse the run as an integer, must be ≥ 2 to qualify
    // as a sequel marker. ("10" qualifies; "1" does not.)
    if bytes.first().is_some_and(|b| b.is_ascii_digit()) {
        let mut i = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if !next_is_word(bytes, i) && lower[..i].parse::<u32>().unwrap_or(0) >= 2 {
            return true;
        }
    }

    // `v` then digits, again parsing the digit run as an integer ≥ 2.
    if bytes.first() == Some(&b'v') && bytes.get(1).is_some_and(|b| b.is_ascii_digit()) {
        let mut i = 2;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if !next_is_word(bytes, i) && lower[1..i].parse::<u32>().unwrap_or(0) >= 2 {
            return true;
        }
    }

    // Roman numerals II..X. Try longest first to avoid `vi` matching when
    // the title actually had `viii`.
    const ROMAN: &[&str] = &["viii", "vii", "iii", "iv", "ix", "vi", "ii", "x"];
    for r in ROMAN {
        if lower.starts_with(r) && !next_is_word(bytes, r.len()) {
            return true;
        }
    }

    false
}

#[inline]
fn next_is_word(bytes: &[u8], i: usize) -> bool {
    i < bytes.len() && is_word_byte(bytes[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_title_blocks() {
        assert!(is_blocked_by("Hypervisor", "hypervisor"));
    }

    #[test]
    fn case_insensitive() {
        assert!(is_blocked_by("HyPeRViSoR", "Hypervisor"));
    }

    #[test]
    fn sequel_digit_passes() {
        assert!(!is_blocked_by("Hypervisor 2", "hypervisor"));
        assert!(!is_blocked_by("Hypervisor 10", "hypervisor"));
        assert!(!is_blocked_by("Hypervisor: 2", "hypervisor"));
        assert!(!is_blocked_by("Hypervisor - 3", "hypervisor"));
    }

    #[test]
    fn sequel_v_marker_passes() {
        assert!(!is_blocked_by("Hypervisor v2", "hypervisor"));
        assert!(!is_blocked_by("Hypervisor V10", "hypervisor"));
    }

    #[test]
    fn sequel_roman_passes() {
        assert!(!is_blocked_by("Hypervisor II", "hypervisor"));
        assert!(!is_blocked_by("Hypervisor: III", "hypervisor"));
        assert!(!is_blocked_by("Hypervisor IV", "hypervisor"));
        assert!(!is_blocked_by("Hypervisor X", "hypervisor"));
    }

    #[test]
    fn embedded_word_blocks() {
        // Sequel marker absent — base name embedded in a longer title still blocks.
        assert!(is_blocked_by("The Hypervisor Chronicles", "hypervisor"));
    }

    #[test]
    fn partial_word_does_not_match() {
        // No whole-word match → not blocked.
        assert!(!is_blocked_by("Hyperviso", "hypervisor"));
        assert!(!is_blocked_by("Hypervisorx", "hypervisor"));
    }

    #[test]
    fn lone_digit_one_does_not_qualify_as_sequel() {
        // "Hypervisor 1" is the original numbered, still block it.
        assert!(is_blocked_by("Hypervisor 1", "hypervisor"));
        assert!(is_blocked_by("Hypervisor v1", "hypervisor"));
    }

    #[test]
    fn empty_pattern_never_blocks() {
        assert!(!is_blocked_by("anything", ""));
        assert!(!is_blocked_by("Hypervisor", "   "));
    }

    #[test]
    fn multi_pattern_any_match_blocks() {
        let pats = vec!["hypervisor".to_string(), "frogger".to_string()];
        assert!(is_blocked("Hypervisor", &pats));
        assert!(is_blocked("Frogger Returns", &pats));
        assert!(!is_blocked("Halo Infinite", &pats));
        assert!(!is_blocked("Hypervisor 2", &pats));
    }

    #[test]
    fn roman_disambiguation_does_not_match_words() {
        // "Hypervisor: Iron Curtain" starts with "Iron", not roman numeral I.
        // (Single-char "I" is intentionally NOT a sequel marker — the
        // original is "I" implicitly, and "I" prefixes too many real words.)
        assert!(is_blocked_by("Hypervisor: Iron Curtain", "hypervisor"));
        // "Hypervisor: III Edition" — III is a sequel marker.
        assert!(!is_blocked_by("Hypervisor: III Edition", "hypervisor"));
    }
}
