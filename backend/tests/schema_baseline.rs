//! Migrations: one squashed `0001_init.sql`, frozen from 2026-11-17. Later changes are
//! new numbered files.
//! - A history whose version 1 is this file is kept; later files run once.
//! - A history from an older schema (its version 1 differs: the pre-November numbered
//!   files, the first frozen 0001, or 0001–0005) is cleared and 0001 runs again over
//!   the existing tables, keeping every row.
//! - A later file edited after it ran, or one this build doesn't ship, stops start-up.
//!
//! Each upgrade test below undoes part of the schema on a seeded database, marks the
//! history as one of the older schemas, and connects again.

mod common;

use grace_ride::{connect_db, connect_db_with, state::DbConfig};
use sqlx::PgPool;

/// Make the history look like the five files from before the squash.
async fn mark_history_as_pre_squash(db: &PgPool) {
    sqlx::query("DELETE FROM _sqlx_migrations")
        .execute(db)
        .await
        .unwrap();
    for (version, description) in [
        (1, "init"),
        (2, "constraints_indexes"),
        (3, "trips"),
        (4, "member_roles"),
        (5, "json_columns"),
    ] {
        sqlx::query(
            "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
             VALUES ($1, $2, true, $3, 0)",
        )
        .bind(version as i64)
        .bind(description)
        .bind(vec![version as u8; 48])
        .execute(db)
        .await
        .unwrap();
    }
}

async fn versions(db: &PgPool) -> Vec<i64> {
    sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
        .fetch_all(db)
        .await
        .unwrap()
}

#[tokio::test]
async fn an_older_history_replays_the_schema_without_losing_rows() {
    let url = common::fresh_db().await;
    let db = connect_db(&url).await.expect("schema");
    sqlx::query("INSERT INTO areas (name) VALUES ('历史区')")
        .execute(&db)
        .await
        .unwrap();
    mark_history_as_pre_squash(&db).await;
    // The pre-November numbered files went up to 0016.
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES (16, 'help_claim_attach', true, $1, 0)",
    )
    .bind(vec![9u8; 8])
    .execute(&db)
    .await
    .unwrap();
    db.close().await;

    let db = connect_db(&url).await.expect("replay");
    assert_eq!(versions(&db).await, vec![1]);
    let kept: Option<(String,)> = sqlx::query_as("SELECT name FROM areas WHERE name = '历史区'")
        .fetch_optional(&db)
        .await
        .unwrap();
    assert!(kept.is_some());
    let rates: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reimbursement_rates")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(rates, 2);
    let settings: Vec<String> = sqlx::query_scalar("SELECT key FROM system_settings ORDER BY key")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(
        settings,
        [
            "audit_retention_days",
            "chat_retention_days",
            "max_live_drivers",
            "session_timeout_mins",
            "timezone"
        ]
    );
    let retired: Option<String> =
        sqlx::query_scalar("SELECT to_regclass(current_schema() || '.user_roles')::text")
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(retired.is_none());
    db.close().await;

    // The next start keeps the new history as it is.
    let db = connect_db(&url).await.expect("second start");
    assert_eq!(versions(&db).await, vec![1]);
}

/// A later file that was edited after it ran (or that this build doesn't ship) must
/// stop start-up rather than silently replay anything.
#[tokio::test]
async fn an_unknown_later_migration_stops_start_up() {
    let url = common::fresh_db().await;
    let db = connect_db(&url).await.expect("schema");
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES (2, 'from a newer build', true, $1, 0)",
    )
    .bind(vec![3u8; 48])
    .execute(&db)
    .await
    .unwrap();
    db.close().await;

    let err = connect_db(&url).await.expect_err("diverged history");
    assert!(err.to_string().contains("0002"), "{err}");
}

