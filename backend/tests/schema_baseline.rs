//! The database has one schema script. A history that still lists the old
//! numbered migrations is cleared, the script runs again, and existing rows stay.

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
         VALUES (2, 'passenger_locations', true, $1, 0),
                (16, 'help_claim_attach', true, $1, 0)",
    )
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
    assert_eq!(versions, vec![1]);
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
    assert_eq!(versions, vec![1]);
}
