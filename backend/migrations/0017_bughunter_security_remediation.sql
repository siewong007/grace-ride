-- Security audit remediation: privilege cooldown tracking, passkey reauth challenge isolation,
-- kind-scoped ride reimbursement uniqueness, complementary reimbursement SoD constraints,
-- and case-insensitive transaction-reference indexing.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS last_privileged_at TEXT;

UPDATE users u
SET last_privileged_at = app_now()
WHERE u.last_privileged_at IS NULL
  AND (
      EXISTS (
          SELECT 1 FROM member_roles mr
          WHERE mr.user_id = u.id
            AND (
                mr.role_key = 'superadmin'
                OR EXISTS (
                    SELECT 1 FROM role_permissions rp
                    WHERE rp.role_key = mr.role_key
                      AND split_part(rp.permission, '.', 1) NOT IN (
                          'rides', 'drive', 'events', 'settlements', 'costs', 'reimbursements', 'help', 'report'
                      )
                )
            )
      )
      OR EXISTS (
          SELECT 1 FROM role_permissions rp
          WHERE rp.role_key = 'member'
            AND split_part(rp.permission, '.', 1) NOT IN (
                'rides', 'drive', 'events', 'settlements', 'costs', 'reimbursements', 'help', 'report'
            )
      )
  );

ALTER TABLE auth_challenges DROP CONSTRAINT IF EXISTS chk_auth_challenges_kind;
ALTER TABLE auth_challenges ADD CONSTRAINT chk_auth_challenges_kind
    CHECK (kind IN ('reg', 'auth', 'reauth', 'bio_reg', 'bio_login', 'bio_reauth'));

DROP INDEX IF EXISTS idx_reimb_live_ride;
CREATE UNIQUE INDEX IF NOT EXISTS idx_reimb_live_ride_kind
    ON reimbursements (ride_id, kind)
    WHERE ride_id IS NOT NULL AND status <> 'rejected';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_reimbursements_payee_not_payer'
          AND conrelid = 'reimbursements'::regclass
    ) THEN
        ALTER TABLE reimbursements
            ADD CONSTRAINT chk_reimbursements_payee_not_payer
            CHECK (paid_by IS NULL OR paid_by <> user_id) NOT VALID;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_reimbursements_payee_not_payer'
          AND conrelid = 'reimbursements'::regclass
          AND NOT convalidated
    ) THEN
        BEGIN
            ALTER TABLE reimbursements VALIDATE CONSTRAINT chk_reimbursements_payee_not_payer;
        EXCEPTION WHEN check_violation THEN
            RAISE WARNING 'rows in reimbursements break chk_reimbursements_payee_not_payer: new rows are checked, existing ones need review';
        END;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_reimbursements_reviewer_not_payer'
          AND conrelid = 'reimbursements'::regclass
    ) THEN
        ALTER TABLE reimbursements
            ADD CONSTRAINT chk_reimbursements_reviewer_not_payer
            CHECK (paid_by IS NULL OR reviewed_by IS NULL OR paid_by <> reviewed_by) NOT VALID;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_reimbursements_reviewer_not_payer'
          AND conrelid = 'reimbursements'::regclass
          AND NOT convalidated
    ) THEN
        BEGIN
            ALTER TABLE reimbursements VALIDATE CONSTRAINT chk_reimbursements_reviewer_not_payer;
        EXCEPTION WHEN check_violation THEN
            RAISE WARNING 'rows in reimbursements break chk_reimbursements_reviewer_not_payer: new rows are checked, existing ones need review';
        END;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_reimbursements_payee_not_reviewer'
          AND conrelid = 'reimbursements'::regclass
    ) THEN
        ALTER TABLE reimbursements
            ADD CONSTRAINT chk_reimbursements_payee_not_reviewer
            CHECK (reviewed_by IS NULL OR reviewed_by <> user_id) NOT VALID;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_reimbursements_payee_not_reviewer'
          AND conrelid = 'reimbursements'::regclass
          AND NOT convalidated
    ) THEN
        BEGIN
            ALTER TABLE reimbursements VALIDATE CONSTRAINT chk_reimbursements_payee_not_reviewer;
        EXCEPTION WHEN check_violation THEN
            RAISE WARNING 'rows in reimbursements break chk_reimbursements_payee_not_reviewer: new rows are checked, existing ones need review';
        END;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_reimbursements_requester_not_reviewer'
          AND conrelid = 'reimbursements'::regclass
    ) THEN
        ALTER TABLE reimbursements
            ADD CONSTRAINT chk_reimbursements_requester_not_reviewer
            CHECK (reviewed_by IS NULL OR requested_by IS NULL OR reviewed_by <> requested_by) NOT VALID;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_reimbursements_requester_not_reviewer'
          AND conrelid = 'reimbursements'::regclass
          AND NOT convalidated
    ) THEN
        BEGIN
            ALTER TABLE reimbursements VALIDATE CONSTRAINT chk_reimbursements_requester_not_reviewer;
        EXCEPTION WHEN check_violation THEN
            RAISE WARNING 'rows in reimbursements break chk_reimbursements_requester_not_reviewer: new rows are checked, existing ones need review';
        END;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_settlement_shares_txn_ref_lower
    ON settlement_shares (LOWER(BTRIM(txn_ref)))
    WHERE txn_ref <> '';
