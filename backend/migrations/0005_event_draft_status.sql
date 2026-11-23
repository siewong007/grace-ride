-- Allow event organisers to save hidden drafts and publish/unpublish upcoming events.
ALTER TABLE events DROP CONSTRAINT IF EXISTS chk_events_status;
ALTER TABLE events ADD CONSTRAINT chk_events_status
    CHECK (status IN ('draft', 'published', 'cancelled', 'finished'));
