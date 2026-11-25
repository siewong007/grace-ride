//! Shared helpers: time, audit log, notifications, permission checks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use rand::{distr::Alphanumeric, RngExt};
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult};
use crate::perms::{Act, Page};

/// Build a dynamically assembled SQL string for `sqlx::query*`.
///
/// sqlx 0.9 only accepts `&'static str` unless the caller asserts the string is safe
/// (see `sqlx::SqlSafeStr`). Every call site using this macro interpolates fixed
/// fragments — column lists, placeholders, `EXISTS`/visibility clauses — and passes
/// any user value through `.bind()`, so the assertion is sound.
#[macro_export]
macro_rules! dyn_sql {
    ($($arg:tt)*) => {
        ::sqlx::AssertSqlSafe(format!($($arg)*))
    };
}

/// The zone every stored time is written in when nothing else is configured.
pub const DEFAULT_TIMEZONE: Tz = chrono_tz::Asia::Kuala_Lumpur;

static ZONE: OnceLock<Tz> = OnceLock::new();
static DEV_MODE: AtomicBool = AtomicBool::new(false);
static PUBLIC_URL: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// Record whether the server runs in development mode so background helpers know whether
/// plaintext HTTP / loopback webhook URLs are allowed without threading `Config`.
pub fn set_dev_mode(dev: bool) {
    DEV_MODE.store(dev, Ordering::Relaxed);
}

pub fn is_dev_mode() -> bool {
    DEV_MODE.load(Ordering::Relaxed)
}

/// Record the configured `PUBLIC_URL` so background Web Push tasks can derive a valid VAPID `sub`.
pub fn set_public_url(url: &str) {
    if let Ok(mut guard) = PUBLIC_URL.write() {
        *guard = url.trim().trim_end_matches('/').to_string();
    }
}

pub fn vapid_subject() -> String {
    if let Ok(guard) = PUBLIC_URL.read() {
        if guard.starts_with("https://") {
            return guard.clone();
        }
    }
    "mailto:noreply@example.org".to_string()
}

pub fn public_url() -> String {
    PUBLIC_URL.read().map(|g| g.clone()).unwrap_or_default()
}

/// Set the app's time zone (`APP_TIMEZONE`). Stored times are local text in this
/// zone, so it is fixed for the life of the process: a second call with a different
/// zone is ignored with a warning.
pub fn set_timezone(tz: Tz) {
    if ZONE.set(tz).is_err() && zone() != tz {
        tracing::warn!("time zone already set to {}; ignoring {tz}", zone());
    }
}

/// The app's time zone. Never the host's: `TZ` on the server does not matter.
pub fn zone() -> Tz {
    *ZONE.get().unwrap_or(&DEFAULT_TIMEZONE)
}

/// The current time in the app's zone.
pub fn local_now() -> DateTime<Tz> {
    Utc::now().with_timezone(&zone())
}

/// Today's date in the app's zone.
pub fn today_local() -> NaiveDate {
    local_now().date_naive()
}

/// The stored text form of a time: `YYYY-MM-DDTHH:MM:SS`, local to the app's zone.
pub fn ts(at: DateTime<Tz>) -> String {
    at.format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn now() -> String {
    ts(local_now())
}

pub fn in_days(days: i64) -> String {
    ts(local_now() + Duration::days(days))
}

pub fn in_hours(hours: i64) -> String {
    ts(local_now() + Duration::hours(hours))
}

/// Timestamp `minutes` from now (negative: ago), in the same text format as `now()`.
pub fn in_minutes(minutes: i64) -> String {
    ts(local_now() + Duration::minutes(minutes))
}

/// Timestamp `minutes` ago, in the same text format as `now()`, for comparisons.
pub fn minutes_ago(minutes: i64) -> String {
    in_minutes(-minutes)
}

/// Longest display name accepted. Names show on every list, chat and notification.
pub const MAX_NAME_CHARS: usize = 60;

pub fn check_name(lang: Lang, name: &str) -> AppResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(bad(lang, ErrorKey::NameRequired));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(bad(lang, ErrorKey::NameTooLong));
    }
    Ok(())
}

/// Phone is optional; when given it must fit the database rule (10–20 characters).
pub fn check_phone(lang: Lang, phone: &str) -> AppResult<()> {
    let n = phone.trim().chars().count();
    if n != 0 && !(10..=20).contains(&n) {
        return Err(bad(lang, ErrorKey::PhoneInvalid));
    }
    Ok(())
}

/// Digits only. A single leading 0 is rewritten to 60 so 012… and +60 12… match.
pub fn phone_key(phone: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    match digits.strip_prefix('0') {
        Some(rest) => format!("60{rest}"),
        None => digits,
    }
}

