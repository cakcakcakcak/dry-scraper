DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'defending_side') THEN
        CREATE TYPE defending_side AS ENUM ('left', 'right');

END IF;
END $$;

CREATE TABLE IF NOT EXISTS nhl_play (
    game_id INTEGER NOT NULL REFERENCES nhl_game (id),
    event_id INTEGER NOT NULL,
    period_descriptor_number INTEGER NOT NULL,
    period_descriptor_type period_type NOT NULL,
    period_descriptor_max_regulation_periods INTEGER NOT NULL,
    time_in_period INTERVAL NOT NULL,
    time_remaining INTERVAL NOT NULL,
    situation_code TEXT,
    home_team_defending_side defending_side,
    type_code INTEGER NOT NULL,
    type_desc_key TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    details JSONB,
    raw_json JSONB NOT NULL,
    endpoint TEXT NOT NULL REFERENCES api_cache (endpoint),
    last_updated TIMESTAMPTZ DEFAULT now() NOT NULL,
    PRIMARY KEY (game_id, event_id),
    UNIQUE (game_id, sort_order)
)
