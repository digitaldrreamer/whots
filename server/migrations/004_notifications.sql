CREATE TABLE notifications (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind       VARCHAR(30) NOT NULL,
    payload    JSONB       NOT NULL DEFAULT '{}',
    read       BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_notifications_user_unread
    ON notifications (user_id, created_at DESC) WHERE NOT read;
