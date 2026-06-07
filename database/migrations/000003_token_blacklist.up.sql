CREATE TABLE IF NOT EXISTS token_blacklist (
    id         TEXT          PRIMARY KEY,
    token_hash VARCHAR(64)  NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ  NOT NULL,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_token_blacklist_hash
    ON token_blacklist (token_hash);

CREATE INDEX IF NOT EXISTS idx_token_blacklist_expires
    ON token_blacklist (expires_at);
