-- migrations/003_matches_dedup.sql
-- Prevent duplicate match rows so notifications don't re-fire on every poll.
CREATE UNIQUE INDEX IF NOT EXISTS idx_matches_dedup
    ON matches (search_term_id, source_id, item_guid);
