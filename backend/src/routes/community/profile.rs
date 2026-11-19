//! Member profile update endpoints.

use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{audit, check_name, check_phone, now};

// ---------------- Profile ----------------

/// Avatar background colours. The client maps each key to a themed CSS class, so no
/// member-supplied colour ever reaches a style attribute.
pub const AVATAR_COLORS: &[&str] = &[
    "purple", "teal", "coral", "pink", "blue", "green", "amber", "gray",
];
/// Avatar icons, drawn by the client from its own icon set.
pub const AVATAR_ICONS: &[&str] = &[
    "car", "heart", "star", "music", "sun", "leaf", "coffee", "book", "church", "bike", "smile",
    "flower",
];

/// Accepts "" (reset) or a key from `allowed`.
fn check_key(lang: crate::i18n::Lang, v: &Option<String>, allowed: &[&str]) -> AppResult<()> {
    match v.as_deref() {
        None | Some("") => Ok(()),
        Some(k) if allowed.contains(&k) => Ok(()),
        Some(_) => Err(bad(lang, ErrorKey::InvalidAction)),
    }
}

#[derive(Deserialize)]
pub(super) struct ProfileIn {
    /// Roughly every field is optional; callers may only send what changes,
    /// e.g. { "lang": "en" } from the language switcher.
    name: Option<String>,
    phone: Option<String>,
    photo_file_id: Option<i64>,
    /// Removes the profile photo; the avatar falls back to the icon or initial.
    clear_photo: Option<bool>,
    /// A key from [`AVATAR_COLORS`]; "" resets to the default background.
    avatar_color: Option<String>,
    /// A key from [`AVATAR_ICONS`]; "" shows the initial instead.
    avatar_icon: Option<String>,
    badges_opt_out: Option<bool>,
    lang: Option<String>,
    readiness_prompt_mins: Option<i64>,
    auto_lock_mins: Option<i64>,
    telegram_chat_id: Option<String>,
    whatsapp_opt_in: Option<bool>,
    push_opt_in: Option<bool>,
    caregiver_user_id: Option<i64>,
    clear_caregiver: Option<bool>,
    /// Hides the Getting started checklist on Home. Only `true` does anything: it is not un-hidden.
    onboarding_dismissed: Option<bool>,
}

pub(super) async fn update_profile(
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
    if let Some(m) = req.readiness_prompt_mins {
        if !(0..=180).contains(&m) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    if let Some(m) = req.auto_lock_mins {
        if !(-1..=1440).contains(&m) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    check_key(u.lang, &req.avatar_color, AVATAR_COLORS)?;
    check_key(u.lang, &req.avatar_icon, AVATAR_ICONS)?;
    if let Some(f) = req.photo_file_id {
        let (owner, mime): (i64, String) =
            sqlx::query_as("SELECT owner_id, mime FROM files WHERE id=$1")
                .bind(f)
                .fetch_optional(&st.db)
                .await?
                .ok_or(not_found(u.lang))?;
        if owner != u.id {
            return Err(forbidden(u.lang, ErrorKey::InvalidPhoto));
        }
        if !mime.starts_with("image/") {
            return Err(bad(u.lang, ErrorKey::InvalidPhoto));
        }
    }
    if let Some(cg) = req.caregiver_user_id {
        if cg == u.id {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        let ok: Option<(i64,)> = sqlx::query_as(
            "SELECT cg.id FROM users cg
             WHERE cg.id=$1 AND cg.status='active'
               AND EXISTS (
                   SELECT 1 FROM group_members g1
                   JOIN group_members g2 ON g2.group_id = g1.group_id
                   WHERE g1.user_id = $1 AND g2.user_id = $2
               )",
        )
        .bind(cg)
        .bind(u.id)
        .fetch_optional(&st.db)
        .await?;
        if ok.is_none() {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    let tg = req
        .telegram_chat_id
        .as_deref()
        .map(|s| s.trim().chars().take(64).collect::<String>());
    sqlx::query(
        "UPDATE users SET
            name=COALESCE($1, name),
            phone=COALESCE($2, phone),
            photo_file_id=CASE WHEN $16 THEN NULL ELSE COALESCE($3, photo_file_id) END,
            avatar_color=CASE WHEN $17::TEXT IS NULL THEN avatar_color ELSE NULLIF($17, '') END,
            avatar_icon=CASE WHEN $18::TEXT IS NULL THEN avatar_icon ELSE NULLIF($18, '') END,
            badges_opt_out=COALESCE($4, badges_opt_out),
            lang=COALESCE($5, lang),
            readiness_prompt_mins=COALESCE($6, readiness_prompt_mins),
            auto_lock_mins=COALESCE($7, auto_lock_mins),
            telegram_chat_id=COALESCE($8, telegram_chat_id),
            whatsapp_opt_in=COALESCE($9, whatsapp_opt_in),
            push_opt_in=COALESCE($10, push_opt_in),
            caregiver_user_id=CASE WHEN $11 THEN NULL ELSE COALESCE($12, caregiver_user_id) END,
            onboarding_dismissed_at=CASE WHEN $14 THEN COALESCE(onboarding_dismissed_at, $15) ELSE onboarding_dismissed_at END
         WHERE id=$13",
    )
    .bind(req.name.as_deref().map(str::trim))
    .bind(req.phone.as_deref().map(str::trim))
    .bind(req.photo_file_id)
    .bind(req.badges_opt_out.map(|b| b as i64))
    .bind(req.lang.as_deref())
    .bind(req.readiness_prompt_mins)
    .bind(req.auto_lock_mins)
    .bind(tg.as_deref())
    .bind(req.whatsapp_opt_in.map(|b| b as i64))
    .bind(req.push_opt_in.map(|b| b as i64))
    .bind(req.clear_caregiver.unwrap_or(false))
    .bind(req.caregiver_user_id)
    .bind(u.id)
    .bind(req.onboarding_dismissed == Some(true))
    .bind(now())
    .bind(req.clear_photo.unwrap_or(false))
    .bind(req.avatar_color.as_deref())
    .bind(req.avatar_icon.as_deref())
    .execute(&st.db)
    .await?;
    // Which fields were sent, not their values: the log is read by staff.
    let changed: Vec<&str> = [
        ("name", req.name.is_some()),
        ("phone", req.phone.is_some()),
        (
            "photo",
            req.photo_file_id.is_some() || req.clear_photo.unwrap_or(false),
        ),
        (
            "avatar",
            req.avatar_color.is_some() || req.avatar_icon.is_some(),
        ),
        ("badges_opt_out", req.badges_opt_out.is_some()),
        ("lang", req.lang.is_some()),
        ("readiness_prompt", req.readiness_prompt_mins.is_some()),
        ("auto_lock", req.auto_lock_mins.is_some()),
        (
            "alerts",
            req.telegram_chat_id.is_some()
                || req.whatsapp_opt_in.is_some()
                || req.push_opt_in.is_some(),
        ),
        (
            "caregiver",
            req.caregiver_user_id.is_some() || req.clear_caregiver.unwrap_or(false),
        ),
        ("onboarding", req.onboarding_dismissed.is_some()),
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