pub fn random_token(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

/// Parse "YYYY-MM-DDTHH:MM" or "YYYY-MM-DDTHH:MM:SS" (also accepts a space separator).
pub fn parse_dt(s: &str) -> Option<NaiveDateTime> {
    let s = s.replace(' ', "T");
    NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M"))
        .ok()
}

pub fn norm_dt(s: &str) -> Option<String> {
    parse_dt(s).map(|d| d.format("%Y-%m-%dT%H:%M").to_string())
}

const AUDIT_ENTITY_NAME_EXPR: &str = "COALESCE(
    CASE $3::text
        WHEN 'user' THEN (SELECT name FROM users WHERE id = $4)
        WHEN 'ride' THEN (SELECT p.name FROM rides r JOIN users p ON p.id = r.passenger_id WHERE r.id = $4)
        WHEN 'event' THEN (SELECT title FROM events WHERE id = $4)
        WHEN 'group' THEN (SELECT name FROM groups WHERE id = $4)
        WHEN 'area' THEN (SELECT name FROM areas WHERE id = $4)
        WHEN 'settlement' THEN (SELECT title FROM settlements WHERE id = $4)
        WHEN 'settlement_share' THEN (SELECT p.name FROM settlement_shares s JOIN users p ON p.id = s.user_id WHERE s.id = $4)
        WHEN 'reimbursement' THEN (SELECT p.name FROM reimbursements r JOIN users p ON p.id = r.user_id WHERE r.id = $4)
        WHEN 'announcement' THEN (SELECT title FROM announcements WHERE id = $4)
        WHEN 'help_ticket' THEN (SELECT subject FROM help_tickets WHERE id = $4)
        WHEN 'file' THEN (SELECT NULLIF(original_name, '') FROM files WHERE id = $4)
        WHEN 'invite' THEN (SELECT COALESCE(NULLIF(name, ''), NULLIF(email, '')) FROM invites WHERE id = $4)
        WHEN 'recovery' THEN (SELECT email FROM recovery_requests WHERE id = $4)
        WHEN 'visitor_pass' THEN (SELECT name FROM visitor_passes WHERE id = $4)
        WHEN 'guide_workflow' THEN (SELECT COALESCE(NULLIF(title_zh, ''), title_en) FROM guide_workflows WHERE id = $4)
        WHEN 'role' THEN CASE
            WHEN $2 = 'role.update' THEN COALESCE(
                (SELECT NULLIF(name, '') FROM roles WHERE key = split_part($5, ':', 1)),
                NULLIF(split_part($5, ':', 1), ''))
            ELSE substring($5 from '^(.*) \\([a-z][a-z0-9_]*\\)[:;]')
        END
    END,
    CASE
        WHEN $2 IN ('group.create', 'group.update', 'group.delete', 'area.create',
                    'announcement.create', 'announcement.update', 'announcement.delete',
                    'settlement.create', 'event.create', 'event.cancel')
            THEN NULLIF($5, '')
        WHEN $2 = 'settlement.delete' THEN NULLIF(regexp_replace($5, '^draft: ', ''), '')
    END,
    ''
)";

pub async fn audit(
    db: &PgPool,
    actor: Option<i64>,
    action: &str,
    entity: &str,
    entity_id: Option<i64>,
    detail: impl AsRef<str>,
) {
    let sql = crate::dyn_sql!(
        "INSERT INTO audit_logs (actor_id, action, entity, entity_id, entity_name, detail, created_at)
         VALUES ($1,$2,$3,$4,{AUDIT_ENTITY_NAME_EXPR},$5,$6)"
    );
    let r = sqlx::query(sql)
        .bind(actor)
        .bind(action)
        .bind(entity)
        .bind(entity_id)
        .bind(detail.as_ref())
        .bind(now())
        .execute(db)
        .await;
    if let Err(e) = r {
        tracing::error!("audit insert failed: {e}");
    }
}

/// One audit row per id, in a single statement (inside a transaction when given one).
pub async fn audit_many<'e>(
    db: impl sqlx::PgExecutor<'e>,
    actor: Option<i64>,
    action: &str,
    entity: &str,
    entity_ids: &[i64],
    detail: &str,
) -> sqlx::Result<()> {
    if entity_ids.is_empty() {
        return Ok(());
    }
    let sql = crate::dyn_sql!(
        "INSERT INTO audit_logs (actor_id, action, entity, entity_id, entity_name, detail, created_at)
         SELECT $1, $2, $3, t.id, {}, $5, $6 FROM unnest($4::BIGINT[]) AS t(id)",
        AUDIT_ENTITY_NAME_EXPR.replace("$4", "t.id")
    );
    sqlx::query(sql)
        .bind(actor)
        .bind(action)
        .bind(entity)
        .bind(entity_ids)
        .bind(detail)
        .bind(now())
        .execute(db)
        .await?;
    Ok(())
}

