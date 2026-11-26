//! Account recovery requests, invites preview/acceptance, and terms consent.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{create_session, visitor_lang, INVITE_BY_TOKEN};
use crate::auth::{hash_token, CurrentUser};
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, check_name, check_phone, minutes_ago, notify_perm, now};

// ---------- Recovery ----------

#[derive(Deserialize)]
pub(super) struct RecoveryReq {
    email: String,
    reason: String,
    #[serde(default)]
    turnstile_token: Option<String>,
}

pub(super) async fn recovery_request(
    State(st): State<AppState>,
    peer: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Json(req): Json<RecoveryReq>,
) -> AppResult<Json<Value>> {
    let lang = visitor_lang(&headers);
    let ip = crate::security::client_ip(&headers, peer.map(|p| p.0 .0), st.cfg.trust_proxy);
    st.turnstile
        .verify(
            req.turnstile_token.as_deref(),
            "recovery_request",
            Some(&ip),
            lang,
        )
        .await?;
    // Always answer the same way so the endpoint can't be used to probe member emails.
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || email.len() > 254 {
        return Ok(Json(json!({ "ok": true })));
    }
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE email=$1")
        .bind(&email)
        .fetch_optional(&st.db)
        .await?;
    if let Some((uid,)) = exists {
        sqlx::query("INSERT INTO recovery_requests (email, reason, created_at) VALUES ($1,$2,$3)")
            .bind(&email)
            .bind(req.reason.chars().take(500).collect::<String>())
            .bind(now())
            .execute(&st.db)
            .await?;
        // No actor: the request comes from someone who cannot sign in.
        audit(
            &st.db,
            None,
            "recovery.request",
            "user",
            Some(uid),
            req.email.trim(),
        )
        .await;
        notify_perm(
            &st.db,
            Page::AdminRecovery,
            Act::Update,
            ("新的账号恢复申请", "New account recovery request"),
            (
                &format!("会员 {} 提交了账号恢复申请", req.email),
                &format!("Member {} submitted an account recovery request", req.email),
            ),
            "/admin/members",
        )
        .await;
    }
    Ok(Json(json!({ "ok": true })))
}

// ---------- Invites ----------

