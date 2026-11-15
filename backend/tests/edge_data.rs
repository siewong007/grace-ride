//! Verification test for the `seed_edge` edge-case fixtures.
//! Runs `setup()` then `seed_edge()` and asserts every edge-case entity is
//! present with correct attributes.
//!
//! `cargo test --test edge_data -- --nocapture`

#[path = "common/mod.rs"]
mod common;

use common::*;
use grace_ride::seed::{EDGE_BATCH, SIMULATED_TOTP_SECRET};

#[tokio::test]
async fn seed_edge_provides_edge_case_users() {
    let (_app, st) = setup().await;
    seed_edge(&st).await;

    // 10 edge users (ids 90001..=90010).
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE demo_batch = $1")
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(n, 10, "expected 10 edge users");

    // Disabled user.
    let (status,): (String,) = sqlx::query_as("SELECT status FROM users WHERE email = $1")
        .bind("edgedisabled@example.org")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(status, "disabled");

    // No-password (passkey/OAuth-only) active user.
    let (pw, lang, terms): (Option<String>, String, Option<String>) =
        sqlx::query_as("SELECT password_hash, lang, terms_accepted_at FROM users WHERE email = $1")
            .bind("edgenopw@example.org")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(pw.is_none(), "edgenopw should have no password hash");
    assert_eq!(lang, "zh");
    assert!(terms.is_none());

    // English-language user with long phone (boundary).
    let (lang, phone): (String, String) =
        sqlx::query_as("SELECT lang, phone FROM users WHERE email = $1")
            .bind("edgeen@example.org")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(lang, "en");
    assert_eq!(phone, "012-3456789012345678");

    // TOTP-enabled staff user with combined roles.
    let (roles, totp_enabled): (String, i64) =
        sqlx::query_as("SELECT roles, totp_enabled FROM users WHERE email = $1")
            .bind("edgefinance@example.org")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(roles.contains("finance"));
    assert_eq!(totp_enabled, 1);

    let (totp,): (String,) = sqlx::query_as("SELECT totp_secret FROM users WHERE email = $1")
        .bind("edgefinance@example.org")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(totp, SIMULATED_TOTP_SECRET);
}

#[tokio::test]
async fn seed_edge_provides_vehicles_and_areas() {
    let (_app, st) = setup().await;
    seed_edge(&st).await;

    // Edge transport & carpool drivers each have a vehicle.
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM vehicles WHERE demo_batch = $1")
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(n, 2);

    // Driver areas assigned.
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM driver_areas WHERE demo_batch = $1")
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(n, 4);

    // Edge carpool driver has spare_seats=8 and carpool enabled.
    let (spare, carpool): (i64, i64) =
        sqlx::query_as("SELECT spare_seats, carpool_enabled FROM users WHERE email = $1")
            .bind("edgecarpool@example.org")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(spare, 8);
    assert_eq!(carpool, 1);
}

#[tokio::test]
async fn seed_edge_provides_events_and_signups() {
    let (_app, st) = setup().await;
    seed_edge(&st).await;

    // 6 edge events (ids 9001..=9006).
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events WHERE demo_batch = $1")
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(n, 6);

    // Event 9006 has no groups (edge case: ungrouped event).
    let (g,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM event_groups WHERE event_id = $1 AND demo_batch = $2")
            .bind(9006i64)
            .bind(EDGE_BATCH)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(g, 0);

    // Statuses span finished / cancelled / published.
    let statuses: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT status FROM events WHERE demo_batch = $1 ORDER BY id",
    )
    .bind(EDGE_BATCH)
    .fetch_all(&st.db)
    .await
    .unwrap();
    assert!(statuses.contains(&"finished".to_string()));
    assert!(statuses.contains(&"cancelled".to_string()));
    assert!(statuses.contains(&"published".to_string()));

    // Signups include going / waitlist / cancelled, with ride & drive offers.
    let (going, waitlist, cancelled): (i64, i64, i64) = (
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM event_signups WHERE demo_batch = $1 AND status = 'going'",
        )
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap(),
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM event_signups WHERE demo_batch = $1 AND status = 'waitlist'",
        )
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap(),
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM event_signups WHERE demo_batch = $1 AND status = 'cancelled'",
        )
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap(),
    );
    assert!(going > 0);
    assert_eq!(waitlist, 1);
    assert_eq!(cancelled, 1);
}

