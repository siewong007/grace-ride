-- =====================================================================
-- 0004 · Data type improvements and additional constraints
-- =====================================================================
--
-- This migration addresses various data type issues identified in the schema
-- analysis and adds missing constraints for data integrity.
--
-- =====================================================================

-- 1. Add CHECK constraint for phone number format (E.164 optional, stored as TEXT)
-- Note: Application-level validation should enforce E.164 format (+60xxxxxxxxxxx)
-- but we add a generic check for non-empty and reasonable length
ALTER TABLE users DROP CONSTRAINT IF EXISTS chk_users_phone;
ALTER TABLE users ADD CONSTRAINT chk_users_phone CHECK (
    phone = '' OR (length(trim(phone)) >= 10 AND length(trim(phone)) <= 20)
);

-- 2. Add CHECK constraint for carpool_enabled (should be 0 or 1)
ALTER TABLE users DROP CONSTRAINT IF EXISTS chk_users_carpool_enabled;
ALTER TABLE users ADD CONSTRAINT chk_users_carpool_enabled CHECK (
    carpool_enabled = 0 OR carpool_enabled = 1
);

-- 3. Add CHECK constraint for spare_seats reasonableness
ALTER TABLE users DROP CONSTRAINT IF EXISTS chk_users_spare_seats;
ALTER TABLE users ADD CONSTRAINT chk_users_spare_seats CHECK (
    spare_seats >= 1 AND spare_seats <= 8
);

-- 4. Add CHECK constraint for TOTP fields (should be 0 or 1 for BIGINT boolean fields)
ALTER TABLE users DROP CONSTRAINT IF EXISTS chk_users_totp_enabled;
ALTER TABLE users ADD CONSTRAINT chk_users_totp_enabled CHECK (totp_enabled = 0 OR totp_enabled = 1);
ALTER TABLE users DROP CONSTRAINT IF EXISTS chk_users_totp_last_step;
ALTER TABLE users ADD CONSTRAINT chk_users_totp_last_step CHECK (totp_last_step >= 0);

-- 5. Add CHECK constraint for badges_opt_out
ALTER TABLE users DROP CONSTRAINT IF EXISTS chk_users_badges_opt_out;
ALTER TABLE users ADD CONSTRAINT chk_users_badges_opt_out CHECK (
    badges_opt_out = 0 OR badges_opt_out = 1
);

-- 6. Add CHECK constraint for demo_batch not being overly long
ALTER TABLE users DROP CONSTRAINT IF EXISTS chk_users_demo_batch;
ALTER TABLE users ADD CONSTRAINT chk_users_demo_batch CHECK (length(demo_batch) <= 32);

-- 7. Vehicles: add constraint for color format (basic validation)
ALTER TABLE vehicles DROP CONSTRAINT IF EXISTS chk_vehicle_color;
ALTER TABLE vehicles ADD CONSTRAINT chk_vehicle_color CHECK (length(trim(color)) >= 1);

-- 8. Vehicles: add constraint for plate format (basic non-empty check)
ALTER TABLE vehicles DROP CONSTRAINT IF EXISTS chk_vehicle_plate;
ALTER TABLE vehicles ADD CONSTRAINT chk_vehicle_plate CHECK (length(trim(plate)) >= 1);

-- 9. Events: add constraint for capacity (must be non-negative)
ALTER TABLE events DROP CONSTRAINT IF EXISTS chk_events_capacity;
ALTER TABLE events ADD CONSTRAINT chk_events_capacity CHECK (capacity >= 0);

-- 10. Events: add constraint for cost_note reasonableness
ALTER TABLE events DROP CONSTRAINT IF EXISTS chk_events_cost_note;
ALTER TABLE events ADD CONSTRAINT chk_events_cost_note CHECK (length(cost_note) <= 100);

-- 11. Rides: add constraint for seats reasonableness
ALTER TABLE rides DROP CONSTRAINT IF EXISTS chk_rides_seats;
ALTER TABLE rides ADD CONSTRAINT chk_rides_seats CHECK (seats >= 1 AND seats <= 15);

-- 12. Rides: add constraint for assistance values
ALTER TABLE rides DROP CONSTRAINT IF EXISTS chk_rides_assistance;
ALTER TABLE rides ADD CONSTRAINT chk_rides_assistance CHECK (
    assistance IN ('', 'wheelchair', 'stroller', 'oxygen', 'other')
);

-- 13. Help tickets: add constraint for status values
ALTER TABLE help_tickets DROP CONSTRAINT IF EXISTS chk_help_tickets_status;
ALTER TABLE help_tickets ADD CONSTRAINT chk_help_tickets_status CHECK (
    status IN ('open','answered','closed')
);

-- 14. Settlements: add constraint for subsidy_cents non-negative
ALTER TABLE settlements DROP CONSTRAINT IF EXISTS chk_settlements_subsidy;
ALTER TABLE settlements ADD CONSTRAINT chk_settlements_subsidy CHECK (subsidy_cents >= 0);

-- 15. Settlement shares: add constraint for amount_cents non-negative
ALTER TABLE settlement_shares DROP CONSTRAINT IF EXISTS chk_settlement_shares_amount;
ALTER TABLE settlement_shares ADD CONSTRAINT chk_settlement_shares_amount CHECK (amount_cents >= 0);

-- 16. Reimbursements: add constraint for amount_cents non-negative
ALTER TABLE reimbursements DROP CONSTRAINT IF EXISTS chk_reimbursements_amount;
ALTER TABLE reimbursements ADD CONSTRAINT chk_reimbursements_amount CHECK (amount_cents >= 0);

-- 17. Add index for users phone lookups (common query pattern)
CREATE INDEX IF NOT EXISTS idx_users_phone ON users(phone) WHERE phone != '';

-- 18. Add index for vehicles plate lookups
CREATE INDEX IF NOT EXISTS idx_vehicles_plate ON vehicles(plate) WHERE plate != '';

-- 19. Add index for events capacity queries
CREATE INDEX IF NOT EXISTS idx_events_capacity ON events(capacity) WHERE capacity > 0;

-- 20. Comment: reminder about converting TEXT temporal columns to TIMESTAMPTZ
-- Future migration should consider:
--   ALTER TABLE users ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'Asia/Kuala_Lumpur';
--   (similar for all other tables' created_at/updated_at columns)
--
-- =====================================================================
-- End of 0004
-- =====================================================================