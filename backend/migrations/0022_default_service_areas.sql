-- Frequent driver service areas, so the traffic lead can price trips between them.
-- Additive only: an area that already exists (by name) is left as it is, and no fares
-- are set here. The traffic lead fills in each pair's fare in Settings → Fares; a pair
-- with no fare keeps asking the driver for a mileage receipt.
INSERT INTO areas (name, active)
VALUES ('UT', 1), ('Sec 17', 1), ('UM', 1), ('KK', 1), ('Pacific/RM', 1)
ON CONFLICT (name) DO NOTHING;
