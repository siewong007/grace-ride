-- When a member hid the "Getting started" checklist on Home. NULL means it is still shown
-- (until every step is done).
--
-- Members already using the app never see it, so existing rows get a value. That value
-- comes in as the column default rather than an UPDATE of every row: the default is
-- stored once and not written to (or re-checked against) each existing row. The default
-- is then dropped so new members start with NULL. Members still waiting for approval are
-- new members too, so they are reset; their emails were lowercased on sign-up.
ALTER TABLE users ADD COLUMN IF NOT EXISTS onboarding_dismissed_at TEXT DEFAULT '2026-11-19T00:00:00';
ALTER TABLE users ALTER COLUMN onboarding_dismissed_at DROP DEFAULT;
UPDATE users SET onboarding_dismissed_at = NULL WHERE status = 'pending';
