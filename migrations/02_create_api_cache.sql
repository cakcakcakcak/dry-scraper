CREATE TABLE IF NOT EXISTS api_cache (
    endpoint TEXT PRIMARY KEY,
    manually_edited BOOLEAN DEFAULT FALSE NOT NULL,
    raw_data TEXT NOT NULL,
    last_updated TIMESTAMPTZ DEFAULT now() NOT NULL
)
