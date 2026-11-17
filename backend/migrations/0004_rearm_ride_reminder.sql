-- A ride's one-hour reminder goes to whoever is driving it. When the driver changes
-- (they drop out and another takes over, staff reassign, the transport plan moves the
-- ride), the reminder is due again, so the new driver gets it and the passenger's names
-- the right person. Done here because the driver is changed from several places.
--
-- Safe to replay: the function is replaced and the trigger recreated.

CREATE OR REPLACE FUNCTION rides_rearm_reminder() RETURNS trigger
LANGUAGE plpgsql
SET search_path FROM CURRENT
AS $$
BEGIN
    IF NEW.driver_id IS DISTINCT FROM OLD.driver_id THEN
        NEW.reminder_sent := 0;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS trg_rides_rearm_reminder ON rides;
CREATE TRIGGER trg_rides_rearm_reminder
    BEFORE UPDATE OF driver_id ON rides
    FOR EACH ROW EXECUTE FUNCTION rides_rearm_reminder();
