-- Add 'abandoned' as a valid game status (for cleanup task)
ALTER TABLE games DROP CONSTRAINT IF EXISTS games_status_check;
ALTER TABLE games ADD CONSTRAINT games_status_check
    CHECK (status IN ('waiting', 'playing', 'finished', 'abandoned'));
