-- Hardening session TOTP revocation, Web Push session binding, and biometric attestation status.

ALTER TABLE sessions
    ADD COLUMN IF NOT EXISTS failed_totp_attempts BIGINT NOT NULL DEFAULT 0;

ALTER TABLE push_subscriptions
    ADD COLUMN IF NOT EXISTS session_token TEXT REFERENCES sessions(token) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_push_subs_session_token
    ON push_subscriptions(session_token)
    WHERE session_token IS NOT NULL;

ALTER TABLE biometric_credentials
    ADD COLUMN IF NOT EXISTS attested BOOLEAN NOT NULL DEFAULT FALSE;
