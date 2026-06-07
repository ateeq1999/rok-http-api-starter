CREATE TABLE IF NOT EXISTS password_resets (
    id         TEXT          PRIMARY KEY,
    email      VARCHAR(255) NOT NULL,
    token_hash VARCHAR(64)  NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ  NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_password_resets_email
    ON password_resets (email);

CREATE INDEX IF NOT EXISTS idx_password_resets_token
    ON password_resets (token_hash);
