CREATE TABLE IF NOT EXISTS two_factor_backup_codes (
    id         TEXT          PRIMARY KEY,
    user_id    TEXT          NOT NULL,
    code_hash  VARCHAR(64)  NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_2fa_backup_user_id
    ON two_factor_backup_codes (user_id);
