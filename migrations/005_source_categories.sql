-- Adds an optional comma-separated list of newznab category IDs for search-based
-- sources (Prowlarr/Torznab/Newznab). When present, the source appends each id
-- as a `&categories=<n>` query param to narrow results. NULL or empty string =
-- no filter (old behavior).
ALTER TABLE sources ADD COLUMN categories TEXT;
