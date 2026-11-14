//! Notifications, announcements, help center, Grab reimbursements, monthly reports & badges, profile.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{
    audit, check_name, check_phone, fmt_rm, notify, notify_perm, now, require, require_one, t, Lang,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", axum::routing::put(update_profile))
        .route("/notifications", get(list_notifications))
        .route("/notifications/read-all", post(read_all))
        .route("/notifications/{id}/read", post(read_one))
        .route(
            "/announcements",
            get(list_announcements).post(create_announcement),
        )
        .route("/announcements/{id}", delete(delete_announcement))
        .route("/help", get(list_tickets).post(create_ticket))
        .route("/help/{id}", get(get_ticket))
        .route("/help/{id}/replies", post(reply_ticket))
        .route("/help/{id}/close", post(close_ticket))
        .route(
            "/reimbursements",
            get(list_reimbursements).post(create_reimbursement),
        )
        .route(
            "/reimbursements/{id}",
            get(get_reimbursement).put(resubmit_reimbursement),
        )
        .route("/reimbursements/{id}/review", post(review_reimbursement))
        .route("/reimbursements/{id}/pay", post(pay_reimbursement))
        .route("/reports/monthly", get(monthly_report))
        .route("/badges", get(my_badges))
}

// ---------------- Profile ----------------

#[derive(Deserialize)]
struct ProfileIn {
    /// Roughly every field is optional; callers may only send what changes,
    /// e.g. { "lang": "en" } from the language switcher.
    name: Option<String>,
    phone: Option<String>,
    photo_file_id: Option<i64>,
    badges_opt_out: Option<bool>,
    lang: Option<String>,
    readiness_prompt_mins: Option<i64>,
    auto_lock_mins: Option<i64>,
}

