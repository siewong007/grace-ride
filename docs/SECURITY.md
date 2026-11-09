# Security model

## What we protect

Home addresses, phone numbers, live GPS, payment receipts and bank/TNG details of about 1,000 church members, many of them elderly. The likely threats are account takeover, a curious insider browsing data they don't need, cross-site attacks against a logged-in member, and a leaked backup. Malaysian PDPA (2024 amendment) applies.

## Controls

| Area | Control | Where |
|---|---|---|
| **Sign-up** | Invite-only, single-use links that expire in 14 days. No public registration. | `routes/auth.rs`, `routes/admin.rs` |
| **Authentication** | Passkeys (WebAuthn) first. Password + TOTP for privileged roles. Admin, transport and finance features refuse sessions without passkey/2FA (`require_any`). | `routes/auth.rs`, `util.rs` |
| **Sessions** | 48-character random token in an HttpOnly, SameSite=Lax, `__Host-`-prefixed Secure cookie. **Only the SHA-256 hash is stored.** 12 h lifetime for privileged roles, 30 days for members. Revoked on logout, on disable and on recovery. | `auth.rs` |
| **Brute force** | Per-IP limits: login 10/min, recovery 3/h, invites 20/10 min, uploads 30/10 min. Per-account lock after 5 failures in 15 min. Constant-time response for unknown emails. | `security.rs`, `routes/auth.rs` |
| **2FA integrity** | TOTP codes are single-use (last step stored). An existing authenticator cannot be replaced from a session; that requires admin recovery. | `routes/auth.rs` |
| **Account recovery** | Admin verifies identity offline, then issues a 24 h one-time link. Recovering a privileged account (admin, transport, finance) needs a second admin to confirm, and only the second admin receives the link. Old sessions, passkeys and TOTP are wiped on final approval. | `routes/admin.rs` |
| **CSRF** | SameSite cookies plus an Origin / `Sec-Fetch-Site` check on every non-GET request. | `security.rs` |
| **XSS / clickjacking** | Strict CSP (`script-src 'self'`, no inline script), `frame-ancestors 'none'`, nosniff, COOP/CORP. React escapes output. | `security.rs` |
| **Least privilege** | Server-side projections per viewer. Drivers see only the area until approval. Passengers see driver phone only after approval. Receipts are visible only to the file owner, the settlement organiser/payee, the reimbursement claimant or reviewing transport/admin staff, and finance. Settlement participants see bill lines but not the organiser's receipts. Admins are *not* implicitly finance. Signup lists are organiser-only. | `routes/*.rs` |
| **Live location** | Accepted only while the driver has an `in_progress` ride. Deleted on completion. Purged by job if stale. Capped at `MAX_LIVE_DRIVERS`. | `routes/rides.rs`, `jobs.rs` |
| **Uploads** | Type detected from magic bytes (JPG/PNG/WEBP/HEIC/PDF), 8 MB cap, random file names. Served with `CSP: sandbox`. PDFs download instead of rendering. Access checked per use. | `routes/files.rs` |
| **Audit** | Every state change and failed login is logged. Visible to admin and transport. Retained `AUDIT_RETENTION_DAYS`. | `util::audit` |
| **Retention** | Trip chats purged after `CHAT_RETENTION_DAYS` (180). Read notifications after 90 days. GPS on trip end. | `jobs.rs` |
| **Config safety** | Startup refuses `DEV_MODE` on a non-localhost URL, and refuses plain http outside localhost. | `state.rs` |
| **Container** | Non-root user, read-only root filesystem, all capabilities dropped, `no-new-privileges`, memory limit. | `Dockerfile`, `docker-compose.yml` |
| **Supply chain** | `cargo audit` and `npm audit` in CI, plus weekly Dependabot. Lockfiles enforced (`--locked`, `npm ci`). | `.github/` |

## Accepted risks / known gaps

- **RUSTSEC-2023-0071 (`rsa`)** is ignored. It reaches `Cargo.lock` only through sqlx's unused MySQL driver and is never compiled (`cargo tree -i rsa --target all` is empty).
- **`style-src 'unsafe-inline'`** is needed by Leaflet's marker markup. Script injection is still blocked.
- **In-memory rate limiter** resets on restart and is per instance. The per-account lockout is in the database, so it survives restarts.
- **The data at rest is not encrypted by the app.** Use an encrypted disk/volume on the host and encrypt backups (`scripts/backup.sh` output contains personal data).
- **Notifications** carry names and times in-app only. If email or push is added, keep addresses and phone numbers out of message bodies.

## Before launch

- [ ] PDPA privacy notice and data-retention periods approved by the church (`CHAT_RETENTION_DAYS`, `AUDIT_RETENTION_DAYS`)
- [ ] Encrypted volume and off-site encrypted backups, and a test restore (`scripts/restore.sh`)
- [ ] At least two admins with passkeys, so recovery never depends on one person
- [ ] Run `make lint test audit` and an external TLS scan (e.g. SSL Labs) of the live domain

## Reporting a vulnerability

Email the PJ State CMC IT contact listed in `.env.production` (`ACME_EMAIL`). Do not open public issues for security problems.
