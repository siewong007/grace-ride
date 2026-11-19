-- Grab reimbursements must be associated with an event that the claimant is registered for.
ALTER TABLE reimbursements ADD COLUMN IF NOT EXISTS event_id BIGINT REFERENCES events(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_reimbursements_event_id ON reimbursements(event_id) WHERE event_id IS NOT NULL;

-- Backfill event_id from rides where possible
UPDATE reimbursements r
SET event_id = rd.event_id
FROM rides rd
WHERE r.ride_id = rd.id AND r.event_id IS NULL AND rd.event_id IS NOT NULL;
