-- =====================================================================
-- 0002 · Indexes and constraints (post-init improvements)
-- Idempotent: safe when 0001 already created overlapping indexes.
-- =====================================================================

CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
CREATE INDEX IF NOT EXISTS idx_rides_status_created ON rides(status, created_at);
CREATE INDEX IF NOT EXISTS idx_rides_driver_active ON rides(driver_id)
  WHERE driver_id IS NOT NULL AND status = 'confirmed';
CREATE INDEX IF NOT EXISTS idx_reimbursements_status ON reimbursements(status);
CREATE INDEX IF NOT EXISTS idx_settlements_due_date ON settlements(due_date)
  WHERE due_date IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_event_signups_event_user ON event_signups(event_id, user_id);
-- Already created in 0001; keep IF NOT EXISTS so fresh installs and upgrades both work.
CREATE INDEX IF NOT EXISTS idx_demo_batch_users ON users(demo_batch) WHERE demo_batch IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_has_admin_role ON users((roles LIKE '%admin%'))
  WHERE roles LIKE '%admin%';
CREATE INDEX IF NOT EXISTS idx_help_tickets_status ON help_tickets(status);
CREATE INDEX IF NOT EXISTS idx_reimbursements_ride ON reimbursements(ride_id);
CREATE INDEX IF NOT EXISTS idx_settlements_event ON settlements(event_id);

-- Constraints (drop-if-exists then add so re-runs / overlapping 0001 defs are safe)
ALTER TABLE vehicles DROP CONSTRAINT IF EXISTS chk_vehicle_seats;
ALTER TABLE vehicles ADD CONSTRAINT chk_vehicle_seats CHECK (seats > 0 AND seats <= 8);

-- Must include every status the app writes (cancelled / no_show / alt_transport / pending_driver_confirm).
ALTER TABLE rides DROP CONSTRAINT IF EXISTS chk_rides_status;
ALTER TABLE rides ADD CONSTRAINT chk_rides_status CHECK (status IN (
  'pending_driver','pending_passenger','pending_approval','pending_driver_confirm',
  'confirmed','in_progress','completed','cancelled','no_show','alt_transport'
));

ALTER TABLE events DROP CONSTRAINT IF EXISTS chk_events_kind;
ALTER TABLE events ADD CONSTRAINT chk_events_kind CHECK (kind IN ('badminton','bowling','kayak','walk','other'));

ALTER TABLE reimbursements DROP CONSTRAINT IF EXISTS chk_reimbursements_status;
ALTER TABLE reimbursements ADD CONSTRAINT chk_reimbursements_status CHECK (
  status IN ('pending','returned','rejected','approved','paid')
);

ALTER TABLE passkeys DROP CONSTRAINT IF EXISTS chk_passkey_label;
ALTER TABLE passkeys ADD CONSTRAINT chk_passkey_label CHECK (length(trim(label)) >= 0);
