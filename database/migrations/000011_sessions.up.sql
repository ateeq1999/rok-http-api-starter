CREATE TABLE IF NOT EXISTS sessions (
    id            TEXT          PRIMARY KEY,
    user_id       TEXT          NOT NULL,
    device_info   TEXT,
    ip_address    TEXT,
    access_token_hash VARCHAR(64) NOT NULL,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    last_seen_at  TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    revoked_at    TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id
    ON sessions (user_id);

CREATE INDEX IF NOT EXISTS idx_sessions_access_token_hash
    ON sessions (access_token_hash);

CREATE INDEX IF NOT EXISTS idx_sessions_revoked
    ON sessions (revoked_at)
    WHERE revoked_at IS NULL;
