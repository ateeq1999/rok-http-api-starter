CREATE TABLE IF NOT EXISTS users (
    id                BIGSERIAL     PRIMARY KEY,
    email             TEXT          NOT NULL UNIQUE,
    password_hash     TEXT          NOT NULL,
    name              TEXT          NOT NULL,
    roles             TEXT          NOT NULL DEFAULT 'user',
    email_verified_at TIMESTAMPTZ,
    created_at        TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ   NOT NULL DEFAULT NOW()
);
