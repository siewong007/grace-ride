//! Regression coverage for concurrency / integrity defects found by review.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::util::now;
use serde_json::json;

async fn insert_completed_ride(
    st: &grace_ride::state::AppState,
    driver: i64,
    passenger: i64,
    pickup_at: &str,
) {
    sqlx::query(
        "INSERT INTO rides (passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, status, completed_at)
         VALUES ($1,$2,$3,1,'test pickup','test destination',1,'','completed',$3)",
    )
    .bind(passenger)
    .bind(driver)
    .bind(pickup_at)
    .execute(&st.db)
    .await
    .unwrap();
}

#[tokio::test]
async fn duplicate_payment_proof_rejected_under_concurrency() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let file_id = upload_png(&app, &member).await;

    let (member_id,): (i64,) =
        sqlx::query_as("SELECT id FROM users WHERE email='member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();

    let mut share_ids = Vec::new();
    for title in ["Duplicate proof A", "Duplicate proof B"] {
        let (sid,): (i64,) = sqlx::query_as(
            "INSERT INTO settlements (event_id,created_by,title,status,created_at) VALUES (5,4,$1,'published',$2) RETURNING id",
        )
        .bind(title)
        .bind(now())
        .fetch_one(&st.db)
        .await
        .unwrap();
        let (share,): (i64,) = sqlx::query_as(
            "INSERT INTO settlement_shares (settlement_id,user_id,amount_cents,status,updated_at) VALUES ($1,$2,1000,'unpaid',$3) RETURNING id",
        )
        .bind(sid)
        .bind(member_id)
        .bind(now())
        .fetch_one(&st.db)
        .await
        .unwrap();
        share_ids.push(share);
    }

    let path = |id: i64| format!("/api/shares/{id}/proof");
    let body = json!({"proof_file_id": file_id, "txn_ref": "SAME-PAYMENT-1234"});
    let app1 = app.clone();
    let app2 = app.clone();
    let c1 = member.clone();
    let c2 = member.clone();
    let p1 = path(share_ids[0]);
    let p2 = path(share_ids[1]);
    let (r1, r2) = tokio::join!(
        call(&app1, Method::POST, &p1, Some(&c1), Some(body.clone())),
        call(&app2, Method::POST, &p2, Some(&c2), Some(body)),
    );
    let oks = [&r1, &r2]
        .iter()
        .filter(|r| r.status == StatusCode::OK)
        .count();
    println!("Duplicate proof: {} {} {}", r1.status, r1.json, r2.status);
    assert_eq!(oks, 1, "Same receipt/reference was accepted for two bills");
}

#[tokio::test]
async fn closed_bill_rejects_payment_reset() {
    let (app, st) = setup().await;
    let organiser = login(&st, "driver@example.org").await;

    // Seeded published bill 40001, all shares confirmed so it can be closed.
    sqlx::query(
        "UPDATE settlement_shares SET status='confirmed', updated_at=$1 WHERE settlement_id=40001",
    )
    .bind(now())
    .execute(&st.db)
    .await
    .unwrap();
    let closed = call(
        &app,
        Method::POST,
        "/api/settlements/40001/close",
        Some(&organiser),
        None,
    )
    .await;
    assert_eq!(closed.status, StatusCode::OK, "{}", closed.json);

    let (share,): (i64,) = sqlx::query_as(
        "SELECT id FROM settlement_shares WHERE settlement_id=40001 ORDER BY id LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    let reset = call(
        &app,
        Method::POST,
        &format!("/api/shares/{share}/verify"),
        Some(&organiser),
        Some(json!({"action": "reset"})),
    )
    .await;
    println!(
        "Reset share in closed bill: {} {}",
        reset.status, reset.json
    );
    assert_ne!(reset.status, StatusCode::OK);

    let (status,): (String,) = sqlx::query_as("SELECT status FROM settlement_shares WHERE id=$1")
        .bind(share)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(status, "confirmed", "Closed bill payment state changed");
}

