CREATE TABLE IF NOT EXISTS nhl_franchise (
    id CHAR(2) PRIMARY KEY,
    full_name TEXT NOT NULL,
    team_common_name TEXT NOT NULL,
    team_place_name TEXT NOT NULL,
    raw_json JSONB NOT NULL,
    api_cache_endpoint TEXT NOT NULL REFERENCES api_cache(endpoint),
    last_updated TIMESTAMP DEFAULT now()
)