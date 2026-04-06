-- migrations/001_initial.sql

CREATE TABLE IF NOT EXISTS search_terms (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    name                TEXT NOT NULL,
    query               TEXT NOT NULL,
    enabled             BOOLEAN NOT NULL DEFAULT 1,
    max_age_days        INTEGER DEFAULT 30,
    disallowed_keywords TEXT,
    created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS sources (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    name                TEXT NOT NULL,
    source_type         TEXT NOT NULL CHECK(source_type IN ('rss','newznab','torznab')),
    url                 TEXT NOT NULL,
    api_key             TEXT,
    enabled             BOOLEAN NOT NULL DEFAULT 1,
    poll_interval_mins  INTEGER NOT NULL DEFAULT 720,
    last_polled_at      DATETIME
);

CREATE TABLE IF NOT EXISTS matches (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    search_term_id        INTEGER NOT NULL REFERENCES search_terms(id),
    source_id             INTEGER NOT NULL REFERENCES sources(id),
    item_title            TEXT NOT NULL,
    item_url              TEXT,
    item_guid             TEXT,
    matched_at            DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    notification_channels TEXT
);
