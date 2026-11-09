-- =====================================================================
-- 0006 · Terms acceptance tracking (idempotent; column may already exist in 0001)
-- =====================================================================
ALTER TABLE users ADD COLUMN IF NOT EXISTS terms_accepted_at TEXT;
CREATE INDEX IF NOT EXISTS idx_users_terms_accepted ON users(terms_accepted_at)
  WHERE terms_accepted_at IS NOT NULL;