/// An existing row that breaks a rule must not stop a deploy: the rule still guards
/// new rows, and is left unvalidated for someone to review.
#[tokio::test]
async fn an_old_bad_row_leaves_the_rule_unvalidated_instead_of_failing() {
    let url = common::fresh_db().await;
    let db = connect_db(&url).await.expect("schema");
    sqlx::query("ALTER TABLE groups DROP CONSTRAINT chk_groups_is_all")
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO groups (name, is_all) VALUES ('旧数据', 2)")
        .execute(&db)
        .await
        .unwrap();
    mark_history_as_pre_squash(&db).await;
    db.close().await;

    let db = connect_db(&url)
        .await
        .expect("a bad old row must not fail the migration");
    let validated: bool = sqlx::query_scalar(
        "SELECT convalidated FROM pg_constraint WHERE conname = 'chk_groups_is_all' AND conrelid = 'groups'::regclass",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(!validated, "left for review");
    let bad = sqlx::query("INSERT INTO groups (name, is_all) VALUES ('新数据', 3)")
        .execute(&db)
        .await;
    assert!(bad.is_err(), "new rows are still checked");
}

/// `0001_init.sql` must not change: an edited file no longer matches the history on
/// existing databases. Put schema changes in a new numbered file.
#[test]
fn the_initial_schema_is_frozen() {
    use sha2::{Digest, Sha256};
    let sql = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/0001_init.sql"
    ))
    .unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&sql)),
        "4b83742e84488e6cf8d4722d834b831a4043d4491ccccd8487a1a98e323745c6",
        "0001_init.sql changed; add a new migration file instead"
    );
}

/// Replaying 0001 over a database full of demo and edge-case rows validates every
/// rule against that data: none of them is left unvalidated.
#[tokio::test]
async fn every_rule_holds_for_existing_seeded_data() {
    let (_app, st) = common::setup().await;
    common::seed_edge(&st).await;
    // Undo every rule added after a table was created, as an older database lacks them.
    let added: Vec<(String, String)> = sqlx::query_as(
        "SELECT conrelid::regclass::text, conname FROM pg_constraint
         WHERE connamespace = current_schema()::regnamespace AND conname = ANY($1)",
    )
    .bind(LATER_RULES)
    .fetch_all(&st.db)
    .await
    .unwrap();
    assert_eq!(added.len(), LATER_RULES.len(), "0001 added every rule");
    for (table, name) in &added {
        sqlx::query(&format!("ALTER TABLE {table} DROP CONSTRAINT {name}"))
            .execute(&st.db)
            .await
            .unwrap();
    }
    mark_history_as_pre_squash(&st.db).await;
    let db = connect_db(&st.cfg.database_url).await.expect("upgrade");
    let unvalidated: Vec<String> = sqlx::query_scalar(
        "SELECT conname::text FROM pg_constraint
         WHERE connamespace = current_schema()::regnamespace AND NOT convalidated",
    )
    .fetch_all(&db)
    .await
    .unwrap();
    assert!(unvalidated.is_empty(), "seeded rows break {unvalidated:?}");
}

const LATER_RULES: &[&str] = &[
    "chk_users_status",
    "chk_users_email_lower",
    "chk_recovery_requests_status",
    "chk_invites_kind",
    "chk_invites_group_ids",
    "chk_auth_challenges_kind",
    "chk_events_status",
    "chk_event_signups_status",
    "chk_event_signups_attended",
    "chk_event_signups_offer_seats",
    "chk_settlements_status",
    "chk_settlements_split_mode",
    "chk_settlement_items_amount",
    "chk_settlement_shares_status",
    "chk_visitor_passes_window",
    "chk_sessions_mfa_ok",
    "chk_areas_active",
    "chk_groups_is_all",
    "chk_groups_is_visitor",
    "chk_groups_not_own_parent",
    "chk_events_reminder_sent",
    "chk_event_signups_need_ride",
    "chk_event_signups_offer_drive",
    "chk_rides_needs_attention",
    "chk_rides_reminder_sent",
    "chk_rides_is_proxy",
    "chk_rides_delay_minutes",
    "chk_announcements_pinned",
    "chk_reimbursements_payee_verified",
    "chk_rides_pickup_at_format",
    "chk_events_starts_at_format",
    "chk_sessions_expires_at_format",
];