pub use crate::i18n::{t, ErrorKey, Lang};

/// Insert one notification. Title and body are (中文, English) pairs and the
/// recipient's saved language decides which side is rendered.
pub async fn notify(
    db: &PgPool,
    user_id: i64,
    title: (&str, &str),
    body: (&str, &str),
    link: &str,
) {
    let meta = crate::notifications::infer_meta(link, title.1);
    crate::notifications::notify_with(db, user_id, title, body, link, meta).await;
}

/// Wake every member's open tab for this trip (new chat message).
pub async fn notify_trip(db: &PgPool, trip_id: i64) {
    let ids: Vec<(i64,)> = sqlx::query_as(
        "SELECT passenger_id FROM rides WHERE trip_id=$1 AND status <> 'cancelled'
         UNION SELECT driver_id FROM rides WHERE trip_id=$1 AND driver_id IS NOT NULL",
    )
    .bind(trip_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    for (id,) in ids {
        crate::stream::publish(id, "chat");
    }
}

/// The same notification for many members in one statement; each recipient gets the
/// side of each (中文, English) pair that matches their saved language. Ids that are
/// not members are skipped.
pub async fn notify_many(
    db: &PgPool,
    user_ids: &[i64],
    title: (&str, &str),
    body: (&str, &str),
    link: &str,
) {
    let meta = crate::notifications::infer_meta(link, title.1);
    crate::notifications::notify_many_with(db, user_ids, title, body, link, meta).await;
}

static ALERT_HTTP: OnceLock<reqwest::Client> = OnceLock::new();

pub fn alert_client() -> &'static reqwest::Client {
    ALERT_HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default()
    })
}

pub async fn validate_http_target(raw: &str, max_len: usize, dev_mode: bool) -> bool {
    crate::notifications::resolve_and_validate_http_target(raw, max_len, dev_mode)
        .await
        .is_some()
}

const DEFAULT_PUSH_HOST_RULES: &[&str] = &[
    "fcm.googleapis.com",
    "push.googleapis.com",
    "android.googleapis.com",
    "updates.push.services.mozilla.com",
    ".push.services.mozilla.com",
    ".push.apple.com",
    ".notify.windows.com",
    ".push.samsungosp.com",
];

fn matches_host_rule(host: &str, rule: &str) -> bool {
    let r = rule.trim().trim_end_matches('.').to_ascii_lowercase();
    if r.is_empty() {
        return false;
    }
    if let Some(suffix) = r.strip_prefix('.') {
        !suffix.is_empty() && host.len() > r.len() && host.ends_with(&r)
    } else {
        host == r
    }
}

/// True when `host` belongs to a known browser Web Push vendor (or `PUSH_ALLOWED_HOSTS` override).
pub fn is_allowed_push_host(host: &str) -> bool {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if h.is_empty() || h.parse::<std::net::IpAddr>().is_ok() {
        return false;
    }
    if let Ok(custom) = std::env::var("PUSH_ALLOWED_HOSTS") {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            return trimmed.split(',').any(|rule| matches_host_rule(&h, rule));
        }
    }
    DEFAULT_PUSH_HOST_RULES
        .iter()
        .any(|rule| matches_host_rule(&h, rule))
}

/// Validates an external alert webhook URL against SSRF targets.
pub async fn validate_webhook_target(raw: &str, dev_mode: bool) -> bool {
    validate_http_target(raw, 500, dev_mode).await
}

/// Validates a browser Web Push subscription endpoint against SSRF targets and,
/// outside `DEV_MODE`, restricts destinations to known Web Push service domains.
pub async fn validate_push_endpoint(raw: &str, dev_mode: bool) -> bool {
    if !dev_mode {
        let Ok(parsed) = url::Url::parse(raw.trim()) else {
            return false;
        };
        let Some(url::Host::Domain(domain)) = parsed.host() else {
            return false;
        };
        if !is_allowed_push_host(domain) {
            return false;
        }
    }
    validate_http_target(raw, 1000, dev_mode).await
}

/// Notify every active member who may do `act` on `page`; each recipient gets the
/// message in their own language.
pub async fn notify_perm(
    db: &PgPool,
    page: Page,
    act: Act,
    title: (&str, &str),
    body: (&str, &str),
    link: &str,
) {
    crate::notifications::notify_perm_except(db, page, act, None, title, body, link).await;
}

/// Chat system message: one shared row for the whole trip, so both languages
/// ride along (中文 / English) and every reader understands it.
pub async fn system_message(db: &PgPool, trip_id: i64, zh_body: &str, en_body: &str) {
    let _ = sqlx::query(
        "INSERT INTO ride_messages (trip_id, user_id, body, created_at) VALUES ($1,NULL,$2,$3)",
    )
    .bind(trip_id)
    .bind(if zh_body == en_body {
        zh_body.to_string()
    } else {
        format!("{zh_body} / {en_body}")
    })
    .bind(now())
    .execute(db)
    .await;
    notify_trip(db, trip_id).await;
}

