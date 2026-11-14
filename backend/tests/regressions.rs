//! Regression coverage for six concurrency / integrity defects found by review.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::util::now;
use serde_json::json;

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
