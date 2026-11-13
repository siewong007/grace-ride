//! A throwaway Postgres schema per test, and cleanup of ones left by earlier runs.
//!
//! Tests can't reliably drop their own schema (no async teardown, and a panicking test
//! skips any cleanup), so each schema is labelled with its creation time and the first
//! test of every test binary drops labelled schemas older than an hour, plus unlabelled
//! ones from before labelling existed. Without this the test database collects hundreds
//! of schemas and `pg_dump` (the backup drill) runs out of lock memory.

use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::OnceCell;

/// Schemas this old belong to finished runs, never to one in progress.
const STALE_AFTER_SECS: u64 = 3600;
const LABEL: &str = "grace-ride test schema, created ";

static CLEANED: OnceCell<()> = OnceCell::const_new();

fn base_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/grace_ride_test".into())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

async fn drop_stale(pool: &sqlx::PgPool) {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT nspname::text, obj_description(oid, 'pg_namespace')
         FROM pg_namespace WHERE nspname ~ '^gr_(cap_)?[0-9a-f]{32}$'",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let cutoff = now_secs().saturating_sub(STALE_AFTER_SECS);
    for (name, label) in rows {
        let created = label
            .as_deref()
            .and_then(|l| l.strip_prefix(LABEL))
            .and_then(|t| t.parse::<u64>().ok());
        if created.is_none_or(|t| t < cutoff) {
            // One schema per statement keeps each drop's lock count small.
            let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS \"{name}\" CASCADE"))
                .execute(pool)
                .await;
        }
    }
}

/// Database URL whose search_path is a fresh schema named `{prefix}{uuid}`.
pub async fn fresh_schema(prefix: &str) -> String {
    let base = base_url();
    let pool = sqlx::PgPool::connect(&base).await.unwrap();
    CLEANED.get_or_init(|| drop_stale(&pool)).await;
    let schema = format!("{prefix}{}", uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(&format!(
        "COMMENT ON SCHEMA \"{schema}\" IS '{LABEL}{}'",
        now_secs()
    ))
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    let sep = if base.contains('?') { '&' } else { '?' };
    format!("{base}{sep}options[search_path]={schema},public")
}
