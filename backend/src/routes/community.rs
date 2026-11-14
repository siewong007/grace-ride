//! Notifications, announcements, help center, Grab reimbursements, monthly reports & badges, profile.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, not_found, AppResult};
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
        .route("/help/{id}/reopen", post(reopen_ticket))
        .route("/help/counts", get(ticket_counts))
        .route("/help/members", get(help_members))
        .route("/help/guides/reorder", post(reorder_guides))
        .route("/help/guides/{id}/feedback", post(guide_feedback))
        .route("/help/guides", get(list_guides).post(create_guide))
        .route(
            "/help/guides/{key}",
            get(get_guide).put(update_guide).delete(delete_guide),
        )
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
        if !(-1..=1440).contains(&m) {
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
    status: Option<String>,
    q: Option<String>,
}

/// Escape LIKE wildcards so a search term is matched literally.
fn like_pattern(q: &str) -> String {
    let esc: String = q
        .trim()
        .chars()
        .flat_map(|c| match c {
            '%' | '_' | '\\' => vec!['\\', c],
            _ => vec![c],
        })
        .collect();
    format!("%{esc}%")
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
    let status = q
        .status
        .as_deref()
        .filter(|s| matches!(*s, "open" | "answered" | "closed"));
    let search =
        q.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(like_pattern);
    #[allow(clippy::type_complexity)]
    let rows: Vec<(i64, String, String, Option<i64>, i64, String, String, String, Option<String>, Option<bool>, Option<String>)> = sqlx::query_as(
        "SELECT t.id, t.subject, t.status, t.ride_id, t.user_id, u.name, t.created_at, t.updated_at,
                lr.body, lr.staff, lr.created_at
         FROM help_tickets t JOIN users u ON u.id=t.user_id
         LEFT JOIN LATERAL (
             SELECT LEFT(r.body, 160) AS body, (r.user_id <> t.user_id) AS staff, r.created_at
             FROM help_replies r WHERE r.ticket_id=t.id ORDER BY r.id DESC LIMIT 1
         ) lr ON TRUE
         WHERE ($1 OR t.user_id=$2)
           AND ($3::text IS NULL OR t.status=$3)
           AND ($4::text IS NULL OR t.subject ILIKE $4 ESCAPE '\\' OR u.name ILIKE $4 ESCAPE '\\')
         ORDER BY CASE t.status WHEN 'open' THEN 0 WHEN 'answered' THEN 1 ELSE 2 END, t.updated_at DESC LIMIT 200",
    )
    .bind(all)
    .bind(u.id)
    .bind(status)
    .bind(search)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(
            |(id, s, status, r, uid, name, c, upd, body, staff, at)| json!({
                "id": id, "subject": s, "status": status, "ride_id": r, "user_id": uid,
                "user_name": name, "created_at": c, "updated_at": upd,
                "last_body": body, "last_staff": staff, "last_at": at,
            })
        )
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct CountsQuery {
    all: Option<bool>,
}

async fn ticket_counts(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<CountsQuery>,
) -> AppResult<Json<Value>> {
    let all = q.all.unwrap_or(false) && u.can(Page::AdminHelp, Act::View);
    if !all {
        require(&u, Page::Help, Act::View)?;
    }
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status, COUNT(*)::bigint FROM help_tickets WHERE ($1 OR user_id=$2) GROUP BY status",
    )
    .bind(all)
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    let get = |k: &str| rows.iter().find(|(s, _)| s == k).map_or(0, |(_, n)| *n);
    Ok(Json(
        json!({"open": get("open"), "answered": get("answered"), "closed": get("closed")}),
    ))
}

#[derive(Deserialize)]
struct MemberQuery {
    q: Option<String>,
}

/// Member picker for staff raising a request on someone's behalf: id and name only.
async fn help_members(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<MemberQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let search = like_pattern(q.q.as_deref().unwrap_or(""));
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, name FROM users WHERE name ILIKE $1 ESCAPE '\\' ORDER BY name LIMIT 20",
    )
    .bind(search)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|(id, name)| json!({"id": id, "name": name}))
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct TicketIn {
    subject: String,
    body: String,
    ride_id: Option<i64>,
    /// Staff only: raise the request on behalf of this member.
    for_user_id: Option<i64>,
}

