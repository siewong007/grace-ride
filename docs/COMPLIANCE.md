# Compliance: privacy law, terms, copyright and third-party services

> **Draft, 17 November 2026.** This review was prepared from the code and public sources. It is not legal advice. PJ State CMC must have it, the in-app terms (version 2) and the Malay translation reviewed by a Malaysian lawyer and approved by church leadership before launch. [Features](FEATURES.md) R-34 still applies: nothing here is approved policy until the church signs it off.

[Security](SECURITY.md) owns access controls and technical safeguards. This document owns the legal side: what the law asks of the church, what the app does about it, and what is still open.

## Summary

| Area | Status | Owner of the next step |
|---|---|---|
| Privacy notice in Malay and English (PDPA s.7) | Implemented as terms v2, with Malay a tap away. The Malay text is a draft translation. | Church: qualified translator |
| Explicit consent for sensitive data (religion, health) | Covered in the notice and the single opt-in | Counsel: is one tick enough? |
| Re-consent when terms change | Implemented (`TERMS_VERSION`, migration 0007) | Developer: raise the version on each material change |
| Visitors (no account) | Notice on `/visit/:token`; the host is told to get the visitor's agreement first | Counsel: confirm the host-consent approach |
| Data subject rights: access, correction, portability, withdrawal | Profile edit only; everything else is a manual admin request | Developer: build self-service export and account erasure |
| Retention | Chats, notifications, GPS, audit and backups have limits. Closed accounts, ride history and receipts do not. | Church: set periods. Developer: add a purge job. |
| Breach notification | Runbook below | Church: name a DPO or person responsible |
| Cross-border transfer (map services) | Disclosed in the notice; names are never sent | Church: accept the transfer, or self-host |
| Public OSM/OSRM servers | Allowed for small non-commercial use. No service guarantee. | Ops: self-host OSRM before launch |
| Volunteer driving and transport law | No-fare model stated in the terms | Counsel: check reimbursements and insurance |
| Software copyright | `LICENSE`: proprietary, licensed to the church | Developer and church: sign a written licence |
| Church logo in the Grace Ride mark | Not yet documented | Church: written permission |
| Open-source licences | `THIRD_PARTY_NOTICES.md` and a generated `/third-party-licenses.txt` | Re-check on dependency changes |

## Personal Data Protection Act 2010 (PDPA)

### Does it apply?

The PDPA covers personal data processed "in respect of commercial transactions". A free, members-only church ride service may fall outside that scope, and the cost-sharing and reimbursement features make the answer less clear. **Counsel should decide this.** Either way, this review recommends complying fully. The data is sensitive (about 1,000 members, many elderly, with home addresses, live location, health-related assistance needs and payee details), and full compliance costs little more than partial compliance.

The Personal Data Protection (Amendment) Act 2024 came into force in phases. The changes to cross-border transfer rules and penalties, and the new duties on data processors, started on 1 April 2025. Mandatory DPO appointment, breach notification and data portability started on 1 June 2025. Check the Commissioner's (JPDP) current guidelines before launch, because some guidelines were still being finalised when the sources below were written.

### What the app holds

| Data | Where | Sensitive? |
|---|---|---|
| Name, email, phone, photo | `users` | No |
| Church membership (implied by having an account) and group | `users`, `group_members` | **Yes: religious belief** |
| Assistance needs (wheelchair, oxygen, stroller, other) | `rides.assistance` | **Yes: physical health** |
| Home and pickup addresses, ride times | `rides`, geocode cache | No, but a high safety risk for elderly members |
| Live GPS during a trip | memory and `rides`, cleared at trip end | No, but a high safety risk |
| Payee name, phone or account, receipt photos | `settlements`, `reimbursements`, `files` | No, but financial |
| Trip chats, notifications, help requests | various | No |
| Sign-in log with IP addresses, audit trail | `audit_log` | No |
| Visitor name, phone, pickup | visitor passes | No. The person has no account and did not see the terms. |

### Notice and choice (s.7)

- The notice must be in **both Malay and English** (s.7(3)). The app's languages are Chinese and English. The terms page therefore has a "Baca dalam Bahasa Malaysia" link that shows the full Malay text (`frontend/src/locales/terms.ms.ts`). The Malay is a **draft translation** and needs a qualified translator.
- Terms v2 (`frontend/src/locales/terms.en.ts`) states who the data controller is, what data is collected, why, who sees it, the overseas map services, retention periods, members' rights, the right to complain to JPDP, the volunteer-driver basis and copyright.
- **Still to fill in:** a named contact (an office email or phone) for data requests. The notice says "the church office or an admin". Counsel may want a specific address.

### Sensitive data and explicit consent (s.40)

Religious belief and physical health are sensitive personal data, and processing them needs explicit consent. Version 2 has a "Sensitive information" section, and the opt-in records the time and version (`users.terms_accepted_at`, `users.terms_version`, plus a `terms.accept` audit entry naming the version). The assistance field is optional, and the notice says it may be left empty.

