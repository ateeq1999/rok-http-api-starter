DROP INDEX IF EXISTS idx_refresh_token_used_family;
ALTER TABLE refresh_token_used DROP COLUMN IF EXISTS family_id;
