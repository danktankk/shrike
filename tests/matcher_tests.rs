// tests/matcher_tests.rs
use shrike::matcher::{age_ok, keywords_ok, whole_word_match};
use chrono::Utc;

#[test]
fn whole_word_match_exact() {
    assert!(whole_word_match("Hollow Knight", "Hollow Knight"));
}

#[test]
fn whole_word_match_partial_word_rejected() {
    // "hollow" should NOT match "HollowKnight" (no word boundary)
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
fn whole_word_match_rejects_no_match() {
    assert!(!whole_word_match("hollow knight", "Dark Souls III"));
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

#[test]
fn keywords_ok_empty_list_always_passes() {
    assert!(keywords_ok("Elden Ring TRAINER", &[]));
}

#[test]
fn whole_word_match_utf8_adjacent_no_false_positive() {
    // "ring" should NOT match "éring" (é directly precedes 'r')
    assert!(!whole_word_match("ring", "éring"));
}

#[test]
fn keywords_ok_mixed_case_keyword_blocked() {
    // Caller passes mixed-case keyword — should still block
    assert!(!keywords_ok("Elden Ring TRAINER", &["Trainer".to_string()]));
}
