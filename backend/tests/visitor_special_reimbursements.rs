//! Special reimbursements for plan visitors: a traffic lead asks for an amount with a
//! reason, paid to the visitor's driver, through the usual review and finance payout.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::{json, Value};

type St = grace_ride::state::AppState;

/// user, event, amount, status, kind, visitor, reason, requested by, receipt.
type ClaimRow = (
    i64,
    Option<i64>,
    i64,
    String,
    String,
    Option<i64>,
    String,
    Option<i64>,
    Option<i64>,
);

async fn user_id(st: &St, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn event_by(st: &St, creator: i64, title: &str) -> i64 {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO events (title, starts_at, created_by) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(title)
    .bind(tomorrow("19:00"))
    .bind(creator)
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO event_groups (event_id, group_id) SELECT $1, id FROM groups WHERE is_all = 1",
    )
    .bind(id)
    .execute(&st.db)
    .await
    .unwrap();
    id
}

/// The seeded driver, with a car so the plan accepts them.
async fn driver(st: &St) -> i64 {
    let d = user_id(st, "driver@example.org").await;
    sqlx::query(
        "INSERT INTO vehicles (user_id, model, color, plate, seats, is_active)
         VALUES ($1, 'Myvi', 'White', 'VSR 1', 5, 1) ON CONFLICT DO NOTHING",
    )
    .bind(d)
    .execute(&st.db)
    .await
    .unwrap();
    d
}

