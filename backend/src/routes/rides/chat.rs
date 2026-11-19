//! Group trip chat endpoints for rides: messages, emoji reactions and
//! delivered / read receipts.

use std::collections::{BTreeMap, HashMap};

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::perms::Act;
use crate::state::AppState;
use crate::util::{notify_trip, now, ErrorKey};

use super::{load_ride, traffic_can};

/// Reactions a member can put on a message (one per member per message).
pub const CHAT_REACTIONS: &[&str] = &[
    "\u{1F44D}",        // 👍
    "\u{2764}\u{FE0F}", // ❤️
    "\u{1F602}",        // 😂
    "\u{1F62E}",        // 😮
    "\u{1F622}",        // 😢
    "\u{1F64F}",        // 🙏
];

// ---------------- Trip chat ----------------

struct ChatAccess {
    trip: i64,
    open: bool,
    /// Passenger (not cancelled) or driver of a ride on this trip. Only members move
    /// receipt cursors and count as recipients; staff who can view the chat do not.
    is_member: bool,
    /// Every member's user id.
    members: Vec<i64>,
}

async fn chat_access(st: &AppState, u: &CurrentUser, ride_id: i64) -> AppResult<ChatAccess> {
    let r = load_ride(st, u.lang, ride_id).await?;
    let trip = r
        .trip_id
        .ok_or_else(|| bad(u.lang, ErrorKey::GroupChatNotApproved))?;
    let rows: Vec<(i64, Option<i64>, String)> =
        sqlx::query_as("SELECT passenger_id, driver_id, status FROM rides WHERE trip_id=$1")
            .bind(trip)
            .fetch_all(&st.db)
            .await?;
    let mut members: Vec<i64> = Vec::new();
    for (p, d, s) in &rows {
        if s != "cancelled" && !members.contains(p) {
            members.push(*p);
        }
        // A cancelled ride's driver is no longer on the trip; counting them would pin
        // every receipt at "sent" because they never open the chat again.
        if let Some(d) = (*d).filter(|_| s != "cancelled") {
            if !members.contains(&d) {
                members.push(d);
            }
        }
    }
    let is_member = members.contains(&u.id);
    if !is_member && !traffic_can(u, &r, Act::View) {
        return Err(forbidden(u.lang, ErrorKey::OnlyTripMembersCanViewChat));
    }
    let open = rows
        .iter()
        .any(|(_, _, s)| s == "confirmed" || s == "in_progress");
    Ok(ChatAccess {
        trip,
        open,
        is_member,
        members,
    })
}

/// Raise a member's delivered (and, with `read`, read) cursor to `up_to`; never lowers it.
async fn bump_cursor(
    st: &AppState,
    trip: i64,
    user_id: i64,
    up_to: i64,
    read: bool,
) -> AppResult<()> {
    let sql = if read {
        "INSERT INTO ride_chat_reads (trip_id, user_id, last_delivered_id, last_read_id, created_at, updated_at)
         VALUES ($1,$2,$3,$3,$4,$4)
         ON CONFLICT (trip_id, user_id) DO UPDATE SET
           last_delivered_id = GREATEST(ride_chat_reads.last_delivered_id, EXCLUDED.last_delivered_id),
           last_read_id = EXCLUDED.last_read_id,
           updated_at = EXCLUDED.updated_at
         WHERE ride_chat_reads.last_read_id < EXCLUDED.last_read_id"
    } else {
        "INSERT INTO ride_chat_reads (trip_id, user_id, last_delivered_id, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$4)
         ON CONFLICT (trip_id, user_id) DO UPDATE SET
           last_delivered_id = EXCLUDED.last_delivered_id,
           updated_at = EXCLUDED.updated_at
         WHERE ride_chat_reads.last_delivered_id < EXCLUDED.last_delivered_id"
    };
    sqlx::query(sql)
        .bind(trip)
        .bind(user_id)
        .bind(up_to)
        .bind(now())
        .execute(&st.db)
        .await?;
    Ok(())
}

#[derive(Deserialize)]
pub(super) struct AfterQuery {
    after: Option<i64>,
}