**Open question for counsel:** is a single tick covering the whole notice enough, or does sensitive data need its own separate tick? If a separate tick is needed, add a second checkbox to `TermsGate` and store its own timestamp.

### Re-consent on change

Before this change, `accept_terms` kept the first acceptance time forever, so nobody was asked again when the terms changed. Now:

- `backend/src/routes/auth.rs` defines `TERMS_VERSION` (currently 2).
- `/api/auth/me` reports `terms_accepted` only when the stored version is current, so the existing `TermsGate` asks again.
- Migration `0007_terms_version.sql` marks everyone who had already agreed as version 1, so every existing member is asked to accept v2 once.
- Test: `backend/tests/gaps.rs::older_terms_version_asks_again`.

To publish a new version: edit all three `terms.*.ts` files, raise `TERMS_VERSION`, and add a migration that raises the column default to the same number.

**Gap:** the terms gate is enforced in the web app only. The API itself does not refuse requests from a member who has not opted in. An unapproved member can do little (their status is `pending`), but a member gated by a new terms version could still call the API directly. Consider enforcing the gate in the `CurrentUser` extractor for every route except `/auth/*`.

### Visitors

A visitor's details are entered by the member who invites them. Now:

- The invite card tells the host to add a visitor only after the visitor agrees (`visit.*.ts` `bringConsent`).
- The visitor's page explains how their details are used (`privacyNote`).

Counsel should confirm that consent obtained through the host is acceptable. A firmer option is to make the host tick a confirmation box, recorded on the visitor pass.

### Minors

The terms ask a parent or guardian to opt in for anyone under 18. The app does not check age. If children ride, the church should collect a guardian's consent outside the app (for example, on a youth ministry form).

### Data subject rights

| Right | Today | Gap |
|---|---|---|
| Access | Manual: an admin reads the record | Add "Download my data" (JSON) on Profile |
| Correction | Self-service profile edit | None |
| Portability (since 1 June 2025) | None | The same JSON export covers it |
| Withdraw consent / erasure | An admin disables the account; the data stays | Add account closure that anonymises the profile and keeps only records the church must hold |

Answer requests within 21 days (the PDPA access-request period). Log each request and its outcome.

### Retention (s.10)

| Data | Period | Set by |
|---|---|---|
| Live GPS | Until the trip ends | `jobs.rs` |
| Trip chats | 180 days | `CHAT_RETENTION_DAYS` |
| Notifications | 90 days read, 1 year unread | `NOTIFICATION_RETENTION_DAYS` |
| Audit log | 730 days | `AUDIT_RETENTION_DAYS` |
| Backups | 30 days | `scripts/backup.sh` `KEEP_DAYS` |
| Profiles, ride history, receipts, settlements | **No limit** | Church must decide. Finance records may need about 7 years; other records probably need less. |
| Disabled accounts | **No limit** | Church must decide, then a purge or anonymisation job is needed |

If the church changes any period, update the terms text in all three languages and raise `TERMS_VERSION`.

### Security (s.9)

See [Security](SECURITY.md). The open items most relevant to the PDPA are data-at-rest encryption (use an encrypted volume and encrypted backups) and keeping staff permissions to the minimum.

### Data protection officer

The 2024 amendment requires data controllers to appoint a DPO from 1 June 2025, subject to the thresholds in the Commissioner's DPO guideline. Check whether the church falls within them. Either way, name one responsible person (a church leader, not the developer) for data requests and breaches.

### Personal data breach runbook

1. **Contain.** Revoke the affected sessions (an admin can disable accounts; `DELETE FROM sessions` signs everyone out), rotate secrets and close the hole.
2. **Assess within 24 hours.** What data was involved, whose, how many people, and whether the breach is likely to cause significant harm. Location, addresses and health data for elderly members count as high risk.
3. **Notify the Commissioner** as soon as practicable. The Commissioner's consultation proposed 72 hours from becoming aware of the breach. Check the final Data Breach Notification Guideline for the current deadline and form.
4. **Notify affected members** without undue delay when significant harm is likely. Tell them, in plain words in Chinese, English and Malay, what happened and what they should do.
5. **Record** the breach, your decisions and your evidence. The audit log (`/admin/audit`) shows who did what. Keep the record even when you decide not to notify.

### Cross-border transfer (s.129)

The server sends **addresses and coordinates, never names,** to:

| Service | Operator, likely location | Data sent |
|---|---|---|
| Nominatim (geocoding) | OSM Foundation, UK/EU | Pickup and destination addresses |
| OSRM demo server (routing) | FOSSGIS, Germany | Coordinates |
| Photon (address suggestions) | komoot, Germany | Partial addresses as typed |
| OSM tile servers | OSM Foundation, CDN | Each member's IP address and the map area viewed |

Since the 2024 amendment, a transfer is allowed where the destination has a law substantially similar to the PDPA or ensures an adequate level of protection. The UK and EU (GDPR) likely meet that test, and the notice discloses the transfer. To keep data inside Malaysia instead, self-host Nominatim and OSRM (both are open source) and set `NOMINATIM_URL` and `OSRM_URL`.

## Third-party services

These are free, community-run servers with usage policies:

