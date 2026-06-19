-- Fix mode check to match serde snake_case serialization (no-stack → no_stack)
ALTER TABLE games DROP CONSTRAINT IF EXISTS games_mode_check;
ALTER TABLE games ADD CONSTRAINT games_mode_check CHECK (mode IN ('stack', 'no_stack'));

-- Columns the game engine writes
ALTER TABLE games ADD COLUMN IF NOT EXISTS winner_seat     INTEGER;
ALTER TABLE games ADD COLUMN IF NOT EXISTS last_activity_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Support up to 6 players (was capped at 4)
ALTER TABLE game_seats DROP CONSTRAINT IF EXISTS game_seats_seat_index_check;
ALTER TABLE game_seats ADD CONSTRAINT game_seats_seat_index_check CHECK (seat_index BETWEEN 0 AND 5);

-- Index for cleanup task (games stuck in 'playing' past their TTL)
CREATE INDEX IF NOT EXISTS idx_games_status_activity
    ON games (status, last_activity_at)
    WHERE status = 'playing';
