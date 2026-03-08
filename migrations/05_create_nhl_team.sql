CREATE TABLE IF NOT EXISTS nhl_team (
    id INTEGER PRIMARY KEY,
    franchise_id INTEGER REFERENCES nhl_franchise (id),
    full_name TEXT NOT NULL,
    league_id INTEGER NOT NULL,
    raw_tricode CHAR(3) NOT NULL,
    tricode CHAR(3) NOT NULL,
    raw_json JSONB NOT NULL,
    endpoint TEXT NOT NULL REFERENCES api_cache (endpoint),
    last_updated TIMESTAMPTZ DEFAULT now() NOT NULL
)
