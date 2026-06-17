ALTER TABLE refresh_token_used ADD COLUMN IF NOT EXISTS family_id VARCHAR(64);

CREATE INDEX IF NOT EXISTS idx_refresh_token_used_family
    ON refresh_token_used (family_id)
    WHERE family_id IS NOT NULL;