/// Trips on a database full of demo and edge-case chats: every trip in use gets a row,
/// both foreign keys hold, and rides given a trip later still work without app changes.
#[tokio::test]
async fn trips_link_rides_and_chats() {
    let (_app, st) = common::setup().await;
    common::seed_edge(&st).await;
    // Undo trips as if this database came from before they existed.
    for sql in [
        "ALTER TABLE ride_messages DROP CONSTRAINT fk_ride_messages_trip",
        "ALTER TABLE rides DROP CONSTRAINT fk_rides_trip",
        "DROP TRIGGER trg_rides_ensure_trip ON rides",
        "DROP TABLE trips",
    ] {
        sqlx::query(sql).execute(&st.db).await.unwrap();
    }
    mark_history_as_pre_squash(&st.db).await;
    let chats: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT trip_id) FROM ride_messages")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert!(chats > 0, "the seed has chats to carry over");

    let db = connect_db(&st.cfg.database_url).await.expect("upgrade");
    let missing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM (SELECT trip_id FROM rides WHERE trip_id IS NOT NULL
                               UNION SELECT trip_id FROM ride_messages) used
         WHERE NOT EXISTS (SELECT 1 FROM trips t WHERE t.id = used.trip_id)",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(missing, 0);

    // Setting a trip on a ride adds the trip; a message for an unknown trip is refused.
    let ride: i64 =
        sqlx::query_scalar("SELECT id FROM rides WHERE trip_id IS NULL ORDER BY id LIMIT 1")
            .fetch_one(&db)
            .await
            .unwrap();
    sqlx::query("UPDATE rides SET trip_id = 987654 WHERE id = $1")
        .bind(ride)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ride_messages (trip_id, body) VALUES (987654, 'hi')")
        .execute(&db)
        .await
        .expect("message on the new trip");
    let stray = sqlx::query("INSERT INTO ride_messages (trip_id, body) VALUES (987655, 'hi')")
        .execute(&db)
        .await;
    assert!(stray.is_err(), "no such trip");

    // Removing a trip takes its chat and unlinks its rides.
    sqlx::query("DELETE FROM trips WHERE id = 987654")
        .execute(&db)
        .await
        .unwrap();
    let (left, linked): (i64, Option<i64>) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM ride_messages WHERE trip_id = 987654),
                (SELECT trip_id FROM rides WHERE id = $1)",
    )
    .bind(ride)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!((left, linked), (0, None));
}

/// member_roles on a database full of demo and edge-case members: it matches each
/// member's listed roles, a name that is not a role grants nothing until the role is
/// created, and writes to users.roles (the seed, tests, start-up) keep it in step.
#[tokio::test]
async fn member_roles_follow_users_roles() {
    let (_app, st) = common::setup().await;
    common::seed_edge(&st).await;
    // Undo member_roles as if this database came from before it existed.
    for sql in [
        "DROP TRIGGER trg_roles_attach_members ON roles",
        "DROP TRIGGER trg_users_sync_member_roles ON users",
        "DROP TABLE member_roles",
    ] {
        sqlx::query(sql).execute(&st.db).await.unwrap();
    }
    mark_history_as_pre_squash(&st.db).await;
    sqlx::query("UPDATE users SET roles = 'driver, ghost_team' WHERE email = 'member@example.org'")
        .execute(&st.db)
        .await
        .unwrap();

    let db = connect_db(&st.cfg.database_url).await.expect("upgrade");
    common::assert_member_roles_in_step(&db).await;
    let held = |email: &'static str| {
        let db = db.clone();
        async move {
            sqlx::query_scalar::<_, String>(
                "SELECT mr.role_key FROM member_roles mr JOIN users u ON u.id = mr.user_id
                 WHERE u.email = $1 ORDER BY 1",
            )
            .bind(email)
            .fetch_all(&db)
            .await
            .unwrap()
        }
    };
    assert_eq!(held("member@example.org").await, ["driver"]);
    let admins: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM member_roles WHERE role_key = 'admin'")
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(admins > 0, "seeded admins carried over");

    // The role is created later: the member who already listed it now holds it.
    sqlx::query("INSERT INTO roles (key, name) VALUES ('ghost_team', 'Ghost team')")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(held("member@example.org").await, ["driver", "ghost_team"]);

    // Taking a role off in users.roles takes it out of member_roles.
    sqlx::query("UPDATE users SET roles = 'ghost_team' WHERE email = 'member@example.org'")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(held("member@example.org").await, ["ghost_team"]);
    common::assert_member_roles_in_step(&db).await;
}

