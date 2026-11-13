-- 恩典同行 Grace Ride — auto-reimbursement & extended report schema
--
-- Adds vehicle type classification (standard / other) so that per-km
-- reimbursement rates can differ by vehicle type. Stores the computed
-- trip distance and suggested reimbursement on each completed ride so the
-- driver report can surface reimbursement totals without recomputing.
--
-- Applied at API startup via sqlx::migrate!("./migrations") in connect_db().

-- ====================================================================
-- Vehicle type classification
-- ====================================================================
ALTER TABLE vehicles
    ADD COLUMN vehicle_type TEXT NOT NULL DEFAULT 'standard'
        CHECK (vehicle_type IN ('standard', 'other'));

-- ====================================================================
-- Ride-level distance & reimbursement
-- ====================================================================
ALTER TABLE rides
    ADD COLUMN distance_km BIGINT,
    ADD COLUMN reimbursement_amount_cents BIGINT NOT NULL DEFAULT 0;

-- ====================================================================
-- Reimbursement rate settings (cents per km)
-- ====================================================================
CREATE TABLE IF NOT EXISTS reimbursement_rates (
    vehicle_type TEXT PRIMARY KEY CHECK (vehicle_type IN ('standard', 'other')),
    rate_cents_per_km BIGINT NOT NULL CHECK (rate_cents_per_km >= 0),
    updated_at TEXT NOT NULL DEFAULT (to_char(NOW() AT TIME ZONE 'Asia/Kuala_Lumpur', 'YYYY-MM-DD"T"HH24:MI:SS'))
);

INSERT INTO reimbursement_rates (vehicle_type, rate_cents_per_km)
VALUES ('standard', 30), ('other', 50)
ON CONFLICT (vehicle_type) DO NOTHING;

CREATE INDEX idx_reimbursement_rates_type ON reimbursement_rates(vehicle_type);

-- ====================================================================
-- Indexes for report queries by driver/passenger + period
-- ====================================================================
CREATE INDEX idx_rides_driver_completed ON rides(driver_id, completed_at) WHERE status = 'completed';
CREATE INDEX idx_rides_passenger_completed ON rides(passenger_id, completed_at) WHERE status = 'completed';