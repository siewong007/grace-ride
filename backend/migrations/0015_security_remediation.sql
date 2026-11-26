-- Harden financial separation-of-duties constraints on payout batches and requester/payer separation.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_payout_batches_dual_control'
          AND conrelid = 'payout_batches'::regclass
    ) THEN
        ALTER TABLE payout_batches
            ADD CONSTRAINT chk_payout_batches_dual_control
            CHECK (paid_by IS NULL OR paid_by <> created_by) NOT VALID;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_payout_batches_dual_control'
          AND conrelid = 'payout_batches'::regclass
          AND NOT convalidated
    ) THEN
        BEGIN
            ALTER TABLE payout_batches VALIDATE CONSTRAINT chk_payout_batches_dual_control;
        EXCEPTION WHEN check_violation THEN
            RAISE WARNING 'rows in payout_batches break chk_payout_batches_dual_control: new rows are checked, existing ones need review';
        END;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_reimbursements_requester_not_payer'
          AND conrelid = 'reimbursements'::regclass
    ) THEN
        ALTER TABLE reimbursements
            ADD CONSTRAINT chk_reimbursements_requester_not_payer
            CHECK (paid_by IS NULL OR requested_by IS NULL OR paid_by <> requested_by) NOT VALID;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_reimbursements_requester_not_payer'
          AND conrelid = 'reimbursements'::regclass
          AND NOT convalidated
    ) THEN
        BEGIN
            ALTER TABLE reimbursements VALIDATE CONSTRAINT chk_reimbursements_requester_not_payer;
        EXCEPTION WHEN check_violation THEN
            RAISE WARNING 'rows in reimbursements break chk_reimbursements_requester_not_payer: new rows are checked, existing ones need review';
        END;
    END IF;
END $$;
