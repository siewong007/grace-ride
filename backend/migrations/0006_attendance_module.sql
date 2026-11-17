-- Optional attendance module and check-in QR code for events.
-- Any member can participate in an event without requiring a ride,
-- and can scan the event's check-in QR code to record attendance.

ALTER TABLE events ADD COLUMN IF NOT EXISTS attendance_enabled BIGINT NOT NULL DEFAULT 1;
ALTER TABLE events ADD COLUMN IF NOT EXISTS checkin_code TEXT;

-- Backfill existing events with a check-in code
UPDATE events
   SET checkin_code = substr(md5(random()::text || id::text), 1, 10)
 WHERE checkin_code IS NULL;
