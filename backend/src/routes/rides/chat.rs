//! Group trip chat endpoints for rides.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult};
use crate::perms::Act;
use crate::state::AppState;
use crate::util::{notify_trip, now, ErrorKey};

use super::{load_ride, traffic_can};

// ---------------- Trip chat ----------------

async fn chat_access(st: &AppState, u: &CurrentUser, ride_id: i64) -> AppResult<(i64, bool)> {
    let r = load_ride(st, u.lang, ride_id).await?;
    let trip = r
        .trip_id
        .ok_or_else(|| bad(u.lang, ErrorKey::GroupChatNotApproved))?;
    let members: Vec<(i64, Option<i64>, String)> =
        sqlx::query_as("SELECT passenger_id, driver_id, status FROM rides WHERE trip_id=$1")
            .bind(trip)
            .fetch_all(&st.db)
            .await?;
    let is_member = members
        .iter()
        .any(|(p, d, s)| (*p == u.id && s != "cancelled") || *d == Some(u.id));
    if !is_member && !traffic_can(u, &r, Act::View) {
        return Err(forbidden(u.lang, ErrorKey::OnlyTripMembersCanViewChat));
    }
    let open = members
        .iter()
        .any(|(_, _, s)| s == "confirmed" || s == "in_progress");
    Ok((trip, open))
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
    let (trip, open) = chat_access(&st, &u, id).await?;
    let rows: Vec<(i64, Option<i64>, Option<String>, String, String)> = sqlx::query_as(
        "SELECT m.id, m.user_id, u.name, m.body, m.created_at FROM ride_messages m LEFT JOIN users u ON u.id=m.user_id
         WHERE m.trip_id=$1 AND m.id > $2 ORDER BY m.id LIMIT 300",
    )
    .bind(trip)
    .bind(q.after.unwrap_or(0))
    .fetch_all(&st.db)
    .await?;
    let members: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT u.name FROM rides r JOIN users u ON u.id = r.passenger_id OR u.id = r.driver_id
         WHERE r.trip_id=$1 AND r.status NOT IN ('cancelled')",
    )
    .bind(trip)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!({
        "open": open,
        "members": members.into_iter().map(|m| m.0).collect::<Vec<_>>(),
        "messages": rows.into_iter().map(|(id, uid, name, body, at)| json!({"id": id, "user_id": uid, "name": name, "body": body, "created_at": at, "mine": uid == Some(u.id)})).collect::<Vec<_>>(),
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
    let (trip, open) = chat_access(&st, &u, id).await?;
    if !open {
        return Err(bad(u.lang, ErrorKey::GroupChatClosed));
    }
    let body = req.body.trim();
    if body.is_empty() || body.chars().count() > 1000 {
        return Err(bad(u.lang, ErrorKey::InvalidMessageLength));
    }
    sqlx::query(
        "INSERT INTO ride_messages (trip_id, user_id, body, created_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(trip)
    .bind(u.id)
    .bind(body)
    .bind(now())
    .execute(&st.db)
    .await?;
    notify_trip(&st.db, trip).await;
    Ok(Json(json!({"ok": true})))
}
