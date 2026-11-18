-- Schema hardening: partial B-tree indexes for file access checks, GIN index for
-- guide workflow step lookups, event check-in / pass-code lookup indexes, and
-- NOT VALID + validated foreign keys for profile and event cover photos.
--
-- Fully repeatable for schema replay and upgrade safety.

-- 1. Partial B-tree indexes backing files::can_access_file lookups
CREATE INDEX IF NOT EXISTS idx_users_photo_file
    ON users(photo_file_id) WHERE photo_file_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_events_photo_file
    ON events(photo_file_id) WHERE photo_file_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_settlement_items_receipt_file
    ON settlement_items(receipt_file_id) WHERE receipt_file_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_settlement_shares_proof_file
    ON settlement_shares(proof_file_id) WHERE proof_file_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_settlements_payee_qr_file
    ON settlements(payee_qr_file_id) WHERE payee_qr_file_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_reimbursements_receipt_file
    ON reimbursements(receipt_file_id) WHERE receipt_file_id IS NOT NULL;

-- 2. GIN index for guide_workflows.steps @> jsonb containment checks
CREATE INDEX IF NOT EXISTS idx_guide_workflows_steps
    ON guide_workflows USING gin (steps);

-- 3. Event check-in code & member pass code lookup indexes
CREATE INDEX IF NOT EXISTS idx_events_checkin_code
    ON events(checkin_code) WHERE checkin_code IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_event_signups_pass_code
    ON event_signups(event_id, pass_code) WHERE pass_code IS NOT NULL;

-- 4. Foreign keys on users.photo_file_id and events.photo_file_id (ON DELETE SET NULL)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'users'::regclass AND conname = 'fk_users_photo_file'
    ) THEN
        ALTER TABLE users
            ADD CONSTRAINT fk_users_photo_file
            FOREIGN KEY (photo_file_id) REFERENCES files(id) ON DELETE SET NULL
            NOT VALID;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'users'::regclass AND conname = 'fk_users_photo_file' AND NOT convalidated
    ) THEN
        BEGIN
            ALTER TABLE users VALIDATE CONSTRAINT fk_users_photo_file;
        EXCEPTION WHEN check_violation OR foreign_key_violation THEN
            RAISE WARNING 'rows in users break fk_users_photo_file: new rows are checked, existing ones need review';
        END;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'events'::regclass AND conname = 'fk_events_photo_file'
    ) THEN
        ALTER TABLE events
            ADD CONSTRAINT fk_events_photo_file
            FOREIGN KEY (photo_file_id) REFERENCES files(id) ON DELETE SET NULL
            NOT VALID;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'events'::regclass AND conname = 'fk_events_photo_file' AND NOT convalidated
    ) THEN
        BEGIN
            ALTER TABLE events VALIDATE CONSTRAINT fk_events_photo_file;
        EXCEPTION WHEN check_violation OR foreign_key_violation THEN
            RAISE WARNING 'rows in events break fk_events_photo_file: new rows are checked, existing ones need review';
        END;
    END IF;
END
$$;
