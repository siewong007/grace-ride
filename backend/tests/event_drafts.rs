//! Event draft lifecycle: hidden draft creation, member/module isolation, hide_drafts filter,
//! publish (invites + audit), unpublish (guardrails + 409 conflict), and cancelling a draft.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

async fn notif_count(db: &sqlx::PgPool, uid: i64) -> i64 {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id=$1")
        .bind(uid)
        .fetch_one(db)
        .await
        .unwrap();
    n
}

#[tokio::test]
async fn event_draft_hide_and_publish_lifecycle() {
    let (app, st) = setup().await;
    let all_gid: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE is_all=1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let member_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE email='member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();

    let initial_notifs = notif_count(&st.db, member_id).await;

    // Rotate member calendar feed so we can verify draft omission from .ics
    let cal_res = call(
        &app,
        Method::POST,
        "/api/me/calendar-feed/rotate",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(cal_res.status, StatusCode::OK, "{}", cal_res.json);
    let ics_path = cal_res.json["ics_path"].as_str().unwrap().to_string();

    // 1. Create a hidden draft event
    let draft_res = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Secret Planning Retreat",
            "kind": "other",
            "icon": "tent",
            "starts_at": tomorrow("10:00"),
            "location": "Retreat Centre",
            "capacity": 20,
            "attendance_enabled": true,
            "group_ids": [all_gid],
            "status": "draft",
        })),
    )
    .await;
    assert_eq!(draft_res.status, StatusCode::OK, "{}", draft_res.json);
    assert_eq!(draft_res.json["status"], "draft");
    let ev_id = draft_res.json["id"].as_i64().unwrap();

    // Zero notifications sent for draft creation
    assert_eq!(notif_count(&st.db, member_id).await, initial_notifs);

    // Creator sees draft status and publish capability
    let admin_detail = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(admin_detail.status, StatusCode::OK, "{}", admin_detail.json);
    assert_eq!(admin_detail.json["status"], "draft");
    assert_eq!(admin_detail.json["can_publish"], true);
    assert_eq!(admin_detail.json["can_unpublish"], false);

    // 2. Draft is isolated from target-group members across lists, detail, signup, and .ics
    for list_path in [
        "/api/events",
        "/api/events?scope=upcoming",
        "/api/events?scope=past",
        "/api/events?scope=registered",
    ] {
        let r = call(&app, Method::GET, list_path, Some(&member), None).await;
        assert_eq!(r.status, StatusCode::OK, "{list_path}: {}", r.json);
        let arr = r.json.as_array().unwrap();
        assert!(
            !arr.iter().any(|x| x["id"].as_i64() == Some(ev_id)),
            "draft leaked in {list_path}: {}",
            r.json
        );
    }
    let member_detail = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(member_detail.status, StatusCode::FORBIDDEN);

    let member_signup = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/signup"),
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(member_signup.status, StatusCode::FORBIDDEN);

    let (ics_status, _, ics_body) = call_raw(&app, Method::GET, &ics_path, None).await;
    assert_eq!(ics_status, StatusCode::OK);
    assert!(
        !ics_body.contains(&format!("gr-event-{ev_id}@")),
        "draft leaked in calendar .ics"
    );

    // 3. Downstream mutations on a draft are blocked with EVENT_NOT_OPEN
    let att_res = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/attendance"),
        Some(&admin),
        Some(json!({ "user_id": member_id, "attended": true })),
    )
    .await;
    assert_eq!(att_res.status, StatusCode::BAD_REQUEST, "{}", att_res.json);
    assert_eq!(att_res.json["code"], "EVENT_NOT_OPEN");

    let settle_res = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/settlements"),
        Some(&admin),
        Some(json!({
            "title": "Draft Expense",
            "items": [{ "label": "Venue", "amount_cents": 1000 }],
            "split_mode": "equal",
            "participants": [{ "user_id": member_id }],
            "payee_name": "Admin",
            "payee_account": "123456"
        })),
    )
    .await;
    assert_eq!(
        settle_res.status,
        StatusCode::BAD_REQUEST,
        "{}",
        settle_res.json
    );
    assert_eq!(settle_res.json["code"], "EVENT_NOT_OPEN");

    let receipt_id = upload_png(&app, &member).await;
    let reimb_res = call_exact(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&member),
        Some(json!({
            "event_id": ev_id,
            "amount_cents": 1500,
            "description": "Grab to draft event",
            "receipt_file_id": receipt_id,
            "payee_name": "Member",
            "payee_phone": "0123456789"
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(
        reimb_res.status,
        StatusCode::BAD_REQUEST,
        "{}",
        reimb_res.json
    );
    assert_eq!(reimb_res.json["code"], "EVENT_NOT_OPEN");

    // 4. Organiser sees draft in scope=managing, hide_drafts=true omits it, editing keeps draft silent
    let managing = call(
        &app,
        Method::GET,
        "/api/events?scope=managing",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(managing.status, StatusCode::OK);
    assert!(managing
        .json
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"].as_i64() == Some(ev_id) && x["status"] == "draft"));

    let managing_hidden = call(
        &app,
        Method::GET,
        "/api/events?scope=managing&hide_drafts=true",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(managing_hidden.status, StatusCode::OK);
    assert!(!managing_hidden
        .json
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"].as_i64() == Some(ev_id)));

    let upd_res = call(
        &app,
        Method::PUT,
        &format!("/api/events/{ev_id}"),
        Some(&admin),
        Some(json!({
            "title": "Church Fellowship Retreat",
            "kind": "other",
            "icon": "tent",
            "starts_at": tomorrow("10:00"),
            "location": "Retreat Centre",
            "capacity": 20,
            "attendance_enabled": true,
            "group_ids": [all_gid]
        })),
    )
    .await;
    assert_eq!(upd_res.status, StatusCode::OK, "{}", upd_res.json);
    assert_eq!(notif_count(&st.db, member_id).await, initial_notifs);

    // 5. Publish sends notifications, writes audit log, exposes event, and blocks double-publish
    let pub_res = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/publish"),
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(pub_res.status, StatusCode::OK, "{}", pub_res.json);
    assert_eq!(pub_res.json["status"], "published");
    assert!(notif_count(&st.db, member_id).await > initial_notifs);

    let (pub_audits,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action='event.publish' AND entity_id=$1",
    )
    .bind(ev_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(pub_audits, 1);

    let member_list_after = call(&app, Method::GET, "/api/events", Some(&member), None).await;
    assert!(member_list_after
        .json
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"].as_i64() == Some(ev_id)));
    let member_detail_after = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(member_detail_after.status, StatusCode::OK);
    assert_eq!(member_detail_after.json["status"], "published");
    let as_gid = member_detail_after.json["join_as"][0]["id"]
        .as_i64()
        .unwrap();

    let (ics_status_before_signup, _, ics_body_before_signup) =
        call_raw(&app, Method::GET, &ics_path, None).await;
    assert_eq!(ics_status_before_signup, StatusCode::OK);
    assert!(!ics_body_before_signup.contains(&format!("gr-event-{ev_id}@")));

    let double_pub = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/publish"),
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(
        double_pub.status,
        StatusCode::BAD_REQUEST,
        "{}",
        double_pub.json
    );
    assert_eq!(double_pub.json["code"], "EVENT_NOT_DRAFT");

    // 6. Hide / Unpublish with 0 signups succeeds and writes audit log
    let admin_before_unpub = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(admin_before_unpub.json["can_unpublish"], true);

    let unpub_res = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/unpublish"),
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(unpub_res.status, StatusCode::OK, "{}", unpub_res.json);
    assert_eq!(unpub_res.json["status"], "draft");

    let (unpub_audits,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action='event.unpublish' AND entity_id=$1",
    )
    .bind(ev_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(unpub_audits, 1);

    let member_hidden_again = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(member_hidden_again.status, StatusCode::FORBIDDEN);

    // 7. Re-publish, member signs up -> unpublish blocked with 409 EVENT_CANNOT_HIDE_WITH_SIGNUPS
    let repub = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/publish"),
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(repub.status, StatusCode::OK);

    let signup = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/signup"),
        Some(&member),
        Some(json!({ "as_group_id": as_gid })),
    )
    .await;
    assert_eq!(signup.status, StatusCode::OK, "{}", signup.json);

    let (ics_status_after, _, ics_body_after) = call_raw(&app, Method::GET, &ics_path, None).await;
    assert_eq!(ics_status_after, StatusCode::OK);
    assert!(
        ics_body_after.contains(&format!("gr-event-{ev_id}@")),
        "signed-up published event missing from calendar .ics"
    );

    let admin_after_signup = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(admin_after_signup.json["can_unpublish"], false);

    let blocked_unpub = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/unpublish"),
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(
        blocked_unpub.status,
        StatusCode::CONFLICT,
        "{}",
        blocked_unpub.json
    );
    assert_eq!(blocked_unpub.json["code"], "EVENT_CANNOT_HIDE_WITH_SIGNUPS");

    // 8. Cancelling a draft succeeds without notifying members
    let draft2 = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Cancelled Draft",
            "kind": "other",
            "starts_at": tomorrow("15:00"),
            "location": "Room 3",
            "group_ids": [all_gid],
            "status": "draft",
        })),
    )
    .await;
    assert_eq!(draft2.status, StatusCode::OK);
    let draft2_id = draft2.json["id"].as_i64().unwrap();
    let before_cancel_notifs = notif_count(&st.db, member_id).await;

    let cancel_res = call(
        &app,
        Method::POST,
        &format!("/api/events/{draft2_id}/cancel"),
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(cancel_res.status, StatusCode::OK, "{}", cancel_res.json);
    assert_eq!(notif_count(&st.db, member_id).await, before_cancel_notifs);
    let cancelled_detail = call(
        &app,
        Method::GET,
        &format!("/api/events/{draft2_id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(cancelled_detail.json["status"], "cancelled");
}
