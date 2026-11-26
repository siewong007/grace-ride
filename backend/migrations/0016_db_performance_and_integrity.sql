-- 0016_db_performance_and_integrity.sql
-- Additive, idempotent PostgreSQL 18 performance indexes, uniqueness guards, and trigram search support.
-- Safe for fresh installs and existing databases replaying migrations (0001_init.sql remains frozen).

-- ---------------------------------------------------------------------------
-- 1. Drop strictly redundant indexes
-- ---------------------------------------------------------------------------
DROP INDEX IF EXISTS idx_notifications_unread;
DROP INDEX IF EXISTS idx_user_badge_icons_user;
DROP INDEX IF EXISTS idx_vehicles_user_id;
DROP INDEX IF EXISTS idx_audit_logs_actor;

-- ---------------------------------------------------------------------------
-- 2. Rides hot-path composite, partial, and range indexes
-- ---------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_rides_driver_pickup
    ON rides (driver_id, pickup_at)
    WHERE driver_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_rides_passenger_pickup
    ON rides (passenger_id, pickup_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_rides_driver_in_progress
    ON rides (driver_id)
    WHERE status = 'in_progress';

CREATE INDEX IF NOT EXISTS idx_rides_event_status
    ON rides (event_id, status)
    WHERE event_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_rides_pending_driver_pickup
    ON rides (pickup_at)
    WHERE status = 'pending_driver';

CREATE INDEX IF NOT EXISTS idx_rides_reminder_pending
    ON rides (pickup_at)
    WHERE status = 'confirmed' AND reminder_sent = 0;

CREATE INDEX IF NOT EXISTS idx_rides_driver_reminder_pending
    ON rides (pickup_at)
    WHERE status = 'confirmed' AND driver_reminder_sent = 0;

CREATE INDEX IF NOT EXISTS idx_rides_stale_pending
    ON rides (pickup_at)
    WHERE status IN ('pending_driver', 'pending_passenger', 'pending_approval', 'pending_driver_confirm');

CREATE INDEX IF NOT EXISTS idx_rides_attention_open
    ON rides (pickup_at)
    WHERE needs_attention = 1 AND status NOT IN ('cancelled', 'completed', 'no_show', 'alt_transport');

CREATE INDEX IF NOT EXISTS idx_rides_status_pickup
    ON rides (status, pickup_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_rides_updated_at
    ON rides (updated_at);

CREATE INDEX IF NOT EXISTS idx_rides_approved_by
    ON rides (approved_by)
    WHERE approved_by IS NOT NULL;

-- ---------------------------------------------------------------------------
-- 3. Events & event_signups indexes
-- ---------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_events_checkin_code_norm
    ON events (LOWER(TRIM(checkin_code)))
    WHERE checkin_code IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_events_reminder_pending
    ON events (starts_at)
    WHERE status = 'published' AND reminder_sent = 0;

CREATE INDEX IF NOT EXISTS idx_events_status_starts
    ON events (status, starts_at);

CREATE INDEX IF NOT EXISTS idx_event_signups_pass_code_lower
    ON event_signups (event_id, LOWER(pass_code))
    WHERE pass_code IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_event_signups_event_status_order
    ON event_signups (event_id, status, created_at, id);

CREATE INDEX IF NOT EXISTS idx_event_signups_user_status
    ON event_signups (user_id, status);

-- ---------------------------------------------------------------------------
-- 4. Reimbursements, payout_batches, settlements & shares indexes
-- ---------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_reimbursements_effective_paid_at
    ON reimbursements ((COALESCE(NULLIF(paid_at, ''), updated_at, created_at)))
    WHERE status = 'paid';

CREATE INDEX IF NOT EXISTS idx_reimbursements_status_created
    ON reimbursements (status, created_at);

CREATE INDEX IF NOT EXISTS idx_reimbursements_status_updated
    ON reimbursements (status, updated_at)
    WHERE status = 'approved';

CREATE INDEX IF NOT EXISTS idx_reimbursements_reviewed_by
    ON reimbursements (reviewed_by)
    WHERE reviewed_by IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_reimbursements_plan_visitor_latest
    ON reimbursements (plan_visitor_id, id DESC)
    WHERE plan_visitor_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_payout_batches_status_id
    ON payout_batches (status, id DESC);

CREATE INDEX IF NOT EXISTS idx_settlements_status_id
    ON settlements (status, id DESC);

CREATE INDEX IF NOT EXISTS idx_settlement_shares_txn_ref
    ON settlement_shares (txn_ref)
    WHERE txn_ref <> '';

CREATE INDEX IF NOT EXISTS idx_settlement_shares_settlement_status
    ON settlement_shares (settlement_id, status);

-- ---------------------------------------------------------------------------
-- 5. Users, help, audit, drafts, passes & housekeeping indexes
-- ---------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_users_status_name_id
    ON users (status, name, id);

CREATE INDEX IF NOT EXISTS idx_users_phone_key
    ON users ((CASE
        WHEN regexp_replace(phone, '[^0-9]', '', 'g') LIKE '0%'
            THEN '60' || substring(regexp_replace(phone, '[^0-9]', '', 'g') from 2)
        ELSE regexp_replace(phone, '[^0-9]', '', 'g')
    END))
    WHERE phone <> '';

CREATE INDEX IF NOT EXISTS idx_help_tickets_assignee_status
    ON help_tickets (assignee_id, status)
    WHERE assignee_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_help_tickets_status_updated
    ON help_tickets (status, updated_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_audit_logs_action_id
    ON audit_logs (action, id DESC);

CREATE INDEX IF NOT EXISTS idx_audit_logs_actor_id
    ON audit_logs (actor_id, id DESC)
    WHERE actor_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_auth_challenges_created
    ON auth_challenges (created_at);

CREATE INDEX IF NOT EXISTS idx_login_failures_created
    ON login_failures (created_at);

CREATE INDEX IF NOT EXISTS idx_tg_link_tokens_expires
    ON telegram_link_tokens (expires_at);

CREATE INDEX IF NOT EXISTS idx_notifications_read_at
    ON notifications (read_at)
    WHERE read_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_files_created
    ON files (created_at);

CREATE INDEX IF NOT EXISTS idx_files_stored_name
    ON files (stored_name);

CREATE INDEX IF NOT EXISTS idx_caregiver_passes_open_until
    ON caregiver_passes (valid_until)
    WHERE closed_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_visitor_passes_open_until
    ON visitor_passes (valid_until)
    WHERE closed_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_visitor_passes_host
    ON visitor_passes (host_id);

CREATE INDEX IF NOT EXISTS idx_visitor_sessions_expires
    ON visitor_sessions (expires_at);

CREATE INDEX IF NOT EXISTS idx_oauth_states_created
    ON oauth_states (created_at);

CREATE INDEX IF NOT EXISTS idx_invites_user
    ON invites (user_id)
    WHERE user_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_invites_open_expires
    ON invites (expires_at)
    WHERE used_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_recovery_requests_status_id
    ON recovery_requests (status, id DESC);

CREATE INDEX IF NOT EXISTS idx_proxy_riders_user_name
    ON proxy_riders (user_id, name);

CREATE INDEX IF NOT EXISTS idx_ride_change_drafts_status_updated
    ON ride_change_drafts (status, updated_at DESC, id DESC)
    WHERE status <> 'superseded';

CREATE INDEX IF NOT EXISTS idx_event_plan_drafts_reviewed_by
    ON event_plan_drafts (reviewed_by)
    WHERE reviewed_by IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_ride_change_drafts_reviewed_by
    ON ride_change_drafts (reviewed_by)
    WHERE reviewed_by IS NOT NULL;

-- ---------------------------------------------------------------------------
-- 6. Case-insensitive area uniqueness guard (skip cleanly if legacy rows collide)
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM areas GROUP BY LOWER(name) HAVING COUNT(*) > 1
    ) THEN
        EXECUTE 'CREATE UNIQUE INDEX IF NOT EXISTS idx_areas_lower_name ON areas (LOWER(name))';
    END IF;
END
$$;

-- ---------------------------------------------------------------------------
-- 7. Concurrency-safe pg_trgm extension + GIN trigram indexes for ILIKE search
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    trgm_schema TEXT;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('grace_ride_pg_trgm_install', 0));
    BEGIN
        CREATE EXTENSION IF NOT EXISTS pg_trgm SCHEMA public;
    EXCEPTION
        WHEN insufficient_privilege OR undefined_file OR duplicate_object OR unique_violation THEN
            NULL;
    END;

    SELECT extnamespace::regnamespace::text
    INTO trgm_schema
    FROM pg_extension
    WHERE extname = 'pg_trgm';

    IF trgm_schema IS NOT NULL THEN
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_users_name_trgm ON users USING gin (name %s.gin_trgm_ops)', trgm_schema);
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_users_email_trgm ON users USING gin (email %s.gin_trgm_ops)', trgm_schema);
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_users_phone_trgm ON users USING gin (phone %s.gin_trgm_ops)', trgm_schema);
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_rides_destination_trgm ON rides USING gin (destination %s.gin_trgm_ops)', trgm_schema);
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_rides_rider_name_trgm ON rides USING gin (rider_name %s.gin_trgm_ops)', trgm_schema);
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_audit_logs_detail_trgm ON audit_logs USING gin (detail %s.gin_trgm_ops)', trgm_schema);
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_audit_logs_entity_name_trgm ON audit_logs USING gin (entity_name %s.gin_trgm_ops)', trgm_schema);
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_reimbursements_description_trgm ON reimbursements USING gin (description %s.gin_trgm_ops)', trgm_schema);
    END IF;
END
$$;