#[tokio::test]
async fn published_revision_rejects_absent_member() {
    let (app, st) = setup().await;
    let organiser = login(&st, "driver@example.org").await;

    // Existing seeded published bill; add a member who did not attend event 5.
    let update = call(
        &app,
        Method::PUT,
        "/api/settlements/40001",
        Some(&organiser),
        Some(json!({
            "title": "上周羽毛球费用",
            "items": [
                {"label": "场租", "amount_cents": 12000},
                {"label": "羽球", "amount_cents": 4800}
            ],
            "subsidy_cents": 0,
            "split_mode": "equal",
            "participants": [
                {"user_id": 4}, {"user_id": 6}, {"user_id": 7},
                {"user_id": 5}, {"user_id": 2}, {"user_id": 8},
                {"user_id": 12}
            ],
            "payee_name": "李德明",
            "payee_account": "012-300 0004",
            "reason": "Review revision"
        })),
    )
    .await;
    println!(
        "Revise published bill to absent member: {} {}",
        update.status, update.json
    );
    assert_ne!(update.status, StatusCode::OK);

    let (charged,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlement_shares WHERE settlement_id=40001 AND user_id=12",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(charged, 0, "Published revision charged a non-attendee");
}

#[tokio::test]
async fn all_unpaid_shares_remain_accessible() {
    let (app, st) = setup().await;
    let member = login(&st, "enci@example.org").await;
    let (uid,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email='enci@example.org'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM settlement_shares WHERE user_id=$1")
        .bind(uid)
        .execute(&st.db)
        .await
        .unwrap();

    for i in 0..101 {
        let (sid,): (i64,) = sqlx::query_as(
            "INSERT INTO settlements (event_id,created_by,title,status,created_at) VALUES (5,4,$1,'published',$2) RETURNING id",
        )
        .bind(format!("Unpaid bill {i}"))
        .bind(now())
        .fetch_one(&st.db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO settlement_shares(settlement_id,user_id,amount_cents,status,updated_at) VALUES ($1,$2,1000,'unpaid',$3)",
        )
        .bind(sid)
        .bind(uid)
        .bind(now())
        .execute(&st.db)
        .await
        .unwrap();
    }

    let res = call(&app, Method::GET, "/api/me/shares", Some(&member), None).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let count = res.json.as_array().unwrap().len();
    println!("#101 unpaid shares, response count: {count}");
    assert_eq!(
        count, 101,
        "Older unpaid bill silently missing, with no pagination cursor"
    );
}

#[tokio::test]
async fn concurrent_claims_respect_driver_capacity() {
    let (app, st) = setup().await;
    sqlx::query("UPDATE rides SET status='cancelled' WHERE driver_id=(SELECT id FROM users WHERE email='driver@example.org')")
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE users SET spare_seats=1, carpool_enabled=1 WHERE email='driver@example.org'",
    )
    .execute(&st.db)
    .await
    .unwrap();

    let yawen = login(&st, "yawen@example.org").await;
    let member = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;

    let body = json!({
        "pickup_at": tomorrow("22:30"),
        "pickup_area_id": 1,
        "pickup_address": "race addr",
        "destination": "社区中心",
        "seats": 1,
        "assistance": ""
    });
    let r1 = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&yawen),
        Some(body.clone()),
    )
    .await;
    let r2 = call(&app, Method::POST, "/api/rides", Some(&member), Some(body)).await;
    assert_eq!(r1.status, StatusCode::OK, "{}", r1.json);
    assert_eq!(r2.status, StatusCode::OK, "{}", r2.json);
    let id1 = r1.json["id"].as_i64().unwrap();
    let id2 = r2.json["id"].as_i64().unwrap();

    // Force both handlers to read capacity before either writes.
    let mut lock = st.db.begin().await.unwrap();
    sqlx::query("LOCK TABLE rides IN SHARE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let app1 = app.clone();
    let app2 = app.clone();
    let c1 = driver.clone();
    let c2 = driver.clone();
    let p1 = format!("/api/rides/{id1}/claim");
    let p2 = format!("/api/rides/{id2}/claim");
    let f1 = tokio::spawn(async move { call(&app1, Method::POST, &p1, Some(&c1), None).await });
    let f2 = tokio::spawn(async move { call(&app2, Method::POST, &p2, Some(&c2), None).await });
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    lock.rollback().await.unwrap();
    let (c1, c2) = tokio::join!(f1, f2);
    let (c1, c2) = (c1.unwrap(), c2.unwrap());
    let accepted = [&c1, &c2]
        .iter()
        .filter(|r| r.status == StatusCode::OK)
        .count();
    println!("claim: {} {}", c1.status, c2.status);
    assert_eq!(
        accepted, 1,
        "One available seat accepted two concurrent bookings"
    );
}

