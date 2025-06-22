CREATE TABLE IF NOT EXISTS nhl_team (
    id CHAR(2) PRIMARY KEY,
    franchise_id CHAR(2) NOT NULL REFERENCES nhl_franchise(id),
    full_name TEXT NOT NULL,
    league_id CHAR(2) NOT NULL,
    raw_tricode CHAR(3) NOT NULL,
    tricode CHAR(3) NOT NULL,
    raw_json JSONB NOT NULL,
    api_cache_endpoint TEXT NOT NULL REFERENCES api_cache(endpoint),
    last_updated TIMESTAMP DEFAULT now()
)