async fn visitor(app: &axum::Router, lead: &str, ev: i64, name: &str, driver: Option<i64>) -> i64 {
    let r = call(
        app,
        Method::POST,
        &format!("/api/admin/events/{ev}/plan/visitors"),
        Some(lead),
        Some(json!({"name": name, "seats": 1, "driver_id": driver})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    r.json["id"].as_i64().unwrap()
}

fn path(ev: i64, vid: i64) -> String {
    format!("/api/admin/events/{ev}/plan/visitors/{vid}/reimbursement")
}

async fn review(app: &axum::Router, who: &str, rid: i64, action: &str, note: &str) -> Res {
    call(
        app,
        Method::POST,
        &format!("/api/reimbursements/{rid}/review"),
        Some(who),
        Some(json!({"action": action, "note": note})),
    )
    .await
}

#[tokio::test]
async fn traffic_lead_requests_it_and_it_goes_through_review_and_payout() {
    let (app, st) = setup().await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let head_id = user_id(&st, "traffichead@example.org").await;
    let d = driver(&st).await;
    let ev = event_by(&st, admin_id, "Sunday service").await;
    let head = login(&st, "traffichead@example.org").await;
    let vid = visitor(&app, &head, ev, "Mr Tan", Some(d)).await;

    let r = call(
        &app,
        Method::POST,
        &path(ev, vid),
        Some(&head),
        Some(json!({"amount_cents": 3000, "reason": "  Detour to Klang to fetch him  "})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let rid = r.json["id"].as_i64().unwrap();

    // An ordinary pending claim for the driver, marked as a visitor special, no receipt.
    let row: ClaimRow = sqlx::query_as(
        "SELECT user_id, event_id, amount_cents, status, kind, plan_visitor_id, special_reason,
                    requested_by, receipt_file_id FROM reimbursements WHERE id = $1",
    )
    .bind(rid)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(
        row,
        (
            d,
            Some(ev),
            3000,
            "pending".into(),
            "visitor_special".into(),
            Some(vid),
            "Detour to Klang to fetch him".into(),
            Some(head_id),
            None
        )
    );
    // The driver hears about it; the audit log has who, how much and why.
    let told: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND link = $2")
            .bind(d)
            .bind(format!("/reimbursements/{rid}"))
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(told, 1);
    let detail: String = sqlx::query_scalar(
        "SELECT detail FROM audit_logs WHERE action = 'reimbursement.visitor_special' AND entity_id = $1",
    )
    .bind(rid)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert!(
        detail.contains("Detour to Klang") && detail.contains("Mr Tan"),
        "{detail}"
    );

    // The plan shows it on the visitor.
    let plan = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{ev}/plan"),
        Some(&head),
        None,
    )
    .await;
    let sr = &plan.json["visitors"][0]["special_reimbursement"];
    assert_eq!(sr["id"], rid);
    assert_eq!(sr["status"], "pending");
    assert_eq!(sr["amount_cents"], 3000);
    assert_eq!(sr["reason"], "Detour to Klang to fetch him");

    // One live request per visitor.
    let r = call(
        &app,
        Method::POST,
        &path(ev, vid),
        Some(&head),
        Some(json!({"amount_cents": 1000, "reason": "Again"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.json);
    assert_eq!(r.json["code"], "VISITOR_REIMB_ALREADY_LIVE");

    // Reviewers see it marked, with the reason, the visitor and who asked.
    let transport = login(&st, "transport@example.org").await;
    let list = call(
        &app,
        Method::GET,
        "/api/reimbursements?scope=review",
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(list.status, StatusCode::OK, "{}", list.json);
    let items: &Vec<Value> = list.json.as_array().unwrap();
    let it = items
        .iter()
        .find(|x| x["id"] == rid)
        .expect("in the review queue");
    assert_eq!(it["kind"], "visitor_special");
    assert_eq!(it["special_reason"], "Detour to Klang to fetch him");
    assert_eq!(it["visitor"]["name"], "Mr Tan");
    assert_eq!(it["requested_by"]["id"], head_id);
    assert!(it["receipt_file_id"].is_null());

    // Whoever asked for it may not approve it.
    let r = review(&app, &head, rid, "approve", "").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);

    // Sent back: the driver can't rewrite it; a traffic lead corrects and resends it.
    let r = review(&app, &transport, rid, "return", "Which road?").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let drv = login(&st, "driver@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/reimbursements/{rid}"),
        Some(&drv),
        Some(
            json!({"amount_cents": 40000, "description": "mine", "receipt_file_id": null,
                    "payee_name": "x", "payee_phone": "0123456789", "event_id": ev}),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "ONLY_RETURNED_REIMB_CAN_BE_RESUBMITTED");
    let admin = login(&st, "admin@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        &format!("{}/{rid}", path(ev, vid)),
        Some(&admin),
        Some(json!({"amount_cents": 3500, "reason": "Detour via Jalan Klang Lama, 25 km"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let (status, amount, by): (String, i64, Option<i64>) = sqlx::query_as(
        "SELECT status, amount_cents, requested_by FROM reimbursements WHERE id = $1",
    )
    .bind(rid)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(
        (status.as_str(), amount, by),
        ("pending", 3500, Some(admin_id))
    );
    // Only a returned one can be resent.
    let r = call(
        &app,
        Method::PUT,
        &format!("{}/{rid}", path(ev, vid)),
        Some(&head),
        Some(json!({"amount_cents": 3500, "reason": "x"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);

    // Now the head (no longer the requester) approves it, and finance pays it.
    let r = review(&app, &head, rid, "approve", "").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let fin = login(&st, "finance@example.org").await;
    let r = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{rid}/pay"),
        Some(&fin),
        Some(json!({"payee_verified": true, "paid_ref": "TNG-55120"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // The visitor stays on record while it has a live special reimbursement.
    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/admin/events/{ev}/plan/visitors/{vid}"),
        Some(&head),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.json);
    assert_eq!(r.json["code"], "VISITOR_HAS_REIMBURSEMENT");

    // It doesn't count as the driver's post-event mileage for this event.
    let area: i64 = sqlx::query_scalar("SELECT id FROM areas WHERE active = 1 ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev}/post-event-reimbursements"),
        Some(&head),
        Some(
            json!({"claims": [{"driver_id": d, "trip_type": "direct_return",
                                "destination_area_id": area, "amount_cents": 1500}]}),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
}

#[tokio::test]
async fn only_traffic_leads_ask_and_the_request_is_checked() {
    let (app, st) = setup().await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let cell_id = user_id(&st, "cellleader@example.org").await;
    let d = driver(&st).await;
    let ev = event_by(&st, admin_id, "Camp").await;
    let other = event_by(&st, cell_id, "Cell supper").await;
    sqlx::query("UPDATE users SET roles = 'traffic_member' WHERE email = 'amina@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    let head = login(&st, "traffichead@example.org").await;
    let body = json!({"amount_cents": 2000, "reason": "Fetched from Seremban"});

    // Not traffic leads: refused, even with plan or review rights.
    let vid = visitor(&app, &head, ev, "Visitor A", Some(d)).await;
    for who in [
        "transport@example.org",
        "secretary@example.org",
        "amina@example.org",
        "cellleader@example.org",
        "member@example.org",
        "driver@example.org",
        "finance@example.org",
        "auditor@example.org",
    ] {
        let c = login(&st, who).await;
        let r = call(
            &app,
            Method::POST,
            &path(ev, vid),
            Some(&c),
            Some(body.clone()),
        )
        .await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{who}: {}", r.json);
    }
    let none: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM reimbursements WHERE kind = 'visitor_special'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(none, 0);

    // Traffic leads: Head of Traffic, admin, President, Vice-President.
    for who in [
        "traffichead@example.org",
        "admin@example.org",
        "president@example.org",
        "vicepresident@example.org",
    ] {
        let c = login(&st, who).await;
        let v = visitor(&app, &head, ev, &format!("Guest of {who}"), Some(d)).await;
        let r = call(
            &app,
            Method::POST,
            &path(ev, v),
            Some(&c),
            Some(body.clone()),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{who}: {}", r.json);
    }

    // Input checks.
    for (b, code) in [
        (
            json!({"amount_cents": 0, "reason": "x"}),
            "AMOUNT_MUST_BE_POSITIVE",
        ),
        (
            json!({"amount_cents": -500, "reason": "x"}),
            "AMOUNT_MUST_BE_POSITIVE",
        ),
        (
            json!({"amount_cents": 1000, "reason": "   "}),
            "REASON_REQUIRED",
        ),
        (json!({"amount_cents": 1000}), "REASON_REQUIRED"),
    ] {
        let r = call(
            &app,
            Method::POST,
            &path(ev, vid),
            Some(&head),
            Some(b.clone()),
        )
        .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{b}: {}", r.json);
        assert_eq!(r.json["code"], code, "{b}");
    }
    let long = json!({"amount_cents": 1000, "reason": "x".repeat(301)});
    let r = call(&app, Method::POST, &path(ev, vid), Some(&head), Some(long)).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);

    // No maximum: a large amount is accepted, and flagged as high to reviewers and on the plan.
    let transport = login(&st, "transport@example.org").await;
    let big = visitor(&app, &head, ev, "Long trip guest", Some(d)).await;
    let r = call(
        &app,
        Method::POST,
        &path(ev, big),
        Some(&head),
        Some(json!({"amount_cents": 120_000, "reason": "Fetched from Ipoh and back"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let big_rid = r.json["id"].as_i64().unwrap();
    let claim = call(
        &app,
        Method::GET,
        &format!("/api/reimbursements/{big_rid}"),
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(claim.json["amount_cents"], 120_000);
    assert_eq!(claim.json["high_amount"], true);
    let flagged: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE title LIKE '%over RM 100%' OR title LIKE '%超过 RM100%'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert!(flagged > 0, "reviewers are told it is a high amount");
    let plan = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{ev}/plan"),
        Some(&head),
        None,
    )
    .await;
    let guest = plan.json["visitors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "Long trip guest")
        .unwrap()
        .clone();
    assert_eq!(guest["special_reimbursement"]["high_amount"], true);
    // RM 100 exactly is not "over RM 100".
    let edge = visitor(&app, &head, ev, "Edge guest", Some(d)).await;
    let r = call(
        &app,
        Method::POST,
        &path(ev, edge),
        Some(&head),
        Some(json!({"amount_cents": 10_000, "reason": "Exactly a hundred"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let claim = call(
        &app,
        Method::GET,
        &format!("/api/reimbursements/{}", r.json["id"]),
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(claim.json["high_amount"], false);

    // It pays the visitor's driver, so the visitor needs a car.
    let carless = visitor(&app, &head, ev, "No car yet", None).await;
    let r = call(
        &app,
        Method::POST,
        &path(ev, carless),
        Some(&head),
        Some(body.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "VISITOR_REIMB_NEEDS_DRIVER");

    // A visitor id under another event is not found.
    let r = call(
        &app,
        Method::POST,
        &path(other, vid),
        Some(&head),
        Some(body.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND, "{}", r.json);

    // A rejected request makes room for a new one.
    let r = call(
        &app,
        Method::POST,
        &path(ev, vid),
        Some(&head),
        Some(body.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let rid = r.json["id"].as_i64().unwrap();
    let transport = login(&st, "transport@example.org").await;
    let r = review(
        &app,
        &transport,
        rid,
        "reject",
        "Already covered by the fare",
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let r = call(
        &app,
        Method::POST,
        &path(ev, vid),
        Some(&head),
        Some(body.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // Not on a cancelled event.
    sqlx::query("UPDATE events SET status = 'cancelled' WHERE id = $1")
        .bind(ev)
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        &path(ev, carless),
        Some(&head),
        Some(body),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.json);
}