pub(super) async fn list_messages(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Query(q): Query<AfterQuery>,
) -> AppResult<Json<Value>> {
    let acc = chat_access(&st, &u, id).await?;
    let trip = acc.trip;
    let after = q.after.unwrap_or(0);
    let rows: Vec<(i64, Option<i64>, Option<String>, String, String)> = sqlx::query_as(
        "SELECT m.id, m.user_id, u.name, m.body, m.created_at FROM ride_messages m LEFT JOIN users u ON u.id=m.user_id
         WHERE m.trip_id=$1 AND m.id > $2 ORDER BY m.id LIMIT 300",
    )
    .bind(trip)
    .bind(after)
    .fetch_all(&st.db)
    .await?;

    // This member's app now holds these messages: they count as delivered.
    if acc.is_member {
        if let Some(max_id) = rows.last().map(|r| r.0) {
            bump_cursor(&st, trip, u.id, max_id, false).await?;
        }
    }

    let members: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT u.name FROM rides r JOIN users u ON u.id = r.passenger_id OR u.id = r.driver_id
         WHERE r.trip_id=$1 AND r.status NOT IN ('cancelled')",
    )
    .bind(trip)
    .fetch_all(&st.db)
    .await?;

    // Reactions, grouped per message and emoji in first-reacted order.
    let reaction_rows: Vec<(i64, String, i64, Option<String>)> = sqlx::query_as(
        "SELECT r.message_id, r.emoji, r.user_id, u.name FROM ride_message_reactions r
         JOIN ride_messages m ON m.id = r.message_id
         LEFT JOIN users u ON u.id = r.user_id
         WHERE m.trip_id=$1 AND m.id > $2 ORDER BY r.created_at, r.user_id",
    )
    .bind(trip)
    .bind(after)
    .fetch_all(&st.db)
    .await?;
    let mut reactions: HashMap<i64, Vec<(String, Vec<String>, bool)>> = HashMap::new();
    for (mid, emoji, uid, name) in reaction_rows {
        let list = reactions.entry(mid).or_default();
        let idx = match list.iter().position(|(e, _, _)| *e == emoji) {
            Some(i) => i,
            None => {
                list.push((emoji, Vec::new(), false));
                list.len() - 1
            }
        };
        let entry = &mut list[idx];
        entry.1.push(name.unwrap_or_default());
        entry.2 |= uid == u.id;
    }

    // Receipts for the caller's own messages: every other member's lowest cursor.
    let others: Vec<i64> = acc.members.iter().copied().filter(|m| *m != u.id).collect();
    let (delivered_floor, read_floor) = if others.is_empty() {
        (0, 0)
    } else {
        let cursors: BTreeMap<i64, (i64, i64)> = sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT user_id, last_delivered_id, last_read_id FROM ride_chat_reads
             WHERE trip_id=$1 AND user_id = ANY($2)",
        )
        .bind(trip)
        .bind(&others)
        .fetch_all(&st.db)
        .await?
        .into_iter()
        .map(|(uid, d, r)| (uid, (d.max(r), r)))
        .collect();
        others.iter().fold((i64::MAX, i64::MAX), |(d, r), m| {
            let (md, mr) = cursors.get(m).copied().unwrap_or((0, 0));
            (d.min(md), r.min(mr))
        })
    };

    let messages: Vec<Value> = rows
        .into_iter()
        .map(|(id, uid, name, body, at)| {
            let mine = uid == Some(u.id);
            let reacts: Vec<Value> = reactions
                .remove(&id)
                .unwrap_or_default()
                .into_iter()
                .map(|(emoji, names, me)| {
                    json!({"emoji": emoji, "count": names.len(), "mine": me, "names": names})
                })
                .collect();
            let mut m = json!({
                "id": id, "user_id": uid, "name": name, "body": body,
                "created_at": at, "mine": mine, "reactions": reacts,
            });
            if mine {
                let status = if id <= read_floor {
                    "read"
                } else if id <= delivered_floor {
                    "delivered"
                } else {
                    "sent"
                };
                m["status"] = json!(status);
            }
            m
        })
        .collect();

    Ok(Json(json!({
        "open": acc.open,
        "members": members.into_iter().map(|m| m.0).collect::<Vec<_>>(),
        "reactions_allowed": CHAT_REACTIONS,
        "messages": messages,
    })))
}

