//! Who an event is for, who may join it as which group, and who may drive its rides.
//!
//! Groups form a tree (`groups.parent_id`). An event targets groups in `event_groups`:
//! - it is **visible** to members of a targeted group or of any group below it, to its
//!   organiser, and to event admins; the `is_all` group (全体会员) opens it to everyone;
//! - a member **joins as** one group: one of the event's choices that they belong to
//!   directly or through a sub-group (required, `event_signups.as_group_id`);
//! - a driver may **take an event ride** whose group is one of their groups or below one
//!   of them. Transport admins may assign anyone (audited as `ride.assign_outside_group`).
//!
//! Every SQL check that an event is visible goes through [`visible_events_sql`]; a test
//! fails if `event_groups` visibility is written out anywhere else.

use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::perms::{Act, Page};

/// The member's (groups + ancestors, groups + descendants). See `CurrentUser`.
pub async fn group_scopes(db: &PgPool, user_id: i64) -> Result<(Vec<i64>, Vec<i64>), sqlx::Error> {
    sqlx::query_as(
        "WITH RECURSIVE up AS (
             SELECT group_id AS id FROM group_members WHERE user_id = $1
             UNION
             SELECT g.parent_id FROM groups g JOIN up ON g.id = up.id WHERE g.parent_id IS NOT NULL
         ), down AS (
             SELECT group_id AS id FROM group_members WHERE user_id = $1
             UNION
             SELECT g.id FROM groups g JOIN down ON g.parent_id = down.id
         )
         SELECT COALESCE((SELECT array_agg(id) FROM up), '{}'::BIGINT[]),
                COALESCE((SELECT array_agg(id) FROM down), '{}'::BIGINT[])",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
}

/// SQL condition: event `e` is visible to the member whose id is bound at `me` and whose
/// `group_scope` is bound at `scope` (a `BIGINT[]`). Organiser, `is_all`, or a targeted
/// group in scope. Event admins are let through in Rust, not here.
pub fn visible_events_sql(me: &str, scope: &str) -> String {
    format!(
        "(e.created_by = {me} OR EXISTS (SELECT 1 FROM event_groups eg JOIN groups g ON g.id = eg.group_id
            WHERE eg.event_id = e.id AND (g.is_all = 1 OR eg.group_id = ANY({scope}))))"
    )
}

/// SQL: the ids of every active member an event (bound at `event`) is for, including
/// members of sub-groups. Used to invite them.
pub fn audience_users_sql(event: &str) -> String {
    format!(
        "WITH RECURSIVE aud AS (
             SELECT eg.group_id AS id FROM event_groups eg WHERE eg.event_id = {event}
             UNION
             SELECT g.id FROM groups g JOIN aud ON g.parent_id = aud.id
         )
         SELECT DISTINCT u.id FROM users u WHERE u.status = 'active' AND (
             EXISTS (SELECT 1 FROM event_groups eg JOIN groups g ON g.id = eg.group_id WHERE eg.event_id = {event} AND g.is_all = 1)
             OR u.id IN (SELECT gm.user_id FROM group_members gm WHERE gm.group_id IN (SELECT id FROM aud)))"
    )
}

/// The same visibility rule as [`visible_events_sql`], for an event already loaded.
pub fn sees_event(u: &CurrentUser, created_by: i64, is_all: bool, groups: &[i64]) -> bool {
    u.can(Page::AdminEvents, Act::View)
        || (u.can(Page::Events, Act::View)
            && (created_by == u.id || is_all || groups.iter().any(|g| u.group_scope.contains(g))))
}

/// May this driver take an event ride matched to `group` (None: not an event ride, or one
/// from before groups were recorded — any driver)?
pub fn drives_for(u: &CurrentUser, group: Option<i64>) -> bool {
    group.is_none_or(|g| u.drive_scope.contains(&g))
}

