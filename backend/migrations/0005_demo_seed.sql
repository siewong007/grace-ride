-- Demo-seed administration (2026-11).
--
-- Every row inserted by `seed demo` / `seed edge` is stamped with demo_batch so
-- demo data can be reset (deleted) without touching real member records.
--
-- `seed reset` deletes every row whose demo_batch is one of the batches owned
-- by this tool (demo-2026 / edge-2026), in dependency order, then re-seeds.

CREATE TABLE IF NOT EXISTS demo_batches (
    batch     TEXT NOT NULL PRIMARY KEY,
    scale     TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (to_char(NOW() AT TIME ZONE 'Asia/Kuala_Lumpur', 'YYYY-MM-DD\"T\"HH24:MI:SS')),
    note      TEXT NOT NULL DEFAULT ''
);

ALTER TABLE users ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE areas ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE group_members ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE driver_areas ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE vehicles ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE invites ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE passkeys ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE events ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE event_groups ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE event_signups ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE rides ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE ride_messages ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE driver_locations ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE notifications ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE announcements ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE help_tickets ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE help_replies ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE files ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE reimbursements ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE settlements ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE settlement_items ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE settlement_shares ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE settlement_revisions ADD COLUMN IF NOT EXISTS demo_batch TEXT;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS demo_batch TEXT;

CREATE INDEX IF NOT EXISTS idx_demo_batch_users ON users(demo_batch) WHERE demo_batch IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_demo_batch_rides ON rides(demo_batch) WHERE demo_batch IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_demo_batch_events ON events(demo_batch) WHERE demo_batch IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_demo_batch_notifications ON notifications(demo_batch) WHERE demo_batch IS NOT NULL;