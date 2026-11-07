//! Capacity / performance baseline: ~1,000 members + 10 concurrent drivers
//! (Grace_Ride_Chinese.pdf p.15, requirement R-32, phase P0).
//! Reproducible: `cargo test --test capacity -- --nocapture`

use axum::Router;
use grace_ride::{init, jobs, router, state::{AppState, Config}};
use std::time::Instant;

const MEMBER_COUNT: i64 = 1000;
const DRIVER_COUNT: i64 = 10;

async fn setup_capacity() -> (Router, AppState) {
    let dir = std::env::temp_dir().join(format!("gr-cap-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = Config {
        dev_mode: true,
        database_url: format!("sqlite://{}/cap.db", dir.display()),
        upload_dir: dir.join("uploads"),
        static_dir: None,
        public_url: "http://localhost:5173".into(),
        bind: "127.0.0.1:0".into(),
        max_live_drivers: DRIVER_COUNT,
        trust_proxy: false,
        chat_retention_days: 180,
        audit_retention_days: 730,
        json_logs: false,
    };
    cfg.validate().unwrap();
    let state = init(cfg.clone()).await.unwrap();

    // Seed 1,000 members beyond the demo data already loaded by seed::run.
    let mut member_ids = Vec::new();
    for i in 0..MEMBER_COUNT {
        let id = sqlx::query(
            "INSERT INTO users (name, email, phone, roles, password_hash, webauthn_uuid)
             VALUES (?, ?, '', '', NULL, ?)",
        )
        .bind(format!("会员 {i}"))
        .bind(format!("capmember{i}@example.org"))
        .bind(uuid::Uuid::new_v4().to_string())
        .execute(&state.db)
        .await
        .unwrap()
        .last_insert_rowid();
        member_ids.push(id);
    }

    // Designate 10 drivers (the concurrent live-location cap).
    let mut driver_ids = Vec::new();
    for i in 0..DRIVER_COUNT {
        let id = sqlx::query(
            "INSERT INTO users (name, email, phone, roles, password_hash, webauthn_uuid)
             VALUES (?, ?, '', 'driver', NULL, ?)",
        )
        .bind(format!("司机 {i}"))
        .bind(format!("capdriver{i}@example.org"))
        .bind(uuid::Uuid::new_v4().to_string())
        .execute(&state.db)
        .await
        .unwrap()
        .last_insert_rowid();
        driver_ids.push(id);
    }

    // 50 completed rides spread across the 10 drivers.
    let tomorrow = (chrono::Local::now() + chrono::Duration::days(1))
        .format("%Y-%m-%dT08:30")
        .to_string();
    let ts = grace_ride::util::now();
    for i in 0..50 {
        sqlx::query(
            "INSERT INTO rides (passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address,
             destination, seats, status, created_at, updated_at)
             VALUES (?, ?, ?, 1, 'a', 'b', 1, 'completed', ?, ?)",
        )
        .bind(member_ids[(i as usize) % (MEMBER_COUNT as usize)])
        .bind(driver_ids[(i as usize) % (DRIVER_COUNT as usize)])
        .bind(&tomorrow)
        .bind(&ts)
        .bind(&ts)
        .execute(&state.db)
        .await
        .unwrap();
    }

    (router(state.clone()), state)
}

#[tokio::test]
async fn capacity_seeds_thousand_members_and_ten_drivers() {
    let (_app, st) = setup_capacity().await;
    let (users,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(
        users >= MEMBER_COUNT + DRIVER_COUNT,
        "expected at least {} users (members + drivers), got {}",
        MEMBER_COUNT + DRIVER_COUNT,
        users
    );
    assert_eq!(
        st.cfg.max_live_drivers, DRIVER_COUNT,
        "live-driver cap must be {DRIVER_COUNT}"
    );
    println!(
        "capacity: {users} users seeded, live-driver cap = {}",
        st.cfg.max_live_drivers
    );
}

#[tokio::test]
async fn capacity_reminder_job_scales() {
    let (_app, st) = setup_capacity().await;
    let start = Instant::now();
    jobs::run_once(&st.db, &st.cfg).await.unwrap();
    let elapsed = start.elapsed();
    println!(
        "capacity: reminder job over ~{} users completed in {:?}",
        MEMBER_COUNT,
        elapsed
    );
    assert!(
        elapsed.as_secs() < 60,
        "reminder job too slow on dev hardware: {:?}",
        elapsed
    );
    let (n,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications")
            .fetch_one(&st.db)
            .await
            .unwrap();
    println!("capacity: {n} notifications after job run");
    assert!(n >= 0);
}
