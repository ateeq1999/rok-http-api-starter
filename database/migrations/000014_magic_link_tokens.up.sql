CREATE TABLE IF NOT EXISTS magic_link_tokens (
    id         TEXT          PRIMARY KEY,
    email      VARCHAR(255) NOT NULL,
    token_hash VARCHAR(64)  NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ  NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_magic_link_email
    ON magic_link_tokens (email);

CREATE INDEX IF NOT EXISTS idx_magic_link_token_hash
    ON magic_link_tokens (token_hash);