#[derive(Deserialize)]
pub(super) struct MsgIn {
    body: String,
}

pub(super) async fn post_message(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<MsgIn>,
) -> AppResult<Json<Value>> {
    let acc = chat_access(&st, &u, id).await?;
    if !acc.open {
        return Err(bad(u.lang, ErrorKey::GroupChatClosed));
    }
    let body = req.body.trim();
    if body.is_empty() || body.chars().count() > 1000 {
        return Err(bad(u.lang, ErrorKey::InvalidMessageLength));
    }
    let (mid,): (i64,) = sqlx::query_as(
        "INSERT INTO ride_messages (trip_id, user_id, body, created_at) VALUES ($1,$2,$3,$4) RETURNING id",
    )
    .bind(acc.trip)
    .bind(u.id)
    .bind(body)
    .bind(now())
    .fetch_one(&st.db)
    .await?;
    // The sender has obviously seen everything up to their own message.
    if acc.is_member {
        bump_cursor(&st, acc.trip, u.id, mid, true).await?;
    }
    notify_trip(&st.db, acc.trip).await;
    Ok(Json(json!({"ok": true, "id": mid})))
}

#[derive(Deserialize)]
pub(super) struct ReadIn {
    up_to: i64,
}

/// Mark the trip chat read up to a message id (the client calls this while the chat is
/// on screen). Staff viewers who are not trip members are accepted but not recorded.
pub(super) async fn mark_read(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReadIn>,
) -> AppResult<Json<Value>> {
    let acc = chat_access(&st, &u, id).await?;
    if acc.is_member && req.up_to > 0 {
        // Never past the newest message, so a cursor can't pre-read future messages.
        let (latest,): (Option<i64>,) =
            sqlx::query_as("SELECT MAX(id) FROM ride_messages WHERE trip_id=$1")
                .bind(acc.trip)
                .fetch_one(&st.db)
                .await?;
        let up_to = req.up_to.min(latest.unwrap_or(0));
        if up_to > 0 {
            bump_cursor(&st, acc.trip, u.id, up_to, true).await?;
        }
    }
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub(super) struct ReactionIn {
    emoji: String,
}

/// Toggle the caller's reaction on a message: a new emoji replaces their old one, the
/// same emoji again removes it.
pub(super) async fn react(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, mid)): Path<(i64, i64)>,
    Json(req): Json<ReactionIn>,
) -> AppResult<Json<Value>> {
    let acc = chat_access(&st, &u, id).await?;
    if !acc.open {
        return Err(bad(u.lang, ErrorKey::GroupChatClosed));
    }
    if !CHAT_REACTIONS.contains(&req.emoji.as_str()) {
        return Err(bad(u.lang, ErrorKey::InvalidReaction));
    }
    // The message must belong to this ride's trip and be a member's (not a system) message.
    let found: Option<(Option<i64>,)> =
        sqlx::query_as("SELECT user_id FROM ride_messages WHERE id=$1 AND trip_id=$2")
            .bind(mid)
            .bind(acc.trip)
            .fetch_optional(&st.db)
            .await?;
    match found {
        None => return Err(not_found(u.lang)),
        Some((None,)) => return Err(bad(u.lang, ErrorKey::InvalidReaction)),
        Some((Some(_),)) => {}
    }
    let removed = sqlx::query(
        "DELETE FROM ride_message_reactions WHERE message_id=$1 AND user_id=$2 AND emoji=$3",
    )
    .bind(mid)
    .bind(u.id)
    .bind(&req.emoji)
    .execute(&st.db)
    .await?
    .rows_affected();
    if removed > 0 {
        return Ok(Json(json!({"ok": true, "emoji": null})));
    }
    let ts = now();
    sqlx::query(
        "INSERT INTO ride_message_reactions (message_id, user_id, emoji, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$4)
         ON CONFLICT (message_id, user_id) DO UPDATE SET emoji = EXCLUDED.emoji, updated_at = EXCLUDED.updated_at",
    )
    .bind(mid)
    .bind(u.id)
    .bind(&req.emoji)
    .bind(&ts)
    .execute(&st.db)
    .await?;
    Ok(Json(json!({"ok": true, "emoji": req.emoji})))
}
