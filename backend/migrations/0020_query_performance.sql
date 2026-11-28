-- Query performance tuning verified at 2,000-member scale.
-- Drops confirmed duplicate indexes while preserving every index asserted by tests,
-- and adds targeted composite/partial/covering indexes for hot list, dashboard,
-- planning, and analytics report endpoints.

-- 1. Drop duplicate indexes (keeping idx_oauth_states_created and idx_reimbursements_plan_visitor_latest).
DROP INDEX IF EXISTS idx_oauth_states_created_at;
DROP INDEX IF EXISTS idx_reimbursements_plan_visitor;

-- 2. Rides: pickup window scans, daily changes count, event stats, and latest visitor pass lookup.
CREATE INDEX IF NOT EXISTS idx_rides_pickup_status
    ON rides (pickup_at, status);

CREATE INDEX IF NOT EXISTS idx_rides_updated_changes
    ON rides (updated_at)
    WHERE needs_attention = 1 OR status IN ('cancelled', 'no_show', 'alt_transport');

CREATE INDEX IF NOT EXISTS idx_rides_event_active
    ON rides (event_id, status, driver_id)
    WHERE event_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_visitor_passes_ride_latest
    ON visitor_passes (ride_id, id DESC) INCLUDE (host_group_id)
    WHERE ride_id IS NOT NULL;

-- 3. Users: unfiltered alphabetical pagination without full-table sort.
CREATE INDEX IF NOT EXISTS idx_users_name_id
    ON users (name, id);

-- 4. Events, signups & plan visitors: report aggregations without wide table/index scans.
CREATE INDEX IF NOT EXISTS idx_event_signups_event_stats
    ON event_signups (event_id, status, attended, as_group_id);

CREATE INDEX IF NOT EXISTS idx_events_starts_active
    ON events (starts_at, id)
    WHERE status NOT IN ('draft', 'cancelled');

CREATE INDEX IF NOT EXISTS idx_plan_visitors_event_seats
    ON plan_visitors (event_id) INCLUDE (seats);

-- 5. Reimbursements: compact queue index for dashboard pending/approved counts & default sorts.
CREATE INDEX IF NOT EXISTS idx_reimbursements_open_queue
    ON reimbursements (status, id DESC)
    WHERE status IN ('pending', 'approved');
