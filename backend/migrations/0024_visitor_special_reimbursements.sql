-- Special reimbursements for visitors on a transport plan.
-- A traffic lead asks for a one-off payment (amount + reason) tied to a recorded visitor,
-- paid to the driver who carried them. It is an ordinary claim row (kind 'visitor_special')
-- so it goes through the usual review (approve / return / reject) and finance payout.
-- Additive only: two nullable columns, one with a default, a wider kind check and indexes.

-- 1. Allow the new kind. The old check only allowed 'grab' and 'mileage'; the new one is a
--    superset, so every existing row still passes.
DO $$
DECLARE c record;
BEGIN
    FOR c IN
        SELECT conname FROM pg_constraint
        WHERE conrelid = 'reimbursements'::regclass AND contype = 'c'
          AND pg_get_constraintdef(oid) LIKE '%kind%'
    LOOP
        EXECUTE format('ALTER TABLE reimbursements DROP CONSTRAINT %I', c.conname);
    END LOOP;
END $$;
ALTER TABLE reimbursements ADD CONSTRAINT chk_reimbursements_kind
    CHECK (kind IN ('grab', 'mileage', 'visitor_special'));

-- 2. Which visitor it is for, why, and who asked for it (they may not also approve it).
ALTER TABLE reimbursements ADD COLUMN IF NOT EXISTS plan_visitor_id BIGINT
    REFERENCES plan_visitors(id) ON DELETE SET NULL;
ALTER TABLE reimbursements ADD COLUMN IF NOT EXISTS special_reason TEXT NOT NULL DEFAULT '';
ALTER TABLE reimbursements ADD COLUMN IF NOT EXISTS requested_by BIGINT
    REFERENCES users(id) ON DELETE SET NULL;

-- 3. One live special reimbursement per visitor: a new one only after the last was rejected.
CREATE UNIQUE INDEX IF NOT EXISTS uq_reimbursements_visitor_live
    ON reimbursements(plan_visitor_id)
    WHERE plan_visitor_id IS NOT NULL AND status <> 'rejected';
CREATE INDEX IF NOT EXISTS idx_reimbursements_plan_visitor
    ON reimbursements(plan_visitor_id) WHERE plan_visitor_id IS NOT NULL;
