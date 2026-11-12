-- Address → coordinates lookups from the geocoder (OpenStreetMap Nominatim), so each
-- pickup address and destination is looked up once instead of on every route refresh.
-- `query` is the normalised address text. lat/lng NULL = the geocoder found nothing
-- (kept briefly so a bad address isn't retried on every request). `approx` = only a
-- shortened form of the address matched (e.g. street level without the house number).
-- Rows hold home-address coordinates, so they are purged by the housekeeping job.
CREATE TABLE geocode_cache (
    query      TEXT PRIMARY KEY,
    lat        DOUBLE PRECISION,
    lng        DOUBLE PRECISION,
    approx     BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TEXT NOT NULL DEFAULT (to_char(NOW() AT TIME ZONE 'Asia/Kuala_Lumpur', 'YYYY-MM-DD"T"HH24:MI:SS'))
);
CREATE INDEX idx_geocode_cache_updated ON geocode_cache(updated_at);
