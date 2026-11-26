//! Help center ticket creation, replies, status transitions, and staff claiming.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::notifications::notify_perm_except;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, notify, now, require};

use super::like_pattern;

// ---------------- Help center ----------------

#[derive(Deserialize)]
pub(super) struct HelpQuery {
    all: Option<bool>,
    status: Option<String>,
    q: Option<String>,
    /// "me": only requests this staff member has claimed.
    assigned: Option<String>,
}

/// A photo on a help message must be an image the sender uploaded themselves,
/// so nobody can surface someone else's file by naming its id.
async fn check_attachment(st: &AppState, u: &CurrentUser, file_id: Option<i64>) -> AppResult<()> {
    let Some(fid) = file_id else { return Ok(()) };
    let ok: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM files WHERE id=$1 AND owner_id=$2 AND mime LIKE 'image/%'")
            .bind(fid)
            .bind(u.id)
            .fetch_optional(&st.db)
            .await?;
    ok.map(|_| ())
        .ok_or(bad(u.lang, ErrorKey::HelpAttachmentInvalid))
}

pub(super) async fn list_tickets(
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
    let mine_only = all && q.assigned.as_deref() == Some("me");
    let rows: Vec<(
        i64,
        String,
        String,
        Option<i64>,
        i64,
        String,
        String,
        String,
        Option<String>,
        Option<bool>,
        Option<String>,
        Option<i64>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT t.id, t.subject, t.status, t.ride_id, t.user_id, u.name, t.created_at, t.updated_at,
                lr.body, lr.staff, lr.created_at, t.assignee_id, a.name
         FROM help_tickets t JOIN users u ON u.id=t.user_id
         LEFT JOIN users a ON a.id=t.assignee_id
         LEFT JOIN LATERAL (
             SELECT LEFT(r.body, 160) AS body, (r.user_id <> t.user_id) AS staff, r.created_at
             FROM help_replies r WHERE r.ticket_id=t.id ORDER BY r.id DESC LIMIT 1
         ) lr ON TRUE
         WHERE ($1 OR t.user_id=$2)
           AND ($3::text IS NULL OR t.status=$3)
           AND ($4::text IS NULL OR t.subject ILIKE $4 ESCAPE '\\' OR u.name ILIKE $4 ESCAPE '\\')
           AND (NOT $5 OR t.assignee_id=$2)
         ORDER BY CASE t.status WHEN 'open' THEN 0 WHEN 'answered' THEN 1 ELSE 2 END, t.updated_at DESC LIMIT 200",
    )
    .bind(all)
    .bind(u.id)
    .bind(status)
    .bind(search)
    .bind(mine_only)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(
            |(id, s, status, r, uid, name, c, upd, body, staff, at, aid, aname)| json!({
                "id": id, "subject": s, "status": status, "ride_id": r, "user_id": uid,
                "user_name": name, "created_at": c, "updated_at": upd,
                "last_body": body, "last_staff": staff, "last_at": at,
                "assignee_id": aid, "assignee_name": aname,
            })
        )
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
pub(super) struct CountsQuery {
    all: Option<bool>,
    assigned: Option<String>,
}

pub(super) async fn ticket_counts(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<CountsQuery>,
) -> AppResult<Json<Value>> {
    let all = q.all.unwrap_or(false) && u.can(Page::AdminHelp, Act::View);
    if !all {
        require(&u, Page::Help, Act::View)?;
    }
    let mine_only = all && q.assigned.as_deref() == Some("me");
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status, COUNT(*)::bigint FROM help_tickets
         WHERE ($1 OR user_id=$2) AND (NOT $3 OR assignee_id=$2) GROUP BY status",
    )
    .bind(all)
    .bind(u.id)
    .bind(mine_only)
    .fetch_all(&st.db)
    .await?;
    let get = |k: &str| rows.iter().find(|(s, _)| s == k).map_or(0, |(_, n)| *n);
    Ok(Json(
        json!({"open": get("open"), "answered": get("answered"), "closed": get("closed")}),
    ))
}

#[derive(Deserialize)]
pub(super) struct MemberQuery {
    q: Option<String>,
}