#[tokio::test]
async fn seed_edge_provides_every_ride_status() {
    let (_app, st) = setup().await;
    seed_edge(&st).await;

    // 10 edge rides (ids 80001..=80010).
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rides WHERE demo_batch = $1")
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(n, 10);

    // Every ride status is represented.
    let statuses: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT status FROM rides WHERE demo_batch = $1 ORDER BY status",
    )
    .bind(EDGE_BATCH)
    .fetch_all(&st.db)
    .await
    .unwrap();
    let expected = [
        "alt_transport",
        "cancelled",
        "completed",
        "confirmed",
        "in_progress",
        "no_show",
        "pending_approval",
        "pending_driver",
        "pending_passenger",
    ];
    for s in expected {
        assert!(statuses.contains(&s.to_string()), "missing ride status {s}");
    }

    // Proxy booking edge case: ride 80003 is proxy with rider details.
    let (is_proxy, rider_name, rider_relationship): (i64, String, String) =
        sqlx::query_as("SELECT is_proxy, rider_name, rider_relationship FROM rides WHERE id = $1")
            .bind(80003i64)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(is_proxy, 1);
    assert_eq!(rider_name, "Edge Elder");
    assert_eq!(rider_relationship, "parent");

    // Needs-attention edge case: ride 80010 flagged.
    let (attention, reason): (i64, String) =
        sqlx::query_as("SELECT needs_attention, attention_reason FROM rides WHERE id = $1")
            .bind(80010i64)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(attention, 1);
    assert_eq!(reason, "needs attention");

    // Completed ride has distance + reimbursement.
    let (dist, reimb): (Option<i64>, i64) =
        sqlx::query_as("SELECT distance_km, reimbursement_amount_cents FROM rides WHERE id = $1")
            .bind(80006i64)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(dist, Some(10));
    assert_eq!(reimb, 3000);
}

#[tokio::test]
async fn seed_edge_provides_reimbursements_all_statuses() {
    let (_app, st) = setup().await;
    seed_edge(&st).await;

    // 6 edge reimbursements.
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reimbursements WHERE demo_batch = $1")
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(n, 6);

    // All five statuses present.
    let statuses: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT status FROM reimbursements WHERE demo_batch = $1 ORDER BY status",
    )
    .bind(EDGE_BATCH)
    .fetch_all(&st.db)
    .await
    .unwrap();
    for s in ["approved", "paid", "pending", "rejected", "returned"] {
        assert!(
            statuses.contains(&s.to_string()),
            "missing reimbursement status {s}"
        );
    }

    // A paid reimbursement has payee ref + paid by + paid at.
    let (ref_, by, at): (Option<String>, Option<i64>, Option<String>) =
        sqlx::query_as("SELECT paid_ref, paid_by, paid_at FROM reimbursements WHERE id = $1")
            .bind(70005i64)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(ref_, Some("DNT-2026-05".to_string()));
    assert_eq!(by, Some(90001));
    assert!(at.is_some());
}

#[tokio::test]
async fn seed_edge_provides_settlements_items_shares() {
    let (_app, st) = setup().await;
    seed_edge(&st).await;

    // 3 settlements spanning draft / published / closed.
    let statuses: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT status FROM settlements WHERE demo_batch = $1 ORDER BY id",
    )
    .bind(EDGE_BATCH)
    .fetch_all(&st.db)
    .await
    .unwrap();
    assert_eq!(
        statuses,
        vec![
            "draft".to_string(),
            "published".to_string(),
            "closed".to_string()
        ]
    );

    // Settlement items.
    let (n,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM settlement_items WHERE demo_batch = $1")
            .bind(EDGE_BATCH)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(n, 5);

    // Shares include unpaid and needs_info.
    let (unpaid, needs): (i64, i64) = (
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM settlement_shares WHERE demo_batch = $1 AND status = 'unpaid'")
            .bind(EDGE_BATCH).fetch_one(&st.db).await.unwrap(),
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM settlement_shares WHERE demo_batch = $1 AND status = 'needs_info'")
            .bind(EDGE_BATCH).fetch_one(&st.db).await.unwrap(),
    );
    assert_eq!(unpaid, 2);
    assert_eq!(needs, 1);

    // A settlement revision exists.
    let (rev,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM settlement_revisions WHERE demo_batch = $1")
            .bind(EDGE_BATCH)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(rev, 1);
}

#[tokio::test]
async fn seed_edge_provides_help_notifications_announcements_audit() {
    let (_app, st) = setup().await;
    seed_edge(&st).await;

    // Help tickets span open / answered / closed, one with assignee.
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM help_tickets WHERE demo_batch = $1")
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(n, 4);

    // Notifications: mix of read / unread.
    let (read, unread): (i64, i64) = (
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM notifications WHERE demo_batch = $1 AND read_at IS NOT NULL",
        )
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap(),
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM notifications WHERE demo_batch = $1 AND read_at IS NULL",
        )
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap(),
    );
    assert_eq!(read, 1);
    assert_eq!(unread, 3);

    // Announcements: one pinned.
    let (pinned,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM announcements WHERE demo_batch = $1 AND pinned = 1")
            .bind(EDGE_BATCH)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(pinned, 1);

    // Audit logs.
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE demo_batch = $1")
        .bind(EDGE_BATCH)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(n, 3);
}

#[tokio::test]
async fn seed_edge_provides_proxy_riders_and_guide() {
    let (_app, st) = setup().await;
    seed_edge(&st).await;

    // Proxy rider for edge member.
    let (rel, notes): (String, String) =
        sqlx::query_as("SELECT relationship, notes FROM proxy_riders WHERE id = $1")
            .bind(11001i64)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(rel, "parent");
    assert!(notes.contains("wheelchair"));

    // Edge guide workflow + feedback.
    let (audience,): (String,) =
        sqlx::query_as("SELECT audience FROM guide_workflows WHERE id = $1")
            .bind(90001i64)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(audience, "member");

    let (fb,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM guide_feedback WHERE guide_id = $1")
        .bind(90001i64)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(fb, 1);
}
