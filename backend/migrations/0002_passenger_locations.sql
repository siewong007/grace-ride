-- Passenger-shared pickup location. Opt-in by the passenger, only while the
-- driver is on the way (ride in progress, not yet picked up). One row per ride
-- so carpool passengers each show up separately on the driver's map.
CREATE TABLE passenger_locations (
    ride_id    BIGINT PRIMARY KEY REFERENCES rides(id) ON DELETE CASCADE,
    lat        DOUBLE PRECISION NOT NULL,
    lng        DOUBLE PRECISION NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (to_char(NOW() AT TIME ZONE 'Asia/Kuala_Lumpur', 'YYYY-MM-DD"T"HH24:MI:SS'))
);
