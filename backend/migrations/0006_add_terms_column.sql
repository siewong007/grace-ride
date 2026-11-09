-- =====================================================================
-- 0006 · Terms acceptance tracking
-- =====================================================================
--
-- This migration adds a column to track when users accept the terms of service.
-- NULL means the person has not agreed yet.
--
-- =====================================================================

-- Add terms_accepted_at column to users table
-- Stores the timestamp when the user accepted terms (ISO format Asia/Kuala_Lumpur)
ALTER TABLE users ADD COLUMN terms_accepted_at TEXT;

-- Add index for querying users who have/hasn't accepted terms
CREATE INDEX idx_users_terms_accepted ON users(terms_accepted_at) WHERE terms_accepted_at IS NOT NULL;

-- Add comment to explain the column purpose
-- NULL = not agreed, NOT NULL = agreed at the stored timestamp
-- Application should set this value on first login/onboarding after terms acceptance

-- =====================================================================
-- End of 0006
-- =====================================================================