pub(super) async fn invite_preview(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> AppResult<Json<Value>> {
    let lang = visitor_lang(&headers);
    let row: Option<(
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
    )> = sqlx::query_as(crate::dyn_sql!(
        "SELECT invites.kind, invites.email, invites.name, invites.expires_at, invites.used_at,
                invites.phone, f.name
         FROM invites
         LEFT JOIN families f ON f.id = invites.family_id
         WHERE {INVITE_BY_TOKEN}"
    ))
    .bind(&token)
    .bind(hash_token(&token))
    .fetch_optional(&st.db)
    .await?;
    let Some((kind, email, name, expires, used, phone, family_name)) = row else {
        return Err(not_found(lang));
    };
    if used.is_some() || expires < now() {
        return Err(bad(lang, ErrorKey::InviteExpired));
    }
    if kind == "family" {
        return Ok(Json(json!({
            "kind": kind,
            "email": email,
            "name": name,
            "family_name": family_name.unwrap_or_default(),
            "phone": phone,
        })));
    }
    Ok(Json(json!({ "kind": kind, "email": email, "name": name })))
}

/// Version of the terms and privacy notice in `frontend/src/locales/terms.*.ts`. Raise it
/// when their meaning changes (and the column default in a new migration, see 0007): every
/// member who agreed to an older version is asked to opt in again.
pub const TERMS_VERSION: i32 = 3;

#[derive(Deserialize)]
pub(super) struct TermsIn {
    accepted: bool,
}

pub(super) async fn accept_terms(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<TermsIn>,
) -> AppResult<Json<Value>> {
    if !req.accepted {
        return Err(bad(u.lang, ErrorKey::TermsRequired));
    }
    // Each opt-in records when and to which version, so a later version asks again.
    sqlx::query("UPDATE users SET terms_accepted_at=$1, terms_version=$2 WHERE id=$3")
        .bind(now())
        .bind(TERMS_VERSION)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "terms.accept",
        "user",
        Some(u.id),
        &format!("version {TERMS_VERSION}"),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(super) struct AcceptReq {
    name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    terms: Option<bool>,
    #[serde(default)]
    turnstile_token: Option<String>,
}

pub(super) async fn invite_accept(
    State(st): State<AppState>,
    peer: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(token): Path<String>,
    Json(req): Json<AcceptReq>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let lang = visitor_lang(&headers);
    let ip = crate::security::client_ip(&headers, peer.map(|p| p.0 .0), st.cfg.trust_proxy);
    st.turnstile
        .verify(
            req.turnstile_token.as_deref(),
            "invite_accept",
            Some(&ip),
            lang,
        )
        .await?;
    let row: Option<(
        i64,
        String,
        String,
        String,
        String,
        String,
        Option<i64>,
        String,
        Option<String>,
        String,
        Option<i64>,
    )> = sqlx::query_as(
        crate::dyn_sql!("SELECT id, kind, email, name, roles, group_ids::text, user_id, expires_at, used_at, phone, family_id FROM invites WHERE {INVITE_BY_TOKEN}"),
    )
    .bind(&token)
    .bind(hash_token(&token))
    .fetch_optional(&st.db)
    .await?;
    let Some((
        inv_id,
        kind,
        email,
        inv_name,
        _roles,
        _group_ids,
        rec_user,
        expires,
        used,
        invite_phone,
        family_id,
    )) = row
    else {
        return Err(not_found(lang));
    };
    if used.is_some() || expires < now() {
        return Err(bad(lang, ErrorKey::InviteExpired));
    }
    if req.terms != Some(true) {
        return Err(bad(lang, ErrorKey::TermsRequired));
    }
    let joined_email;
    let joined_name;
    if kind == "recovery" {
        joined_email = email;
        joined_name = inv_name;
    } else {
        joined_name = req
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();
        joined_email = req.email.unwrap_or_default().trim().to_lowercase();
        check_name(lang, &joined_name)?;
        // A family link locks the phone the owner typed. The form value is ignored, so a
        // short or empty body phone must not reject the link or be stored.
        if kind != "family" {
            // Checked before the invite is marked used: a value the database would refuse
            // used to burn the single-use link and then report a misleading error.
            check_phone(lang, req.phone.as_deref().unwrap_or(""))?;
        }
        if !joined_email.contains('@') {
            return Err(bad(lang, ErrorKey::EmailInvalid));
        }
        let (exists,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email=$1")
            .bind(&joined_email)
            .fetch_one(&st.db)
            .await?;
        if exists > 0 {
            return Err(bad(lang, ErrorKey::EmailAlreadyRegistered));
        }
    }
    let stored_phone = if kind == "family" {
        invite_phone
    } else {
        req.phone.as_deref().unwrap_or("").trim().to_string()
    };
    // Consuming the invite and opening the account happen together or not at all: an
    // account insert that fails (e.g. a concurrent signup took the email) rolls the invite
    // back to unused, so the person can retry with a corrected email. The conditional
    // UPDATE still makes the link single use under double submit.
    let mut tx = st.db.begin().await?;
    if kind == "recovery" {
        let id = rec_user.ok_or_else(|| bad(lang, ErrorKey::RecoveryLinkInvalid))?;
        let u_status: Option<(String,)> =
            sqlx::query_as("SELECT status FROM users WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        if !matches!(
            u_status.as_ref().map(|s| s.0.as_str()),
            Some("active" | "pending")
        ) {
            return Err(forbidden(lang, ErrorKey::AuthDisabled));
        }
    }
    let ts_now = now();
    let r = sqlx::query(
        "UPDATE invites SET used_at=$1, email=$2, name=$3 WHERE id=$4 AND used_at IS NULL AND expires_at >= $1",
    )
    .bind(&ts_now)
    .bind(&joined_email)
    .bind(&joined_name)
    .bind(inv_id)
    .execute(&mut *tx)
    .await?;
    if r.rows_affected() == 0 {
        return Err(bad(lang, ErrorKey::InviteAlreadyUsed));
    }

    if kind == "family" {
        let Some(family_id) = family_id else {
            return Err(not_found(lang));
        };
        let locked: Option<i64> =
            sqlx::query_scalar("SELECT id FROM families WHERE id=$1 FOR UPDATE")
                .bind(family_id)
                .fetch_optional(&mut *tx)
                .await?;
        if locked.is_none() {
            return Err(not_found(lang));
        }
        let counted: i64 = sqlx::query_scalar(
            "SELECT
                (SELECT COUNT(*) FROM family_members WHERE family_id = $1)
              + (SELECT COUNT(*) FROM family_requests WHERE family_id = $1)
              + (SELECT COUNT(*) FROM invites
                 WHERE family_id = $1 AND kind = 'family' AND used_at IS NULL AND expires_at > $2
                   AND id <> $3)",
        )
        .bind(family_id)
        .bind(now())
        .bind(inv_id)
        .fetch_one(&mut *tx)
        .await?;
        if counted + 1 > crate::routes::community::FAMILY_CAP {
            return Err(bad(
                lang,
                ErrorKey::FamilyFull {
                    max: crate::routes::community::FAMILY_CAP,
                },
            ));
        }
    }

    let uid = if kind == "recovery" {
        let id = rec_user.ok_or_else(|| bad(lang, ErrorKey::RecoveryLinkInvalid))?;
        let u_status: Option<(String,)> = sqlx::query_as("SELECT status FROM users WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        if !matches!(
            u_status.as_ref().map(|s| s.0.as_str()),
            Some("active" | "pending")
        ) {
            return Err(forbidden(lang, ErrorKey::AuthDisabled));
        }
        // A recovery link signs in as the member. Once one is used, no sibling
        // recovery link may remain live (e.g. one issued before a replacement).
        sqlx::query("UPDATE invites SET expires_at=$1 WHERE kind='recovery' AND user_id=$2 AND id<>$3 AND used_at IS NULL")
            .bind(minutes_ago(1))
            .bind(id)
            .bind(inv_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE users SET terms_accepted_at=$1, terms_version=$2 WHERE id=$3")
            .bind(now())
            .bind(TERMS_VERSION)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        id
    } else {
        // The invite only opens a profile. Role and group are assigned when an admin approves.
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid, terms_accepted_at, terms_version) VALUES ($1,$2,$3,'','pending',$4,$5,$6,$7) RETURNING id",
        )
        .bind(&joined_name)
        .bind(&joined_email)
        .bind(&stored_phone)
        .bind(None::<String>)
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(now())
        .bind(TERMS_VERSION)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| bad(lang, ErrorKey::EmailAlreadyRegistered))?;
        if kind == "family" {
            let Some(family_id) = family_id else {
                return Err(not_found(lang));
            };
            sqlx::query(
                "INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1, $2, $3)",
            )
            .bind(family_id)
            .bind(id)
            .bind(now())
            .execute(&mut *tx)
            .await?;
        }
        id
    };
    tx.commit().await?;
    let jar = create_session(&st, jar, uid, false).await?;
    if kind != "recovery" {
        // A new profile can do nothing until it is approved, so tell the people who approve it.
        notify_perm(
            &st.db,
            Page::AdminMembers,
            Act::Update,
            ("新成员等待审核", "New member waiting for approval"),
            (
                &format!("{joined_name} 已通过邀请链接加入，请分配职务与群体。"),
                &format!("{joined_name} joined from an invite link. Assign their role and group."),
            ),
            "/admin/members?status=pending",
        )
        .await;
    }
    audit(
        &st.db,
        Some(uid),
        if kind == "recovery" {
            "recovery.use"
        } else {
            "invite.accept"
        },
        "user",
        Some(uid),
        &joined_email,
    )
    .await;
    Ok((jar, Json(json!({ "ok": true, "kind": kind }))))
}
