-- Event-specific mileage claim eligibility.
-- Only events authorized by leadership (superadmin, president, vice_president)
-- allow volunteer drivers to receive mileage reimbursement for rides.
ALTER TABLE events ADD COLUMN IF NOT EXISTS mileage_claimable BIGINT NOT NULL DEFAULT 0;
