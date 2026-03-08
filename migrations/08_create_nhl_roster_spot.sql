CREATE TABLE IF NOT EXISTS nhl_roster_spot (
    game_id INTEGER NOT NULL REFERENCES nhl_game (id),
    player_id INTEGER NOT NULL REFERENCES nhl_player (id),
    team_id INTEGER NOT NULL REFERENCES nhl_team (id),
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    sweater_number INTEGER NOT NULL,
    position_code TEXT NOT NULL,
    headshot TEXT NOT NULL,
    raw_json JSONB NOT NULL,
    endpoint TEXT NOT NULL REFERENCES api_cache (endpoint),
    last_updated TIMESTAMPTZ DEFAULT now() NOT NULL,
    PRIMARY KEY (game_id, player_id)
)
