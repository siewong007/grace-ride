-- 恩典同行 Grace Ride — Keyset & pagination performance indexes
--
-- Adds composite B-Tree indexes to accelerate keyset pagination and
-- infinite scrolling queries across rides, notifications, audit logs, and events.

CREATE INDEX IF NOT EXISTS idx_rides_keyset ON rides(pickup_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_notifications_keyset ON notifications(user_id, id DESC);
CREATE INDEX IF NOT EXISTS idx_events_keyset ON events(starts_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_keyset ON audit_logs(id DESC);
CREATE INDEX IF NOT EXISTS idx_reimbursements_user_id ON reimbursements(user_id, id DESC);
CREATE INDEX IF NOT EXISTS idx_reimbursements_status_id ON reimbursements(status, id DESC);
