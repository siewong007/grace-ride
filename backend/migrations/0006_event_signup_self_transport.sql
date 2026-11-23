-- Allow members signed up for an event to mark that they are going on their own,
-- distinguishing an explicit choice from "transport not arranged yet".
ALTER TABLE event_signups ADD COLUMN IF NOT EXISTS self_transport BIGINT NOT NULL DEFAULT 0;
ALTER TABLE event_signups DROP CONSTRAINT IF EXISTS chk_event_signups_self_transport;
ALTER TABLE event_signups ADD CONSTRAINT chk_event_signups_self_transport
    CHECK (self_transport IN (0, 1));
