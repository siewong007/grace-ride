//! Admin audit log search, pagination, and streaming CSV export.

use std::collections::HashMap;

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use chrono::NaiveDate;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{escape_like, now, require, Lang};

#[derive(Deserialize)]
pub(super) struct AuditQuery {
    entity: Option<String>,
    entity_id: Option<i64>,
    action: Option<String>,
    actor_id: Option<i64>,
    /// Text to find in the actor's name, the action, the object or the detail.
    q: Option<String>,
    /// First and last day to include, `YYYY-MM-DD`.
    from: Option<String>,
    to: Option<String>,
    /// Entries older than this id: the `next_before` of the previous page.
    before: Option<i64>,
    limit: Option<i64>,
}

/// Entries per page of the audit log unless the caller asks for fewer or more.
const AUDIT_PAGE: i64 = 100;
const AUDIT_PAGE_MAX: i64 = 200;

fn audit_attendance(action: &str, detail: &str) -> Option<(i64, &'static str)> {
    if action != "event.attendance" {
        return None;
    }
    let (member, value) = detail.strip_prefix("user ")?.split_once(" = ")?;
    let status = match value {
        "Some(true)" => "attended",
        "Some(false)" => "absent",
        "None" => "cleared",
        _ => return None,
    };
    Some((member.parse().ok()?, status))
}

/// SQL for the name of what an audit entry is about. The live record comes first; once
/// that is gone (or for roles, which have no numeric id) the name the entry itself
/// carried in `detail` stands in, so a deleted item still reads as more than a number.
fn audit_name_sql(lang: Lang) -> String {
    let guide_title = match lang {
        Lang::Zh => "title_zh",
        Lang::En => "title_en",
    };
    format!(
        r"COALESCE(
            CASE a.entity
                WHEN 'user' THEN (SELECT name FROM users WHERE id = a.entity_id)
                WHEN 'ride' THEN (SELECT p.name FROM rides r JOIN users p ON p.id = r.passenger_id WHERE r.id = a.entity_id)
                WHEN 'event' THEN (SELECT title FROM events WHERE id = a.entity_id)
                WHEN 'group' THEN (SELECT name FROM groups WHERE id = a.entity_id)
                WHEN 'area' THEN (SELECT name FROM areas WHERE id = a.entity_id)
                WHEN 'settlement' THEN (SELECT title FROM settlements WHERE id = a.entity_id)
                WHEN 'settlement_share' THEN (SELECT p.name FROM settlement_shares s JOIN users p ON p.id = s.user_id WHERE s.id = a.entity_id)
                WHEN 'reimbursement' THEN (SELECT p.name FROM reimbursements r JOIN users p ON p.id = r.user_id WHERE r.id = a.entity_id)
                WHEN 'announcement' THEN (SELECT title FROM announcements WHERE id = a.entity_id)
                WHEN 'help_ticket' THEN (SELECT subject FROM help_tickets WHERE id = a.entity_id)
                WHEN 'file' THEN (SELECT NULLIF(original_name, '') FROM files WHERE id = a.entity_id)
                WHEN 'invite' THEN (SELECT COALESCE(NULLIF(name, ''), NULLIF(email, '')) FROM invites WHERE id = a.entity_id)
                WHEN 'recovery' THEN (SELECT email FROM recovery_requests WHERE id = a.entity_id)
                WHEN 'visitor_pass' THEN (SELECT name FROM visitor_passes WHERE id = a.entity_id)
                WHEN 'guide_workflow' THEN (SELECT {guide_title} FROM guide_workflows WHERE id = a.entity_id)
                WHEN 'role' THEN CASE
                    WHEN a.action = 'role.update' THEN COALESCE(
                        (SELECT NULLIF(name, '') FROM roles WHERE key = split_part(a.detail, ':', 1)),
                        NULLIF(split_part(a.detail, ':', 1), ''))
                    ELSE substring(a.detail from '^(.*) \([a-z][a-z0-9_]*\)[:;]')
                END
            END,
            CASE
                WHEN a.action IN ('group.create', 'group.update', 'group.delete', 'area.create',
                                  'announcement.create', 'announcement.update', 'announcement.delete',
                                  'settlement.create', 'event.create', 'event.cancel')
                    THEN NULLIF(a.detail, '')
                WHEN a.action = 'settlement.delete' THEN NULLIF(regexp_replace(a.detail, '^draft: ', ''), '')
            END
        )"
    )
}

