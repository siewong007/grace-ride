# ADR: Event audiences and the rides tied to them

**Status:** Proposed. Product decisions were answered on 16 November 2026 (see the
last section).
**Date:** 16 November 2026
**Deciders:** product owner (event rules), transport team (driver matching), maintainer (schema)

## Context

Rides can already belong to an event: `rides.event_id` is a nullable foreign key to
`events`. Signing up with "need a ride" starts that flow, transport staff plan event rides
in `plan.rs`, and withdrawing from or cancelling an event cancels its linked rides.

Who sees an event is decided by `event_groups`. If one of the targeted groups is the
`is_all` group (全体会员), every member sees it. Otherwise only members of the targeted
groups see it, plus the creator and event admins (`can_see_event`). Groups are flat.
Only people with accounts can sign up. A non-member can only ride as a proxy booked by a
member (`is_proxy`, `rider_name`).

The church needs three kinds of event, plus visitors:

| Kind | Example | Who sees and joins | Recorded per person |
|---|---|---|---|
| **Open, with attribution** | Church anniversary dinner | All members, plus invited visitors | The group they join as (required) |
| **Group-only** | Youth camp | Youth and its sub-groups | The group they join as (required) |
| **Sub-group-only** | Youth staff planning night | Youth Staff only | The group they join as (required) |
| **Visitor** | A friend brought to the dinner | Only through a one-time link | Joins as Visitor |

Gaps in today's model:

1. **No sub-groups.** You can't say "Staff is part of Youth".
2. **No attribution.** You can't record which group someone joined as.
3. **Private event names leak.** `ride_json` returns `event.title` to anyone who can see
   the ride, including a driver browsing `/driver/requests`.
4. **Driving isn't tied to groups.** Any driver in the pickup area can offer to drive
   anyone.
5. **Visitors have no access of their own.** They can't see their ride, their driver, or
   say "I'm ready" unless a member relays it.

## Decision

Keep `event_groups` as the single source of who an event is for, and build four things
on it.

### 1. Sub-groups: `groups.parent_id`

```sql
ALTER TABLE groups ADD COLUMN IF NOT EXISTS parent_id BIGINT REFERENCES groups(id) ON DELETE SET NULL;
-- no cycles (checked in the admin handler); depth capped at 3
```

**Visibility rule:** an event targeting group G is visible to members of G *or of any
group below G*. An event for **Youth** is seen by Youth and Youth Staff. An event for
**Youth Staff** is seen by Youth Staff only. Membership stays explicit, and the tree is
only used when working out an audience.

One shared SQL fragment, `visible_events_sql()` in `events.rs`, replaces the three
copies that exist today. It is used by `can_see_event`, `list_events`, the event
notification audience query, and the ride SELECT:

```sql
WITH RECURSIVE my_groups AS (           -- groups I'm in, plus every ancestor of them
  SELECT group_id AS id FROM group_members WHERE user_id = $me
  UNION
  SELECT g.parent_id FROM groups g JOIN my_groups m ON g.id = m.id WHERE g.parent_id IS NOT NULL
)
... EXISTS (SELECT 1 FROM event_groups eg JOIN groups g ON g.id = eg.group_id
            WHERE eg.event_id = e.id AND (g.is_all = 1 OR eg.group_id IN (SELECT id FROM my_groups)))
```

**Who sees an event's group list ("For: Youth · Youth Staff"):** only people who can see
the event (decision 4). Anyone else sees neither the event nor its groups.

### 2. Joining as: required

```sql
CREATE TABLE IF NOT EXISTS event_attribution_groups (   -- the "joining as" choices
    event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    group_id BIGINT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    PRIMARY KEY (event_id, group_id)
);
ALTER TABLE event_signups ADD COLUMN IF NOT EXISTS as_group_id BIGINT REFERENCES groups(id);
ALTER TABLE rides         ADD COLUMN IF NOT EXISTS as_group_id BIGINT REFERENCES groups(id);
```

- **Defaults:** the choices default to the event's audience groups and their sub-groups.
  The organiser can narrow them.
- **Members:** a member may only pick a group they belong to (directly or through a
  sub-group) that is also one of the event's choices.
- **Visitors:** they are always recorded as the built-in Visitor group. They never pick.
- **Required (decision 2):** signing up for an event without `as_group_id` returns 400.
  The signup form preselects the choice when exactly one fits.
- **Rides copy it:** a ride for an event gets `as_group_id` from the signup when it is
  created. It is copied, not looked up later, so last month's reports still show the
  right group if someone moves. Creating an event ride without a `going` signup is
  refused.
- **Existing data:** old signups and rides keep `NULL`. Reports show them as "Not
  recorded".

### 3. Driving within your group (decision 3)

