-- Pass 2 deep security audit hardening:
-- 1. Normalized uniqueness on proxy_riders(user_id, LOWER(BTRIM(name))).
-- 2. Database-level uniqueness for post-event mileage claims and live receipt reuse.
-- 3. Uniqueness on settlement_revisions(settlement_id, version).
-- 4. Non-negative CHECK invariants on rides.reimbursement_amount_cents and settlement_shares.paid_cents.
-- 5. Preserve reimbursement audit history by preventing deletion of referenced plan_visitors.

-- 1. Deduplicate normalized proxy_riders names (prefer linked caregiver row, then earliest id)
DELETE FROM proxy_riders p
USING (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY user_id, LOWER(BTRIM(name))
               ORDER BY (caregiver_user_id IS NOT NULL) DESC, id ASC
           ) AS rn
    FROM proxy_riders
) d
WHERE p.id = d.id AND d.rn > 1;

CREATE UNIQUE INDEX IF NOT EXISTS uq_proxy_riders_user_name_norm
    ON proxy_riders (user_id, LOWER(BTRIM(name)));

-- 2. Database-enforced claim uniqueness for post-event mileage and non-rejected receipt files
CREATE UNIQUE INDEX IF NOT EXISTS uq_reimbursements_post_event_mileage
    ON reimbursements (event_id, user_id)
    WHERE status <> 'rejected'
      AND kind = 'mileage'
      AND ride_id IS NULL
      AND event_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_reimbursements_receipt_file_live
    ON reimbursements (receipt_file_id)
    WHERE status <> 'rejected' AND receipt_file_id IS NOT NULL AND demo_batch IS NULL;

-- 3. Revision version uniqueness per settlement
CREATE UNIQUE INDEX IF NOT EXISTS uq_settlement_revisions_ver
    ON settlement_revisions (settlement_id, version);

-- 4. Non-negative financial column invariants
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_rides_reimb_nonneg'
          AND conrelid = 'rides'::regclass
    ) THEN
        ALTER TABLE rides
            ADD CONSTRAINT chk_rides_reimb_nonneg
            CHECK (reimbursement_amount_cents >= 0);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_settlement_shares_paid_nonneg'
          AND conrelid = 'settlement_shares'::regclass
    ) THEN
        ALTER TABLE settlement_shares
            ADD CONSTRAINT chk_settlement_shares_paid_nonneg
            CHECK (paid_cents >= 0);
    END IF;
END $$;

-- 5. Enforce ON DELETE RESTRICT on reimbursements.plan_visitor_id
DO $$
DECLARE c record;
BEGIN
    FOR c IN
        SELECT conname
        FROM pg_constraint
        WHERE conrelid = 'reimbursements'::regclass
          AND contype = 'f'
          AND pg_get_constraintdef(oid) LIKE '%(plan_visitor_id) REFERENCES plan_visitors%'
          AND confdeltype <> 'r'
    LOOP
        EXECUTE format('ALTER TABLE reimbursements DROP CONSTRAINT %I', c.conname);
    END LOOP;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'reimbursements'::regclass
          AND contype = 'f'
          AND pg_get_constraintdef(oid) LIKE '%(plan_visitor_id) REFERENCES plan_visitors%'
    ) THEN
        ALTER TABLE reimbursements
            ADD CONSTRAINT fk_reimbursements_plan_visitor
            FOREIGN KEY (plan_visitor_id) REFERENCES plan_visitors(id) ON DELETE RESTRICT;
    END IF;
END $$;
