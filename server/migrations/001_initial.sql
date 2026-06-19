-- ── Users ─────────────────────────────────────────────────────────────────────

CREATE TABLE users (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    username      VARCHAR(30) UNIQUE NOT NULL,
    display_name  VARCHAR(50) NOT NULL,
    email         VARCHAR(255) UNIQUE,
    phone_hash    VARCHAR(64),        -- SHA-256(E.164 number), for OTP & contact discovery
    password_hash TEXT,               -- NULL for guests
    avatar_url    TEXT,
    is_guest      BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_username   ON users (username);
CREATE INDEX idx_users_phone_hash ON users (phone_hash) WHERE phone_hash IS NOT NULL;

-- ── Sessions ───────────────────────────────────────────────────────────────────

CREATE TABLE refresh_tokens (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash  VARCHAR(64) NOT NULL UNIQUE,  -- SHA-256 of the opaque token sent to client
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_user ON refresh_tokens (user_id);

-- ── Social graph ───────────────────────────────────────────────────────────────

-- Contact hashes uploaded by the native app for mutual discovery.
-- Deferred in PWA — endpoint exists, client just never calls it yet.
CREATE TABLE contact_hashes (
    user_id      UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    contact_hash VARCHAR(64) NOT NULL,
    PRIMARY KEY (user_id, contact_hash)
);

CREATE TABLE friends (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    requester_id UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    addressee_id UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status       VARCHAR(20) NOT NULL DEFAULT 'pending'
                     CHECK (status IN ('pending', 'accepted', 'blocked')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (requester_id, addressee_id),
    CHECK  (requester_id <> addressee_id)
);

CREATE INDEX idx_friends_addressee ON friends (addressee_id);
CREATE INDEX idx_friends_requester ON friends (requester_id);

-- ── Games ──────────────────────────────────────────────────────────────────────

CREATE TABLE games (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    mode        VARCHAR(20) NOT NULL CHECK (mode IN ('stack', 'no-stack')),
    status      VARCHAR(20) NOT NULL DEFAULT 'waiting'
                    CHECK (status IN ('waiting', 'playing', 'finished')),
    created_by  UUID        REFERENCES users (id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ
);

CREATE TABLE game_seats (
    game_id       UUID        NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    seat_index    INTEGER     NOT NULL CHECK (seat_index BETWEEN 0 AND 3),
    user_id       UUID        REFERENCES users (id) ON DELETE SET NULL,
    is_ai         BOOLEAN     NOT NULL DEFAULT FALSE,
    ai_difficulty VARCHAR(20),
    is_winner     BOOLEAN     NOT NULL DEFAULT FALSE,
    PRIMARY KEY (game_id, seat_index)
);

-- ── Helpers ────────────────────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN NEW.updated_at = NOW(); RETURN NEW; END;
$$;

CREATE TRIGGER trg_users_updated_at   BEFORE UPDATE ON users   FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_friends_updated_at BEFORE UPDATE ON friends FOR EACH ROW EXECUTE FUNCTION set_updated_at();
