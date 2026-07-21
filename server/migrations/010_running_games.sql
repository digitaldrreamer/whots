-- Whose turn it is, denormalised out of the Redis snapshot on each move so the
-- Games tab can show "your turn" across many games without loading every live
-- game's state. Nullable: finished/abandoned games have no current seat, and
-- rows written before this migration have none either.
ALTER TABLE games ADD COLUMN IF NOT EXISTS current_seat_index INTEGER;

-- Both new lookups start from "which games is this user in": the Games tab, and
-- the check for an existing running game against a given opponent.
CREATE INDEX IF NOT EXISTS idx_game_seats_user
    ON game_seats (user_id)
    WHERE user_id IS NOT NULL;
