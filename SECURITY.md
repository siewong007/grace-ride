# Security Policy

Grace Ride (恩典同行) coordinates volunteer carpooling, fellowship events, and activity cost-sharing for **PJ State CMC**, handling sensitive member personal data (home/pickup addresses, live trip GPS, mobility/health assistance flags, phone numbers, and reimbursement receipts).

## Supported Versions

Grace Ride runs as a continuously deployed single-tenant production service for PJ State CMC. Only the latest `master` deployment is actively supported with security updates:

| Version / Branch | Supported |
|---|---|
| `master` (`0.1.x` active deployment) | Yes |
| Older commits / snapshots | No (upgrade to `master`) |

## Reporting a Vulnerability Privately

**Do not open public GitHub issues, discussions, or pull requests for suspected security or privacy vulnerabilities.**

Report vulnerabilities privately to the PJ State CMC IT / Grace Ride maintainers:

1. **Email:** Contact the PJ State CMC IT / DPO address configured in `.env.production` (`ACME_EMAIL`, e.g. `it@pjstatecmc.org`) or reach out directly to the repository owner via GitHub private security reporting if enabled.
2. **Include:**
   - Affected route, endpoint, role, or component (e.g. `/api/rides`, `/visit/:token`, `frontend/public/sw.js`).
   - Step-by-step reproduction against a local `DEV_MODE=true` synthetic instance (`@example.org` demo accounts—never test against live member accounts or real personal data).
   - Expected vs. actual authorization, privacy projection, or state behavior, and potential impact.
3. **Response Targets:**
   - Initial acknowledgement within **48 hours**.
   - Triage and remediation plan within **7 days** (immediate containment within **24 hours** for active data exposure; see the PDPA breach runbook).

## Security Architecture Summary

Full control specifications and source pointers are maintained in [`docs/TECHNICAL.md`](docs/TECHNICAL.md):

- **Authentication & Step-Up MFA:** Invite-only onboarding (single-use 14-day hashed tokens, no public self-registration). WebAuthn passkeys are primary (with optional `ANDROID_CERT_SHA256_FINGERPRINTS` serving `/.well-known/assetlinks.json` and `android:apk-key-hash:` origins for native Android passkeys), paired on Android with hardware-backed `BIOMETRIC_STRONG` ECDSA P-256 challenge-response (`BiometricPrompt` + `AndroidKeyStore`, invalidated on OS biometric changes, plus cold-start / idle-resume app-launch lock), alongside password + single-use TOTP and salted SHA-256 backup codes. Accessing any non-Member area (`Transport`, `Finance`, `Admin`) requires a passkey-, biometric-, or 2FA-verified session (`util::require`), and idle or manually locked sessions return HTTP `423` server-side (`security::evaluate_session_lock`). On Android, session cookies and lock state are encrypted via `AndroidKeyStore` AES-256-GCM (`SecureStore`).
- **Granular RBAC & Chain of Command:** 29 page keys and 73 page/action permissions across 14 built-in roles plus custom roles, governed by a 5-tier leadership rank hierarchy (`superadmin > president > vice_president > finance > auditor`). `Finance` permissions can never be self-granted, monthly payout batches enforce two-person separation (`created_by != paid_by`), and privileged account recovery requires two distinct administrators.
- **Scoped Capability Passes:** Zero-account guest links (`/visit/:token`) bind to an isolated `gr_visit` `HttpOnly` cookie scoped to `/api/visit` that never yields an authenticated member session (`CurrentUser`). Caregiver Safe Arrival links (`/care/:token`) store SHA-256 token hashes, open 24 hours before pickup (closing on expiry or terminal ride state), and omit street addresses and raw coordinates while surfacing trip status and readiness controls.
- **Browser, CSRF, CSP, Location & Outbox Hardening:** `__Host-`-prefixed `Secure` `HttpOnly` `SameSite=Lax` session cookies (12 h TTL for staff, 30 d for members), mandatory `Origin` / `Sec-Fetch-Site` checks on all state-changing requests, bounded per-IP rate limiters (evicting the oldest windows at the 50k cap rather than locking out new addresses), optional Cloudflare Turnstile human verification on public web auth, account recovery, invite acceptance, and visitor-pass claim endpoints (verified server-side via `siteverify` before consuming one-time tokens), strict CSP (`script-src 'self'` by default, conditionally allowing `https://challenges.cloudflare.com` in `script-src`/`frame-src`/`connect-src` only when Turnstile is enabled, `frame-ancestors 'none'`, `CSP: sandbox` on uploaded files), magic-byte upload verification (`JPEG`, `PNG`, `WebP`, `GIF`, `HEIC`, `PDF`) plus EXIF/XMP/IPTC stripping on area photos, DNS-pinned SSRF checks (`notifications/ssrf.rs`) and HMAC-SHA256 signing on outbound webhooks via the durable `notification_deliveries` outbox, per-user lock-screen notification redaction, viewer-scoped pickup area masking prior to staff approval, `LiveTripLocationService` GPS strictly scoped to active `in_progress` rides, and a privacy-safe Service Worker that never caches `/api/*` responses paired with immediate wiping of local offline ride snapshots (`gr_today_rides_v1`) and offline action queues (`gr_offline_actions_v1`) on logout, lock, or HTTP `401`.

## Further Reading

- [Security Model — Threat Model, Controls & Accepted Risks](docs/TECHNICAL.md#7-security-model)
- [Compliance, PDPA & Malaysian Law — Data Inventory & 72-Hour Breach Runbook](docs/TECHNICAL.md#8-compliance-pdpa--malaysian-law)
- [Native Android Security, Signing & Passkeys](docs/ANDROID.md)
