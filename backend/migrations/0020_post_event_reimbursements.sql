-- Post-event driver dispersal & supper reimbursement:
-- 1. Ensure event_id exists on reimbursements
ALTER TABLE reimbursements ADD COLUMN IF NOT EXISTS event_id BIGINT REFERENCES events(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_reimbursements_event_id ON reimbursements(event_id) WHERE event_id IS NOT NULL;

-- 2. Allow receipt_file_id to be NULL for leader-authorized mileage claims
ALTER TABLE reimbursements ALTER COLUMN receipt_file_id DROP NOT NULL;

-- 3. Add destination_area_id and trip_type to reimbursements
ALTER TABLE reimbursements ADD COLUMN IF NOT EXISTS destination_area_id BIGINT REFERENCES areas(id) ON DELETE SET NULL;
ALTER TABLE reimbursements ADD COLUMN IF NOT EXISTS trip_type TEXT NOT NULL DEFAULT 'standard';

CREATE INDEX IF NOT EXISTS idx_reimbursements_dest_area ON reimbursements(destination_area_id) WHERE destination_area_id IS NOT NULL;

-- 4. Add supper_choice to rides for passenger return trip preference
ALTER TABLE rides ADD COLUMN IF NOT EXISTS supper_choice TEXT NOT NULL DEFAULT 'unspecified'
    CHECK (supper_choice IN ('unspecified', 'supper', 'direct_home'));
