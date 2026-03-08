CREATE TABLE IF NOT EXISTS data_source_error (
    id SERIAL PRIMARY KEY,
    error_message TEXT NOT NULL,
    occurred_at TIMESTAMPTZ DEFAULT now() NOT NULL
)
