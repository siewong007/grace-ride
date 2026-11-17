//! Migrations: `0001_init.sql` is frozen and later changes are new numbered files.
//! - A history that is the start of this build's files is kept; new files run once.
//! - A history from the old numbered files (other contents under the same versions)
//!   is cleared, the scripts run again, and existing rows stay.

mod common;

use grace_ride::connect_db;

#[tokio::test]
async fn stale_migration_history_replays_the_schema_without_losing_rows() {
    let url = common::fresh_db().await;
    let db = connect_db(&url).await.expect("schema");
    sqlx::query("INSERT INTO areas (name) VALUES ('历史区')")
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES (16, 'help_claim_attach', true, $1, 0)",
    )
    .bind(vec![9u8; 8])
    .execute(&db)
    .await
    .unwrap();
    // The old 0002 had other contents under the same version.
    sqlx::query("UPDATE _sqlx_migrations SET checksum = $1, description = 'passenger_locations' WHERE version = 2")
        .bind(vec![9u8; 8])
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("UPDATE _sqlx_migrations SET checksum = $1 WHERE version = 1")
        .bind(vec![1u8; 8])
        .execute(&db)
        .await
        .unwrap();
    db.close().await;

    let db = connect_db(&url).await.expect("rebase");
    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(versions, vec![1, 2, 3, 4, 5]);
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
    let settings: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM system_settings")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(settings, 4);
    let assignee: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.columns
         WHERE table_schema = current_schema()
           AND table_name = 'help_tickets' AND column_name = 'assignee_id'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(assignee, 1);
    let retired: Option<String> =
        sqlx::query_scalar("SELECT to_regclass(current_schema() || '.user_roles')::text")
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(retired.is_none());
    db.close().await;

    let db = connect_db(&url).await.expect("second start");
    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(versions, vec![1, 2, 3, 4, 5]);
}

/// A database that has run 0001 only (every database before 2026-11-17) keeps its
/// history and gains 0002 on the next start, without clearing anything.
#[tokio::test]
async fn new_migration_files_run_once_after_the_frozen_schema() {
    let url = common::fresh_db().await;
    let db = connect_db(&url).await.expect("schema");
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version > 1")
        .execute(&db)
        .await
        .unwrap();
    let first_run: Vec<u8> =
        sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 1")
            .fetch_one(&db)
            .await
            .unwrap();
    let installed_on: String =
        sqlx::query_scalar("SELECT installed_on::text FROM _sqlx_migrations WHERE version = 1")
            .fetch_one(&db)
            .await
            .unwrap();
    db.close().await;

    let db = connect_db(&url).await.expect("upgrade");
    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(versions, vec![1, 2, 3, 4, 5]);
    // 0001 was left alone (same row), not cleared and replayed.
    let (checksum, installed): (Vec<u8>, String) = sqlx::query_as(
        "SELECT checksum, installed_on::text FROM _sqlx_migrations WHERE version = 1",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!((checksum, installed), (first_run, installed_on));
    // 0002's rules are in force.
    let bad = sqlx::query("INSERT INTO groups (name, is_all) VALUES ('坏数据', 5)")
        .execute(&db)
        .await;
    assert!(bad.is_err(), "a flag must be 0 or 1");
}

/// An existing row that breaks a new rule must not stop a deploy: the rule still
/// guards new rows, and is left unvalidated for someone to review.
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
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version > 1")
        .execute(&db)
        .await
        .unwrap();
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
        "d3e06f39dd6bdebae1fe06f8010268b3192ef5f4804bf7c52641f60fabbed87d",
        "0001_init.sql changed; add a new migration file instead"
    );
}

