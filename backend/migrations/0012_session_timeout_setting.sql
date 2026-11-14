-- 恩典同行 Grace Ride — System Session Timeout & Auto-Lock Setting
--
-- Administrator-configurable default session inactivity timeout in minutes (default: 15 mins; 0 = disabled)

INSERT INTO system_settings (key, value, updated_at, updated_by)
VALUES ('session_timeout_mins', '15', CURRENT_TIMESTAMP, NULL)
ON CONFLICT (key) DO NOTHING;
