-- 恩典同行 Grace Ride — ride readiness check, driver delay reporting, and gate arrival
--
-- Adds readiness check columns to rides (unconfirmed, ready, delayed_5m, delayed_10m, unable_to_join),
-- driver-reported delay minutes, and gate arrival timestamp.
-- Adds configurable readiness prompt timing and gate waiting tolerance to users.

ALTER TABLE rides
    ADD COLUMN IF NOT EXISTS readiness_status TEXT NOT NULL DEFAULT 'unconfirmed'
        CHECK (readiness_status IN ('unconfirmed', 'ready', 'delayed_5m', 'delayed_10m', 'unable_to_join')),
    ADD COLUMN IF NOT EXISTS readiness_updated_at TEXT,
    ADD COLUMN IF NOT EXISTS delay_minutes BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS arrived_at_gate_at TEXT;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS readiness_prompt_mins BIGINT NOT NULL DEFAULT 45,
    ADD COLUMN IF NOT EXISTS gate_wait_mins BIGINT NOT NULL DEFAULT 5;

CREATE INDEX IF NOT EXISTS idx_rides_readiness ON rides(readiness_status);
