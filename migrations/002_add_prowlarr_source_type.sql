-- migrations/002_add_prowlarr_source_type.sql
-- SQLite does not support ALTER TABLE ... DROP CONSTRAINT.
-- Recreate the sources table with the updated CHECK constraint.

CREATE TABLE sources_new (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    name                TEXT NOT NULL,
    source_type         TEXT NOT NULL CHECK(source_type IN ('rss','newznab','torznab','prowlarr')),
    url                 TEXT NOT NULL,
    api_key             TEXT,
    enabled             BOOLEAN NOT NULL DEFAULT 1,
    poll_interval_mins  INTEGER NOT NULL DEFAULT 720,
    last_polled_at      DATETIME
);

INSERT INTO sources_new SELECT * FROM sources;

DROP TABLE sources;

ALTER TABLE sources_new RENAME TO sources;