**Rule:** a driver may offer to drive an event ride only when the ride's `as_group_id` is
one of the driver's groups or below one of them. A Youth driver may drive Youth and
Youth Staff passengers. A Youth Staff driver may drive Youth Staff passengers only.
**Transport admins can assign any driver** (`admin_rides.update`). Admin assignment is
the only way past the rule, and it is recorded in the audit log as
`ride.assign_outside_group`.

| Where | Change |
|---|---|
| `/driver/requests` | Event rides are listed only to drivers who match the rule |
| Driver offer (`POST /rides/{id}/offer` and similar) | 403 when the driver doesn't match the rule |
| Admin assign and plan-transport | Matching drivers are listed first; others are listed under "Outside group (admin only)" |
| Rides without an event | Unchanged: area-based, any driver |
| Visitor rides | Matched against the **host's** joining-as group (see 4), because Visitor is not a driving group |

**Showing the event on a ride:** `ride_json` shows the event's title only when the
viewer can see the event. Everyone else sees "Church event" (教会活动) and no link. The
check is computed in the ride SELECT as a boolean column. In practice the rule above
already keeps outside drivers away from these rides. The masking still matters for
admin-assigned drivers and for screens where rides are shared.

### 4. Visitor passes: a one-time link that only works for the journey (decision 1)

A member who is going, or an organiser, invites a visitor from the event page. The
visitor gets a link and needs no account or password.

```sql
CREATE TABLE IF NOT EXISTS visitor_passes (
    id            BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    token_hash    TEXT NOT NULL UNIQUE,              -- hash_token(random_token(48)); the link holds the raw token
    event_id      BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    host_id       BIGINT NOT NULL REFERENCES users(id),     -- the member who invited them
    host_group_id BIGINT NOT NULL REFERENCES groups(id),    -- host's joining-as group: used to match drivers
    ride_id       BIGINT REFERENCES rides(id),
    name          TEXT NOT NULL,
    phone         TEXT NOT NULL DEFAULT '',
    valid_from    TEXT NOT NULL,                     -- pickup time minus 24 h (or when created, if later)
    valid_until   TEXT NOT NULL,                     -- hard stop: event start + 12 h
    claimed_at    TEXT,                              -- first open; the link is then bound to that browser
    closed_at     TEXT,                              -- set when the journey ends
    closed_reason TEXT,                              -- completed | cancelled | revoked | expired
    created_at    TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS visitor_sessions (     -- cookie for the claimed browser, like sessions
    token_hash TEXT PRIMARY KEY,
    pass_id    BIGINT NOT NULL REFERENCES visitor_passes(id) ON DELETE CASCADE,
    expires_at TEXT NOT NULL
);
```

**Lifecycle:**

| Stage | What happens |
|---|---|
| **Create** | The host enters the visitor's name and phone. A ride is created for the event as a proxy ride (the passenger is the host, `is_proxy = 1`, the rider is the visitor's name and phone). Its `as_group_id` is the Visitor group, so it shows in reports, while matching uses `host_group_id`. The host shares the link (WhatsApp share sheet). |
| **Before `valid_from`** | The link shows "Your ride opens on Sun 22 Nov, 8:00 am" and nothing else. |
| **One-time** | The first open within the window **claims** the link: it is used up, and that browser gets an httpOnly `visitor_session` cookie. Later opens from other browsers see "This link was already used. Ask {host} to send a new one." The host can reissue it, which revokes the old link and makes a new one. |
| **Open** | The visitor sees one page and only that page: their ride (time, pickup, driver name, car, plate), the driver's live location while the ride is in progress, and "I'm ready", "Running late" and "Can't make it". No other app routes or APIs accept a visitor session. |
| **Close** | It closes automatically and permanently in three cases: the ride is completed (driver marks drop-off) → `closed_reason = completed`; the ride is cancelled or "can't make it" → `cancelled`; or `valid_until` passes → `expired`, as a backstop when a driver forgets to complete. The host or an admin can revoke at any time. On close, the visitor session is deleted, so the page shows "This journey has ended. Thank you for coming!" |

**What the scope covers:**

- Visitor endpoints live under `/visit/{…}` and use a separate extractor (`VisitorPass`)
  that never produces a `CurrentUser`. Member endpoints can't be reached with a visitor
  cookie.
- Tokens are stored hashed, as sessions already are. Lookups are rate-limited like
  invite lookups.
- The phone number is shown only to the assigned driver, the host and staff, as for
  proxy riders today.
- Closing runs in the same transaction as the ride status change (completed or
  cancelled). The `expired` backstop runs in the existing reminder tick.

## Options considered

### Option A: extend groups and `event_groups`, plus visitor passes (recommended)

| Dimension | Assessment |
|---|---|
| Complexity | Medium: 1 column on groups, 3 tables, 2 columns, 1 shared SQL fragment, 1 visitor page |
| Cost | About 4–5 days including UI and tests (visitor passes are about half of that) |
| Scale | Group trees are tiny (tens of groups); visitor passes are a handful per event |
| Team familiarity | Same patterns as sessions, invites and proxy rides |

