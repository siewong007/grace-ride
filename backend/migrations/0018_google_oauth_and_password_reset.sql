-- Google OAuth PKCE/nonce state tracking, single-Google-per-user constraint,
-- and Google-verified password reset tokens.

ALTER TABLE oauth_states
    ADD COLUMN IF NOT EXISTS purpose TEXT NOT NULL DEFAULT 'login',
    ADD COLUMN IF NOT EXISTS user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    ADD COLUMN IF NOT EXISTS code_verifier TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS redirect_to TEXT NOT NULL DEFAULT '/',
    ADD COLUMN IF NOT EXISTS lang TEXT NOT NULL DEFAULT 'zh',
    ADD COLUMN IF NOT EXISTS email_hint TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS nonce_hash TEXT NOT NULL DEFAULT '';

ALTER TABLE oauth_states DROP CONSTRAINT IF EXISTS chk_oauth_states_purpose;
ALTER TABLE oauth_states ADD CONSTRAINT chk_oauth_states_purpose
    CHECK (purpose IN ('login', 'link', 'password_reset'));

CREATE UNIQUE INDEX IF NOT EXISTS idx_oauth_accounts_user_provider
    ON oauth_accounts (user_id, provider);

CREATE TABLE IF NOT EXISTS password_reset_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (app_now())
);

CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_user
    ON password_reset_tokens (user_id);

CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_expires
    ON password_reset_tokens (expires_at);

ALTER TABLE auth_challenges DROP CONSTRAINT IF EXISTS chk_auth_challenges_kind;
ALTER TABLE auth_challenges ADD CONSTRAINT chk_auth_challenges_kind
    CHECK (kind IN ('reg', 'auth', 'reauth', 'bio_reg', 'bio_login', 'bio_reauth', 'google_totp'));

CREATE INDEX IF NOT EXISTS idx_oauth_states_created_at
    ON oauth_states (created_at);

CREATE INDEX IF NOT EXISTS idx_oauth_states_user
    ON oauth_states (user_id);
