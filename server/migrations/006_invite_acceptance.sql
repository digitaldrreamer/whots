-- Track whether a human seat holder has accepted their game invite.
-- NULL = pending, NOT NULL = accepted at that timestamp.
-- AI seats and the game creator are considered auto-accepted.
ALTER TABLE game_seats ADD COLUMN IF NOT EXISTS accepted_at TIMESTAMPTZ;