/// Refuse unless the member may do `act` on `page`. A member who holds the permission
/// but signed in without a passkey / 2FA is told to sign in properly instead.
pub fn require(u: &CurrentUser, page: Page, act: Act) -> AppResult<()> {
    if u.can(page, act) {
        Ok(())
    } else if u.perms.has(page, act) {
        Err(forbidden(u.lang, ErrorKey::MfaRequired))
    } else {
        Err(forbidden(u.lang, ErrorKey::NoPermission))
    }
}

/// As `require`, for a page reached through more than one permission.
pub fn require_one(u: &CurrentUser, any: &[(Page, Act)]) -> AppResult<()> {
    if any.iter().any(|(p, a)| u.can(*p, *a)) {
        Ok(())
    } else if any.iter().any(|(p, a)| u.perms.has(*p, *a)) {
        Err(forbidden(u.lang, ErrorKey::MfaRequired))
    } else {
        Err(forbidden(u.lang, ErrorKey::NoPermission))
    }
}

pub fn fmt_rm(cents: i64) -> String {
    format!("RM {}.{:02}", cents / 100, (cents % 100).abs())
}

/// Escape `%`, `_`, and `\` for `ILIKE ... ESCAPE '\'`.
pub fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Split an opaque `"timestamp|id"` pagination cursor, falling back to plain `"timestamp"`.
pub fn split_cursor(raw: &str) -> (&str, Option<i64>) {
    match raw.rsplit_once('|') {
        Some((ts, id_str)) => match id_str.parse::<i64>() {
            Ok(id) => (ts, Some(id)),
            Err(_) => (raw, None),
        },
        None => (raw, None),
    }
}

/// Build an `ORDER BY` body from an allow-list of `(client_key, SQL expression)`.
/// Unknown keys use `default`; any direction other than `desc` sorts ascending.
/// `tiebreak` keeps paginated results stable when the sort value repeats.
pub fn order_by(
    sort: Option<&str>,
    dir: Option<&str>,
    allowed: &[(&str, &str)],
    default: &str,
    tiebreak: &str,
) -> String {
    let Some((_, column)) = sort
        .map(str::trim)
        .and_then(|key| allowed.iter().find(|(name, _)| *name == key))
    else {
        return default.to_string();
    };
    let direction = if dir
        .map(str::trim)
        .is_some_and(|d| d.eq_ignore_ascii_case("desc"))
    {
        "DESC"
    } else {
        "ASC"
    };
    format!("{column} {direction} NULLS LAST, {tiebreak}")
}

#[cfg(test)]
mod order_by_tests {
    use super::order_by;

    const ALLOWED: &[(&str, &str)] = &[("name", "u.name"), ("created", "u.created_at")];

    #[test]
    fn uses_allow_listed_column_and_direction() {
        assert_eq!(
            order_by(
                Some("name"),
                Some("desc"),
                ALLOWED,
                "u.name ASC",
                "u.id ASC"
            ),
            "u.name DESC NULLS LAST, u.id ASC"
        );
        assert_eq!(
            order_by(Some("name"), None, ALLOWED, "u.name ASC", "u.id ASC"),
            "u.name ASC NULLS LAST, u.id ASC"
        );
    }

    #[test]
    fn falls_back_for_unknown_sort_and_direction() {
        assert_eq!(
            order_by(
                Some("name; DROP TABLE users"),
                Some("desc"),
                ALLOWED,
                "u.name ASC, u.id ASC",
                "u.id ASC"
            ),
            "u.name ASC, u.id ASC"
        );
        assert_eq!(
            order_by(
                Some("created"),
                Some("sideways"),
                ALLOWED,
                "u.name ASC",
                "u.id ASC"
            ),
            "u.created_at ASC NULLS LAST, u.id ASC"
        );
    }
}

#[cfg(test)]
mod phone_key_tests {
    use super::{check_phone, phone_key};
    use crate::i18n::Lang;

    #[test]
    fn phone_key_folds_a_single_leading_zero() {
        assert_eq!(phone_key("012-345 6789"), "60123456789");
        assert_eq!(phone_key("+60 12-345 6789"), "60123456789");
        assert_eq!(phone_key("60123456789"), "60123456789");
        assert_eq!(phone_key("1234567890"), "1234567890");
    }

    #[test]
    fn check_phone_still_allows_empty_and_rejects_bad_lengths() {
        assert!(check_phone(Lang::En, "").is_ok());
        assert!(check_phone(Lang::En, "012-345 6789").is_ok());
        assert!(check_phone(Lang::En, "123").is_err());
        assert!(check_phone(Lang::En, &"1".repeat(21)).is_err());
    }
}
