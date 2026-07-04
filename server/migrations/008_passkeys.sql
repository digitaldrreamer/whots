-- WebAuthn passkeys — passwordless, email-free account credentials. A user can
-- have several (one per device). `credential` is the serialized webauthn Passkey
-- (public key, counter, etc.); credential_id is its raw id for lookup.
CREATE TABLE passkeys (
    credential_id BYTEA       PRIMARY KEY,
    user_id       UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    credential    JSONB       NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_passkeys_user ON passkeys (user_id);
