-- 0002: allowed values for status and yes/no columns, missing and redundant indexes,
-- and one date format fix. Written to be repeatable (see lib.rs::connect_db).
--
-- Each rule is added NOT VALID (it applies to new and changed rows at once) and then
-- validated against existing rows. If an existing row breaks it, the migration logs a
-- warning and leaves the rule unvalidated instead of failing the deploy; find those
-- rows with: SELECT conrelid::regclass, conname FROM pg_constraint WHERE NOT convalidated;

CREATE OR REPLACE FUNCTION pg_temp.add_check(tbl regclass, name text, expr text) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = tbl AND conname = name) THEN
        EXECUTE format('ALTER TABLE %s ADD CONSTRAINT %I CHECK (%s) NOT VALID', tbl, name, expr);
    END IF;
    BEGIN
        EXECUTE format('ALTER TABLE %s VALIDATE CONSTRAINT %I', tbl, name);
    EXCEPTION WHEN check_violation THEN
        RAISE WARNING 'rows in % break %: new rows are checked, existing ones need review', tbl, name;
    END;
END
$$;

-- ---------------------------------------------------------------- statuses and kinds
SELECT pg_temp.add_check('users', 'chk_users_status', $$status IN ('active', 'pending', 'disabled')$$);
SELECT pg_temp.add_check('recovery_requests', 'chk_recovery_requests_status',
    $$status IN ('pending', 'awaiting_second', 'approved', 'rejected', 'superseded')$$);
SELECT pg_temp.add_check('invites', 'chk_invites_kind', $$kind IN ('invite', 'recovery')$$);
SELECT pg_temp.add_check('auth_challenges', 'chk_auth_challenges_kind', $$kind IN ('reg', 'auth')$$);
SELECT pg_temp.add_check('events', 'chk_events_status', $$status IN ('published', 'cancelled', 'finished')$$);
SELECT pg_temp.add_check('event_signups', 'chk_event_signups_status', $$status IN ('going', 'waitlist', 'cancelled')$$);
SELECT pg_temp.add_check('event_signups', 'chk_event_signups_attended', $$attended IS NULL OR attended IN (0, 1)$$);
SELECT pg_temp.add_check('event_signups', 'chk_event_signups_offer_seats', $$offer_seats >= 0$$);
SELECT pg_temp.add_check('settlements', 'chk_settlements_status', $$status IN ('draft', 'published', 'closed')$$);
SELECT pg_temp.add_check('settlements', 'chk_settlements_split_mode', $$split_mode IN ('equal', 'custom')$$);
SELECT pg_temp.add_check('settlement_shares', 'chk_settlement_shares_status',
    $$status IN ('unpaid', 'pending', 'confirmed', 'needs_info')$$);
SELECT pg_temp.add_check('visitor_passes', 'chk_visitor_passes_window', $$valid_from <= valid_until$$);

-- ---------------------------------------------------------------- yes/no flags (0 or 1)
SELECT pg_temp.add_check('sessions', 'chk_sessions_mfa_ok', $$mfa_ok IN (0, 1)$$);
SELECT pg_temp.add_check('areas', 'chk_areas_active', $$active IN (0, 1)$$);
SELECT pg_temp.add_check('groups', 'chk_groups_is_all', $$is_all IN (0, 1)$$);
SELECT pg_temp.add_check('groups', 'chk_groups_is_visitor', $$is_visitor IN (0, 1)$$);
SELECT pg_temp.add_check('groups', 'chk_groups_not_own_parent', $$parent_id IS NULL OR parent_id <> id$$);
SELECT pg_temp.add_check('events', 'chk_events_reminder_sent', $$reminder_sent IN (0, 1)$$);
SELECT pg_temp.add_check('event_signups', 'chk_event_signups_need_ride', $$need_ride IN (0, 1)$$);
SELECT pg_temp.add_check('event_signups', 'chk_event_signups_offer_drive', $$offer_drive IN (0, 1)$$);
SELECT pg_temp.add_check('rides', 'chk_rides_needs_attention', $$needs_attention IN (0, 1)$$);
SELECT pg_temp.add_check('rides', 'chk_rides_reminder_sent', $$reminder_sent IN (0, 1)$$);
SELECT pg_temp.add_check('rides', 'chk_rides_is_proxy', $$is_proxy IN (0, 1)$$);
SELECT pg_temp.add_check('rides', 'chk_rides_delay_minutes', $$delay_minutes >= 0$$);
SELECT pg_temp.add_check('announcements', 'chk_announcements_pinned', $$pinned IN (0, 1)$$);
SELECT pg_temp.add_check('reimbursements', 'chk_reimbursements_payee_verified', $$payee_verified IN (0, 1)$$);

-- ---------------------------------------------------------------- dates
-- The reference settings rows were stamped with CURRENT_TIMESTAMP ("2026-11-17
-- 02:47:48.123+08"), unlike every other date. Rewrite them in the app's format.
UPDATE system_settings
SET updated_at = to_char(updated_at::timestamptz AT TIME ZONE 'Asia/Kuala_Lumpur', 'YYYY-MM-DD"T"HH24:MI:SS')
WHERE updated_at !~ '^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$'
  AND updated_at ~ '^\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}';

-- ---------------------------------------------------------------- indexes
-- Missing: lookups the app makes on every event page, help thread, audit search,
-- nightly clean-up and upload.
CREATE INDEX IF NOT EXISTS idx_rides_event ON rides(event_id) WHERE event_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_help_replies_ticket ON help_replies(ticket_id, id);
CREATE INDEX IF NOT EXISTS idx_help_tickets_user ON help_tickets(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created ON audit_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_logs_actor ON audit_logs(actor_id) WHERE actor_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_files_owner ON files(owner_id, created_at);
CREATE INDEX IF NOT EXISTS idx_ride_messages_created ON ride_messages(created_at);

-- Redundant: the same columns as a primary key or unique constraint.
DROP INDEX IF EXISTS idx_event_signups_event_user;  -- UNIQUE (event_id, user_id)
DROP INDEX IF EXISTS idx_demo_batches_lookup;       -- primary key
DROP INDEX IF EXISTS idx_reimbursement_rates_type;  -- primary key
DROP INDEX IF EXISTS idx_group_members_group;       -- primary key (group_id, user_id)
-- Not useful: a handful of distinct values, or a text match that also caught "superadmin".
DROP INDEX IF EXISTS idx_events_kind;
DROP INDEX IF EXISTS idx_events_capacity;
DROP INDEX IF EXISTS idx_rides_readiness;
DROP INDEX IF EXISTS idx_users_has_admin_role;
