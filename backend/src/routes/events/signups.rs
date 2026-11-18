//! Event signup, waitlist promotion, withdrawal, and organiser signup listing.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{can_see_event, can_see_signups, load_event};
use crate::audience::join_choices;
use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, audit_many, notify, now, random_token};

pub(super) async fn allocate_pass_code(
    conn: &mut sqlx::PgConnection,
    event_id: i64,
) -> AppResult<String> {
    for _ in 0..8 {
        let candidate = random_token(10);
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM event_signups
             WHERE event_id=$1 AND pass_code IS NOT NULL AND lower(pass_code)=lower($2)
             LIMIT 1",
        )
        .bind(event_id)
        .bind(&candidate)
        .fetch_optional(&mut *conn)
        .await?;
        if exists.is_none() {
            return Ok(candidate);
        }
    }
    Ok(random_token(10))
}

#[derive(Deserialize)]
pub(super) struct SignupIn {
    need_ride: Option<bool>,
    offer_drive: Option<bool>,
    offer_seats: Option<i64>,
    /// The group the member joins as. Required; filled in when only one fits.
    as_group_id: Option<i64>,
}

pub(super) async fn signup(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SignupIn>,
) -> AppResult<Json<Value>> {
    if !can_see_event(&st, &u, id).await? {
        return Err(forbidden(u.lang, ErrorKey::EventNotOpenToGroup));
    }
    if req.offer_drive.unwrap_or(false) && !u.can(Page::Drive, Act::Create) {
        return Err(bad(u.lang, ErrorKey::DriverRegistrationRequired));
    }
    let choices = join_choices(&st.db, &u, id).await?;
    // Lock the event row while counting seats and inserting the signup. Concurrent
    // signups then queue behind each other and the last seat cannot be handed out twice.
    let mut tx = st.db.begin().await?;
    let (capacity, status_now, deadline, starts_at): (i64, String, Option<String>, String) =
        sqlx::query_as(
            "SELECT capacity, status, signup_deadline, starts_at FROM events WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if status_now != "published" {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    let n = now()[..16].to_string();
    if deadline.as_deref().map(|d| d < n.as_str()).unwrap_or(false) || starts_at < n {
        return Err(bad(u.lang, ErrorKey::SignupClosed));
    }
    // Which group they come as: checked once the event is known to be open, so a closed
    // event says so rather than asking for a group.
    let as_group = match req.as_group_id {
        Some(g) if choices.iter().any(|(c, _)| *c == g) => g,
        Some(_) => return Err(bad(u.lang, ErrorKey::JoinAsNotAllowed)),
        None if choices.len() == 1 => choices[0].0,
        None => return Err(bad(u.lang, ErrorKey::JoinAsRequired)),
    };
    let (going,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM event_signups WHERE event_id=$1 AND status='going'")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    let mine: Option<(String,)> =
        sqlx::query_as("SELECT status FROM event_signups WHERE event_id=$1 AND user_id=$2")
            .bind(id)
            .bind(u.id)
            .fetch_optional(&mut *tx)
            .await?;
    let already_going = mine.as_ref().is_some_and(|(s,)| s == "going");
    let status = if already_going || capacity == 0 || going < capacity {
        "going"
    } else {
        "waitlist"
    };
    let pass_code = allocate_pass_code(&mut tx, id).await?;
    sqlx::query(
        "INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, created_at, as_group_id, pass_code) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
         ON CONFLICT(event_id, user_id) DO UPDATE SET status=excluded.status, need_ride=excluded.need_ride, offer_drive=excluded.offer_drive, offer_seats=excluded.offer_seats, as_group_id=excluded.as_group_id,
         pass_code = COALESCE(event_signups.pass_code, excluded.pass_code),
         created_at = CASE WHEN event_signups.status = 'cancelled' THEN excluded.created_at ELSE event_signups.created_at END",
    )
    .bind(id)
    .bind(u.id)
    .bind(status)
    .bind(req.need_ride.unwrap_or(false) as i64)
    .bind(req.offer_drive.unwrap_or(false) as i64)
    .bind(req.offer_seats.unwrap_or(0))
    .bind(now())
    .bind(as_group)
    .bind(&pass_code)
    .execute(&mut *tx)
    .await?;
    // A ride already booked for this event follows a changed "joining as".
    sqlx::query("UPDATE rides SET as_group_id=$1 WHERE event_id=$2 AND passenger_id=$3 AND status NOT IN ('completed','cancelled','no_show','alt_transport') AND is_proxy=0")
        .bind(as_group)
        .bind(id)
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "event.signup",
        "event",
        Some(id),
        format!(
            "{status}; as {}{}{}",
            choices
                .iter()
                .find(|(c, _)| *c == as_group)
                .map_or("?", |(_, n)| n.as_str()),
            if req.need_ride.unwrap_or(false) {
                "; needs a ride"
            } else {
                ""
            },
            if req.offer_drive.unwrap_or(false) {
                "; offers to drive"
            } else {
                ""
            },
        ),
    )
    .await;
    Ok(Json(json!({"ok": true, "status": status})))
}

/// Moves waitlisted members into every free place, oldest signup first. Unlimited
/// capacity (0) frees them all. Runs inside the caller's transaction, which must hold the
/// event row lock so concurrent signups and withdrawals see the same count.
pub(super) async fn promote_waitlist(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: i64,
    capacity: i64,
    event_status: &str,
) -> AppResult<Vec<i64>> {
    if event_status != "published" {
        return Ok(Vec::new());
    }
    let rows: Vec<(i64,)> = sqlx::query_as(
        "UPDATE event_signups SET status='going'
         WHERE id IN (
           SELECT id FROM event_signups WHERE event_id=$1 AND status='waitlist'
           ORDER BY created_at, id
           LIMIT CASE WHEN $2 = 0 THEN NULL
                      ELSE GREATEST(0, $2 - (SELECT COUNT(*) FROM event_signups WHERE event_id=$1 AND status='going'))
                 END
         )
         RETURNING user_id",
    )
    .bind(id)
    .bind(capacity)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows.into_iter().map(|(uid,)| uid).collect())
}