**Pros:**
- One rule ("event for G, so G and everything under it") covers events, driving and
  visibility.
- Visitors reuse the proxy-ride path, so drivers' screens don't change.
- Existing events and rides behave as before.

**Cons:**
- The tree must be guarded against cycles.
- A claimed link is lost if the visitor changes phone. The host has to reissue it.

### Option B: a general audience rule table

`event_audience(event_id, group_id, include_subgroups, role, exclude)` with allow and
deny rules.

**Pros:** expresses unusual cases such as "Youth except staff".
**Cons:** none of the requested cases need it. Organisers would have to learn rule
precedence, and it would take about 2 weeks.

### Option C: visitors as temporary accounts

Create a real `users` row with a Visitor role and an expiry.

**Pros:** no separate visitor auth path.
**Cons:**
- It pollutes member lists, counts and the audit trail with one-day accounts.
- A forgotten account stays signed in with member-level APIs.
- Deleting it later breaks the ride's foreign keys.

A pass scoped to one ride is safer and simpler.

## Trade-off analysis

Option A matches every decision the church made with the fewest new ideas. Groups gain
a parent, event signups record a group, and visitors get a narrow, expiring pass instead
of an account. Option B can express more, but nobody needs that. Option C saves one auth
path but leaves accounts that can do too much and outlive the journey, which is exactly
what decision 1 rules out.

## Consequences

**Easier:**
- Youth-only and staff-only events with group-matched drivers.
- Per-group headcounts, ride planning and reports.
- Bringing a friend without creating an account.

**Harder:**
- Every "who can see this event" check must use the shared fragment. A grep test in
  `backend/tests` should fail if anyone writes `event_groups eg JOIN groups` outside it.
- Driver supply per group can run thin. Admins need to see unmatched event rides early.
  Add a dashboard counter for "event rides with no in-group driver" 24 hours before
  pickup.

**To revisit:**
- Visitors who come often may later want "convert to member" from a pass.
- Per-group cost splits in settlements.

## Action items

1. [ ] **Schema:** add `groups.parent_id`, `event_attribution_groups`,
   `event_signups.as_group_id`, `rides.as_group_id`, `visitor_passes` and
   `visitor_sessions`, plus a seeded Visitor group, in `0001_init.sql`. Use
   `IF NOT EXISTS` like the existing lines.
2. [ ] **Shared rule:** write `visible_events_sql()` and switch all audience checks over
   to it.
3. [ ] **Admin groups screen:** add a "Part of" picker, guard against cycles, and cap
   depth at 3.
4. [ ] **Event form:** label the audience picker "Who is this for", and add "Joining as"
   choices (defaulting to the audience).
5. [ ] **Signup:** make "Joining as" required (400 without it), and copy it to the ride.
   Refuse event rides without a `going` signup.
6. [ ] **Driver matching:** filter `/driver/requests`, return 403 on out-of-group
   offers, and add the admin assign override with an audit entry. Plan-transport
   groups drivers into "In group" and "Outside group".
7. [ ] **Masking:** show the masked event title in `ride_json` for viewers who can't
   see the event.
8. [ ] **Visitor passes:**
   - backend: create, reissue and revoke for the host or admin; the `/visit/{token}`
     claim; the `VisitorPass` extractor; readiness endpoints scoped to the pass;
     closing on ride completed or cancelled; the expiry backstop in the reminder tick;
   - frontend: an "Invite a visitor" sheet on the event page, and a single-page
     `/visit` route with the closed and not-yet-open states.
9. [ ] **Tests:**
   - sub-group visibility both ways;
   - "joining as" is required and limited to own groups;
   - an out-of-group driver can't see or offer, but an admin can assign;
   - a masked title for an outside viewer;
   - a visitor link before the window, on claim, on a second-browser open, after
     completed, after cancelled, and after expiry;
   - a visitor cookie is rejected by member APIs.
10. [ ] **Docs:** update `docs/ARCHITECTURE.md`, `FEATURES.md` and `SECURITY.md` (the
    visitor auth path).

## Product decisions (16 November 2026)

1. **Visitors** get a one-time link that works only for their journey and closes when
   the journey ends. → Section 4.
2. **"Joining as" is required.** → Section 2.
3. **Drivers pick up only within their group and its sub-groups,** unless an admin
   assigns them. → Section 3.
4. **The event and its group list** are visible only to people eligible through the
   event's groups or sub-groups. → Sections 1 and 3.

### Assumptions to confirm

- **Who can invite a visitor:** any member who is going to the event, plus organisers.
- **Visitor ride matching:** uses the host's joining-as group, so a Youth member's visitor
  is driven by a Youth driver.
- **Window:** the link opens 24 hours before pickup and stops 12 hours after the event
  starts if the ride was never completed.
- **The rule applies to event rides only.** Everyday rides without an event stay
  matched by area, as today.
