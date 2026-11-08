//! Notifications, announcements, help center, Grab reimbursements, monthly reports & badges, profile.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult, not_found};
use crate::state::AppState;
use crate::util::{Lang, audit, fmt_rm, notify, notify_roles, now, require_any, t};

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
}

async fn update_profile(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ProfileIn>,
) -> AppResult<Json<Value>> {
    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err(bad(t(u.lang, "请填写姓名", "Please enter your name")));
        }
    }
    if let Some(ref lang) = req.lang {
        if lang != "zh" && lang != "en" {
            return Err(bad(t(u.lang, "语言只支持中文或英文", "Language must be zh or en")));
        }
    }
    if let Some(f) = req.photo_file_id {
        let (owner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=$1")
            .bind(f)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
        if owner != u.id {
            return Err(forbidden(t(u.lang, "照片无效", "Invalid photo")));
        }
    }
    sqlx::query(
        "UPDATE users SET
            name=COALESCE($1, name),
            phone=COALESCE($2, phone),
            photo_file_id=COALESCE($3, photo_file_id),
            badges_opt_out=COALESCE($4, badges_opt_out),
            lang=COALESCE($5, lang)
         WHERE id=$6",
    )
    .bind(req.name.as_deref().map(str::trim))
    .bind(req.phone.as_deref().map(str::trim))
    .bind(req.photo_file_id)
    .bind(req.badges_opt_out.map(|b| b as i64))
    .bind(req.lang.as_deref())
    .bind(u.id)
    .execute(&st.db)
    .await?;
    audit(&st.db, Some(u.id), "profile.update", "user", Some(u.id), "").await;
    Ok(Json(json!({"ok": true})))
}

// ---------------- Notifications ----------------