#[tokio::test]
async fn concurrent_event_signups_respect_capacity() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let amina = login(&st, "amina@example.org").await;

    sqlx::query("DELETE FROM event_signups WHERE event_id=1")
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE events SET capacity=1, starts_at=$1, signup_deadline=NULL, status='published' WHERE id=1",
    )
    .bind(tomorrow("19:30"))
    .execute(&st.db)
    .await
    .unwrap();

    let mut lock = st.db.begin().await.unwrap();
    sqlx::query("LOCK TABLE event_signups IN SHARE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let app1 = app.clone();
    let app2 = app.clone();
    let c1 = member.clone();
    let c2 = amina.clone();
    let body = json!({"need_ride": false, "offer_drive": false});
    let body1 = body.clone();
    let body2 = body;
    let f1 = tokio::spawn(async move {
        call(
            &app1,
            Method::POST,
            "/api/events/1/signup",
            Some(&c1),
            Some(body1),
        )
        .await
    });
    let f2 = tokio::spawn(async move {
        call(
            &app2,
            Method::POST,
            "/api/events/1/signup",
            Some(&c2),
            Some(body2),
        )
        .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    lock.rollback().await.unwrap();
    let (r1, r2) = tokio::join!(f1, f2);
    let (r1, r2) = (r1.unwrap(), r2.unwrap());
    println!("signup: {} {}", r1.json, r2.json);
    let going = [&r1, &r2]
        .iter()
        .filter(|r| r.json["status"] == json!("going"))
        .count();
    assert_eq!(
        going, 1,
        "Capacity-one event accepted two attendees instead of waitlisting one"
    );
}

#[tokio::test]
async fn replacement_recovery_link_invalidates_previous_link() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;

    for _ in 0..2 {
        let requested = call(
            &app,
            Method::POST,
            "/api/auth/recovery-request",
            None,
            Some(json!({"email": "member@example.org", "reason": "lost phone"})),
        )
        .await;
        assert_eq!(requested.status, StatusCode::OK, "{}", requested.json);
    }
    let rids: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM recovery_requests WHERE email='member@example.org' ORDER BY id",
    )
    .fetch_all(&st.db)
    .await
    .unwrap();

    let mut links = Vec::new();
    for (rid,) in rids {
        let approved = call(
            &app,
            Method::POST,
            &format!("/api/admin/recovery/{rid}/approve"),
            Some(&admin),
            None,
        )
        .await;
        assert_eq!(approved.status, StatusCode::OK, "{}", approved.json);
        links.push(
            approved.json["url"]
                .as_str()
                .unwrap()
                .rsplit('/')
                .next()
                .unwrap()
                .to_string(),
        );
    }

    // The replacement link works...
    let used = call(
        &app,
        Method::POST,
        &format!("/api/invites/{}/accept", links[1]),
        None,
        Some(json!({"terms": true})),
    )
    .await;
    assert_eq!(used.status, StatusCode::OK, "{}", used.json);

    // ...and the earlier one can no longer preview or sign in.
    let preview = call(
        &app,
        Method::GET,
        &format!("/api/invites/{}", links[0]),
        None,
        None,
    )
    .await;
    assert_eq!(preview.status, StatusCode::BAD_REQUEST, "{}", preview.json);
    assert_eq!(preview.json["code"], "INVITE_EXPIRED");

    let reused = call(
        &app,
        Method::POST,
        &format!("/api/invites/{}/accept", links[0]),
        None,
        Some(json!({"terms": true})),
    )
    .await;
    assert_eq!(reused.status, StatusCode::BAD_REQUEST, "{}", reused.json);
}

#[tokio::test]
async fn monthly_report_includes_the_selected_month() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    insert_completed_ride(&st, 6, 7, "2024-03-15T10:00:00").await;

    let report = call(
        &app,
        Method::GET,
        "/api/reports/monthly?month=2024-03&range=monthly",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(report.status, StatusCode::OK, "{}", report.json);
    assert_eq!(report.json["period_start"], "2024-03-01");
    assert_eq!(report.json["period_end"], "2024-04-01");
    assert_eq!(report.json["rides_driven"], 1, "{}", report.json);
}

