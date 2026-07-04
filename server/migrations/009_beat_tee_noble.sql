-- The "you beat Tee-Noble" badge — a permanent bragging right, awarded when a
-- human wins a game that had a Tee-Noble opponent.
ALTER TABLE users ADD COLUMN IF NOT EXISTS beat_tee_noble BOOLEAN NOT NULL DEFAULT FALSE;

-- Backfill: anyone who ALREADY beat a Tee-Noble opponent earns it retroactively.
UPDATE users u SET beat_tee_noble = TRUE
WHERE EXISTS (
    SELECT 1
    FROM game_seats win
    JOIN game_seats tee ON tee.game_id = win.game_id
    WHERE win.user_id = u.id
      AND win.is_winner
      AND win.is_ai = FALSE
      AND tee.is_ai
      AND tee.ai_difficulty = 'tee_noble'
);