async fn list_notifications(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
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
    require_any(&u, &["transport", "admin"])?;
    if req.title.trim().is_empty() || req.body.trim().is_empty() {
        return Err(bad(t(u.lang, "请填写标题及内容", "Please enter both a title and content")));
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
            notify(&st.db, uid, ("新公告", "New announcement"), (title, title), "/").await;
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
    require_any(&u, &["transport", "admin"])?;
    sqlx::query("DELETE FROM announcements WHERE id=$1")
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "announcement.delete",
        "announcement",
        Some(id),
        "",
    )
    .await;
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
    let all = q.all.unwrap_or(false) && u.is_staff();
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
    if req.subject.trim().is_empty() || req.body.trim().is_empty() {
        return Err(bad(t(u.lang, "请填写问题与说明", "Please enter your question and details")));
    }
    if let Some(r) = req.ride_id {
        let (p, d): (i64, Option<i64>) =
            sqlx::query_as("SELECT passenger_id, driver_id FROM rides WHERE id=$1")
                .bind(r)
                .fetch_optional(&st.db)
                .await?
                .ok_or(not_found(u.lang))?;
        if p != u.id && d != Some(u.id) {
            return Err(forbidden(t(
                u.lang,
                "只能关联自己的接送",
                "You can only link your own rides",
            )));
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
    notify_roles(
        &st.db,
        &["transport", "admin"],
        ("帮助中心：新的求助", "Help center: new request"),
        (
            &format!("{} · {}", u.name, req.subject.trim()),
            &format!("{} · {}", u.name, req.subject.trim()),
        ),
        &format!("/help/{id}"),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

async fn ticket_owner(
    st: &AppState,
    u: &CurrentUser,
    id: i64,
) -> AppResult<(i64, String, String, Option<i64>)> {
    let row: (i64, String, String, Option<i64>) =
        sqlx::query_as("SELECT user_id, subject, status, ride_id FROM help_tickets WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
    if row.0 != u.id && !u.is_staff() {
        return Err(forbidden(t(u.lang, "无权限", "You don't have permission")));
    }
    Ok(row)
}

async fn get_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let (owner, subject, status, ride) = ticket_owner(&st, &u, id).await?;
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
    let (owner, subject, status, _) = ticket_owner(&st, &u, id).await?;
    if status == "closed" {
        return Err(bad(t(u.lang, "此求助已结案", "This request has already been closed")));
    }
    if req.body.trim().is_empty() {
        return Err(bad(t(u.lang, "请填写内容", "Please enter your reply")));
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
        notify_roles(
            &st.db,
            &["transport", "admin"],
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
    Ok(Json(json!({"ok": true})))
}

async fn close_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    ticket_owner(&st, &u, id).await?;
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
    let finance = u.has("finance") && u.mfa_ok;
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
}

async fn list_reimbursements(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<ReimbQuery>,
) -> AppResult<Json<Value>> {
    let rows = match q.scope.as_deref() {
        Some("review") => {
            require_any(&u, &["transport"])?;
            sqlx::query_as::<_, ReimbRow>(&format!("{REIMB_SELECT} ORDER BY CASE r.status WHEN 'pending' THEN 0 ELSE 1 END, r.id DESC LIMIT 200"))
                .fetch_all(&st.db)
                .await?
        }
        Some("payout") => {
            require_any(&u, &["finance"])?;
            sqlx::query_as::<_, ReimbRow>(&format!("{REIMB_SELECT} WHERE r.status IN ('approved','paid') ORDER BY CASE r.status WHEN 'approved' THEN 0 ELSE 1 END, r.id DESC LIMIT 200"))
                .fetch_all(&st.db)
                .await?
        }
        _ => {
            sqlx::query_as::<_, ReimbRow>(&format!(
                "{REIMB_SELECT} WHERE r.user_id=$1 ORDER BY r.id DESC LIMIT 100"
            ))
            .bind(u.id)
            .fetch_all(&st.db)
            .await?
        }
    };
    Ok(Json(json!(rows
        .iter()
        .map(|r| reimb_json(r, &u))
        .collect::<Vec<_>>())))
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
    if r.user_id != u.id && !u.is_staff() && !(u.has("finance") && u.mfa_ok) {
        return Err(forbidden(t(u.lang, "无权限", "You don't have permission")));
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
        return Err(bad(t(
            u.lang,
            "金额需介于 RM0.01 – RM500",
            "Amount must be between RM0.01 and RM500",
        )));
    }
    if req.payee_name.trim().is_empty() || req.payee_phone.trim().is_empty() {
        return Err(bad(t(
            u.lang,
            "请填写收款人姓名及手机号（TNG / DuitNow）",
            "Please enter the payee name and phone (TNG / DuitNow)",
        )));
    }
    let (owner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=$1")
        .bind(req.receipt_file_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or_else(|| bad(t(u.lang, "请上传收据", "Please upload a receipt")))?;
    if owner != u.id {
        return Err(forbidden(t(u.lang, "收据无效", "Invalid receipt")));
    }
    if let Some(rid) = req.ride_id {
        let (p,): (i64,) = sqlx::query_as("SELECT passenger_id FROM rides WHERE id=$1")
            .bind(rid)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
        if p != u.id {
            return Err(forbidden(t(
                u.lang,
                "只能关联自己的接送",
                "You can only link your own rides",
            )));
        }
    }
    Ok(())
}

async fn create_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ReimbIn>,
) -> AppResult<Json<Value>> {
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
    notify_roles(
        &st.db,
        &["transport"],
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
        return Err(bad(t(
            u.lang,
            "只有被退回的申请可以补资料重新提交",
            "Only returned requests can be resubmitted",
        )));
    }
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
    notify_roles(
        &st.db,
        &["transport"],
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
        "",
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
    require_any(&u, &["transport"])?;
    let r = load_reimb(&st, u.lang, id).await?;
    if r.status != "pending" {
        return Err(bad(t(
            u.lang,
            "此申请已审核",
            "This request has already been reviewed",
        )));
    }
    if r.user_id == u.id {
        return Err(forbidden(t(
            u.lang,
            "不能审核自己的申请",
            "You cannot review your own request",
        )));
    }
    let note = req.note.unwrap_or_default();
    let status = match req.action.as_str() {
        "approve" => "approved",
        "return" => "returned",
        "reject" => "rejected",
        _ => return Err(bad(t(u.lang, "操作无效", "Invalid action"))),
    };
    if status != "approved" && note.trim().is_empty() {
        return Err(bad(t(u.lang, "请填写原因", "Please provide a reason")));
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
        notify_roles(
            &st.db,
            &["finance"],
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
    require_any(&u, &["finance"])?;
    let r = load_reimb(&st, u.lang, id).await?;
    if r.status != "approved" {
        return Err(bad(t(
            u.lang,
            "只有已批准的申请可以付款",
            "Only approved requests can be paid",
        )));
    }
    if r.user_id == u.id {
        return Err(forbidden(t(
            u.lang,
            "不能为自己的申请付款",
            "You cannot pay your own request",
        )));
    }
    if !req.payee_verified {
        return Err(bad(t(
            u.lang,
            "请先核对收款人姓名、手机号及 DuitNow 收款去向",
            "Please verify the payee name, phone and DuitNow destination first",
        )));
    }
    if req.paid_ref.trim().len() < 4 {
        return Err(bad(t(u.lang, "请填写交易编号", "Please enter the transaction reference")));
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
            t(lang, "完成 3 次或以上晚上 9 点后的接送", "Complete 3 or more rides after 9 pm"),
            night,
            3,
        ),
        (
            "regular",
            t(lang, "勤", "R"),
            t(lang, "常驻司机", "Resident Driver"),
            t(lang, "过去 6 个月中至少 3 个月有完成接送", "Completed rides in at least 3 of the past 6 months"),
            months,
            3,
        ),
        (
            "carpool",
            t(lang, "爱", "C"),
            t(lang, "爱心同行者", "Caring Companion"),
            t(lang, "开启共乘并完成 2 趟或以上多人同车接送", "Enable carpool and complete 2 or more rides with multiple passengers"),
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
async fn monthly_report(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<ReportQuery>,
) -> AppResult<Json<Value>> {
    let target = q.user_id.unwrap_or(u.id);
    if target != u.id {
        require_any(&u, &["admin"])?;
    }
    let month = q.month.unwrap_or_else(|| now()[..7].to_string());
    if month.len() != 7 {
        return Err(bad(t(u.lang, "月份格式应为 YYYY-MM", "Month must be in YYYY-MM format")));
    }
    let (name,): (String,) = sqlx::query_as("SELECT name FROM users WHERE id=$1")
        .bind(target)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))?;
    let (driven,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rides WHERE driver_id=$1 AND status='completed' AND substr(pickup_at,1,7)=$2")
        .bind(target)
        .bind(&month)
        .fetch_one(&st.db)
        .await?;
    let (received,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rides WHERE passenger_id=$1 AND status='completed' AND substr(pickup_at,1,7)=$2")
        .bind(target)
        .bind(&month)
        .fetch_one(&st.db)
        .await?;
    let attended: Vec<(String, i64)> = sqlx::query_as(
        "SELECT e.kind, COUNT(*) FROM event_signups s JOIN events e ON e.id=s.event_id
         WHERE s.user_id=$1 AND s.attended=1 AND substr(e.starts_at,1,7)=$2 GROUP BY e.kind",
    )
    .bind(target)
    .bind(&month)
    .fetch_all(&st.db)
    .await?;
    let events: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT e.id, e.title, e.kind, e.starts_at FROM event_signups s JOIN events e ON e.id=s.event_id
         WHERE s.user_id=$1 AND s.attended=1 AND substr(e.starts_at,1,7)=$2 ORDER BY e.starts_at",
    )
    .bind(target)
    .bind(&month)
    .fetch_all(&st.db)
    .await?;
    let (paid,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(sh.amount_cents),0)::BIGINT FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id JOIN events e ON e.id=s.event_id
         WHERE sh.user_id=$1 AND sh.status='confirmed' AND substr(e.starts_at,1,7)=$2",
    )
    .bind(target)
    .bind(&month)
    .fetch_one(&st.db)
    .await?;
    let total_attended: i64 = attended.iter().map(|a| a.1).sum();
    Ok(Json(json!({
        "user": {"id": target, "name": name}, "month": month,
        "rides_driven": driven, "rides_received": received, "events_attended": total_attended,
        "by_kind": attended.into_iter().map(|(k, n)| json!({"kind": k, "count": n})).collect::<Vec<_>>(),
        "events": events.into_iter().map(|(id, t, k, at)| json!({"id": id, "title": t, "kind": k, "starts_at": at})).collect::<Vec<_>>(),
        "costs_paid_cents": paid,
        "badges": compute_badges(&st, target, u.lang).await?,
    })))
}