#[tokio::test]
async fn quarterly_and_yearly_reports_include_their_last_month() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    insert_completed_ride(&st, 6, 7, "2024-06-15T10:00:00").await;
    insert_completed_ride(&st, 6, 7, "2024-12-31T10:00:00").await;

    // Q2 begins in April; its third month (June) must be included.
    let quarter = call(
        &app,
        Method::GET,
        "/api/reports/monthly?month=2024-04&range=quarterly",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(quarter.status, StatusCode::OK, "{}", quarter.json);
    assert_eq!(quarter.json["period_start"], "2024-04-01");
    assert_eq!(quarter.json["period_end"], "2024-07-01");
    assert_eq!(quarter.json["rides_driven"], 1, "{}", quarter.json);

    // A yearly report must include December.
    let year = call(
        &app,
        Method::GET,
        "/api/reports/monthly?month=2024-01&range=yearly",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(year.status, StatusCode::OK, "{}", year.json);
    assert_eq!(year.json["period_start"], "2024-01-01");
    assert_eq!(year.json["period_end"], "2025-01-01");
    assert_eq!(year.json["rides_driven"], 2, "{}", year.json);
}

#[tokio::test]
async fn passenger_rides_are_not_counted_as_driven() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    insert_completed_ride(&st, 4, 6, "2024-05-10T10:00:00").await;

    let report = call(
        &app,
        Method::GET,
        "/api/reports/monthly?month=2024-05",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(report.status, StatusCode::OK, "{}", report.json);
    assert_eq!(report.json["rides_driven"], 0, "{}", report.json);
    assert_eq!(report.json["rides_received"], 1, "{}", report.json);
}

#[tokio::test]
async fn malformed_report_month_is_a_bad_request() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;

    // 202é-1 is seven bytes but a slice at byte 4 used to split the é and panic.
    for month in ["202%C3%A9-1", "2024-13", "2024-00", "2024-1", "abcd-01"] {
        let report = call(
            &app,
            Method::GET,
            &format!("/api/reports/monthly?month={month}"),
            Some(&member),
            None,
        )
        .await;
        assert_eq!(
            report.status,
            StatusCode::BAD_REQUEST,
            "month={month}: {}",
            report.json
        );
        assert_eq!(report.json["code"], "MONTH_FORMAT_INVALID", "month={month}");
    }
}

#[tokio::test]
async fn concurrent_payouts_only_one_wins() {
    let (app, st) = setup().await;
    let claimant = login(&st, "member@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let finance = login(&st, "finance@example.org").await;
    let receipt = upload_png(&app, &claimant).await;

    let claimed = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&claimant),
        Some(json!({
            "ride_id": null,
            "amount_cents": 2850,
            "description": "Concurrent payout",
            "receipt_file_id": receipt,
            "payee_name": "陈法拉",
            "payee_phone": "012-300 0006"
        })),
    )
    .await;
    assert_eq!(claimed.status, StatusCode::OK, "{}", claimed.json);
    let id = claimed.json["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/reimbursements/{id}/review"),
            Some(&transport),
            Some(json!({"action": "approve", "note": ""}))
        )
        .await
        .status,
        StatusCode::OK
    );

    // Hold UPDATE access so both handlers can read status='approved' first.
    let mut lock = st.db.begin().await.unwrap();
    sqlx::query("LOCK TABLE reimbursements IN SHARE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();

    let path = format!("/api/reimbursements/{id}/pay");
    let app1 = app.clone();
    let app2 = app.clone();
    let c1 = finance.clone();
    let c2 = finance.clone();
    let p1 = path.clone();
    let p2 = path;
    let f1 = tokio::spawn(async move {
        call(
            &app1,
            Method::POST,
            &p1,
            Some(&c1),
            Some(json!({"payee_verified": true, "paid_ref": "REF-AAAA-1111"})),
        )
        .await
    });
    let f2 = tokio::spawn(async move {
        call(
            &app2,
            Method::POST,
            &p2,
            Some(&c2),
            Some(json!({"payee_verified": true, "paid_ref": "REF-BBBB-2222"})),
        )
        .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    lock.rollback().await.unwrap();
    let (r1, r2) = tokio::join!(f1, f2);
    let (r1, r2) = (r1.unwrap(), r2.unwrap());

    let wins = [&r1, &r2]
        .iter()
        .filter(|r| r.status == StatusCode::OK)
        .count();
    assert_eq!(wins, 1, "both payouts succeeded: {} / {}", r1.json, r2.json);
    let loser = if r1.status == StatusCode::OK {
        &r2
    } else {
        &r1
    };
    assert_eq!(loser.status, StatusCode::CONFLICT, "loser: {}", loser.json);
    assert_eq!(loser.json["code"], "REIMBURSEMENT_PROCESSED");

    let (paid_ref,): (String,) = sqlx::query_as("SELECT paid_ref FROM reimbursements WHERE id=$1")
        .bind(id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert!(
        paid_ref == "REF-AAAA-1111" || paid_ref == "REF-BBBB-2222",
        "unexpected winning reference: {paid_ref}"
    );
}
