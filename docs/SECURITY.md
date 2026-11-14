# Security model

Reviewed against source on 14 November 2026. See [Features](FEATURES.md) for scope and [Testing](TESTING.md) for verification limits.

## What we protect

Home addresses, phone numbers, live GPS, payment receipts and bank/TNG details of about 1,000 church members, many of them elderly. The likely threats are account takeover, a curious insider browsing data they don't need, cross-site attacks against a logged-in member, and a leaked backup. The church must obtain review of applicable Malaysian privacy obligations and approve its notice and retention policy before launch.

## Controls

| Area | Control | Where |
|---|---|---|
| **Sign-up** | Invite-only, single-use links that expire in 14 days. No public registration. | `routes/auth.rs`, `routes/admin.rs` |
| **Authentication** | Passkeys (WebAuthn) first. Password + TOTP for privileged accounts. Every staff page (transport, finance, admin) refuses sessions without passkey/2FA (`util::require`), whichever role grants it. | `routes/auth.rs`, `util.rs`, `perms.rs` |
| **Sessions** | 48-character random token in an HttpOnly, SameSite=Lax, `__Host-`-prefixed Secure cookie. The cookie is the only accepted credential (no `Authorization` header), so the pending-member guard can't be bypassed. **Only the SHA-256 hash is stored.** 12 h lifetime for accounts that hold any staff permission, 30 days for members. Revoked on logout, on disable and on recovery. | `auth.rs` |
| **Brute force** | Per-IP limits: login 10/min, recovery 3/h, invites 20/10 min, uploads 30/10 min. Unknown emails, and members without a passkey, get a decoy passkey challenge (stable fake credential ids keyed by a secret in `app_secrets`), so sign-in can't be used to find out who is a member. Uploads: at most 100 files per member per 24 h. | `security.rs`, `routes/auth.rs` |
| **2FA integrity** | TOTP codes are single-use (last step stored). An existing authenticator cannot be replaced from a session; that requires admin recovery. Once an account has a passkey or authenticator, adding or removing one needs a sign-in from the last 15 minutes, so a stolen session can't plant a device. | `routes/auth.rs` |
| **Account recovery** | Admin verifies identity offline, then issues a 24 h one-time link. Recovering a privileged account (one holding any staff permission) needs a second admin to confirm, and only the second admin receives the link. Recovery links are shown once, never listed afterwards, and only their SHA-256 is stored. Old sessions, passkeys and TOTP are wiped on final approval. | `routes/admin.rs` |
| **CSRF** | SameSite cookies plus an Origin / `Sec-Fetch-Site` check on every non-GET request. | `security.rs` |
| **XSS / clickjacking** | Strict CSP (`script-src 'self'`, no inline script), `frame-ancestors 'none'`, nosniff, COOP/CORP. React escapes output. | `security.rs` |
| **Roles & permissions** | A permission is one action (view / create / update / delete) on one page; roles are editable sets of permissions stored in the database (`roles`, `role_permissions`) and every handler checks the permission, never a role name. Built-in roles (`member`, held by every active member, plus `superadmin`, `admin`, `transport`, `finance`, `leader`, `driver`) cannot be deleted, and all but `superadmin` can be edited; administrators add custom roles at `/admin/roles`. Changes apply on the member's next request. Fixed rules no configuration can remove: staff pages need passkey/2FA; nobody can give themselves a finance permission (by taking a role or by editing a role they hold); `superadmin` always holds every permission (including pages added later) and is the one role that cannot be edited; only a super admin can give that role or change a super admin's account, and the last active one can be neither demoted nor disabled, so there is always someone who can repair a misconfiguration (on a first run, or a database from before the role existed, start-up gives it to the original administrator and records that in the audit log); nobody can remove their own right to manage roles; giving a member a role needs the roles page's update permission, so approving members does not allow self-promotion. Role and permission changes are audited. | `perms.rs`, `routes/roles.rs`, `routes/admin.rs` |
| **Least privilege** | Server-side projections per viewer. Drivers see only the area until approval. Passengers see driver phone only after approval. Receipts are visible only to the file owner, the settlement organiser/payee, the reimbursement claimant, reimbursement reviewers, and finance (payout and bill viewers). Settlement participants see bill lines but not the organiser's receipts. Admins are *not* implicitly finance. Signup lists are organiser-only. | `routes/*.rs` |
| **Live location** | Accepted only while the driver has an `in_progress` ride. Deleted on completion. Purged by job if stale. Capped at `MAX_LIVE_DRIVERS`. | `routes/rides.rs`, `jobs.rs` |
| **Request size** | API request bodies are capped at 128 KB (uploads 9 MB). Names are at most 60 characters and phone numbers 10–20. | `lib.rs`, `util.rs` |
| **Uploads** | Type detected from magic bytes (JPG/PNG/WEBP/HEIC/PDF), 8 MB cap, random file names, 100 per member per day. A bill can only attach the organiser's own uploads. Served with `CSP: sandbox`. PDFs download instead of rendering. Access checked per use. | `routes/files.rs` |
| **Audit** | Every state change is logged with who did it, including sign-in, sign-out, failed sign-ins (with the caller's IP), passkey and 2FA changes, uploads, help requests and the changes the app makes on its own (ride escalation, waitlist promotion, rides cancelled with their event), which carry no actor. Entries name what changed, not private values (a profile edit lists the fields, a help reply has no text). Not logged by design: reads, notification read marks, chat messages (kept in the chat itself) and GPS positions (only the passenger opting in and either side switching sharing off). Visible to holders of the audit-log permission (admin and transport by default). Retained `AUDIT_RETENTION_DAYS`. | `util::audit`, `tests/audit.rs` |
| **Pickup routing** | To order pickups and estimate ETAs, pickup addresses and the destination are sent to the geocoder (OpenStreetMap Nominatim by default) and coordinates to the router (OSRM): text and coordinates only, never names or phone numbers. Results are cached in `geocode_cache` (purged after 30 days) and logs never include the address. A passenger's ETA response carries only their own distance/ETA and how many pickups come first; the route line is sent to a passenger only when no one else's pickup lies on it. Address autocomplete sends what a member types (3+ characters, debounced) to Photon (komoot, OpenStreetMap data) through the server, signed-in members only, so the browser never contacts Photon and Photon never sees members' IP addresses; queries are not logged. Set `ROUTING_ENABLED=false` to keep addresses in-house (autocomplete then turns off), or self-host via `OSRM_URL` / `NOMINATIM_URL` / `PHOTON_URL`. | `routing.rs`, `jobs.rs` |
| **Retention** | Trip chats purged after `CHAT_RETENTION_DAYS` (180). Read notifications after 90 days. GPS on trip end. | `jobs.rs` |
| **Config safety** | Startup refuses `DEV_MODE` on a non-localhost URL, and refuses plain http outside localhost. | `state.rs` |
| **Container** | Non-root user, read-only root filesystem, all capabilities dropped, `no-new-privileges`, memory limit. | `Dockerfile`, `docker-compose.yml` |
| **Supply chain** | `cargo audit` and `npm audit` in CI, plus weekly Dependabot. Lockfiles enforced (`--locked`, `npm ci`). Automatic deployment follows successful CI for the pushed commit; `deploy.yml` also exposes explicit manual dispatch. | `.github/` |

## Auto-lock and backup codes

Profile supports hashed, single-use TOTP backup codes; recovery clears them. The organization default `session_timeout_mins` and member `auto_lock_mins` drive the React `AutoLockProvider`. Positive member values override the default; `-1` disables auto-lock and `0` uses the organization default. The current Unlock button only clears local UI state. This is a screen overlay, not reauthentication or server-side idle expiration; API session validity remains controlled by the session extractor and expiration timestamp.

Source: `frontend/src/components/AutoLock.tsx`, `backend/src/auth.rs`, `backend/src/routes/auth.rs`, migrations 0010 and 0012.

## Accepted risks / known gaps

- **Dependency exceptions:** CI currently ignores `RUSTSEC-2023-0071` and `RUSTSEC-2026-0459`. Inspect `.github/workflows/ci.yml` and the Cargo audit configuration when reviewing a release; this documentation audit did not rerun the dependency scan or verify current advisory applicability.
- **`style-src 'unsafe-inline'`** is needed by Leaflet's marker markup. Script injection is still blocked.
- **In-memory rate limiter** resets on restart and is per instance. The per-account lockout is in the database, so it survives restarts.
- **The data at rest is not encrypted by the app.** Use an encrypted disk/volume on the host and encrypt backups (`scripts/backup.sh` output contains personal data).
- **Notifications** carry names and times in-app only. If email or push is added, keep addresses and phone numbers out of message bodies.

## Before launch

- [ ] PDPA privacy notice and data-retention periods approved by the church (`CHAT_RETENTION_DAYS`, `AUDIT_RETENTION_DAYS`)
- [ ] Encrypted volume and off-site encrypted backups, and a test restore (`scripts/restore.sh`)
- [ ] At least two admins with passkeys, so recovery never depends on one person
- [ ] Run the full checks in [Testing](TESTING.md) and an external TLS scan (e.g. SSL Labs) of the live domain

## Reporting a vulnerability

Email the PJ State CMC IT contact listed in `.env.production` (`ACME_EMAIL`). Do not open public issues for security problems.