async fn create_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<TicketIn>,
) -> AppResult<Json<Value>> {
    let on_behalf = req.for_user_id.filter(|id| *id != u.id);
    if on_behalf.is_some() {
        require(&u, Page::AdminHelp, Act::Update)?;
    } else {
        require(&u, Page::Help, Act::Create)?;
    }
    if req.subject.trim().is_empty() || req.body.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::QuestionAndDetailsRequired));
    }
    let owner_id = on_behalf.unwrap_or(u.id);
    if on_behalf.is_some() {
        let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE id=$1")
            .bind(owner_id)
            .fetch_optional(&st.db)
            .await?;
        exists.ok_or(not_found(u.lang))?;
    }
    if let Some(r) = req.ride_id {
        let (p, d): (i64, Option<i64>) =
            sqlx::query_as("SELECT passenger_id, driver_id FROM rides WHERE id=$1")
                .bind(r)
                .fetch_optional(&st.db)
                .await?
                .ok_or(not_found(u.lang))?;
        if p != owner_id && d != Some(owner_id) {
            return Err(forbidden(u.lang, ErrorKey::OnlyLinkOwnRides));
        }
    }
    let status = if on_behalf.is_some() {
        "answered"
    } else {
        "open"
    };
    let (id,): (i64,) = sqlx::query_as("INSERT INTO help_tickets (user_id, ride_id, subject, status, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id")
        .bind(owner_id)
        .bind(req.ride_id)
        .bind(req.subject.trim())
        .bind(status)
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
    if on_behalf.is_some() {
        notify(
            &st.db,
            owner_id,
            ("帮助中心已回复", "Help center has replied"),
            (req.subject.trim(), req.subject.trim()),
            &format!("/help/{id}"),
        )
        .await;
    } else {
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
    }
    audit(
        &st.db,
        Some(u.id),
        "help.create",
        "help_ticket",
        Some(id),
        &if on_behalf.is_some() {
            format!("{} (for user {owner_id})", req.subject.trim())
        } else {
            req.subject.trim().to_string()
        },
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
    let member = if u.can(Page::AdminHelp, Act::View) {
        let m: Option<(String, String)> =
            sqlx::query_as("SELECT name, phone FROM users WHERE id=$1")
                .bind(owner)
                .fetch_optional(&st.db)
                .await?;
        let others: Vec<(i64,)> = sqlx::query_as(
            "SELECT id FROM help_tickets WHERE user_id=$1 AND id<>$2 AND status<>'closed' ORDER BY updated_at DESC LIMIT 10",
        )
        .bind(owner)
        .bind(id)
        .fetch_all(&st.db)
        .await?;
        m.map(|(name, phone)| json!({"name": name, "phone": phone, "other_open": others.into_iter().map(|(i,)| i).collect::<Vec<_>>()}))
    } else {
        None
    };
    Ok(Json(json!({
        "id": id, "subject": subject, "status": status, "ride_id": ride, "user_id": owner, "member": member,
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

async fn reopen_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let (owner, subject, status, _) = ticket_owner(&st, &u, id, Act::Update).await?;
    if owner == u.id && !u.can(Page::AdminHelp, Act::Update) {
        require(&u, Page::Help, Act::Update)?;
    }
    if status != "closed" {
        return Ok(Json(json!({"ok": true})));
    }
    sqlx::query("UPDATE help_tickets SET status='open', updated_at=$1 WHERE id=$2")
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    if u.id == owner {
        notify_perm(
            &st.db,
            Page::AdminHelp,
            Act::Update,
            ("帮助中心：请求已重新打开", "Help center: request reopened"),
            (&subject, &subject),
            &format!("/help/{id}"),
        )
        .await;
    } else {
        notify(
            &st.db,
            owner,
            (
                "帮助中心已重新打开你的请求",
                "Help center reopened your request",
            ),
            (&subject, &subject),
            &format!("/help/{id}"),
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        "help.reopen",
        "help_ticket",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

// ---------------- Admin workflow guides ----------------

/// The in-app manual is readable by anyone who can see any staff page and
/// written by help-centre staff only (see CRUD handlers below).
fn any_admin_page(u: &CurrentUser) -> bool {
    crate::perms::CATALOG
        .iter()
        .any(|d| d.page.area() != crate::perms::Area::Member && u.can(d.page, Act::View))
}

#[derive(Deserialize)]
struct GuideQuery {
    include_drafts: Option<bool>,
    /// Also return each guide's steps (for in-browser search).
    full: Option<bool>,
}

async fn list_guides(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<GuideQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Help, Act::View)?;
    let staff = any_admin_page(&u);
    let with_drafts = q.include_drafts.unwrap_or(false) && u.can(Page::AdminHelp, Act::Update);
    let can_edit = u.can(Page::AdminHelp, Act::Update);
    let full = q.full.unwrap_or(false);
    #[allow(clippy::type_complexity)]
    let rows: Vec<(i64, String, String, String, String, String, bool, String, i64, i32, String, i64, i64)> = sqlx::query_as(
        "SELECT g.id, g.slug, g.title_en, g.title_zh, g.summary_en, g.summary_zh, g.published, g.audience,
                jsonb_array_length(g.steps)::bigint, g.sort_order, g.steps::text,
                COUNT(f.user_id) FILTER (WHERE f.helpful)::bigint,
                COUNT(f.user_id) FILTER (WHERE NOT f.helpful)::bigint
         FROM guide_workflows g LEFT JOIN guide_feedback f ON f.guide_id=g.id
         WHERE ($1 OR g.published) AND ($2 OR g.audience='member')
         GROUP BY g.id ORDER BY g.sort_order, g.id",
    )
    .bind(with_drafts)
    .bind(staff)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(Value::Array(
        rows.into_iter()
            .map(|(id, slug, ten, tzh, sen, szh, published, audience, n, sort, steps, yes, no)| {
                let mut v = json!({
                    "id": id, "slug": slug, "title_en": ten, "title_zh": tzh,
                    "summary_en": sen, "summary_zh": szh, "published": published, "audience": audience,
                    "step_count": n, "sort_order": sort,
                });
                if full {
                    v["steps"] = serde_json::from_str(&steps).unwrap_or(json!([]));
                }
                if can_edit {
                    v["helpful_yes"] = json!(yes);
                    v["helpful_no"] = json!(no);
                }
                v
            })
            .collect(),
    )))
}

#[derive(Deserialize)]
struct FeedbackIn {
    helpful: bool,
}

/// One vote per user per guide; voting again replaces the earlier answer.
async fn guide_feedback(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<FeedbackIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Help, Act::View)?;
    let staff = any_admin_page(&u);
    let can_edit = u.can(Page::AdminHelp, Act::Update);
    let visible: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM guide_workflows WHERE id=$1 AND ($2 OR published) AND ($3 OR audience='member')",
    )
    .bind(id)
    .bind(can_edit)
    .bind(staff)
    .fetch_optional(&st.db)
    .await?;
    visible.ok_or(not_found(u.lang))?;
    sqlx::query(
        "INSERT INTO guide_feedback (guide_id, user_id, helpful, created_at) VALUES ($1,$2,$3,$4)
         ON CONFLICT (guide_id, user_id) DO UPDATE SET helpful=EXCLUDED.helpful, created_at=EXCLUDED.created_at",
    )
    .bind(id)
    .bind(u.id)
    .bind(req.helpful)
    .bind(now())
    .execute(&st.db)
    .await?;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct ReorderIn {
    ids: Vec<i64>,
}

async fn reorder_guides(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ReorderIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let mut tx = st.db.begin().await?;
    for (i, id) in req.ids.iter().enumerate() {
        sqlx::query("UPDATE guide_workflows SET sort_order=$1 WHERE id=$2")
            .bind(i as i32)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "guide.reorder",
        "guide_workflow",
        None,
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn get_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Help, Act::View)?;
    let staff = any_admin_page(&u);
    let can_edit = u.can(Page::AdminHelp, Act::Update);
    let row: Option<(i64, String, String, String, String, String, bool, String, String)> = sqlx::query_as(
        "SELECT id, slug, title_en, title_zh, summary_en, summary_zh, published, audience, steps::text
             FROM guide_workflows WHERE slug=$1 AND ($2 OR published) AND ($3 OR audience='member')",
    )
    .bind(&slug)
    .bind(can_edit)
    .bind(staff)
    .fetch_optional(&st.db)
    .await?;
    let (id, slug, ten, tzh, sen, szh, published, audience, steps) =
        row.ok_or(not_found(u.lang))?;
    let steps: Value = serde_json::from_str(&steps).unwrap_or(json!([]));
    Ok(Json(json!({
        "id": id, "slug": slug, "title_en": ten, "title_zh": tzh,
        "summary_en": sen, "summary_zh": szh, "published": published, "audience": audience, "steps": steps,
    })))
}

#[derive(Deserialize)]
struct GuideStepIn {
    title_en: Option<String>,
    title_zh: Option<String>,
    body_en: Option<String>,
    body_zh: Option<String>,
    link: Option<String>,
}

#[derive(Deserialize)]
struct GuideIn {
    slug: String,
    title_en: Option<String>,
    title_zh: Option<String>,
    summary_en: Option<String>,
    summary_zh: Option<String>,
    published: Option<bool>,
    #[serde(default = "default_guide_audience")]
    audience: String,
    #[serde(default)]
    steps: Vec<GuideStepIn>,
}

fn default_guide_audience() -> String {
    "staff".into()
}

fn trim_opt(v: &Option<String>) -> &str {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
}

/// Normalise + validate a guide payload; returns columns ready to bind,
/// with steps serialised to JSON text (sqlx has no json feature).
#[allow(clippy::type_complexity)]
fn validate_guide(
    req: &GuideIn,
    lang: crate::i18n::Lang,
) -> AppResult<(String, String, String, String, String, bool, String, String)> {
    let (title_en, title_zh) = (trim_opt(&req.title_en), trim_opt(&req.title_zh));
    if title_en.is_empty() && title_zh.is_empty() {
        return Err(bad(lang, ErrorKey::GuideTitleRequired));
    }
    let slug = req.slug.trim().to_lowercase();
    let ok_slug = !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok_slug {
        return Err(bad(lang, ErrorKey::GuideSlugRequired));
    }
    if !matches!(req.audience.as_str(), "member" | "staff") {
        return Err(bad(lang, ErrorKey::GuideAudienceInvalid));
    }
    if req.steps.len() > 30 {
        return Err(bad(lang, ErrorKey::GuideStepInvalid));
    }
    let mut steps: Vec<Value> = Vec::new();
    for s in &req.steps {
        let (ten, tzh) = (trim_opt(&s.title_en), trim_opt(&s.title_zh));
        if ten.is_empty() && tzh.is_empty() {
            return Err(bad(lang, ErrorKey::GuideStepInvalid));
        }
        let link = s.link.as_deref().map(str::trim).filter(|l| !l.is_empty());
        if let Some(l) = link {
            // Internal app paths only: must start with '/', no protocol-relative URLs.
            if !l.starts_with('/') || l.starts_with("//") || l.contains(':') {
                return Err(bad(lang, ErrorKey::GuideLinkInvalid));
            }
        }
        steps.push(json!({
            "title_en": ten, "title_zh": tzh,
            "body_en": trim_opt(&s.body_en), "body_zh": trim_opt(&s.body_zh),
            "link": link.unwrap_or(""),
        }));
    }
    Ok((
        slug.to_string(),
        title_en.to_string(),
        title_zh.to_string(),
        trim_opt(&req.summary_en).to_string(),
        trim_opt(&req.summary_zh).to_string(),
        req.published.unwrap_or(true),
        req.audience.clone(),
        Value::Array(steps).to_string(),
    ))
}

async fn create_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<GuideIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let (slug, ten, tzh, sen, szh, published, audience, steps) = validate_guide(&req, u.lang)?;
    let dup: Option<(i64,)> = sqlx::query_as("SELECT id FROM guide_workflows WHERE slug=$1")
        .bind(&slug)
        .fetch_optional(&st.db)
        .await?;
    if dup.is_some() {
        return Err(bad(u.lang, ErrorKey::GuideSlugExists));
    }
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO guide_workflows (slug, title_en, title_zh, summary_en, summary_zh, published, audience, steps, updated_at, updated_by)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8::jsonb,$9,$10) RETURNING id",
    )
    .bind(&slug)
    .bind(&ten)
    .bind(&tzh)
    .bind(&sen)
    .bind(&szh)
    .bind(published)
    .bind(&audience)
    .bind(&steps)
    .bind(now())
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
    audit(
        &st.db,
        Some(u.id),
        "guide.create",
        "guide_workflow",
        Some(id),
        &slug,
    )
    .await;
    Ok(Json(json!({ "id": id })))
}

async fn update_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(key): Path<i64>,
    Json(req): Json<GuideIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let (slug, ten, tzh, sen, szh, published, audience, steps) = validate_guide(&req, u.lang)?;
    let dup: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM guide_workflows WHERE slug=$1 AND id<>$2")
            .bind(&slug)
            .bind(key)
            .fetch_optional(&st.db)
            .await?;
    if dup.is_some() {
        return Err(bad(u.lang, ErrorKey::GuideSlugExists));
    }
    let n = sqlx::query(
        "UPDATE guide_workflows SET slug=$1, title_en=$2, title_zh=$3, summary_en=$4, summary_zh=$5,
                published=$6, audience=$7, steps=$8::jsonb, updated_at=$9, updated_by=$10 WHERE id=$11",
    )
    .bind(&slug)
    .bind(&ten)
    .bind(&tzh)
    .bind(&sen)
    .bind(&szh)
    .bind(published)
    .bind(&audience)
    .bind(&steps)
    .bind(now())
    .bind(u.id)
    .bind(key)
    .execute(&st.db)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(not_found(u.lang));
    }
    audit(
        &st.db,
        Some(u.id),
        "guide.update",
        "guide_workflow",
        Some(key),
        &slug,
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

async fn delete_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(key): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let n = sqlx::query("DELETE FROM guide_workflows WHERE id=$1")
        .bind(key)
        .execute(&st.db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(not_found(u.lang));
    }
    audit(
        &st.db,
        Some(u.id),
        "guide.delete",
        "guide_workflow",
        Some(key),
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true })))
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
    let changed = sqlx::query(
        "UPDATE reimbursements SET status=$1, review_note=$2, reviewed_by=$3, updated_at=$4 WHERE id=$5 AND status='pending'",
    )
    .bind(status)
    .bind(&note)
    .bind(u.id)
    .bind(now())
    .bind(id)
    .execute(&st.db)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(conflict(u.lang, ErrorKey::ReimbAlreadyReviewed));
    }
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
    // The status check above and this update are separate statements, so a second
    // finance admin can pass the check before the first commit. Transitioning from
    // 'approved' in the update itself makes exactly one payout win.
    let paid = sqlx::query("UPDATE reimbursements SET status='paid', payee_verified=1, paid_ref=$1, paid_by=$2, paid_at=$3, updated_at=$4 WHERE id=$5 AND status='approved'")
        .bind(req.paid_ref.trim())
        .bind(u.id)
        .bind(now())
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    if paid.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::ReimbursementProcessed));
    }
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