/// Member picker for staff raising a request on someone's behalf: id and name only.
pub(super) async fn help_members(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<MemberQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let search = like_pattern(q.q.as_deref().unwrap_or(""));
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, name FROM users WHERE status='active' AND name ILIKE $1 ESCAPE '\\' ORDER BY name LIMIT 20",
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
pub(super) struct TicketIn {
    subject: String,
    body: String,
    ride_id: Option<i64>,
    /// Staff only: raise the request on behalf of this member.
    for_user_id: Option<i64>,
    /// Optional photo (an image this user uploaded).
    file_id: Option<i64>,
}

pub(super) async fn create_ticket(
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
    check_attachment(&st, &u, req.file_id).await?;
    if on_behalf.is_some() {
        let status_row: Option<(String,)> = sqlx::query_as("SELECT status FROM users WHERE id=$1")
            .bind(owner_id)
            .fetch_optional(&st.db)
            .await?;
        let Some((target_status,)) = status_row else {
            return Err(not_found(u.lang));
        };
        if target_status == "disabled" {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
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
    let mut tx = st.db.begin().await?;
    let (id,): (i64,) = sqlx::query_as("INSERT INTO help_tickets (user_id, ride_id, subject, status, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id")
        .bind(owner_id)
        .bind(req.ride_id)
        .bind(req.subject.trim())
        .bind(status)
        .bind(now())
        .bind(now())
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO help_replies (ticket_id, user_id, body, file_id, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(u.id)
    .bind(req.body.trim())
    .bind(req.file_id)
    .bind(now())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
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
        notify_perm_except(
            &st.db,
            Page::AdminHelp,
            Act::Update,
            Some(u.id),
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

pub(super) async fn get_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let (owner, subject, status, ride) = ticket_owner(&st, &u, id, Act::View).await?;
    if owner == u.id && !u.can(Page::AdminHelp, Act::View) {
        require(&u, Page::Help, Act::View)?;
    }
    let replies: Vec<(i64, i64, String, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT r.id, r.user_id, u.name, r.body, r.created_at, r.file_id FROM help_replies r JOIN users u ON u.id=r.user_id WHERE r.ticket_id=$1 ORDER BY r.id",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    let (assignee_id, assignee_name): (Option<i64>, Option<String>) = sqlx::query_as(
        "SELECT t.assignee_id, a.name FROM help_tickets t LEFT JOIN users a ON a.id=t.assignee_id WHERE t.id=$1",
    )
    .bind(id)
    .fetch_one(&st.db)
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
        "assignee_id": assignee_id, "assignee_name": assignee_name,
        "replies": replies.into_iter().map(|(rid, uid, name, body, at, fid)| json!({"id": rid, "user_id": uid, "name": name, "body": body, "created_at": at, "staff": uid != owner, "file_id": fid})).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub(super) struct ReplyIn {
    #[serde(default)]
    body: String,
    file_id: Option<i64>,
}

pub(super) async fn reply_ticket(
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
    // A photo on its own is a reply too ("this is where I'm waiting").
    if req.body.trim().is_empty() && req.file_id.is_none() {
        return Err(bad(u.lang, ErrorKey::ReplyContentRequired));
    }
    check_attachment(&st, &u, req.file_id).await?;
    // Lock the ticket and look at its status again: a close saved since the read above
    // must not be undone by this reply flipping the ticket back to open/answered.
    let mut tx = st.db.begin().await?;
    let (current,): (String,) =
        sqlx::query_as("SELECT status FROM help_tickets WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if current == "closed" {
        return Err(bad(u.lang, ErrorKey::RequestAlreadyClosed));
    }
    sqlx::query(
        "INSERT INTO help_replies (ticket_id, user_id, body, file_id, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(u.id)
    .bind(req.body.trim())
    .bind(req.file_id)
    .bind(now())
    .execute(&mut *tx)
    .await?;
    let new_status = if u.id == owner { "open" } else { "answered" };
    sqlx::query("UPDATE help_tickets SET status=$1, updated_at=$2 WHERE id=$3")
        .bind(new_status)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    if u.id == owner {
        notify_perm_except(
            &st.db,
            Page::AdminHelp,
            Act::Update,
            Some(u.id),
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

pub(super) async fn close_ticket(
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
    // No notification for a resolve, but the member's open tab should update straight away.
    if owner != u.id {
        crate::stream::publish(owner, "notify");
    }
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

pub(super) async fn reopen_ticket(
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
    let res = sqlx::query(
        "UPDATE help_tickets SET status='open', updated_at=$1 WHERE id=$2 AND status='closed'",
    )
    .bind(now())
    .bind(id)
    .execute(&st.db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    if u.id == owner {
        notify_perm_except(
            &st.db,
            Page::AdminHelp,
            Act::Update,
            Some(u.id),
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

#[derive(Deserialize)]
pub(super) struct ClaimIn {
    /// true: take the request; false: hand it back to the queue.
    claim: bool,
}

/// A help-desk member takes a request so others know it is being handled.
/// Releasing only clears the caller's own claim.
pub(super) async fn claim_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ClaimIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let exists: Option<(i64, String)> =
        sqlx::query_as("SELECT id, status FROM help_tickets WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
    let Some((_, tk_status)) = exists else {
        return Err(not_found(u.lang));
    };
    if req.claim && tk_status == "closed" {
        return Err(conflict(u.lang, ErrorKey::InvalidAction));
    }
    let n = if req.claim {
        let rows = sqlx::query(
            "UPDATE help_tickets SET assignee_id=$1 WHERE id=$2 AND status <> 'closed' AND (assignee_id IS NULL OR assignee_id=$1)",
        )
        .bind(u.id)
        .bind(id)
        .execute(&st.db)
        .await?
        .rows_affected();
        if rows == 0 {
            return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
        }
        rows
    } else {
        sqlx::query("UPDATE help_tickets SET assignee_id=NULL WHERE id=$1 AND assignee_id=$2")
            .bind(id)
            .bind(u.id)
            .execute(&st.db)
            .await?
            .rows_affected()
    };
    if n > 0 {
        audit(
            &st.db,
            Some(u.id),
            if req.claim {
                "help.claim"
            } else {
                "help.release"
            },
            "help_ticket",
            Some(id),
            "",
        )
        .await;
    }
    Ok(Json(json!({"ok": true, "changed": n > 0})))
}
