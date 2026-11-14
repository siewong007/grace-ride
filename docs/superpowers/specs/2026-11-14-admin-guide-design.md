# Admin Workflow Guide — Design

Date: 2026-11-14 · Status: approved by user in session · Scope: one feature, backend + frontend

## Problem

The Help Centre (`/help`, `frontend/src/pages/Help.tsx`, `backend/src/routes/community.rs`) lets members ask questions and help-desk staff reply, but there is no in-app manual teaching admins how to operate the admin panel. Admins currently rely on out-of-app documentation.

## Decision (user-confirmed choices)

1. **Admin guide page** — a manual for admins on how to use the admin panel; not ticket tooling, not a member FAQ.
2. **Placement**: a staff-only *Admin Guide* tab inside the Help Centre. The member ask-a-question flow is untouched.
3. **Content storage**: PostgreSQL — admins edit guides in-app; no deploy needed to change the manual.
4. **Demonstration style**: ordered **steps + deep links**. Each guide walks through a workflow; each step may deep-link straight into the real admin page (e.g. `/admin/plan`). No overlay product tour in this version.

## Data model — `backend/migrations/0013_admin_guide.sql`

One table; steps are a JSONB array (steps are only ever fetched with their guide, never queried independently):

```sql
CREATE TABLE IF NOT EXISTS guide_workflows (
  id         BIGSERIAL PRIMARY KEY,
  slug       TEXT UNIQUE NOT NULL,
  sort_order INT     NOT NULL DEFAULT 0,
  published  BOOLEAN NOT NULL DEFAULT TRUE,
  title_en   TEXT NOT NULL,
  title_zh   TEXT NOT NULL,
  summary_en TEXT NOT NULL DEFAULT '',
  summary_zh TEXT NOT NULL DEFAULT '',
  steps      JSONB NOT NULL DEFAULT '[]',
  updated_at TEXT NOT NULL,
  updated_by BIGINT REFERENCES users(id) ON DELETE SET NULL
);
```

Step object shape: `{ "title_en": str, "title_zh": str, "body_en": str, "body_zh": str, "link": str | null }`.

- Bilingual content uses paired `_*en` / `_*zh` columns so translations cannot be orphaned. When rendering, an empty field falls back to the other language.
- `link` must be an internal app path starting with `/` (reject anything else — prevents external/phishing links stored in guide content). Empty/`null` means the step is informational only.

## API — added in `backend/src/routes/community.rs` beside the help-centre routes

| Route | Method | Gate |
|---|---|---|
| `/help/guides` | GET | caller holds **any** `admin_*` page view permission |
| `/help/guides` | POST | `admin_help` update |
| `/help/guides/{slug}` | GET | any `admin_*` page view permission |
| `/help/guides/{id}` | PUT | `admin_help` update |
| `/help/guides/{id}` | DELETE | `admin_help` update |

- Reuses the existing permission system (`Page::AdminHelp`, `require()` helper); no new page keys.
- GET lists only `published = TRUE` guides (ordered by `sort_order`, then `id`), except editors may pass `?include_drafts=true` (same `admin_help` update gate) so drafts are previewable.
- Writes record audit entries via the existing audit helper: `guide.create`, `guide.update`, `guide.delete` on entity `guide_workflow`.
- Validation: slug required & URL-safe, unique; at least one of `title_en`/`title_zh` non-empty; at most 30 steps; every step needs at least one non-empty title; `link` empty or internal (starts with `/`).

## Frontend

- **`Help.tsx`**: top-level tabs — *Requests* | *Admin Guide*. The Admin Guide tab renders only for users who can see any admin page; add a helper in `frontend/src/perms.ts` (e.g. `canAnyAdminPage(can)`) mirroring how the admin sidebar decides visibility. The existing staff all/mine toggle stays inside the Requests tab.
- **Guide list** (in tab): cards with title, summary, step count; published/draft pill and edit affordances for editors.
- **Guide detail** `/help/guides/:slug`: numbered steps; each step's `link` renders an *Open →* button navigating to that admin path; back link to Help Centre.
- **Editor** (staff with `admin_help` update): plain controlled form — title/summary EN+ZH, published toggle, add/remove/reorder steps (up/down buttons; no drag-drop dependency), per-step title/body EN+ZH + link path. No rich text; plain text with the app's existing `pre-wrap` rendering.
- Locale files (`help.en.ts`, `help.zh.ts`) gain **UI strings only** (tab label, editor labels, empty states); all guide content comes from the API.
- `api.ts` gains types + call helpers following existing patterns.

## Seed content — `backend/src/seed.rs`

Seed ~6 starter workflow guides, bilingual, idempotent (`ON CONFLICT (slug) DO NOTHING`):

1. Plan transport for an event (`/admin/plan`)
2. Review and pay reimbursements (`/admin/reimbursements`)
3. Send an announcement (`/admin/announcements`)
4. Manage members and invites (`/admin/members`)
5. Work the help desk (`/help`)
6. Audit log and settings (`/admin/audit`, `/admin/settings`)

## Testing

- **Backend integration tests** (alongside `backend/tests/api.rs` patterns):
  - staff user (any admin page) can list/get guides; plain member gets 403;
  - create/update/delete without `admin_help` update → 403;
  - steps round-trip through create → get;
  - external link (e.g. `https://…`) rejected;
  - audit rows written for guide writes.
- **Frontend rtl tests**: Admin Guide tab hidden for a plain member; guide detail renders steps and deep links; editor controls gated by permission.

## Out of scope (deliberate)

- Overlay/on-page product tours. The JSONB `steps` column can later carry a per-step element selector without a migration.
- Member-facing FAQ articles (separate feature if ever wanted).
- Images or attachments in steps.
- Guide versioning/history (Git-style); `updated_at`/`updated_by` are the only provenance.

## Risks

- Deep links rot if admin routes move; mitigated by seed content living in one place and links validated to the app's own route list at authoring time (frontend editor offers a dropdown of known admin routes instead of free text where practical).
- `admin_*`-permission-anyone read gate means finance/transport specialists see all guides, including those for modules they cannot open; deep links simply lead to their existing permission-gated pages, so this leaks nothing.