/// 管理员可查看谁在何时修改了安排。 Newest first, one page at a time: `next_before` is
/// the cursor for the next page and is null on the last one, so the whole retained log
/// can be read, not just its newest entries.
pub(super) async fn audit_log(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<AuditQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAudit, Act::View)?;
    let day = |s: &Option<String>| -> AppResult<Option<NaiveDate>> {
        match s.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => Ok(None),
            Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(Some)
                .map_err(|_| bad(u.lang, ErrorKey::InvalidDate)),
        }
    };
    // `created_at` is ISO text, so a date bounds it as a plain string comparison.
    let from = day(&q.from)?.map(|d| d.to_string());
    let until = day(&q.to)?
        .and_then(|d| d.succ_opt())
        .map(|d| d.to_string());
    // The search text is matched literally: `%` and `_` are not wildcards.
    let like =
        q.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{}%", escape_like(s)));
    let action = q.action.filter(|a| !a.is_empty());
    let entity = q.entity.filter(|e| !e.is_empty());
    let limit = q.limit.unwrap_or(AUDIT_PAGE).clamp(1, AUDIT_PAGE_MAX);
    let sql = format!(
        "SELECT a.id, a.actor_id, u.name, a.action, a.entity, a.entity_id,
                {name},
                CASE WHEN a.entity = 'ride' THEN (SELECT pickup_at FROM rides WHERE id = a.entity_id) END,
                a.detail, a.created_at
         FROM audit_logs a LEFT JOIN users u ON u.id=a.actor_id
         WHERE ($1::text IS NULL OR a.entity = $1)
           AND ($2::bigint IS NULL OR a.entity_id = $2)
           AND ($3::text IS NULL OR a.action ILIKE $3 ESCAPE '\\' OR a.entity ILIKE $3 ESCAPE '\\' OR a.detail ILIKE $3 ESCAPE '\\' OR u.name ILIKE $3 ESCAPE '\\' OR {name} ILIKE $3 ESCAPE '\\')
           AND ($4::text IS NULL OR a.created_at >= $4)
           AND ($5::text IS NULL OR a.created_at < $5)
           AND ($6::bigint IS NULL OR a.id < $6)
           AND ($7::text IS NULL OR a.action = $7)
           AND ($8::bigint IS NULL OR a.actor_id = $8)
         ORDER BY a.id DESC LIMIT $9",
        name = audit_name_sql(u.lang),
    );
    // One row more than the page shows whether another page follows.
    let mut rows: Vec<(
        i64,
        Option<i64>,
        Option<String>,
        String,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
        String,
        String,
    )> = sqlx::query_as(&sql)
        .bind(&entity)
        .bind(q.entity_id)
        .bind(&like)
        .bind(&from)
        .bind(&until)
        .bind(q.before)
        .bind(&action)
        .bind(q.actor_id)
        .bind(limit + 1)
        .fetch_all(&st.db)
        .await?;
    let more = rows.len() as i64 > limit;
    rows.truncate(limit as usize);
    let next_before = rows.last().filter(|_| more).map(|r| r.0);
    let attendance: Vec<_> = rows
        .iter()
        .map(|(_, _, _, action, _, _, _, _, detail, _)| audit_attendance(action, detail))
        .collect();
    let member_ids: Vec<_> = attendance.iter().flatten().map(|(id, _)| *id).collect();
    let member_names: HashMap<i64, String> = if member_ids.is_empty() {
        HashMap::new()
    } else {
        sqlx::query_as("SELECT id, name FROM users WHERE id = ANY($1)")
            .bind(&member_ids)
            .fetch_all(&st.db)
            .await?
            .into_iter()
            .collect()
    };
    let retention_days: i64 = sqlx::query_scalar::<_, String>(
        "SELECT value FROM system_settings WHERE key='audit_retention_days'",
    )
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten()
    .and_then(|s| s.parse::<i64>().ok())
    .unwrap_or(st.cfg.audit_retention_days);

    Ok(Json(json!({
        "items": rows.into_iter().zip(attendance).map(|((id, aid, actor, action, entity, eid, name, entity_at, detail, at), attendance)| {
            let attendance = attendance.map(|(member_id, status)| json!({
                "member_id": member_id, "member_name": member_names.get(&member_id), "status": status,
            }));
            json!({
                "id": id, "actor_id": aid, "actor": actor, "action": action, "entity": entity, "entity_id": eid,
                "entity_name": name, "entity_at": entity_at, "detail": detail, "attendance": attendance, "created_at": at,
            })
        }).collect::<Vec<_>>(),
        "next_before": next_before,
        "retention_days": retention_days,
    })))
}

#[derive(Deserialize)]
pub(super) struct AuditExportQuery {
    entity: Option<String>,
    entity_id: Option<i64>,
    action: Option<String>,
    actor_id: Option<i64>,
    q: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
}

