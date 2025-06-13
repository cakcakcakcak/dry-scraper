CREATE TABLE IF NOT EXISTS api_cache (
    endpoint TEXT PRIMARY KEY,
    raw_data TEXT NOT NULL,
    last_updated TIMESTAMP DEFAULT now()
)