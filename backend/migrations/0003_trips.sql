-- =====================================================================
-- 0003: a trips table, so ride chats belong to something real.
--
-- A trip is the group of confirmed rides one driver takes to one place together; its
-- id is the id of the ride that started it (see routes::rides::assign_trip). Before
-- this file `rides.trip_id` and `ride_messages.trip_id` were bare numbers that no
-- table defined, so nothing stopped a chat message pointing at a trip that never was.
--
-- The app still sets `rides.trip_id` directly: a trigger adds the trips row first.
-- Deleting a trip removes its chat and unlinks its rides.
--
-- Repeatable, like 0002 (an old history may replay every file; see lib.rs).

CREATE TABLE IF NOT EXISTS trips (
    id         BIGINT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (to_char(NOW() AT TIME ZONE 'Asia/Kuala_Lumpur', 'YYYY-MM-DD"T"HH24:MI:SS'))
);

-- Every trip already in use, by a ride or by a chat whose rides have since moved on.
INSERT INTO trips (id)
SELECT trip_id FROM rides WHERE trip_id IS NOT NULL
UNION
SELECT trip_id FROM ride_messages
ON CONFLICT (id) DO NOTHING;

CREATE OR REPLACE FUNCTION rides_ensure_trip() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.trip_id IS NOT NULL THEN
        INSERT INTO trips (id) VALUES (NEW.trip_id) ON CONFLICT (id) DO NOTHING;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS trg_rides_ensure_trip ON rides;
CREATE TRIGGER trg_rides_ensure_trip
    BEFORE INSERT OR UPDATE OF trip_id ON rides
    FOR EACH ROW EXECUTE FUNCTION rides_ensure_trip();

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'rides'::regclass AND conname = 'fk_rides_trip') THEN
        ALTER TABLE rides ADD CONSTRAINT fk_rides_trip
            FOREIGN KEY (trip_id) REFERENCES trips(id) ON DELETE SET NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ride_messages'::regclass AND conname = 'fk_ride_messages_trip') THEN
        ALTER TABLE ride_messages ADD CONSTRAINT fk_ride_messages_trip
            FOREIGN KEY (trip_id) REFERENCES trips(id) ON DELETE CASCADE;
    END IF;
END
$$;