fn escape_csv_field(s: &str) -> String {
    // Prefix single quote on formula triggers (=, +, -, @, \t, \r, |, %) to prevent spreadsheet injection.
    let trimmed = s.trim_start();
    let sanitized = if s.starts_with('\t')
        || s.starts_with('\r')
        || trimmed.starts_with('=')
        || trimmed.starts_with('+')
        || trimmed.starts_with('-')
        || trimmed.starts_with('@')
        || trimmed.starts_with('|')
        || trimmed.starts_with('%')
    {
        format!("'{s}")
    } else {
        s.to_string()
    };

    if sanitized.contains('"')
        || sanitized.contains(',')
        || sanitized.contains('\n')
        || sanitized.contains('\r')
    {
        format!("\"{}\"", sanitized.replace('"', "\"\""))
    } else {
        sanitized
    }
}

const AUDIT_EXPORT_MAX: i64 = 50_000;

/// Stream audit logs directly to client as CSV without buffering the full dataset in memory.
pub(super) async fn audit_export(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<AuditExportQuery>,
) -> AppResult<Response> {
    require(&u, Page::AdminAudit, Act::View)?;
    let day = |s: &Option<String>| -> AppResult<Option<NaiveDate>> {
        match s.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => Ok(None),
            Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(Some)
                .map_err(|_| bad(u.lang, ErrorKey::InvalidDate)),
        }
    };
    let from = day(&q.from)?.map(|d| d.to_string());
    let until = day(&q.to)?
        .and_then(|d| d.succ_opt())
        .map(|d| d.to_string());
    let like =
        q.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{}%", escape_like(s)));
    let action = q.action.filter(|a| !a.is_empty());
    let entity = q.entity.filter(|e| !e.is_empty());
    let limit = q
        .limit
        .unwrap_or(AUDIT_EXPORT_MAX)
        .clamp(1, AUDIT_EXPORT_MAX);

    let pool = st.db.clone();
    let sql = format!(
        "SELECT a.id, a.actor_id, u.name, a.action, a.entity, a.entity_id, {name}, a.detail, a.created_at \
         FROM audit_logs a LEFT JOIN users u ON u.id=a.actor_id \
         WHERE ($1::text IS NULL OR a.entity = $1) \
           AND ($2::bigint IS NULL OR a.entity_id = $2) \
           AND ($3::text IS NULL OR a.action ILIKE $3 ESCAPE '\\' OR a.entity ILIKE $3 ESCAPE '\\' OR a.detail ILIKE $3 ESCAPE '\\' OR u.name ILIKE $3 ESCAPE '\\' OR {name} ILIKE $3 ESCAPE '\\') \
           AND ($4::text IS NULL OR a.created_at >= $4) \
           AND ($5::text IS NULL OR a.created_at < $5) \
           AND ($6::text IS NULL OR a.action = $6) \
           AND ($7::bigint IS NULL OR a.actor_id = $7) \
         ORDER BY a.id DESC LIMIT $8",
        name = audit_name_sql(u.lang),
    );
    let stream = async_stream::stream! {
        // UTF-8 BOM so Excel and spreadsheet viewers recognize UTF-8 encoding (CJK text).
        yield Ok::<String, std::io::Error>("\u{FEFF}ID,Timestamp,Actor ID,Actor,Action,Entity,Entity ID,Entity Name,Detail\r\n".to_string());

        let mut rows = sqlx::query_as::<_, (i64, Option<i64>, Option<String>, String, String, Option<i64>, Option<String>, String, String)>(&sql)
        .bind(&entity)
        .bind(q.entity_id)
        .bind(&like)
        .bind(&from)
        .bind(&until)
        .bind(&action)
        .bind(q.actor_id)
        .bind(limit)
        .fetch(&pool);

        while let Some(row) = rows.next().await {
            match row {
                Ok((id, aid, actor, act, ent, eid, name, detail, at)) => {
                    let line = format!(
                        "{},{},{},{},{},{},{},{},{}\r\n",
                        id,
                        at,
                        aid.map(|v| v.to_string()).unwrap_or_default(),
                        escape_csv_field(&actor.unwrap_or_else(|| "System".to_string())),
                        escape_csv_field(&act),
                        escape_csv_field(&ent),
                        eid.map(|v| v.to_string()).unwrap_or_default(),
                        escape_csv_field(&name.unwrap_or_default()),
                        escape_csv_field(&detail),
                    );
                    yield Ok(line);
                }
                Err(e) => {
                    yield Err(std::io::Error::other(e));
                    break;
                }
            }
        }
    };

    let filename = format!(
        "audit_export_{}.csv",
        &now().replace([':', '-'], "").replace('T', "_")[..15]
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
        .body(Body::from_stream(stream))
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}