- **OSRM demo server** (`router.project-osrm.org`, the default in `routing.rs`): "reasonable, non-commercial" use only, at most 1 request per second, with no uptime guarantee. **Self-host OSRM before launch.** Planning a busy Sunday must not depend on a volunteer demo server.
- **Nominatim**: at most 1 request per second; an identifying User-Agent (not a library default); cache results; no autocomplete on its API. The app caches geocodes and uses Photon for suggestions, so it complies. Set `GEOCODER_USER_AGENT` to include a contact, for example `GraceRide/0.1 (+https://your-domain)`.
- **OSM tiles**: attribution with a link to the copyright page (done in `LiveMap.tsx`), no heavy use, a valid Referer. About 1,000 members should be fine. If usage grows, move to a commercial or self-hosted tile provider and update the CSP in `security.rs`.
- **Photon (komoot)**: fair use, no guarantee.

## Transport law, insurance and liability

- The app runs as **volunteer carpooling with no fare.** Carrying passengers for hire or reward without a licence is regulated under the Land Public Transport Act 2010 and the e-hailing rules. **Counsel should confirm** that church reimbursements and member cost-sharing (fuel and tolls at church-set rates, `settlements` and `reimbursements`) do not amount to "hire or reward".
- **Insurance:** private motor policies may exclude carrying passengers for reward. Ask drivers to confirm their policy covers volunteer driving with cost-sharing. The church should consider volunteer personal-accident or liability cover.
- **Liability:** the terms say rides are arranged in good faith without a guarantee. Under Malaysian law, liability for negligence causing death or personal injury generally cannot be excluded by contract. Rely on insurance, not on the disclaimer.
- **Driver checks:** the terms require a valid licence, road tax and insurance. The app does not verify them. The transport team should check them when approving a driver.
- **Electronic consent:** the Electronic Commerce Act 2006 recognises agreements formed electronically. The app's opt-in records who agreed, when and to which version.

## Copyright and brand

- **Software:** `LICENSE` makes the code proprietary to the developer (all rights reserved), with a non-exclusive licence for PJ State CMC to run and modify it for its own ministry. **Sign a short written agreement** between the developer and the church. It should cover the licence, who hosts the app, the developer's role as **data processor** if they run the server (the amendment puts security duties directly on processors), support, and the return and deletion of data if the arrangement ends. Replace the GitHub handle in `LICENSE` with the developer's legal name.
- `frontend/package.json` is marked `"license": "UNLICENSED"`, and `backend/Cargo.toml` is marked `publish = false`, so neither can be published to a registry by accident.
- **Brand:** according to `docs/brand/grace-ride-logo-kit/README.md`, the Grace Ride mark is set inside the blue arc from the PJ State CMC church logo. The church (or the denomination, if it owns the logo) should give written permission for this use.
- **User content:** members keep the rights to the photos and messages they add, and grant the church a licence to display them in the app (terms "Copyright" section). Receipts are used for finance only.
- **Demo media:** screenshots and recordings in `docs/video` must use demo data only, never real members, before they are shared.
- **Open-source components:** see [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md). The one unusual licence is react-leaflet's Hippocratic-2.1.

## Launch checklist (legal)

- [ ] Counsel confirms PDPA scope, the explicit-consent mechanism, visitor consent and the transport-law position
- [ ] Church approves terms v2 in all three languages; the Malay is checked by a translator
- [ ] Named DPO or responsible person, and a contact address in the notice
- [ ] Retention periods set for profiles, ride history, receipts and disabled accounts
- [ ] Developer–church licence and data-processing agreement signed
- [ ] Church's written permission to use its logo arc
- [ ] OSRM self-hosted; `GEOCODER_USER_AGENT` includes a contact
- [ ] Drivers confirm their licence and insurance cover
- [ ] Data export and account erasure built, or a manual procedure written down

## Sources

- Baker McKenzie, [PDPA (Amendment) Act 2024 to come into force](https://insightplus.bakermckenzie.com/bm/data-technology/malaysia-personal-data-protection-amendment-act-2024-to-come-into-force)
- Rajah & Tann, [Dates of coming into operation of the PDP (Amendment) Act 2024](https://www.rajahtannasia.com/?p=161886)
- Tay & Partners, [Countdown to compliance](https://www.legal500.com/firms/31184-tay-partners/c-malaysia/news-and-developments/countdown-to-compliance-personal-data-protection-amendment-act-2024-in-force-starting-1-january-2025)
- Chambers, [Data Protection & Privacy 2026: Malaysia](https://practiceguides.chambers.com/practice-guides/data-protection-privacy-2026/malaysia)
- Christopher & Lee Ong, [PDPA Amendment Act A1727](https://christopherleeong.com/wp-content/uploads/2024/12/PDPA-Amendment-Act-A1727.pdf)
- OSM Foundation, [Nominatim usage policy](https://operations.osmfoundation.org/policies/nominatim/); OSRM, [demo server](https://github.com/Project-OSRM/osrm-backend/wiki/Demo-server); OSM, [copyright and licence](https://www.openstreetmap.org/copyright)
