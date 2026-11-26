//! Admin invitation links and passkey account-recovery approval endpoints.

use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};

use crate::auth::{hash_token, CurrentUser};
use crate::error::{bad, conflict, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{self, parse_roles, Act, Page};
use crate::state::AppState;
use crate::util::{audit, in_days, minutes_ago, now, random_token, require, Lang};

async fn has_admin_power(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    roles: &[String],
) -> AppResult<bool> {
    if roles.is_empty() {
        return Ok(false);
    }
    if roles.iter().any(|r| r == perms::ADMIN_ROLE) {
        return Ok(true);
    }
    let power = perms::admin_power_keys();
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM role_permissions WHERE role_key = ANY($1) AND permission = ANY($2)",
    )
    .bind(roles)
    .bind(&power)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count > 0)
}

async fn guard_recovery_authority(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    lang: Lang,
    caller_id: i64,
    caller_roles: &[String],
    target_uid: i64,
    target_roles: &[String],
) -> AppResult<()> {
    if target_uid == caller_id {
        return Err(forbidden(lang, ErrorKey::SecondAdminRequired));
    }
    if perms::is_superadmin(target_roles) && !perms::is_superadmin(caller_roles) {
        return Err(forbidden(lang, ErrorKey::SuperadminOnly));
    }
    if !perms::is_superadmin(caller_roles) && !perms::outranks(caller_roles, target_roles) {
        return Err(forbidden(lang, ErrorKey::RankTooLow));
    }
    if has_admin_power(tx, target_roles).await?
        && !perms::can_manage_admins(caller_roles)
        && !has_admin_power(tx, caller_roles).await?
    {
        return Err(forbidden(lang, ErrorKey::AdminAppointmentRestricted));
    }
    Ok(())
}

pub(super) async fn create_invite(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminInvites, Act::Create)?;
    // The invited person types their own name and email. The link carries neither.
    let token = random_token(32);
    let (id,): (i64,) = sqlx::query_as("INSERT INTO invites (token, kind, email, name, roles, group_ids, created_by, expires_at, created_at) VALUES ($1, 'invite', '', '', '', '[]', $2,$3,$4) RETURNING id")
        .bind(&token)
        .bind(u.id)
        .bind(in_days(3))
        .bind(now())
        .fetch_one(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "invite.create",
        "invite",
        Some(id),
        "open",
    )
    .await;
    Ok(Json(
        json!({"id": id, "token": token, "url": format!("{}/invite/{}", st.cfg.public_url, token)}),
    ))
}

pub(super) async fn list_invites(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminInvites, Act::View)?;
    let rows: Vec<(i64, String, String, String, String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT id, token, kind, email, name, roles, expires_at, used_at, created_at FROM invites WHERE kind <> 'family' ORDER BY id DESC LIMIT 200",
    )
    .fetch_all(&st.db)
    .await?;
    let n = now();
    // A recovery link signs its holder in as that member, so it is shown once, to the
    // admin who approved it (`approve_recovery`), and never listed again: otherwise any
    // other admin, including the one who only flagged a privileged recovery, could use it.
    Ok(Json(json!(rows.into_iter().map(|(id, token, kind, email, name, roles, exp, used, c)| json!({
        "id": id, "kind": kind, "email": email, "name": name, "roles": parse_roles(&roles), "expires_at": exp, "used_at": used, "created_at": c,
        "status": if used.is_some() { "used" } else if exp < n { "expired" } else { "open" },
        "url": if kind != "recovery" && used.is_none() && exp >= n { Some(format!("{}/invite/{}", st.cfg.public_url, token)) } else { None },
    })).collect::<Vec<_>>())))
}

pub(super) async fn revoke_invite(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminInvites, Act::Delete)?;
    let existing: Option<(Option<String>,)> =
        sqlx::query_as("SELECT used_at FROM invites WHERE id=$1 AND kind='invite'")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
    let Some((used_at,)) = existing else {
        return Err(not_found(u.lang));
    };
    if used_at.is_some() {
        return Ok(Json(json!({"ok": true})));
    }
    let res = sqlx::query(
        "UPDATE invites SET expires_at=$1 WHERE id=$2 AND kind='invite' AND used_at IS NULL",
    )
    .bind(minutes_ago(1))
    .bind(id)
    .execute(&st.db)
    .await?;
    if res.rows_affected() > 0 {
        audit(&st.db, Some(u.id), "invite.revoke", "invite", Some(id), "").await;
    }
    Ok(Json(json!({"ok": true})))
}

pub(super) async fn list_recovery(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRecovery, Act::View)?;
    let rows: Vec<(i64, String, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT r.id, r.email, r.reason, r.status, r.created_at, u.name FROM recovery_requests r LEFT JOIN users u ON u.email = r.email ORDER BY r.status IN ('pending','awaiting_second') DESC, r.id DESC LIMIT 100",
    )
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, e, reason, s, c, name)| json!({"id": id, "email": e, "reason": reason, "status": s, "created_at": c, "name": name})).collect::<Vec<_>>())))
}

