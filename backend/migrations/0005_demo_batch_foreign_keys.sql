-- =====================================================================
-- 0005 · Demo batch foreign key relationships
-- =====================================================================
--
-- This migration addresses the use of free-text demo_batch columns
-- and establishes proper foreign key relationships with the existing
-- demo_batches table.
--
-- Before: demo_batch TEXT (free-text, used for seed data tracking)
-- After: demo_batch TEXT with foreign key to demo_batches table
--
-- =====================================================================

-- Step 1: Add foreign key constraints from existing tables to demo_batches
-- This establishes the relationship while keeping demo_batch as TEXT for backward compatibility

-- Users table: add foreign key to demo_batches (with NULL for existing rows without match)
ALTER TABLE users ADD CONSTRAINT fk_users_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Vehicles table: add foreign key to demo_batches
ALTER TABLE vehicles ADD CONSTRAINT fk_vehicles_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Invites table: add foreign key to demo_batches
ALTER TABLE invites ADD CONSTRAINT fk_invites_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Passkeys table: add foreign key to demo_batches
ALTER TABLE passkeys ADD CONSTRAINT fk_passkeys_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Rides table: add foreign key to demo_batches
ALTER TABLE rides ADD CONSTRAINT fk_rides_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Events table: add foreign key to demo_batches
ALTER TABLE events ADD CONSTRAINT fk_events_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Event signups table: add foreign key to demo_batches
ALTER TABLE event_signups ADD CONSTRAINT fk_event_signups_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Help tickets table: add foreign key to demo_batches (if column exists)
-- Check if help_tickets has demo_batch column first
-- ALTER TABLE help_tickets ADD CONSTRAINT fk_help_tickets_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Reimbursements table: add foreign key to demo_batches
ALTER TABLE reimbursements ADD CONSTRAINT fk_reimbursements_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Settlements table: add foreign key to demo_batches
ALTER TABLE settlements ADD CONSTRAINT fk_settlements_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Settlement items table: add foreign key to demo_batches
ALTER TABLE settlement_items ADD CONSTRAINT fk_settlement_items_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Settlement shares table: add foreign key to demo_batches
ALTER TABLE settlement_shares ADD CONSTRAINT fk_settlement_shares_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Settlement revisions table: add foreign key to demo_batches
ALTER TABLE settlement_revisions ADD CONSTRAINT fk_settlement_revisions_demo_batch FOREIGN KEY (demo_batch) REFERENCES demo_batches(batch) ON DELETE SET NULL;

-- Step 2: Create index on demo_batches for performance
CREATE INDEX IF NOT EXISTS idx_demo_batches_lookup ON demo_batches(batch);

-- Step 3: Update comments to reflect the new foreign key relationship
-- Application code should now reference demo_batches table instead of using
-- free-text demo_batch values for filtering and cleanup operations

-- Step 4: Add a NOT NULL constraint consideration
-- Currently demo_batch allows NULL (for live data), which is correct
-- but new inserts should ideally always have a demo_batch value
-- or be NULL for production data

-- =====================================================================
-- End of 0005
-- =====================================================================