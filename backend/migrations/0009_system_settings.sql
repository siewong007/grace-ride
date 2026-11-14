-- 恩典同行 Grace Ride — System & Retention Settings
--
-- Operational configuration configurable by administrators:
-- - audit_retention_days (days to retain audit logs before automated purging)
-- - chat_retention_days (days to retain trip coordination chat messages)
-- - max_live_drivers (cap on concurrent live GPS location sharing drivers)

CREATE TABLE IF NOT EXISTS system_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    updated_by BIGINT REFERENCES users(id) ON DELETE SET NULL
);

INSERT INTO system_settings (key, value, updated_at, updated_by)
VALUES
    ('audit_retention_days', '730', CURRENT_TIMESTAMP, NULL),
    ('chat_retention_days', '180', CURRENT_TIMESTAMP, NULL),
    ('max_live_drivers', '10', CURRENT_TIMESTAMP, NULL)
ON CONFLICT (key) DO NOTHING;