/// 已审核的程序恢复账号: admin verifies identity offline, then issues a one-time 24h recovery link.
pub(super) async fn approve_recovery(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRecovery, Act::Update)?;
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_roles_mutation', 0))")
        .execute(&mut *tx)
        .await?;
    let caller_roles_raw: String = sqlx::query_scalar("SELECT roles FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&mut *tx)
        .await?;
    let caller_roles = parse_roles(&caller_roles_raw);
    if !perms::for_roles(&mut *tx, &caller_roles)
        .await?
        .has(Page::AdminRecovery, Act::Update)
    {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let email: String = sqlx::query_scalar("SELECT email FROM recovery_requests WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(not_found(u.lang))?;
    let (uid, target_status): (i64, String) =
        sqlx::query_as("SELECT id, status FROM users WHERE email=lower($1) FOR UPDATE")
            .bind(&email)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(not_found(u.lang))?;
    if target_status == "disabled" {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let (locked_email, status, resolved_by): (String, String, Option<i64>) = sqlx::query_as(
        "SELECT email, status, resolved_by FROM recovery_requests WHERE id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(not_found(u.lang))?;
    if locked_email != email {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    if !matches!(status.as_str(), "pending" | "awaiting_second") {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    let roles: String = sqlx::query_scalar("SELECT roles FROM users WHERE id=$1")
        .bind(uid)
        .fetch_one(&mut *tx)
        .await?;
    let target_roles = parse_roles(&roles);
    guard_recovery_authority(&mut tx, u.lang, u.id, &caller_roles, uid, &target_roles).await?;
    let target_privileged = perms::for_roles(&mut *tx, &target_roles)
        .await?
        .privileged();
    // Recovering a privileged account needs two different admins: the first
    // only flags the request, the second confirms and receives the link.
    if status == "pending" && target_privileged {
        let changed = sqlx::query(
            "UPDATE recovery_requests SET status='awaiting_second', resolved_by=$1 WHERE id=$2 AND status IN ('pending','awaiting_second')",
        )
        .bind(u.id)
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if changed == 0 {
            return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
        }
        tx.commit().await?;
        audit(
            &st.db,
            Some(u.id),
            "recovery.await_second",
            "recovery",
            Some(id),
            format!("user_id={uid}; email={email}"),
        )
        .await;
        return Ok(Json(json!({"awaiting_second": true})));
    }
    if status == "awaiting_second" && resolved_by == Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::SecondAdminRequired));
    }
    // Only the hash is stored (see `routes::auth::INVITE_BY_TOKEN`); the link is returned once.
    let token = random_token(32);
    // The account row was locked above, so link replacement is serialized per user.
    sqlx::query(
        "UPDATE invites SET expires_at=$1 WHERE kind='recovery' AND user_id=$2 AND used_at IS NULL",
    )
    .bind(minutes_ago(1))
    .bind(uid)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO invites (token, kind, email, user_id, created_by, expires_at, created_at) VALUES ($1, 'recovery', $2, $3, $4, $5, $6)")
        .bind(hash_token(&token))
        .bind(&email)
        .bind(uid)
        .bind(u.id)
        .bind(in_days(1))
        .bind(now())
        .execute(&mut *tx)
        .await?;
    let changed = sqlx::query("UPDATE recovery_requests SET status='approved', resolved_by=$1 WHERE id=$2 AND status IN ('pending','awaiting_second')")
        .bind(u.id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if changed.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    sqlx::query("UPDATE recovery_requests SET status='superseded' WHERE email=lower($1) AND id<>$2 AND status IN ('pending','awaiting_second')")
        .bind(&email)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    // The old phone is presumed lost: revoke its sessions, passkeys and authenticator.
    // The member registers a new passkey with the one-time link.
    sqlx::query("DELETE FROM sessions WHERE user_id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM passkeys WHERE user_id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM biometric_credentials WHERE user_id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE users SET totp_enabled=0, totp_secret=NULL, totp_last_step=0, totp_backup_codes=NULL, calendar_token_hash=NULL, telegram_chat_id='' WHERE id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM telegram_link_tokens WHERE user_id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM push_subscriptions WHERE user_id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM mobile_push_tokens WHERE user_id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM auth_challenges WHERE user_id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "recovery.approve",
        "recovery",
        Some(id),
        format!("user_id={uid}; email={email}"),
    )
    .await;
    Ok(Json(
        json!({"url": format!("{}/invite/{}", st.cfg.public_url, token)}),
    ))
}

pub(super) async fn reject_recovery(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRecovery, Act::Update)?;
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_roles_mutation', 0))")
        .execute(&mut *tx)
        .await?;
    let caller_roles_raw: String = sqlx::query_scalar("SELECT roles FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&mut *tx)
        .await?;
    let caller_roles = parse_roles(&caller_roles_raw);
    if !perms::for_roles(&mut *tx, &caller_roles)
        .await?
        .has(Page::AdminRecovery, Act::Update)
    {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let email: String = sqlx::query_scalar("SELECT email FROM recovery_requests WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(not_found(u.lang))?;
    let target_row: Option<(i64, String)> =
        sqlx::query_as("SELECT id, roles FROM users WHERE email=lower($1) FOR UPDATE")
            .bind(&email)
            .fetch_optional(&mut *tx)
            .await?;
    let user_id = target_row.as_ref().map(|(uid, _)| *uid);
    if let Some((uid, roles_raw)) = target_row {
        let target_roles = parse_roles(&roles_raw);
        guard_recovery_authority(&mut tx, u.lang, u.id, &caller_roles, uid, &target_roles).await?;
    }
    let status: String =
        sqlx::query_scalar("SELECT status FROM recovery_requests WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(not_found(u.lang))?;
    if !matches!(status.as_str(), "pending" | "awaiting_second") {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    let changed = sqlx::query("UPDATE recovery_requests SET status='rejected', resolved_by=$1 WHERE id=$2 AND status IN ('pending','awaiting_second')")
        .bind(u.id)
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "recovery.reject",
        "recovery",
        Some(id),
        format!("user_id={user_id:?}; email={email}"),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}
