-- One-use friend-invite links. A creator mints a token, shares the link
-- privately; the first signed-in person to redeem it becomes their friend
-- instantly. `used_by` NULL = unredeemed.
CREATE TABLE friend_invites (
    token      VARCHAR(64) PRIMARY KEY,
    creator_id UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    used_by    UUID        REFERENCES users (id) ON DELETE SET NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '7 days'
);

CREATE INDEX idx_friend_invites_creator ON friend_invites (creator_id);
