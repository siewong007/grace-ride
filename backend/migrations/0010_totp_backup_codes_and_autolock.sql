-- 恩典同行 Grace Ride — TOTP backup recovery codes and user session auto-lock
--
-- Adds:
-- - totp_backup_codes: JSON array of hashed single-use recovery codes
-- - auto_lock_mins: Inactivity auto-lock timeout in minutes (0 = Never, 15, 30, 60)

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS totp_backup_codes TEXT,
    ADD COLUMN IF NOT EXISTS auto_lock_mins BIGINT NOT NULL DEFAULT 0;
