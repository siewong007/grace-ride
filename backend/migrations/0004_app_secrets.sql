-- Secrets the app generates for itself on first use and must keep across restarts,
-- e.g. the HMAC key behind the decoy passkey challenge for unknown emails (rotating it
-- would let someone tell real accounts from decoys). Never sent to clients.
CREATE TABLE app_secrets (
    name  TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
