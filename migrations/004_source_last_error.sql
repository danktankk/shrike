-- migrations/004_source_last_error.sql
-- Surface fetch failures and last-known-good timestamp per source so the UI
-- can show whether an indexer is healthy instead of silently stalling.
ALTER TABLE sources ADD COLUMN last_error TEXT;
ALTER TABLE sources ADD COLUMN last_success_at TIMESTAMP;