async fn update_profile(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ProfileIn>,
) -> AppResult<Json<Value>> {
    if let Some(ref name) = req.name {
        check_name(u.lang, name)?;
    }
    if let Some(ref phone) = req.phone {
        check_phone(u.lang, phone)?;
    }
    if let Some(ref lang) = req.lang {
        if lang != "zh" && lang != "en" {
            return Err(bad(u.lang, ErrorKey::LanguageInvalid));
        }
    }
    if let Some(m) = req.auto_lock_mins {
        if !(0..=1440).contains(&m) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    if let Some(f) = req.photo_file_id {
        let (owner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=$1")
            .bind(f)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
        if owner != u.id {
            return Err(forbidden(u.lang, ErrorKey::InvalidPhoto));
        }
    }
    sqlx::query(
        "UPDATE users SET
            name=COALESCE($1, name),
            phone=COALESCE($2, phone),
            photo_file_id=COALESCE($3, photo_file_id),
            badges_opt_out=COALESCE($4, badges_opt_out),
            lang=COALESCE($5, lang),
            readiness_prompt_mins=COALESCE($6, readiness_prompt_mins),
            auto_lock_mins=COALESCE($7, auto_lock_mins)
         WHERE id=$8",
    )
    .bind(req.name.as_deref().map(str::trim))
    .bind(req.phone.as_deref().map(str::trim))
    .bind(req.photo_file_id)
    .bind(req.badges_opt_out.map(|b| b as i64))
    .bind(req.lang.as_deref())
    .bind(req.readiness_prompt_mins)
    .bind(req.auto_lock_mins)
    .bind(u.id)
    .execute(&st.db)
    .await?;
    // Which fields were sent, not their values: the log is read by staff.
    let changed: Vec<&str> = [
        ("name", req.name.is_some()),
        ("phone", req.phone.is_some()),
        ("photo", req.photo_file_id.is_some()),
        ("badges_opt_out", req.badges_opt_out.is_some()),
        ("lang", req.lang.is_some()),
        ("auto_lock", req.auto_lock_mins.is_some()),
    ]
    .into_iter()
    .filter_map(|(k, sent)| sent.then_some(k))
    .collect();
    audit(
        &st.db,
        Some(u.id),
        "profile.update",
        "user",
        Some(u.id),
        changed.join(", "),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

// ---------------- Notifications ----------------

#[derive(Deserialize)]
struct NotifQuery {
    before: Option<i64>,
    limit: Option<i64>,
}

async fn list_notifications(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<NotifQuery>,
) -> AppResult<Json<Value>> {
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    if q.before.is_some() || q.limit.is_some() {
        let sql = if q.before.is_some() {
            "SELECT id, title, body, link, read_at, created_at FROM notifications WHERE user_id=$1 AND id < $2 ORDER BY id DESC LIMIT $3"
        } else {
            "SELECT id, title, body, link, read_at, created_at FROM notifications WHERE user_id=$1 ORDER BY id DESC LIMIT $2"
        };
        let mut rows: Vec<(i64, String, String, String, Option<String>, String)> =
            if let Some(b) = q.before {
                sqlx::query_as(sql)
                    .bind(u.id)
                    .bind(b)
                    .bind(limit + 1)
                    .fetch_all(&st.db)
                    .await?
            } else {
                sqlx::query_as(sql)
                    .bind(u.id)
                    .bind(limit + 1)
                    .fetch_all(&st.db)
                    .await?
            };
        let more = rows.len() as i64 > limit;
        rows.truncate(limit as usize);
        let next_before = if more { rows.last().map(|r| r.0) } else { None };
        return Ok(Json(json!({
            "items": rows.into_iter().map(|(id, t, b, l, r, c)| json!({"id": id, "title": t, "body": b, "link": l, "read": r.is_some(), "created_at": c})).collect::<Vec<_>>(),
            "next_before": next_before
        })));
    }

    let rows: Vec<(i64, String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT id, title, body, link, read_at, created_at FROM notifications WHERE user_id=$1 ORDER BY id DESC LIMIT 100",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, t, b, l, r, c)| json!({"id": id, "title": t, "body": b, "link": l, "read": r.is_some(), "created_at": c})).collect::<Vec<_>>())))
}

async fn read_all(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    sqlx::query("UPDATE notifications SET read_at=$1 WHERE user_id=$2 AND read_at IS NULL")
        .bind(now())
        .bind(u.id)
        .execute(&st.db)
        .await?;
    Ok(Json(json!({"ok": true})))
}

async fn read_one(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    sqlx::query(
        "UPDATE notifications SET read_at=$1 WHERE id=$2 AND user_id=$3 AND read_at IS NULL",
    )
    .bind(now())
    .bind(id)
    .bind(u.id)
    .execute(&st.db)
    .await?;
    Ok(Json(json!({"ok": true})))
}

// ---------------- Announcements ----------------

async fn list_announcements(State(st): State<AppState>, _u: CurrentUser) -> AppResult<Json<Value>> {
    let rows: Vec<(i64, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT a.id, a.title, a.body, a.pinned, u.name, a.created_at FROM announcements a JOIN users u ON u.id=a.created_by ORDER BY a.pinned DESC, a.id DESC LIMIT 30",
    )
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, t, b, p, by, c)| json!({"id": id, "title": t, "body": b, "pinned": p == 1, "by": by, "created_at": c})).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct AnnIn {
    title: String,
    body: String,
    pinned: Option<bool>,
    notify_all: Option<bool>,
}

async fn create_announcement(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<AnnIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAnnouncements, Act::Create)?;
    if req.title.trim().is_empty() || req.body.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::TitleAndContentRequired));
    }
    let (id,): (i64,) = sqlx::query_as("INSERT INTO announcements (title, body, pinned, created_by, created_at) VALUES ($1,$2,$3,$4,$5) RETURNING id")
        .bind(req.title.trim())
        .bind(req.body.trim())
        .bind(req.pinned.unwrap_or(false) as i64)
        .bind(u.id)
        .bind(now())
        .fetch_one(&st.db)
        .await?;
    if req.notify_all.unwrap_or(false) {
        let users: Vec<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE status='active'")
            .fetch_all(&st.db)
            .await?;
        let title = req.title.trim();
        for (uid,) in users {
            notify(
                &st.db,
                uid,
                ("新公告", "New announcement"),
                (title, title),
                "/",
            )
            .await;
        }
    }
    audit(
        &st.db,
        Some(u.id),
        "announcement.create",
        "announcement",
        Some(id),
        req.title.trim(),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

async fn delete_announcement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAnnouncements, Act::Delete)?;
    let gone: Option<(String,)> =
        sqlx::query_as("DELETE FROM announcements WHERE id=$1 RETURNING title")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
    if let Some((title,)) = gone {
        audit(
            &st.db,
            Some(u.id),
            "announcement.delete",
            "announcement",
            Some(id),
            title,
        )
        .await;
    }
    Ok(Json(json!({"ok": true})))
}

// ---------------- Help center ----------------

#[derive(Deserialize)]
struct HelpQuery {
    all: Option<bool>,
}

async fn list_tickets(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<HelpQuery>,
) -> AppResult<Json<Value>> {
    let all = q.all.unwrap_or(false) && u.can(Page::AdminHelp, Act::View);
    if !all {
        require(&u, Page::Help, Act::View)?;
    }
    let rows: Vec<(i64, String, String, Option<i64>, String, String, String)> = sqlx::query_as(
        "SELECT t.id, t.subject, t.status, t.ride_id, u.name, t.created_at, t.updated_at FROM help_tickets t JOIN users u ON u.id=t.user_id
         WHERE ($1 OR t.user_id=$2) ORDER BY CASE t.status WHEN 'open' THEN 0 WHEN 'answered' THEN 1 ELSE 2 END, t.updated_at DESC LIMIT 200",
    )
    .bind(all)
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, s, status, r, name, c, upd)| json!({"id": id, "subject": s, "status": status, "ride_id": r, "user_name": name, "created_at": c, "updated_at": upd})).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct TicketIn {
    subject: String,
    body: String,
    ride_id: Option<i64>,
}