/// Parse `YYYY-MM` without slicing by byte offset: a 7-byte non-ASCII string used
/// to panic the handler when a slice landed inside a multi-byte character.
fn parse_report_month(raw: &str) -> Option<(i32, u32)> {
    let b = raw.as_bytes();
    if b.len() != 7
        || b[4] != b'-'
        || !b[..4].iter().all(u8::is_ascii_digit)
        || !b[5..].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let year: i32 = raw[..4].parse().ok()?;
    let month: u32 = raw[5..7].parse().ok()?;
    if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    Some((year, month))
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
    let (y, m) =
        parse_report_month(&month).ok_or_else(|| bad(u.lang, ErrorKey::MonthFormatInvalid))?;

    // Period bounds are half-open: `start <= pickup_at < end`. The end is the first
    // day *after* the period, so the selected month/quarter/year is fully included.
    let (start, end): (String, String) = match range {
        "quarterly" => {
            let quarter = (m - 1) / 3;
            let start_m = quarter * 3 + 1;
            let end_m = start_m + 3;
            let (end_y, end_m) = if end_m > 12 {
                (y + 1, end_m - 12)
            } else {
                (y, end_m)
            };
            (
                format!("{:04}-{:02}-01", y, start_m),
                format!("{:04}-{:02}-01", end_y, end_m),
            )
        }
        "yearly" => (format!("{:04}-01-01", y), format!("{:04}-01-01", y + 1)),
        _ => {
            let (end_y, end_m) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
            (
                format!("{:04}-{:02}-01", y, m),
                format!("{:04}-{:02}-01", end_y, end_m),
            )
        }
    };

    let (name,): (String,) = sqlx::query_as("SELECT name FROM users WHERE id=$1")
        .bind(target)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))?;

    // `rides_driven` is the target's own driving, never a ride they merely took as a
    // passenger. The type filter narrows which side of the report is populated.
    let (driven,): (i64,) = if rtype == "passenger" {
        (0,)
    } else {
        sqlx::query_as(
            "SELECT COUNT(*) FROM rides r WHERE r.driver_id=$1 AND r.status='completed' AND r.pickup_at >= $2 AND r.pickup_at < $3",
        )
        .bind(target)
        .bind(&start)
        .bind(&end)
        .fetch_one(&st.db)
        .await?
    };

    let (received,): (i64,) = if rtype == "driver" {
        (0,)
    } else {
        sqlx::query_as(
            "SELECT COUNT(*) FROM rides r WHERE r.passenger_id=$1 AND r.status='completed' AND r.pickup_at >= $2 AND r.pickup_at < $3",
        )
        .bind(target)
        .bind(&start)
        .bind(&end)
        .fetch_one(&st.db)
        .await?
    };

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
