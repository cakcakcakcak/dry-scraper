DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'game_type') THEN
        CREATE TYPE game_type AS ENUM ('preseason', 'regular_season', 'playoffs');
    END IF;
END$$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'period_type') THEN
        CREATE TYPE period_type AS ENUM ('regulation', 'overtime', 'shootout');

END IF;

END $$;

CREATE TABLE IF NOT EXISTS nhl_game (
    id INTEGER PRIMARY KEY,
    season INTEGER REFERENCES nhl_season (id) NOT NULL,
    game_type game_type NOT NULL,
    limited_scoring BOOL NOT NULL,
    game_date DATE NOT NULL,
    venue TEXT NOT NULL,
    venue_location TEXT NOT NULL,
    start_time_utc TIMESTAMPTZ NOT NULL,
    eastern_utc_offset CHAR(6) NOT NULL,
    venue_utc_offset CHAR(6) NOT NULL,
    period_descriptor_number INTEGER NOT NULL,
    period_descriptor_type period_type NOT NULL,
    period_descriptor_max_regulation_periods INTEGER NOT NULL,
    away_team_id INTEGER REFERENCES nhl_team (id) NOT NULL,
    away_team_name TEXT NOT NULL,
    away_team_abbrev CHAR(3) NOT NULL,
    away_team_score INTEGER NOT NULL,
    away_team_sog INTEGER,
    away_team_logo TEXT NOT NULL,
    away_team_dark_logo TEXT NOT NULL,
    away_team_place_name TEXT NOT NULL,
    away_team_place_name_with_preposition TEXT NOT NULL,
    home_team_id INTEGER REFERENCES nhl_team (id) NOT NULL,
    home_team_name TEXT NOT NULL,
    home_team_abbrev CHAR(3) NOT NULL,
    home_team_score INTEGER NOT NULL,
    home_team_sog INTEGER,
    home_team_logo TEXT NOT NULL,
    home_team_dark_logo TEXT NOT NULL,
    home_team_place_name TEXT NOT NULL,
    home_team_place_name_with_preposition TEXT NOT NULL,
    shootout_in_use BOOL NOT NULL,
    ot_in_use BOOL NOT NULL,
    display_period INTEGER NOT NULL,
    max_periods INTEGER,
    game_outcome_last_period_type period_type NOT NULL,
    reg_periods INTEGER NOT NULL,
    raw_json JSONB NOT NULL,
    endpoint TEXT NOT NULL REFERENCES api_cache (endpoint),
    last_updated TIMESTAMPTZ DEFAULT now() NOT NULL
)