async fn create_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<TicketIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Help, Act::Create)?;
    if req.subject.trim().is_empty() || req.body.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::QuestionAndDetailsRequired));
    }
    if let Some(r) = req.ride_id {
        let (p, d): (i64, Option<i64>) =
            sqlx::query_as("SELECT passenger_id, driver_id FROM rides WHERE id=$1")
                .bind(r)
                .fetch_optional(&st.db)
                .await?
                .ok_or(not_found(u.lang))?;
        if p != u.id && d != Some(u.id) {
            return Err(forbidden(u.lang, ErrorKey::OnlyLinkOwnRides));
        }
    }
    let (id,): (i64,) = sqlx::query_as("INSERT INTO help_tickets (user_id, ride_id, subject, created_at, updated_at) VALUES ($1,$2,$3,$4,$5) RETURNING id")
        .bind(u.id)
        .bind(req.ride_id)
        .bind(req.subject.trim())
        .bind(now())
        .bind(now())
        .fetch_one(&st.db)
        .await?;
    sqlx::query(
        "INSERT INTO help_replies (ticket_id, user_id, body, created_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(id)
    .bind(u.id)
    .bind(req.body.trim())
    .bind(now())
    .execute(&st.db)
    .await?;
    notify_perm(
        &st.db,
        Page::AdminHelp,
        Act::Update,
        ("帮助中心：新的求助", "Help center: new request"),
        (
            &format!("{} · {}", u.name, req.subject.trim()),
            &format!("{} · {}", u.name, req.subject.trim()),
        ),
        &format!("/help/{id}"),
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "help.create",
        "help_ticket",
        Some(id),
        req.subject.trim(),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

/// Load a help request for its owner, or for 帮助台 staff who may do `staff_act` on it.
async fn ticket_owner(
    st: &AppState,
    u: &CurrentUser,
    id: i64,
    staff_act: Act,
) -> AppResult<(i64, String, String, Option<i64>)> {
    let row: (i64, String, String, Option<i64>) =
        sqlx::query_as("SELECT user_id, subject, status, ride_id FROM help_tickets WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
    if row.0 != u.id && !u.can(Page::AdminHelp, staff_act) {
        return Err(forbidden(u.lang, ErrorKey::Forbidden));
    }
    Ok(row)
}

async fn get_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let (owner, subject, status, ride) = ticket_owner(&st, &u, id, Act::View).await?;
    let replies: Vec<(i64, i64, String, String, String)> = sqlx::query_as(
        "SELECT r.id, r.user_id, u.name, r.body, r.created_at FROM help_replies r JOIN users u ON u.id=r.user_id WHERE r.ticket_id=$1 ORDER BY r.id",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!({
        "id": id, "subject": subject, "status": status, "ride_id": ride, "user_id": owner,
        "replies": replies.into_iter().map(|(rid, uid, name, body, at)| json!({"id": rid, "user_id": uid, "name": name, "body": body, "created_at": at, "staff": uid != owner})).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
struct ReplyIn {
    body: String,
}

async fn reply_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReplyIn>,
) -> AppResult<Json<Value>> {
    let (owner, subject, status, _) = ticket_owner(&st, &u, id, Act::Update).await?;
    if owner == u.id && !u.can(Page::AdminHelp, Act::Update) {
        require(&u, Page::Help, Act::Update)?;
    }
    if status == "closed" {
        return Err(bad(u.lang, ErrorKey::RequestAlreadyClosed));
    }
    if req.body.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::ReplyContentRequired));
    }
    sqlx::query(
        "INSERT INTO help_replies (ticket_id, user_id, body, created_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(id)
    .bind(u.id)
    .bind(req.body.trim())
    .bind(now())
    .execute(&st.db)
    .await?;
    let new_status = if u.id == owner { "open" } else { "answered" };
    sqlx::query("UPDATE help_tickets SET status=$1, updated_at=$2 WHERE id=$3")
        .bind(new_status)
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    if u.id == owner {
        notify_perm(
            &st.db,
            Page::AdminHelp,
            Act::Update,
            ("帮助中心：会员回复", "Help center: member reply"),
            (&subject, &subject),
            &format!("/help/{id}"),
        )
        .await;
    } else {
        notify(
            &st.db,
            owner,
            ("帮助中心已回复", "Help center has replied"),
            (&subject, &subject),
            &format!("/help/{id}"),
        )
        .await;
    }
    // The reply text stays in the ticket; the log records who answered and when.
    audit(
        &st.db,
        Some(u.id),
        "help.reply",
        "help_ticket",
        Some(id),
        if u.id == owner {
            "by member"
        } else {
            "by staff"
        },
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn close_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let (owner, ..) = ticket_owner(&st, &u, id, Act::Update).await?;
    if owner == u.id && !u.can(Page::AdminHelp, Act::Update) {
        require(&u, Page::Help, Act::Update)?;
    }
    sqlx::query("UPDATE help_tickets SET status='closed', updated_at=$1 WHERE id=$2")
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "help.close",
        "help_ticket",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

// ---------------- Grab reimbursements ----------------
// pending → (交通组) approved | returned | rejected → (财务) paid

#[derive(sqlx::FromRow)]
struct ReimbRow {
    id: i64,
    user_id: i64,
    user_name: String,
    ride_id: Option<i64>,
    ride_at: Option<String>,
    amount_cents: i64,
    description: String,
    receipt_file_id: i64,
    payee_name: String,
    payee_phone: String,
    status: String,
    review_note: String,
    reviewer: Option<String>,
    payee_verified: i64,
    paid_ref: String,
    paid_at: Option<String>,
    created_at: String,
}

const REIMB_SELECT: &str = "SELECT r.id, r.user_id, u.name AS user_name, r.ride_id, rd.pickup_at AS ride_at, r.amount_cents, r.description,
    r.receipt_file_id, r.payee_name, r.payee_phone, r.status, r.review_note, rv.name AS reviewer, r.payee_verified, r.paid_ref, r.paid_at, r.created_at
    FROM reimbursements r JOIN users u ON u.id=r.user_id LEFT JOIN rides rd ON rd.id=r.ride_id LEFT JOIN users rv ON rv.id=r.reviewed_by";

fn reimb_json(r: &ReimbRow, u: &CurrentUser) -> Value {
    let finance = u.can(Page::AdminReimbPay, Act::View);
    let own = r.user_id == u.id;
    json!({
        "id": r.id, "user": {"id": r.user_id, "name": r.user_name}, "ride_id": r.ride_id, "ride_at": r.ride_at,
        "amount_cents": r.amount_cents, "description": r.description, "receipt_file_id": r.receipt_file_id,
        // Payee details only to the claimant and finance.
        "payee_name": if own || finance { Some(r.payee_name.clone()) } else { None },
        "payee_phone": if own || finance { Some(r.payee_phone.clone()) } else { None },
        "status": r.status, "review_note": r.review_note, "reviewer": r.reviewer, "payee_verified": r.payee_verified == 1,
        "paid_ref": if own || finance { Some(r.paid_ref.clone()) } else { None }, "paid_at": r.paid_at, "created_at": r.created_at,
    })
}

#[derive(Deserialize)]
struct ReimbQuery {
    scope: Option<String>, // mine | review | payout
    page: Option<i64>,
    limit: Option<i64>,
}

async fn list_reimbursements(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<ReimbQuery>,
) -> AppResult<Json<Value>> {
    let is_paginated = q.page.is_some() || q.limit.is_some();
    let page_val = q.page.unwrap_or(1).max(1);
    let limit_val = q.limit.unwrap_or(25).clamp(1, 100);
    let offset = (page_val - 1) * limit_val;

    match q.scope.as_deref() {
        Some("review") => {
            require(&u, Page::AdminReimbReview, Act::View)?;
            if is_paginated {
                let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reimbursements r")
                    .fetch_one(&st.db)
                    .await?;
                let rows = sqlx::query_as::<_, ReimbRow>(&format!(
                    "{REIMB_SELECT} ORDER BY CASE r.status WHEN 'pending' THEN 0 ELSE 1 END, r.id DESC LIMIT $1 OFFSET $2"
                ))
                .bind(limit_val)
                .bind(offset)
                .fetch_all(&st.db)
                .await?;
                let total_pages = ((total.0 as f64) / (limit_val as f64)).ceil() as i64;
                Ok(Json(json!({
                    "items": rows.iter().map(|r| reimb_json(r, &u)).collect::<Vec<_>>(),
                    "total": total.0,
                    "page": page_val,
                    "page_size": limit_val,
                    "total_pages": total_pages.max(1),
                })))
            } else {
                let rows = sqlx::query_as::<_, ReimbRow>(&format!("{REIMB_SELECT} ORDER BY CASE r.status WHEN 'pending' THEN 0 ELSE 1 END, r.id DESC LIMIT 200"))
                    .fetch_all(&st.db)
                    .await?;
                Ok(Json(json!(rows
                    .iter()
                    .map(|r| reimb_json(r, &u))
                    .collect::<Vec<_>>())))
            }
        }
        Some("payout") => {
            require(&u, Page::AdminReimbPay, Act::View)?;
            if is_paginated {
                let total: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM reimbursements r WHERE r.status IN ('approved','paid')",
                )
                .fetch_one(&st.db)
                .await?;
                let rows = sqlx::query_as::<_, ReimbRow>(&format!(
                    "{REIMB_SELECT} WHERE r.status IN ('approved','paid') ORDER BY CASE r.status WHEN 'approved' THEN 0 ELSE 1 END, r.id DESC LIMIT $1 OFFSET $2"
                ))
                .bind(limit_val)
                .bind(offset)
                .fetch_all(&st.db)
                .await?;
                let total_pages = ((total.0 as f64) / (limit_val as f64)).ceil() as i64;
                Ok(Json(json!({
                    "items": rows.iter().map(|r| reimb_json(r, &u)).collect::<Vec<_>>(),
                    "total": total.0,
                    "page": page_val,
                    "page_size": limit_val,
                    "total_pages": total_pages.max(1),
                })))
            } else {
                let rows = sqlx::query_as::<_, ReimbRow>(&format!("{REIMB_SELECT} WHERE r.status IN ('approved','paid') ORDER BY CASE r.status WHEN 'approved' THEN 0 ELSE 1 END, r.id DESC LIMIT 200"))
                    .fetch_all(&st.db)
                    .await?;
                Ok(Json(json!(rows
                    .iter()
                    .map(|r| reimb_json(r, &u))
                    .collect::<Vec<_>>())))
            }
        }
        _ => {
            require(&u, Page::Reimbursements, Act::View)?;
            if is_paginated {
                let total: (i64,) =
                    sqlx::query_as("SELECT COUNT(*) FROM reimbursements r WHERE r.user_id=$1")
                        .bind(u.id)
                        .fetch_one(&st.db)
                        .await?;
                let rows = sqlx::query_as::<_, ReimbRow>(&format!(
                    "{REIMB_SELECT} WHERE r.user_id=$1 ORDER BY r.id DESC LIMIT $2 OFFSET $3"
                ))
                .bind(u.id)
                .bind(limit_val)
                .bind(offset)
                .fetch_all(&st.db)
                .await?;
                let total_pages = ((total.0 as f64) / (limit_val as f64)).ceil() as i64;
                Ok(Json(json!({
                    "items": rows.iter().map(|r| reimb_json(r, &u)).collect::<Vec<_>>(),
                    "total": total.0,
                    "page": page_val,
                    "page_size": limit_val,
                    "total_pages": total_pages.max(1),
                })))
            } else {
                let rows = sqlx::query_as::<_, ReimbRow>(&format!(
                    "{REIMB_SELECT} WHERE r.user_id=$1 ORDER BY r.id DESC LIMIT 100"
                ))
                .bind(u.id)
                .fetch_all(&st.db)
                .await?;
                Ok(Json(json!(rows
                    .iter()
                    .map(|r| reimb_json(r, &u))
                    .collect::<Vec<_>>())))
            }
        }
    }
}

async fn load_reimb(st: &AppState, lang: Lang, id: i64) -> AppResult<ReimbRow> {
    sqlx::query_as::<_, ReimbRow>(&format!("{REIMB_SELECT} WHERE r.id=$1"))
        .bind(id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(lang))
}

async fn get_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_reimb(&st, u.lang, id).await?;
    if r.user_id != u.id {
        require_one(
            &u,
            &[
                (Page::AdminReimbReview, Act::View),
                (Page::AdminReimbPay, Act::View),
            ],
        )?;
    }
    Ok(Json(reimb_json(&r, &u)))
}

#[derive(Deserialize)]
struct ReimbIn {
    ride_id: Option<i64>,
    amount_cents: i64,
    description: String,
    receipt_file_id: i64,
    payee_name: String,
    payee_phone: String,
}

async fn validate_reimb(st: &AppState, u: &CurrentUser, req: &ReimbIn) -> AppResult<()> {
    if req.amount_cents <= 0 || req.amount_cents > 50_000 {
        return Err(bad(u.lang, ErrorKey::AmountRangeRM001to500));
    }
    if req.payee_name.trim().is_empty() || req.payee_phone.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::PayeeNameAndPhoneRequired));
    }
    let (owner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=$1")
        .bind(req.receipt_file_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or_else(|| bad(u.lang, ErrorKey::ReceiptRequired))?;
    if owner != u.id {
        return Err(forbidden(u.lang, ErrorKey::InvalidReceipt));
    }
    if let Some(rid) = req.ride_id {
        let (p,): (i64,) = sqlx::query_as("SELECT passenger_id FROM rides WHERE id=$1")
            .bind(rid)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
        if p != u.id {
            return Err(forbidden(u.lang, ErrorKey::OnlyLinkOwnRides));
        }
    }
    Ok(())
}

async fn create_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ReimbIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Reimbursements, Act::Create)?;
    validate_reimb(&st, &u, &req).await?;
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO reimbursements (user_id, ride_id, amount_cents, description, receipt_file_id, payee_name, payee_phone, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
    )
    .bind(u.id)
    .bind(req.ride_id)
    .bind(req.amount_cents)
    .bind(req.description.trim())
    .bind(req.receipt_file_id)
    .bind(req.payee_name.trim())
    .bind(req.payee_phone.trim())
    .bind(now())
    .bind(now())
    .fetch_one(&st.db)
    .await?;
    notify_perm(
        &st.db,
        Page::AdminReimbReview,
        Act::Update,
        ("新的 Grab 报销申请", "New Grab reimbursement request"),
        (
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
        ),
        "/admin/reimbursements",
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.create",
        "reimbursement",
        Some(id),
        fmt_rm(req.amount_cents),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

async fn resubmit_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReimbIn>,
) -> AppResult<Json<Value>> {
    let r = load_reimb(&st, u.lang, id).await?;
    if r.user_id != u.id || r.status != "returned" {
        return Err(bad(u.lang, ErrorKey::OnlyReturnedReimbCanBeResubmitted));
    }
    require(&u, Page::Reimbursements, Act::Update)?;
    validate_reimb(&st, &u, &req).await?;
    sqlx::query("UPDATE reimbursements SET ride_id=$1, amount_cents=$2, description=$3, receipt_file_id=$4, payee_name=$5, payee_phone=$6, status='pending', payee_verified=0, updated_at=$7 WHERE id=$8")
        .bind(req.ride_id)
        .bind(req.amount_cents)
        .bind(req.description.trim())
        .bind(req.receipt_file_id)
        .bind(req.payee_name.trim())
        .bind(req.payee_phone.trim())
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    notify_perm(
        &st.db,
        Page::AdminReimbReview,
        Act::Update,
        ("报销申请已补资料", "Reimbursement updated and resubmitted"),
        (
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
        ),
        "/admin/reimbursements",
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.resubmit",
        "reimbursement",
        Some(id),
        fmt_rm(req.amount_cents),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct ReviewIn {
    action: String, // approve | return | reject
    note: Option<String>,
}

async fn review_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReviewIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbReview, Act::Update)?;
    let r = load_reimb(&st, u.lang, id).await?;
    if r.status != "pending" {
        return Err(bad(u.lang, ErrorKey::ReimbAlreadyReviewed));
    }
    if r.user_id == u.id {
        return Err(forbidden(u.lang, ErrorKey::CannotReviewOwnRequest));
    }
    let note = req.note.unwrap_or_default();
    let status = match req.action.as_str() {
        "approve" => "approved",
        "return" => "returned",
        "reject" => "rejected",
        _ => return Err(bad(u.lang, ErrorKey::InvalidAction)),
    };
    if status != "approved" && note.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::ReasonRequired));
    }
    sqlx::query(
        "UPDATE reimbursements SET status=$1, review_note=$2, reviewed_by=$3, updated_at=$4 WHERE id=$5",
    )
    .bind(status)
    .bind(&note)
    .bind(u.id)
    .bind(now())
    .bind(id)
    .execute(&st.db)
    .await?;
    let title = match status {
        "approved" => ("报销申请已批准", "Reimbursement approved"),
        "returned" => ("报销申请需补资料", "Reimbursement needs more info"),
        _ => ("报销申请未获批准", "Reimbursement not approved"),
    };
    notify(
        &st.db,
        r.user_id,
        title,
        (
            &format!("{} {}", fmt_rm(r.amount_cents), note),
            &format!("{} {}", fmt_rm(r.amount_cents), note),
        ),
        &format!("/reimbursements/{id}"),
    )
    .await;
    if status == "approved" {
        notify_perm(
            &st.db,
            Page::AdminReimbPay,
            Act::Update,
            ("有报销待付款", "Reimbursement awaiting payment"),
            (
                &format!("{} · {}", r.user_name, fmt_rm(r.amount_cents)),
                &format!("{} · {}", r.user_name, fmt_rm(r.amount_cents)),
            ),
            "/admin/reimbursements",
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        &format!("reimbursement.{}", req.action),
        "reimbursement",
        Some(id),
        note,
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct PayIn {
    payee_verified: bool,
    paid_ref: String,
}

/// 财务核对收款人 → 转账并记录 (manual payout via church-approved bank / TNG channel).
async fn pay_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<PayIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbPay, Act::Update)?;
    let r = load_reimb(&st, u.lang, id).await?;
    if r.status != "approved" {
        return Err(bad(u.lang, ErrorKey::OnlyApprovedReimbCanBePaid));
    }
    if r.user_id == u.id {
        return Err(forbidden(u.lang, ErrorKey::CannotPayOwnRequest));
    }
    if !req.payee_verified {
        return Err(bad(u.lang, ErrorKey::PayeeVerificationRequired));
    }
    if req.paid_ref.trim().len() < 4 {
        return Err(bad(u.lang, ErrorKey::TxnRefRequired));
    }
    sqlx::query("UPDATE reimbursements SET status='paid', payee_verified=1, paid_ref=$1, paid_by=$2, paid_at=$3, updated_at=$4 WHERE id=$5")
        .bind(req.paid_ref.trim())
        .bind(u.id)
        .bind(now())
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    notify(
        &st.db,
        r.user_id,
        ("报销款项已转账", "Reimbursement paid"),
        (
            &format!(
                "{} · 交易编号 {}",
                fmt_rm(r.amount_cents),
                req.paid_ref.trim()
            ),
            &format!(
                "{} · Transaction ref {}",
                fmt_rm(r.amount_cents),
                req.paid_ref.trim()
            ),
        ),
        &format!("/reimbursements/{id}"),
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.pay",
        "reimbursement",
        Some(id),
        req.paid_ref.trim(),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

// ---------------- Monthly report & badges ----------------

#[derive(Deserialize)]
struct ReportQuery {
    month: Option<String>, // YYYY-MM
    user_id: Option<i64>,
    /// "driver" | "passenger" | "all" — which side of rides to report (default: all)
    r#type: Option<String>,
    /// "monthly" | "quarterly" | "yearly" — report period granularity (default: monthly)
    range: Option<String>,
}

pub async fn compute_badges(st: &AppState, user_id: i64, lang: Lang) -> AppResult<Vec<Value>> {
    let (opt_out, carpool): (i64, i64) =
        sqlx::query_as("SELECT badges_opt_out, carpool_enabled FROM users WHERE id=$1")
            .bind(user_id)
            .fetch_one(&st.db)
            .await?;
    if opt_out == 1 {
        return Ok(vec![]);
    }
    let (night,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM rides WHERE driver_id=$1 AND status='completed' AND (substr(pickup_at,12,2) >= '21' OR substr(pickup_at,12,2) < '05')",
    )
    .bind(user_id)
    .fetch_one(&st.db)
    .await?;
    let (months,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT substr(pickup_at,1,7)) FROM rides WHERE driver_id=$1 AND status='completed' AND pickup_at >= to_char(LOCALTIMESTAMP - interval '6 months','YYYY-MM-DD')",
    )
    .bind(user_id)
    .fetch_one(&st.db)
    .await?;
    let (total,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM rides WHERE driver_id=$1 AND status='completed'")
            .bind(user_id)
            .fetch_one(&st.db)
            .await?;
    let (shared,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM (SELECT trip_id FROM rides WHERE driver_id=$1 AND status='completed' AND trip_id IS NOT NULL GROUP BY trip_id HAVING COUNT(*) >= 2)",
    )
    .bind(user_id)
    .fetch_one(&st.db)
    .await?;
    // Criteria are public (徽章获得条件应公开透明).
    let defs = [
        (
            "night",
            t(lang, "宵", "N"),
            t(lang, "宵夜王", "Night Owl"),
            t(
                lang,
                "完成 3 次或以上晚上 9 点后的接送",
                "Complete 3 or more rides after 9 pm",
            ),
            night,
            3,
        ),
        (
            "regular",
            t(lang, "勤", "R"),
            t(lang, "常驻司机", "Resident Driver"),
            t(
                lang,
                "过去 6 个月中至少 3 个月有完成接送",
                "Completed rides in at least 3 of the past 6 months",
            ),
            months,
            3,
        ),
        (
            "carpool",
            t(lang, "爱", "C"),
            t(lang, "爱心同行者", "Caring Companion"),
            t(
                lang,
                "开启共乘并完成 2 趟或以上多人同车接送",
                "Enable carpool and complete 2 or more rides with multiple passengers",
            ),
            if carpool == 1 { shared } else { 0 },
            2,
        ),
    ];
    let _ = total;
    Ok(defs
        .iter()
        .map(|(key, ch, name, rule, have, need)| json!({"key": key, "char": ch, "name": name, "rule": rule, "progress": have, "target": need, "earned": have >= need}))
        .collect())
}

async fn my_badges(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    Ok(Json(json!(compute_badges(&st, u.id, u.lang).await?)))
}

/// 个人月报 — only the member themself or an admin (获授权人员) may view.
/// Supports driver/passenger filter and monthly/quarterly/yearly ranges.
async fn monthly_report(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<ReportQuery>,
) -> AppResult<Json<Value>> {
    let target = q.user_id.unwrap_or(u.id);
    if target != u.id {
        require(&u, Page::AdminReports, Act::View)?;
    } else {
        require(&u, Page::Report, Act::View)?;
    }
    let rtype = q.r#type.as_deref().unwrap_or("all");
    let range = q.range.as_deref().unwrap_or("monthly");
    let month = q.month.unwrap_or_else(|| now()[..7].to_string());
    if month.len() != 7 {
        return Err(bad(u.lang, ErrorKey::MonthFormatInvalid));
    }

    // Compute period bounds from month (YYYY-MM) and range
    let y: i32 = month[..4].parse().unwrap_or(0);
    let m: i32 = month[5..7].parse().unwrap_or(1);
    let (start, end): (String, String) = match range {
        "quarterly" => {
            let q = ((m - 1) / 3) + 1;
            let start_m = (q - 1) * 3 + 1;
            let end_m = start_m + 2;
            let end_y = if end_m > 12 { y + 1 } else { y };
            let end_m = if end_m > 12 { end_m - 12 } else { end_m };
            (
                format!("{:04}-{:02}-01", y, start_m),
                format!("{:04}-{:02}-01", end_y, end_m),
            )
        }
        "yearly" => (format!("{:04}-01-01", y), format!("{:04}-12-01", y)),
        _ => (
            format!("{:04}-{:02}-01", y, m),
            format!("{:04}-{:02}-01", y, m),
        ),
    };

    let (name,): (String,) = sqlx::query_as("SELECT name FROM users WHERE id=$1")
        .bind(target)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))?;

    let side_filter = match rtype {
        "driver" => "r.driver_id=$1 AND".to_string(),
        "passenger" => "r.passenger_id=$1 AND".to_string(),
        _ => "(r.driver_id=$1 OR r.passenger_id=$1) AND".to_string(),
    };

    let (driven,): (i64,) = sqlx::query_as(&format!(
        "SELECT COUNT(*) FROM rides r WHERE {} r.status='completed' AND r.pickup_at >= $2 AND r.pickup_at < $3",
        side_filter
    ))
    .bind(target)
    .bind(&start)
    .bind(&end)
    .fetch_one(&st.db)
    .await?;

    let (received,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM rides r WHERE r.passenger_id=$1 AND r.status='completed' AND r.pickup_at >= $2 AND r.pickup_at < $3",
    )
    .bind(target)
    .bind(&start)
    .bind(&end)
    .fetch_one(&st.db)
    .await?;

    let (dist_total, reimb_total): (Option<i64>, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(r.distance_km),0)::BIGINT, COALESCE(SUM(r.reimbursement_amount_cents),0)::BIGINT FROM rides r WHERE r.driver_id=$1 AND r.status='completed' AND r.pickup_at >= $2 AND r.pickup_at < $3",
    )
    .bind(target)
    .bind(&start)
    .bind(&end)
    .fetch_one(&st.db)
    .await?;

    let attended: Vec<(String, i64)> = sqlx::query_as(
        "SELECT e.kind, COUNT(*) FROM event_signups s JOIN events e ON e.id=s.event_id
         WHERE s.user_id=$1 AND s.attended=1 AND e.starts_at >= $2 AND e.starts_at < $3 GROUP BY e.kind",
    )
    .bind(target)
    .bind(&start)
    .bind(&end)
    .fetch_all(&st.db)
    .await?;

    let events: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT e.id, e.title, e.kind, e.starts_at FROM event_signups s JOIN events e ON e.id=s.event_id
         WHERE s.user_id=$1 AND s.attended=1 AND e.starts_at >= $2 AND e.starts_at < $3 ORDER BY e.starts_at",
    )
    .bind(target)
    .bind(&start)
    .bind(&end)
    .fetch_all(&st.db)
    .await?;

    let (paid,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(sh.amount_cents),0)::BIGINT FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id JOIN events e ON e.id=s.event_id
         WHERE sh.user_id=$1 AND sh.status='confirmed' AND e.starts_at >= $2 AND e.starts_at < $3",
    )
    .bind(target)
    .bind(&start)
    .bind(&end)
    .fetch_one(&st.db)
    .await?;

    let total_attended: i64 = attended.iter().map(|a| a.1).sum();
    Ok(Json(json!({
        "user": {"id": target, "name": name},
        "month": month,
        "range": range,
        "type": rtype,
        "period_start": start,
        "period_end": end,
        "rides_driven": driven,
        "rides_received": received,
        "distance_km": dist_total.unwrap_or(0),
        "reimbursement_cents": reimb_total,
        "events_attended": total_attended,
        "by_kind": attended.into_iter().map(|(k, n)| json!({"kind": k, "count": n})).collect::<Vec<_>>(),
        "events": events.into_iter().map(|(id, t, k, at)| json!({"id": id, "title": t, "kind": k, "starts_at": at})).collect::<Vec<_>>(),
        "costs_paid_cents": paid,
        "badges": compute_badges(&st, target, u.lang).await?,
    })))
}
