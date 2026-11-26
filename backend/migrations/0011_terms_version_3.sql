-- Version 3 (25 Nov 2026) expands the Terms & Privacy Notice to cover all
-- location usages (journey-only live tracking, Android foreground location service,
-- passenger waiting pins, optional saved-address pins, external navigation handoff)
-- and other external/device services (Cloudflare Turnstile, Web Push vendors,
-- optional Telegram/WhatsApp alert channels, Google Play Services QR scanner,
-- Care Links, proxy riders, and calendar feeds).
-- Existing members on version 1 or 2 are prompted to re-opt-in at next sign-in.
ALTER TABLE users ALTER COLUMN terms_version SET DEFAULT 3;
