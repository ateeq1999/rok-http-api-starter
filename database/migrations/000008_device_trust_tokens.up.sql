CREATE TABLE IF NOT EXISTS device_trust_tokens (
    id           TEXT PRIMARY KEY,
    user_id      TEXT   NOT NULL,
    token_hash   TEXT   NOT NULL UNIQUE,
    device_name  TEXT,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at   TIMESTAMPTZ NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_device_trust_tokens_user_id ON device_trust_tokens(user_id);
