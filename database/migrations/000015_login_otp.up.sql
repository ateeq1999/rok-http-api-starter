CREATE TABLE IF NOT EXISTS login_otp (
    id         TEXT          PRIMARY KEY,
    email      VARCHAR(255) NOT NULL,
    code_hash  VARCHAR(64)  NOT NULL,
    expires_at TIMESTAMPTZ  NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_login_otp_email
    ON login_otp (email);

CREATE INDEX IF NOT EXISTS idx_login_otp_code_hash
    ON login_otp (code_hash);
