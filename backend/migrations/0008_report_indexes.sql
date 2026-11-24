-- Indexes to accelerate date-range and organiser-scoped report queries.
CREATE INDEX IF NOT EXISTS idx_reimbursements_created_at ON reimbursements(created_at);
CREATE INDEX IF NOT EXISTS idx_reimbursements_paid_at ON reimbursements(paid_at) WHERE paid_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_events_creator_starts ON events(created_by, starts_at);
CREATE INDEX IF NOT EXISTS idx_settlements_creator ON settlements(created_by);