/// Audits and tells each promoted member they have a place. No actor: the place passes
/// on automatically.
pub(super) async fn notify_promoted(st: &AppState, id: i64, title: &str, promoted: &[i64]) {
    for &uid in promoted {
        audit(
            &st.db,
            None,
            "event.waitlist_promote",
            "event",
            Some(id),
            format!("user {uid} waitlist → going"),
        )
        .await;
        let zh = format!("您已获得「{title}」的名额");
        let en = format!("You now have a spot in \"{title}\"");
        notify(
            &st.db,
            uid,
            ("候补成功", "Off the waitlist"),
            (&zh, &en),
            &format!("/events/{id}"),
        )
        .await;
    }
}

/// 乘客无法出席: cancel signup → release seat → promote waitlist → cancel linked rides → tell driver & organiser.
pub(super) async fn withdraw(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    // Same event lock as `signup`: concurrent withdrawals (and signups) queue behind each
    // other, so each sees the seats the others freed or took and the same waitlisted
    // member is never promoted twice while others are left waiting.
    let mut tx = st.db.begin().await?;
    let (capacity, event_status): (i64, String) =
        sqlx::query_as("SELECT capacity, status FROM events WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    let withdrew = sqlx::query(
        "UPDATE event_signups SET status='cancelled' WHERE event_id=$1 AND user_id=$2 AND status <> 'cancelled'",
    )
    .bind(id)
    .bind(u.id)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        > 0;
    // Fill every free place from the waitlist, in order, rather than exactly one per
    // withdrawal; this also heals any place left open earlier.
    let promoted = promote_waitlist(&mut tx, id, capacity, &event_status).await?;
    tx.commit().await?;
    notify_promoted(&st, id, &e.title, &promoted).await;
    let rides: Vec<(i64, Option<i64>, Option<i64>)> = sqlx::query_as(
        // Only their own ride: a visitor they invited may still come, and the host can cancel
        // that ride from the ride page.
        "UPDATE rides SET status='cancelled', needs_attention=0, attention_reason='乘客取消活动报名', updated_at=$3
         WHERE event_id=$1 AND passenger_id=$2 AND is_proxy=0
           AND status IN ('pending_driver','pending_passenger','pending_approval','pending_driver_confirm','confirmed')
         RETURNING id, driver_id, trip_id",
    )
    .bind(id)
    .bind(u.id)
    .bind(now())
    .fetch_all(&st.db)
    .await?;
    let ride_ids: Vec<i64> = rides.iter().map(|(rid, _, _)| *rid).collect();
    audit_many(
        &st.db,
        Some(u.id),
        "ride.cancel",
        "ride",
        &ride_ids,
        &format!("withdrew from event {id}"),
    )
    .await?;
    for (rid, d, t) in rides {
        if let Some(d) = d {
            st.routing.invalidate(d, t.unwrap_or(rid)).await;
            let body = format!("{} · {}", u.name, e.title);
            notify(
                &st.db,
                d,
                (
                    "乘客已取消，座位已释放",
                    "Passenger cancelled, seat released",
                ),
                (&body, &body),
                "/drive",
            )
            .await;
        }
    }
    if withdrew && e.created_by != u.id {
        let body = format!("{} · {}", u.name, e.title);
        notify(
            &st.db,
            e.created_by,
            ("有会员取消报名", "A member withdrew their signup"),
            (&body, &body),
            &format!("/events/{id}"),
        )
        .await;
    }
    audit(&st.db, Some(u.id), "event.withdraw", "event", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

/// Signup list is visible only to the organiser (不公开全体报名名单).
pub(super) async fn list_signups(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_see_signups(&u, e.created_by) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanViewSignups));
    }
    type SignupRow = (
        i64,
        i64,
        String,
        String,
        i64,
        i64,
        i64,
        Option<i64>,
        Option<String>,
        Option<String>,
    );
    let rows: Vec<SignupRow> = sqlx::query_as(
        "SELECT s.id, s.user_id, u.name, s.status, s.need_ride, s.offer_drive, s.offer_seats, s.attended,
            (SELECT r.status FROM rides r WHERE r.event_id = s.event_id AND r.passenger_id = s.user_id AND r.is_proxy = 0 AND r.status != 'cancelled'
             ORDER BY CASE WHEN r.leg='outbound' THEN 0 ELSE 1 END, r.id DESC LIMIT 1),
            (SELECT name FROM groups WHERE id = s.as_group_id)
         FROM event_signups s JOIN users u ON u.id = s.user_id WHERE s.event_id=$1 ORDER BY s.status, s.created_at",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(sid, uid, name, status, nr, od, os, att, ride, as_group)| json!({
        "id": sid, "user_id": uid, "name": name, "status": status, "need_ride": nr == 1, "offer_drive": od == 1,
        "offer_seats": os, "attended": att, "ride_status": ride, "as_group": as_group,
    })).collect::<Vec<_>>())))
}
