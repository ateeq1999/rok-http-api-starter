CREATE TABLE IF NOT EXISTS refresh_token_used (
    id          TEXT         PRIMARY KEY,
    token_hash  VARCHAR(64)  NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS refresh_token_revoked_families (
    id          TEXT         PRIMARY KEY,
    family_id   VARCHAR(64)  NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_refresh_token_used_hash
    ON refresh_token_used (token_hash);

CREATE INDEX IF NOT EXISTS idx_refresh_token_revoked_family
    ON refresh_token_revoked_families (family_id);
