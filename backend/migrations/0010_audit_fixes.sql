-- Additive schema, integrity, and foreign-key fixes from full-application audit.

-- 1. Drop redundant audit_logs created_at index (duplicates idx_audit_logs_created).
DROP INDEX IF EXISTS idx_audit_logs_created_at;

-- 2. Allow deleting groups / users referenced by historical passes and drafts via ON DELETE SET NULL.
DO $$
DECLARE
    c_name text;
BEGIN
    -- visitor_passes.host_group_id -> groups(id)
    ALTER TABLE visitor_passes ALTER COLUMN host_group_id DROP NOT NULL;
    FOR c_name IN
        SELECT conname FROM pg_constraint
        WHERE conrelid = 'visitor_passes'::regclass
          AND contype = 'f'
          AND confrelid = 'groups'::regclass
    LOOP
        EXECUTE format('ALTER TABLE visitor_passes DROP CONSTRAINT %I', c_name);
    END LOOP;
    ALTER TABLE visitor_passes
        ADD CONSTRAINT fk_visitor_passes_host_group
        FOREIGN KEY (host_group_id) REFERENCES groups(id) ON DELETE SET NULL;

    -- event_plan_drafts.updated_by -> users(id)
    ALTER TABLE event_plan_drafts ALTER COLUMN updated_by DROP NOT NULL;
    FOR c_name IN
        SELECT conname FROM pg_constraint
        WHERE conrelid = 'event_plan_drafts'::regclass
          AND contype = 'f'
          AND confrelid = 'users'::regclass
          AND array_length(conkey, 1) = 1
          AND conkey[1] = (SELECT attnum FROM pg_attribute WHERE attrelid = 'event_plan_drafts'::regclass AND attname = 'updated_by')
    LOOP
        EXECUTE format('ALTER TABLE event_plan_drafts DROP CONSTRAINT %I', c_name);
    END LOOP;
    ALTER TABLE event_plan_drafts
        ADD CONSTRAINT fk_event_plan_drafts_updated_by
        FOREIGN KEY (updated_by) REFERENCES users(id) ON DELETE SET NULL;

    -- ride_change_drafts.created_by / updated_by -> users(id)
    ALTER TABLE ride_change_drafts ALTER COLUMN created_by DROP NOT NULL;
    ALTER TABLE ride_change_drafts ALTER COLUMN updated_by DROP NOT NULL;
    FOR c_name IN
        SELECT conname FROM pg_constraint
        WHERE conrelid = 'ride_change_drafts'::regclass
          AND contype = 'f'
          AND confrelid = 'users'::regclass
          AND array_length(conkey, 1) = 1
          AND conkey[1] IN (
              SELECT attnum FROM pg_attribute
              WHERE attrelid = 'ride_change_drafts'::regclass
                AND attname IN ('created_by', 'updated_by')
          )
    LOOP
        EXECUTE format('ALTER TABLE ride_change_drafts DROP CONSTRAINT %I', c_name);
    END LOOP;
    ALTER TABLE ride_change_drafts
        ADD CONSTRAINT fk_ride_change_drafts_created_by
        FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL;
    ALTER TABLE ride_change_drafts
        ADD CONSTRAINT fk_ride_change_drafts_updated_by
        FOREIGN KEY (updated_by) REFERENCES users(id) ON DELETE SET NULL;
END $$;

-- 3. Prevent duplicate non-rejected reimbursements per ride even under concurrency.
CREATE UNIQUE INDEX IF NOT EXISTS idx_reimb_live_ride
    ON reimbursements (ride_id)
    WHERE ride_id IS NOT NULL AND status <> 'rejected';

-- 4. Guard boolean-like integer and enum columns with CHECK constraints.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_payout_batches_totals') THEN
        ALTER TABLE payout_batches
            ADD CONSTRAINT chk_payout_batches_totals
            CHECK (total_cents >= 0 AND claim_count > 0);
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_events_mileage_claimable') THEN
        ALTER TABLE events
            ADD CONSTRAINT chk_events_mileage_claimable
            CHECK (mileage_claimable IN (0, 1));
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_events_attendance_enabled') THEN
        ALTER TABLE events
            ADD CONSTRAINT chk_events_attendance_enabled
            CHECK (attendance_enabled IN (0, 1));
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_rides_mileage_receipt_needed') THEN
        ALTER TABLE rides
            ADD CONSTRAINT chk_rides_mileage_receipt_needed
            CHECK (mileage_receipt_needed IN (0, 1));
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_reimbursements_trip_type') THEN
        ALTER TABLE reimbursements
            ADD CONSTRAINT chk_reimbursements_trip_type
            CHECK (trip_type IS NULL OR trip_type IN ('standard', 'supper', 'direct_return'));
    END IF;
END $$;

-- 5. Re-arm both passenger and driver reminders when driver_id or pickup_at changes.
CREATE OR REPLACE FUNCTION rides_rearm_reminder() RETURNS trigger
LANGUAGE plpgsql
SET search_path FROM CURRENT
AS $$
BEGIN
    IF NEW.driver_id IS DISTINCT FROM OLD.driver_id OR NEW.pickup_at IS DISTINCT FROM OLD.pickup_at THEN
        NEW.reminder_sent := 0;
        NEW.driver_reminder_sent := 0;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS trg_rides_rearm_reminder ON rides;
CREATE TRIGGER trg_rides_rearm_reminder
    BEFORE UPDATE OF driver_id, pickup_at ON rides
    FOR EACH ROW EXECUTE FUNCTION rides_rearm_reminder();
