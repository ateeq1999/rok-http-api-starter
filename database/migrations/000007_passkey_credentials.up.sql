CREATE TABLE IF NOT EXISTS passkey_credentials (
    id            TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL,
    credential_id TEXT NOT NULL UNIQUE,
    passkey_json  TEXT NOT NULL,
    sign_count    BIGINT NOT NULL DEFAULT 0,
    name          TEXT NOT NULL DEFAULT 'Key',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_passkey_credentials_user_id ON passkey_credentials(user_id);