/// The groups a member may join an event as, `(id, name)`, best match first.
///
/// The event's choices are its narrowed list (`event_attribution_groups`) or, when there is
/// none, its audience groups and everything below them (all groups for an `is_all` event).
/// The visitor group is never a member's choice. From those, a member may pick groups they
/// belong to directly or through a sub-group. If none fits (say, an `is_all` event and a
/// member in no group yet), the `is_all` group itself is offered so they can still join.
pub async fn join_choices(
    db: &PgPool,
    u: &CurrentUser,
    event_id: i64,
) -> Result<Vec<(i64, String)>, sqlx::Error> {
    let rows: Vec<(i64, String, bool)> = sqlx::query_as(
        "WITH RECURSIVE base AS (
             SELECT group_id AS id FROM event_attribution_groups WHERE event_id = $1
         ), aud AS (
             SELECT eg.group_id AS id FROM event_groups eg WHERE eg.event_id = $1
             UNION
             SELECT g.id FROM groups g JOIN aud ON g.parent_id = aud.id
         ), choices AS (
             SELECT id FROM base
             UNION
             SELECT id FROM aud WHERE NOT EXISTS (SELECT 1 FROM base)
                 AND NOT EXISTS (SELECT 1 FROM event_groups eg JOIN groups g ON g.id = eg.group_id WHERE eg.event_id = $1 AND g.is_all = 1)
             UNION
             SELECT g.id FROM groups g WHERE NOT EXISTS (SELECT 1 FROM base)
                 AND EXISTS (SELECT 1 FROM event_groups eg JOIN groups ag ON ag.id = eg.group_id WHERE eg.event_id = $1 AND ag.is_all = 1)
         )
         SELECT g.id, g.name, g.is_all = 1 FROM groups g JOIN choices c ON c.id = g.id
         WHERE g.is_visitor = 0
         ORDER BY (g.id = ANY($3)) DESC, g.is_all, g.name",
    )
    .bind(event_id)
    .bind(u.id)
    .bind(&u.group_scope)
    .fetch_all(db)
    .await?;
    // Direct groups first: they are the likeliest answer.
    let direct: Vec<i64> =
        sqlx::query_scalar("SELECT group_id FROM group_members WHERE user_id = $1")
            .bind(u.id)
            .fetch_all(db)
            .await?;
    let mut mine: Vec<(i64, String)> = rows
        .iter()
        .filter(|(id, _, all)| !*all && u.group_scope.contains(id))
        .map(|(id, n, _)| (*id, n.clone()))
        .collect();
    mine.sort_by_key(|(id, _)| !direct.contains(id));
    if mine.is_empty() {
        mine = rows
            .into_iter()
            .filter(|(_, _, all)| *all)
            .map(|(id, n, _)| (id, n))
            .collect();
    }
    Ok(mine)
}

/// Would making `parent` the parent of `group` create a loop, or a tree deeper than 3?
pub async fn parent_ok(db: &PgPool, group: i64, parent: i64) -> Result<bool, sqlx::Error> {
    if group == parent {
        return Ok(false);
    }
    // Ancestors of the new parent, with depth.
    let ancestors: Vec<(i64, i32)> = sqlx::query_as(
        "WITH RECURSIVE up(id, depth) AS (
             SELECT $1::BIGINT, 1
             UNION
             SELECT g.parent_id, up.depth + 1 FROM groups g JOIN up ON g.id = up.id
             WHERE g.parent_id IS NOT NULL AND up.depth < 10
         ) SELECT id, depth FROM up",
    )
    .bind(parent)
    .fetch_all(db)
    .await?;
    if ancestors.iter().any(|(id, _)| *id == group) {
        return Ok(false);
    }
    // Height of the subtree under `group` (1 = no children).
    let (height,): (i32,) = sqlx::query_as(
        "WITH RECURSIVE down(id, depth) AS (
             SELECT $1::BIGINT, 1
             UNION
             SELECT g.id, down.depth + 1 FROM groups g JOIN down ON g.parent_id = down.id WHERE down.depth < 10
         ) SELECT MAX(depth) FROM down",
    )
    .bind(group)
    .fetch_one(db)
    .await?;
    let parent_depth = ancestors.iter().map(|(_, d)| *d).max().unwrap_or(1);
    Ok(parent_depth + height <= 3)
}