/// A seeded database whose JSON columns are still text: they become JSONB, valid JSON
/// keeps its shape, text that isn't JSON is kept as a JSON string, and the database
/// then refuses anything that isn't JSON.
#[tokio::test]
async fn json_columns_become_jsonb() {
    let (_app, st) = common::setup().await;
    common::seed_edge(&st).await;
    // Undo JSONB as if this database came from before it.
    for sql in [
        "ALTER TABLE invites DROP CONSTRAINT chk_invites_group_ids",
        "ALTER TABLE invites ALTER COLUMN group_ids DROP DEFAULT",
        "ALTER TABLE invites ALTER COLUMN group_ids TYPE text USING group_ids::text",
        "ALTER TABLE invites ALTER COLUMN group_ids SET DEFAULT '[]'",
        "ALTER TABLE settlement_revisions ALTER COLUMN snapshot TYPE text USING snapshot::text",
        "UPDATE settlement_revisions SET snapshot = 'initial draft' WHERE id = 63001",
    ] {
        sqlx::query(sql).execute(&st.db).await.unwrap();
    }
    mark_history_as_pre_squash(&st.db).await;

    let db = connect_db(&st.cfg.database_url).await.expect("upgrade");
    let types: Vec<String> = sqlx::query_scalar(
        "SELECT data_type::text FROM information_schema.columns WHERE table_schema = current_schema()
         AND (table_name, column_name) IN (('settlement_revisions', 'snapshot'), ('invites', 'group_ids'))",
    )
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(types, ["jsonb", "jsonb"]);
    let (kept, parsed): (String, String) = sqlx::query_as(
        "SELECT (SELECT jsonb_typeof(snapshot) FROM settlement_revisions WHERE id = 63001),
                (SELECT snapshot->>'title' FROM settlement_revisions WHERE id = 43001)",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(
        (kept.as_str(), parsed.as_str()),
        ("string", "上周羽毛球费用")
    );
    let validated: bool = sqlx::query_scalar(
        "SELECT convalidated FROM pg_constraint WHERE conname = 'chk_invites_group_ids' AND conrelid = 'invites'::regclass",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(validated);

    let bad = sqlx::query(
        "UPDATE settlement_revisions SET snapshot = 'not json'::text::jsonb WHERE id = 43001",
    )
    .execute(&db)
    .await;
    assert!(bad.is_err(), "not JSON");
    let bad = sqlx::query(
        "INSERT INTO invites (token, email, group_ids, created_by, expires_at)
         VALUES ('t-json', '', '{}', (SELECT MIN(id) FROM users), '2099-01-01T00:00:00')",
    )
    .execute(&db)
    .await;
    assert!(bad.is_err(), "groups are a list");
    sqlx::query(
        "INSERT INTO invites (token, email, created_by, expires_at)
         VALUES ('t-json', '', (SELECT MIN(id) FROM users), '2099-01-01T00:00:00')",
    )
    .execute(&db)
    .await
    .expect("the default is an empty list");
}

/// An older database whose time defaults named Asia/Kuala_Lumpur outright, and whose
/// emails were not all lowercase: replaying 0001 points the defaults at app_now() and
/// lowercases the emails (unless that would collide with another account).
#[tokio::test]
async fn older_defaults_and_mixed_case_emails_are_brought_in_line() {
    let url = common::fresh_db().await;
    let db = connect_db(&url).await.expect("schema");
    for sql in [
        "ALTER TABLE users DROP CONSTRAINT chk_users_email_lower",
        "ALTER TABLE areas ADD COLUMN created_at TEXT NOT NULL DEFAULT (to_char(NOW() AT TIME ZONE 'Asia/Kuala_Lumpur', 'YYYY-MM-DD\"T\"HH24:MI:SS'))",
        "INSERT INTO users (name, email, webauthn_uuid) VALUES ('甲', 'Alice@Example.org', 'u1'), ('乙', 'BOB@example.org', 'u2'), ('丙', 'bob@example.org', 'u3')",
    ] {
        sqlx::query(sql).execute(&db).await.unwrap();
    }
    mark_history_as_pre_squash(&db).await;
    db.close().await;

    let db = connect_db(&url).await.expect("upgrade");
    let default: String = sqlx::query_scalar(
        "SELECT column_default FROM information_schema.columns
         WHERE table_schema = current_schema() AND table_name = 'areas' AND column_name = 'created_at'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(default, "app_now()");
    let emails: Vec<String> = sqlx::query_scalar("SELECT email FROM users ORDER BY webauthn_uuid")
        .fetch_all(&db)
        .await
        .unwrap();
    // BOB@ would collide with bob@, so it is left for someone to merge by hand.
    assert_eq!(
        emails,
        ["alice@example.org", "BOB@example.org", "bob@example.org"]
    );
    let validated: bool = sqlx::query_scalar(
        "SELECT convalidated FROM pg_constraint WHERE conname = 'chk_users_email_lower' AND conrelid = 'users'::regclass",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(!validated, "the collision is left for review");
    let bad = sqlx::query(
        "INSERT INTO users (name, email, webauthn_uuid) VALUES ('丁', 'Dan@example.org', 'u4')",
    )
    .execute(&db)
    .await;
    assert!(bad.is_err(), "new emails must be lowercase");
}

/// APP_TIMEZONE decides the zone of every time the database writes by itself
/// (app_now() defaults) and the session zone the app's queries run in.
#[tokio::test]
async fn the_time_zone_follows_the_configuration() {
    let url = common::fresh_db().await;
    let cfg = DbConfig {
        timezone: chrono_tz::Pacific::Auckland,
        ..DbConfig::default()
    };
    let db = connect_db_with(&url, &cfg).await.expect("schema");
    let (stored, session, written): (String, String, String) = sqlx::query_as(
        "SELECT (SELECT value FROM system_settings WHERE key = 'timezone'),
                current_setting('TimeZone'),
                app_now()",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(stored, "Pacific/Auckland");
    assert_eq!(session, "Pacific/Auckland");
    let expected = chrono::Utc::now().with_timezone(&chrono_tz::Pacific::Auckland);
    let written = chrono::NaiveDateTime::parse_from_str(&written, "%Y-%m-%dT%H:%M:%S").unwrap();
    let drift = (expected.naive_local() - written).num_seconds().abs();
    assert!(drift < 60, "app_now() is Auckland time (off by {drift}s)");
    // LOCALTIMESTAMP agrees with app_now() in the app's sessions.
    let agree: bool = sqlx::query_scalar(
        "SELECT abs(extract(epoch FROM LOCALTIMESTAMP - app_now()::timestamp)) < 60",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(agree);
    db.close().await;

    // Back to the default zone: the setting follows.
    let db = connect_db(&url).await.expect("restart");
    let stored: String =
        sqlx::query_scalar("SELECT value FROM system_settings WHERE key = 'timezone'")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(stored, "Asia/Kuala_Lumpur");
}

/// Trigger functions carry their own search_path, so a data-only restore (which runs
/// with an empty search_path) still finds member_roles and trips.
#[tokio::test]
async fn trigger_functions_pin_their_search_path() {
    let url = common::fresh_db().await;
    let db = connect_db(&url).await.expect("schema");
    let unpinned: Vec<String> = sqlx::query_scalar(
        "SELECT proname::text FROM pg_proc
         WHERE pronamespace = current_schema()::regnamespace
           AND proname = ANY($1)
           AND NOT EXISTS (SELECT 1 FROM unnest(proconfig) c WHERE c LIKE 'search_path=%')",
    )
    .bind([
        "app_now",
        "listed_role_keys",
        "sync_member_roles",
        "users_sync_member_roles",
        "roles_attach_members",
        "rides_ensure_trip",
    ])
    .fetch_all(&db)
    .await
    .unwrap();
    assert!(unpinned.is_empty(), "{unpinned:?}");
    let schema: String = sqlx::query_scalar("SELECT current_schema()::text")
        .fetch_one(&db)
        .await
        .unwrap();
    let mut tx = db.begin().await.unwrap();
    sqlx::query("SET LOCAL search_path = ''")
        .execute(&mut *tx)
        .await
        .unwrap();
    // The insert fires roles_attach_members (member_roles, users) and app_now()
    // (system_settings), all named without a schema.
    sqlx::query(&format!(
        "INSERT INTO \"{schema}\".roles (key, name) VALUES ('restore_check', 'Restore check')"
    ))
    .execute(&mut *tx)
    .await
    .expect("trigger and default resolve their tables with an empty search_path");
    tx.rollback().await.unwrap();
}
