-- How much a member has paid towards their share, as last sent or confirmed. A revised
-- bill moves the share to 'needs_info' with a note to reconcile; without this, a second
-- revision could no longer tell that money had changed hands and reset the share to
-- 'unpaid', forgetting the payment. 0 means nothing paid.
--
-- Safe to replay: the column is only added once and the backfill only fills zeros.

ALTER TABLE settlement_shares ADD COLUMN IF NOT EXISTS paid_cents BIGINT NOT NULL DEFAULT 0;

UPDATE settlement_shares
   SET paid_cents = amount_cents
 WHERE paid_cents = 0
   AND amount_cents > 0
   AND status IN ('pending', 'confirmed');
