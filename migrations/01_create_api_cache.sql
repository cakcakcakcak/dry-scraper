CREATE TABLE IF NOT EXISTS api_cache (
    url TEXT PRIMARY KEY,
    raw_json JSONB NOT NULL,
    last_updated TIMESTAMP DEFAULT now()
)