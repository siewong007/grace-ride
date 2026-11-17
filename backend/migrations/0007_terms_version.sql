-- Which version of the terms and privacy notice each member opted in to.
-- Version 2 (17 Nov 2026) adds the PDPA notice: sensitive data, overseas map services,
-- retention periods, rights, drivers and copyright. Members who agreed to version 1 are
-- asked again at their next sign-in.
--
-- The column default is the current version so rows created without going through the
-- opt-in (seed data) count as current once terms_accepted_at is set. Acceptance itself
-- always writes the version explicitly. When TERMS_VERSION in routes/auth.rs is raised,
-- raise this default in the same migration.
ALTER TABLE users ADD COLUMN IF NOT EXISTS terms_version INTEGER NOT NULL DEFAULT 2;
UPDATE users SET terms_version = 1 WHERE terms_accepted_at IS NOT NULL;