/// Upgrading a database full of demo and edge-case rows to 0002 validates every new
/// rule against that data: none of them is left unvalidated.
#[tokio::test]
async fn the_new_rules_hold_for_existing_seeded_data() {
    let (_app, st) = common::setup().await;
    common::seed_edge(&st).await;
    // Undo 0002 as if this database had only ever run 0001, then upgrade it.
    let added: Vec<(String, String)> = sqlx::query_as(
        "SELECT conrelid::regclass::text, conname FROM pg_constraint
         WHERE connamespace = current_schema()::regnamespace AND conname = ANY($1)",
    )
    .bind(RULES_0002)
    .fetch_all(&st.db)
    .await
    .unwrap();
    assert_eq!(added.len(), RULES_0002.len(), "0002 added every rule");
    for (table, name) in &added {
        sqlx::query(&format!("ALTER TABLE {table} DROP CONSTRAINT {name}"))
            .execute(&st.db)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version > 1")
        .execute(&st.db)
        .await
        .unwrap();
    let url = st.cfg.database_url.clone();
    let db = connect_db(&url).await.expect("upgrade");
    let unvalidated: Vec<String> = sqlx::query_scalar(
        "SELECT conname::text FROM pg_constraint
         WHERE connamespace = current_schema()::regnamespace AND conname = ANY($1) AND NOT convalidated",
    )
    .bind(RULES_0002)
    .fetch_all(&db)
    .await
    .unwrap();
    assert!(unvalidated.is_empty(), "seeded rows break {unvalidated:?}");
}

const RULES_0002: &[&str] = &[
    "chk_users_status",
    "chk_recovery_requests_status",
    "chk_invites_kind",
    "chk_auth_challenges_kind",
    "chk_events_status",
    "chk_event_signups_status",
    "chk_event_signups_attended",
    "chk_event_signups_offer_seats",
    "chk_settlements_status",
    "chk_settlements_split_mode",
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
];

/// 0003 on a database full of demo and edge-case chats: every trip in use gets a row,
/// both foreign keys hold, and rides given a trip later still work without app changes.
#[tokio::test]
async fn trips_link_rides_and_chats() {
    let (_app, st) = common::setup().await;
    common::seed_edge(&st).await;
    // Undo 0003 as if this database had stopped at 0002, then upgrade it.
    for sql in [
        "ALTER TABLE ride_messages DROP CONSTRAINT fk_ride_messages_trip",
        "ALTER TABLE rides DROP CONSTRAINT fk_rides_trip",
        "DROP TRIGGER trg_rides_ensure_trip ON rides",
        "DROP TABLE trips",
        "DELETE FROM _sqlx_migrations WHERE version > 2",
    ] {
        sqlx::query(sql).execute(&st.db).await.unwrap();
    }
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
    let ride: i64 = sqlx::query_scalar("SELECT id FROM rides WHERE trip_id IS NULL ORDER BY id LIMIT 1")
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

/// 0004 on a database full of demo and edge-case members: member_roles matches each
/// member's listed roles, a name that is not a role grants nothing until the role is
/// created, and writes to users.roles (the seed, tests, start-up) keep it in step.
#[tokio::test]
async fn member_roles_follow_users_roles() {
    let (_app, st) = common::setup().await;
    common::seed_edge(&st).await;
    // Undo 0004 as if this database had stopped at 0003, then upgrade it.
    for sql in [
        "DROP TRIGGER trg_roles_attach_members ON roles",
        "DROP TRIGGER trg_users_sync_member_roles ON users",
        "DROP TABLE member_roles",
        "DELETE FROM _sqlx_migrations WHERE version > 3",
    ] {
        sqlx::query(sql).execute(&st.db).await.unwrap();
    }
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
    let admins: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM member_roles WHERE role_key = 'admin'")
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

/// 0005 on a seeded database whose JSON columns are still text: they become JSONB,
/// valid JSON keeps its shape, text that isn't JSON is kept as a JSON string, and
/// the database then refuses anything that isn't JSON.
#[tokio::test]
async fn json_columns_become_jsonb() {
    let (_app, st) = common::setup().await;
    common::seed_edge(&st).await;
    // Undo 0005 as if this database had stopped at 0004, then upgrade it.
    for sql in [
        "ALTER TABLE invites DROP CONSTRAINT chk_invites_group_ids",
        "ALTER TABLE invites ALTER COLUMN group_ids DROP DEFAULT",
        "ALTER TABLE invites ALTER COLUMN group_ids TYPE text USING group_ids::text",
        "ALTER TABLE invites ALTER COLUMN group_ids SET DEFAULT '[]'",
        "ALTER TABLE settlement_revisions ALTER COLUMN snapshot TYPE text USING snapshot::text",
        "UPDATE settlement_revisions SET snapshot = 'initial draft' WHERE id = 63001",
        "DELETE FROM _sqlx_migrations WHERE version > 4",
    ] {
        sqlx::query(sql).execute(&st.db).await.unwrap();
    }

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
    assert_eq!((kept.as_str(), parsed.as_str()), ("string", "上周羽毛球费用"));
    let validated: bool = sqlx::query_scalar(
        "SELECT convalidated FROM pg_constraint WHERE conname = 'chk_invites_group_ids'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(validated);

    let bad = sqlx::query("UPDATE settlement_revisions SET snapshot = 'not json'::text::jsonb WHERE id = 43001")
        .execute(&db)
        .await;
    assert!(bad.is_err(), "not JSON");
    let bad = sqlx::query("INSERT INTO invites (token, email, group_ids, created_by, expires_at)
         VALUES ('t-json', '', '{}', (SELECT MIN(id) FROM users), '2099-01-01T00:00:00')")
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
