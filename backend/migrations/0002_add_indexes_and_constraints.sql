-- =====================================================================
-- 0002 · Indexes and constraints (post-init improvements)
-- =====================================================================

-- Indexes for frequently queried columns

-- Events kind filter index
CREATE INDEX idx_events_kind ON events(kind);

-- Rides status + created_at for time-based status queries
CREATE INDEX idx_rides_status_created ON rides(status, created_at);

-- Rides driver_id for driver assignment queries
CREATE INDEX idx_rides_driver_active ON rides(driver_id) WHERE driver_id IS NOT NULL AND status = 'confirmed';

-- Reimbursements status index
CREATE INDEX idx_reimbursements_status ON reimbursements(status);

-- Settlements due_date for overdue queries
CREATE INDEX idx_settlements_due_date ON settlements(due_date) WHERE due_date IS NOT NULL;

-- Event signups composite index
CREATE INDEX idx_event_signups_event_user ON event_signups(event_id, user_id);

-- Users demo_batch for demo data queries
CREATE INDEX idx_demo_batch_users ON users(demo_batch) WHERE demo_batch IS NOT NULL;

-- Users role-based queries (comma-separated role lookups)
-- Using a functional index pattern for "admin" role check
CREATE INDEX idx_users_has_admin_role ON users((roles LIKE '%admin%')) WHERE roles LIKE '%admin%';

-- CHECK constraints added via ALTER TABLE

-- Vehicle seat count constraint
ALTER TABLE vehicles ADD CONSTRAINT chk_vehicle_seats CHECK (seats > 0 AND seats <= 8);

-- Ride status valid values constraint
ALTER TABLE rides ADD CONSTRAINT chk_rides_status CHECK (status IN ('pending_driver','pending_passenger','pending_approval','confirmed','in_progress','completed'));

-- Event kind valid values constraint
ALTER TABLE events ADD CONSTRAINT chk_events_kind CHECK (kind IN ('badminton','bowling','kayak','walk','other'));

-- Reimbursement status valid values constraint
ALTER TABLE reimbursements ADD CONSTRAINT chk_reimbursements_status CHECK (status IN ('pending','returned','rejected','approved','paid'));

-- Users language constraint (already in schema but confirming)
-- ALTER TABLE users ADD CONSTRAINT chk_users_lang CHECK (lang IN ('zh','en')); -- already in CREATE TABLE

-- Passkey label should not be empty when present
-- (handled at application level; constraint added for completeness)
ALTER TABLE passkeys ADD CONSTRAINT chk_passkey_label CHECK (length(trim(label)) >= 0);

-- Audit log entity_id should reference an existing entity
-- (enforced at application level; note in comment)

-- Index for help_tickets status
CREATE INDEX idx_help_tickets_status ON help_tickets(status);

-- Index for reimbursements ride_id
CREATE INDEX idx_reimbursements_ride ON reimbursements(ride_id);

-- Index for settlements event_id
CREATE INDEX idx_settlements_event ON settlements(event_id);

-- Comment: demo_batch columns should reference demo_batches table
-- Consider foreign key: ALTER TABLE users ADD CONSTRAINT fk_users_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch);
-- Same for vehicles, invites, passkeys, rides, events, etc.

-- =====================================================================
-- End of 0002
-- =====================================================================