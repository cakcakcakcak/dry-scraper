-- Migration: 14_add_world_cup_game_type.sql
-- Purpose: Add the `world_cup` label to the existing `game_type` enum (or create type if missing)
-- This migration is idempotent: safe to run on databases that already have the label.
-- Last updated: 2026-03-26

DO $$
BEGIN
    -- If the enum type doesn't exist, create it including the new label.
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'game_type') THEN
        CREATE TYPE game_type AS ENUM ('preseason', 'regular_season', 'playoffs', 'world_cup');
    ELSE
        -- If the enum exists, add the new label only if it's not already present.
        IF NOT EXISTS (
            SELECT 1
            FROM pg_enum e
            JOIN pg_type t ON t.oid = e.enumtypid
            WHERE t.typname = 'game_type' AND e.enumlabel = 'world_cup'
        ) THEN
            -- Use EXECUTE to avoid issues with plpgsql parsing and to run DDL safely.
            EXECUTE 'ALTER TYPE game_type ADD VALUE ''world_cup''';
        END IF;
    END IF;
END
$$;
