CREATE TABLE IF NOT EXISTS personal_access_tokens (
    id            BIGSERIAL     PRIMARY KEY,
    tokenable_id  BIGINT        NOT NULL,
    name          VARCHAR(255)  NOT NULL DEFAULT 'default',
    token         VARCHAR(64)   NOT NULL UNIQUE,
    abilities     TEXT[]        NOT NULL DEFAULT '{}',
    last_used_at  TIMESTAMPTZ,
    expires_at    TIMESTAMPTZ,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pat_tokenable_id ON personal_access_tokens (tokenable_id);
CREATE INDEX IF NOT EXISTS idx_pat_expires_at ON personal_access_tokens (expires_at) WHERE expires_at IS NOT NULL;
