//! Audit-log coverage: every state change, failed sign-in and automatic change leaves
//! an entry. These cover the actions that used to write nothing.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::{jobs, state::AppState};
use serde_json::json;
use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs::prelude::CreationChallengeResponse;

/// (actor, entity id, detail) of every entry with this action, oldest first.
async fn logged(st: &AppState, action: &str) -> Vec<(Option<i64>, Option<i64>, String)> {
    sqlx::query_as("SELECT actor_id, entity_id, detail FROM audit_logs WHERE action=$1 ORDER BY id")
        .bind(action)
        .fetch_all(&st.db)
        .await
        .unwrap()
}

async fn user_id(st: &AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

/// Percent-encode every byte, so any name is safe in a query string.
fn urlencode(s: &str) -> String {
    s.bytes().map(|b| format!("%{b:02X}")).collect()
}

/// A well-formed sign-in response that no passkey produced.
fn forged_credential() -> serde_json::Value {
    json!({
        "id": "AAAA", "rawId": "AAAA", "type": "public-key", "extensions": {},
        "response": {"authenticatorData": "AAAA", "clientDataJSON": "AAAA", "signature": "AAAA", "userHandle": null},
    })
}

#[tokio::test]
async fn sign_in_and_account_security_events_are_logged() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;
    let uid = user_id(&st, "member@example.org").await;

    // Register a passkey so the member gets real sign-in challenges.
    let start = call(
        &app,
        Method::POST,
        "/api/auth/passkey/register/start",
        Some(&cookie),
        None,
    )
    .await;
    let options: CreationChallengeResponse =
        serde_json::from_value(start.json["options"].clone()).unwrap();
    let mut soft = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let credential = soft
        .do_registration(Url::parse(ORIGIN).unwrap(), options)
        .unwrap();
    let finish = call(
        &app,
        Method::POST,
        "/api/auth/passkey/register/finish",
        Some(&cookie),
        Some(json!({"challenge_id": start.json["challenge_id"], "credential": credential, "label": "Audit key"})),
    )
    .await;
    assert_eq!(finish.status, StatusCode::OK, "{}", finish.json);
    assert_eq!(
        logged(&st, "passkey.register").await.last().unwrap(),
        &(Some(uid), Some(uid), "Audit key".to_string())
    );

    // A wrong passkey against a real challenge names the account, with no actor.
    let lstart = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/start",
        None,
        Some(json!({"email": "member@example.org"})),
    )
    .await;
    let bad = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/finish",
        None,
        Some(
            json!({"challenge_id": lstart.json["challenge_id"], "credential": forged_credential()}),
        ),
    )
    .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST, "{}", bad.json);
    // A challenge that never existed is logged too, against nobody.
    let unknown = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/finish",
        None,
        Some(json!({"challenge_id": "no-such-challenge", "credential": forged_credential()})),
    )
    .await;
    assert_eq!(unknown.status, StatusCode::BAD_REQUEST, "{}", unknown.json);
    let failed = logged(&st, "login.failed").await;
    assert_eq!(failed.len(), 2, "{failed:?}");
    assert_eq!((failed[0].0, failed[0].1), (None, Some(uid)));
    assert!(failed[0].2.starts_with("passkey rejected"), "{failed:?}");
    assert_eq!((failed[1].0, failed[1].1), (None, None));

    // Authenticator: starting setup and a wrong code.
    let setup_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(setup_res.status, StatusCode::OK, "{}", setup_res.json);
    assert_eq!(logged(&st, "totp.setup").await.len(), 1);
    let right: u32 = totp_code(
        setup_res.json["secret"].as_str().unwrap(),
        "member@example.org",
    )
    .parse()
    .unwrap();
    let wrong = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&cookie),
        Some(json!({"code": format!("{:06}", (right + 500_000) % 1_000_000)})),
    )
    .await;
    assert_eq!(wrong.status, StatusCode::BAD_REQUEST, "{}", wrong.json);
    assert_eq!(logged(&st, "totp.failed").await.len(), 1);
    assert!(logged(&st, "totp.enable").await.is_empty());

    // Removing a passkey is logged once, with its label; a repeat removes nothing.
    let keys = call(&app, Method::GET, "/api/auth/passkeys", Some(&cookie), None).await;
    let key_id = keys.json[0]["id"].as_i64().unwrap();
    for _ in 0..2 {
        let del = call(
            &app,
            Method::DELETE,
            &format!("/api/auth/passkeys/{key_id}"),
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(del.status, StatusCode::OK, "{}", del.json);
    }
    let deleted = logged(&st, "passkey.delete").await;
    assert_eq!(deleted.len(), 1, "{deleted:?}");
    assert!(deleted[0].2.contains("Audit key"), "{deleted:?}");

    // Recovery requests: logged for a member, silent for an unknown address.
    for email in ["member@example.org", "nobody-here@example.org"] {
        let r = call(
            &app,
            Method::POST,
            "/api/auth/recovery-request",
            None,
            Some(json!({"email": email, "reason": "lost phone"})),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    }
    assert_eq!(
        logged(&st, "recovery.request").await,
        vec![(None, Some(uid), "member@example.org".to_string())]
    );

    let out = call(&app, Method::POST, "/api/auth/logout", Some(&cookie), None).await;
    assert_eq!(out.status, StatusCode::OK);
    assert_eq!(
        logged(&st, "logout").await,
        vec![(Some(uid), Some(uid), String::new())]
    );
}

#[tokio::test]
async fn member_actions_are_logged() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let uid = user_id(&st, "member@example.org").await;
    let did = user_id(&st, "driver@example.org").await;

    let file = upload_png(&app, &member).await;
    let uploads = logged(&st, "file.upload").await;
    assert_eq!(uploads.len(), 1, "{uploads:?}");
    assert_eq!((uploads[0].0, uploads[0].1), (Some(uid), Some(file)));
    assert!(uploads[0].2.starts_with("image/png"), "{uploads:?}");

    // Help centre: opening a request and each reply.
    let t = call(
        &app,
        Method::POST,
        "/api/help",
        Some(&member),
        Some(json!({"subject": "改上车点", "body": "可以改到南门吗？"})),
    )
    .await;
    assert_eq!(t.status, StatusCode::OK, "{}", t.json);
    let tid = t.json["id"].as_i64().unwrap();
    for who in [&transport, &member] {
        let r = call(
            &app,
            Method::POST,
            &format!("/api/help/{tid}/replies"),
            Some(who),
            Some(json!({"body": "好的"})),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    }
    assert_eq!(
        logged(&st, "help.create").await,
        vec![(Some(uid), Some(tid), "改上车点".to_string())]
    );
    let replies: Vec<String> = logged(&st, "help.reply")
        .await
        .into_iter()
        .map(|r| r.2)
        .collect();
    assert_eq!(replies, ["by staff", "by member"]);

    // Profile: the entry names the fields sent, never their values.
    let p = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&member),
        Some(json!({"lang": "en", "phone": "012-345 6789"})),
    )
    .await;
    assert_eq!(p.status, StatusCode::OK, "{}", p.json);
    assert_eq!(
        logged(&st, "profile.update").await.last().unwrap().2,
        "phone, lang"
    );

    // Driver settings and vehicle.
    let d = call(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&driver),
        Some(json!({
            "carpool_enabled": true, "spare_seats": 3, "area_ids": [1],
            "vehicle": {"model": "Myvi", "color": "white", "plate": "wxy 1234", "seats": 5, "vehicle_type": "standard"},
        })),
    )
    .await;
    assert_eq!(d.status, StatusCode::OK, "{}", d.json);
    let profile = logged(&st, "driver.profile_update").await;
    assert_eq!(profile.len(), 1, "{profile:?}");
    assert_eq!((profile[0].0, profile[0].1), (Some(did), Some(did)));
    assert!(profile[0].2.contains("WXY 1234"), "{profile:?}");

    // Deleting a draft bill.
    let create = call(
        &app,
        Method::POST,
        "/api/events/5/settlements",
        Some(&driver),
        Some(json!({
            "title": "Draft to drop",
            "items": [{"label": "court", "amount_cents": 2000}],
            "split_mode": "equal",
            "participants": [{"user_id": 4, "amount_cents": 1000}, {"user_id": 6, "amount_cents": 1000}],
            "payee_name": "李德明", "payee_account": "012-300 0004", "due_date": null,
        })),
    )
    .await;
    assert_eq!(create.status, StatusCode::OK, "{}", create.json);
    let sid = create.json["id"].as_i64().unwrap();
    let del = call(
        &app,
        Method::DELETE,
        &format!("/api/settlements/{sid}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(del.status, StatusCode::OK, "{}", del.json);
    assert_eq!(
        logged(&st, "settlement.delete").await,
        vec![(Some(did), Some(sid), "draft: Draft to drop".to_string())]
    );
}

#[tokio::test]
async fn automatic_and_cascading_changes_are_logged() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let sophie = login(&st, "sophie@example.org").await;
    let aid = user_id(&st, "admin@example.org").await;
    let uid = user_id(&st, "member@example.org").await;
    let ride = |event: Option<i64>, at: String| {
        json!({
            "pickup_at": at, "pickup_area_id": 1, "pickup_address": "audit test",
            "destination": "教会", "seats": 1, "assistance": "", "event_id": event,
        })
    };
    let new_ride = |who: String, body: serde_json::Value| {
        let app = app.clone();
        async move {
            let r = call(&app, Method::POST, "/api/rides", Some(&who), Some(body)).await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.json);
            r.json["id"].as_i64().unwrap()
        }
    };

    // The background job flags a ride with no driver close to pickup: no actor.
    let soon = (chrono::Local::now() + chrono::Duration::hours(3))
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    let lonely = new_ride(member.clone(), ride(None, soon)).await;
    jobs::run_once(&st.db, &st.cfg).await.unwrap();
    assert!(
        logged(&st, "ride.escalate").await.contains(&(
            None,
            Some(lonely),
            "no driver 12h before pickup".to_string()
        )),
        "escalation not logged"
    );

    // An event open to everyone, with rides linked to it.
    let all: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE is_all=1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Audit outing", "kind": "walk", "starts_at": tomorrow("19:00"),
            "location": "教会", "group_ids": [all],
        })),
    )
    .await;
    assert_eq!(ev.status, StatusCode::OK, "{}", ev.json);
    let eid = ev.json["id"].as_i64().unwrap();
    let signup = call(
        &app,
        Method::POST,
        &format!("/api/events/{eid}/signup"),
        Some(&member),
        Some(json!({"need_ride": true})),
    )
    .await;
    assert_eq!(signup.status, StatusCode::OK, "{}", signup.json);
    assert_eq!(
        logged(&st, "event.signup").await.last().unwrap(),
        &(Some(uid), Some(eid), "going; needs a ride".to_string())
    );
    let mine = new_ride(member.clone(), ride(Some(eid), tomorrow("18:00"))).await;
    let theirs = new_ride(sophie.clone(), ride(Some(eid), tomorrow("18:10"))).await;

    // Withdrawing from the event cancels the member's own linked ride.
    let w = call(
        &app,
        Method::POST,
        &format!("/api/events/{eid}/withdraw"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(w.status, StatusCode::OK, "{}", w.json);
    // Cancelling the event cancels every ride still linked to it.
    let c = call(
        &app,
        Method::POST,
        &format!("/api/events/{eid}/cancel"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(c.status, StatusCode::OK, "{}", c.json);
    let cancelled = logged(&st, "ride.cancel").await;
    assert!(
        cancelled.contains(&(Some(uid), Some(mine), format!("withdrew from event {eid}"))),
        "{cancelled:?}"
    );
    assert!(
        cancelled.contains(&(Some(aid), Some(theirs), format!("event {eid} cancelled"))),
        "{cancelled:?}"
    );
    assert_eq!(
        cancelled.iter().filter(|r| r.1 == Some(mine)).count(),
        1,
        "{cancelled:?}"
    );
}

#[tokio::test]
async fn location_sharing_is_logged_when_it_starts_and_stops_only() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let uid = user_id(&st, "member@example.org").await;
    let did = user_id(&st, "driver@example.org").await;
    let created = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "pickup_at": tomorrow("18:00"), "pickup_area_id": 1, "pickup_address": "audit test",
            "destination": "教会", "seats": 1, "assistance": "",
        })),
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    let rid = created.json["id"].as_i64().unwrap();
    sqlx::query("UPDATE rides SET status='in_progress', driver_id=$1 WHERE id=$2")
        .bind(did)
        .bind(rid)
        .execute(&st.db)
        .await
        .unwrap();

    // Three positions from the passenger: one entry, for the first.
    let spot = format!("/api/rides/{rid}/passenger-location");
    for _ in 0..3 {
        let r = call(
            &app,
            Method::POST,
            &spot,
            Some(&member),
            Some(json!({"lat": 3.1, "lng": 101.6})),
        )
        .await;
        assert_eq!(r.json["sharing"], true, "{}", r.json);
    }
    assert_eq!(
        logged(&st, "location.share").await,
        vec![(Some(uid), Some(rid), "passenger".to_string())]
    );

    // Positions from the driver are never logged; switching off is, once.
    let pos = call(
        &app,
        Method::POST,
        "/api/driver/location",
        Some(&driver),
        Some(json!({"lat": 3.1, "lng": 101.6})),
    )
    .await;
    assert_eq!(pos.json["sharing"], true, "{}", pos.json);
    for _ in 0..2 {
        call(&app, Method::DELETE, &spot, Some(&member), None).await;
        call(
            &app,
            Method::DELETE,
            "/api/driver/location",
            Some(&driver),
            None,
        )
        .await;
    }
    assert_eq!(
        logged(&st, "location.stop").await,
        vec![
            (Some(uid), Some(rid), "passenger".to_string()),
            (Some(did), Some(did), "driver".to_string()),
        ]
    );
}

#[tokio::test]
async fn audit_log_pages_through_every_entry_and_filters() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let aid = user_id(&st, "admin@example.org").await;
    for i in 1..=7 {
        grace_ride::util::audit(
            &st.db,
            Some(aid),
            "widget.turn",
            "widget",
            Some(i),
            format!("item {i}"),
        )
        .await;
    }
    grace_ride::util::audit(&st.db, None, "widget.finish", "widget", None, "100%_done").await;
    let get = |query: String| {
        let (app, admin) = (app.clone(), admin.clone());
        async move {
            let r = call(
                &app,
                Method::GET,
                &format!("/api/admin/audit?entity=widget{query}"),
                Some(&admin),
                None,
            )
            .await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.json);
            r.json
        }
    };

    // Three at a time, newest first, until the cursor runs out.
    let mut seen: Vec<i64> = vec![];
    let mut cursor = String::new();
    let mut pages = 0;
    loop {
        let page = get(format!("&limit=3{cursor}")).await;
        let items = page["items"].as_array().unwrap();
        assert!(items.len() <= 3);
        seen.extend(items.iter().map(|i| i["id"].as_i64().unwrap()));
        pages += 1;
        match page["next_before"].as_i64() {
            Some(id) => {
                assert_eq!(Some(&id), seen.last());
                cursor = format!("&before={id}");
            }
            None => break,
        }
    }
    assert_eq!((pages, seen.len()), (3, 8), "{seen:?}");
    assert!(seen.windows(2).all(|w| w[0] > w[1]), "{seen:?}");
    // A page that ends exactly on the last entry says so.
    assert!(get("&limit=8".into()).await["next_before"].is_null());

    // Search ignores case, reaches the actor's name, and treats % and _ literally.
    let count = |v: serde_json::Value| v["items"].as_array().unwrap().len();
    assert_eq!(count(get("&q=ITEM".into()).await), 7);
    assert_eq!(count(get("&q=100%25%5Fdone".into()).await), 1);
    assert_eq!(count(get("&q=%25".into()).await), 1);
    assert_eq!(count(get("&q=%5F".into()).await), 1);
    let (name,): (String,) = sqlx::query_as("SELECT name FROM users WHERE id=$1")
        .bind(aid)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let by_actor = get(format!("&q={}", urlencode(&name))).await;
    assert_eq!(count(by_actor), 7);
    assert_eq!(count(get(format!("&actor_id={aid}")).await), 7);
    assert_eq!(count(get("&actor_id=999999".into()).await), 0);
    assert_eq!(count(get("&entity_id=4".into()).await), 1);
    assert_eq!(count(get("&action=widget.turn".into()).await), 7);
    assert_eq!(count(get("&action=widget.finish".into()).await), 1);
    assert_eq!(count(get("&action=widget.unknown".into()).await), 0);

    // Dates include both ends.
    let day = |offset: i64| {
        (chrono::Local::now() + chrono::Duration::days(offset))
            .format("%Y-%m-%d")
            .to_string()
    };
    assert_eq!(
        count(get(format!("&from={}&to={}", day(0), day(0))).await),
        8
    );
    assert_eq!(count(get(format!("&from={}", day(1))).await), 0);
    assert_eq!(count(get(format!("&to={}", day(-1))).await), 0);
    let bad = call(
        &app,
        Method::GET,
        "/api/admin/audit?from=14-11-2026",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST, "{}", bad.json);
    assert_eq!(bad.json["code"], "INVALID_DATE");
}

#[tokio::test]
async fn audit_retention_purges_entries_past_retention_days() {
    let (_app, st) = setup().await;
    let old_ts = (chrono::Local::now() - chrono::Duration::days(st.cfg.audit_retention_days + 1))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    let new_ts = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    let (old_id,): (i64,) = sqlx::query_as(
        "INSERT INTO audit_logs (actor_id, action, entity, entity_id, detail, created_at)
         VALUES (NULL, 'test.old', 'test', NULL, 'past retention', $1) RETURNING id",
    )
    .bind(&old_ts)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let (new_id,): (i64,) = sqlx::query_as(
        "INSERT INTO audit_logs (actor_id, action, entity, entity_id, detail, created_at)
         VALUES (NULL, 'test.new', 'test', NULL, 'within retention', $1) RETURNING id",
    )
    .bind(&new_ts)
    .fetch_one(&st.db)
    .await
    .unwrap();

    // Run housekeeping job once.
    jobs::run_once(&st.db, &st.cfg).await.unwrap();

    let remaining_old: Option<i64> = sqlx::query_scalar("SELECT id FROM audit_logs WHERE id=$1")
        .bind(old_id)
        .fetch_optional(&st.db)
        .await
        .unwrap();
    assert!(
        remaining_old.is_none(),
        "Old audit entry should be purged by retention job"
    );

    let remaining_new: Option<i64> = sqlx::query_scalar("SELECT id FROM audit_logs WHERE id=$1")
        .bind(new_id)
        .fetch_optional(&st.db)
        .await
        .unwrap();
    assert_eq!(
        remaining_new,
        Some(new_id),
        "Recent audit entry must be retained"
    );
}

#[tokio::test]
async fn audit_stream_export_csv_with_filters_and_utf8_bom() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let admin_uid = user_id(&st, "admin@example.org").await;

    // Insert known audit logs including Chinese text and formula-like characters.
    let now_str = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    sqlx::query(
        "INSERT INTO audit_logs (actor_id, action, entity, entity_id, detail, created_at)
         VALUES ($1, 'stream.test_event', 'test', 101, '教会活动 =1+1 formula test', $2)",
    )
    .bind(admin_uid)
    .bind(&now_str)
    .execute(&st.db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO audit_logs (actor_id, action, entity, entity_id, detail, created_at)
         VALUES (NULL, 'system.routine', 'system', NULL, 'Routine cleanup', $1)",
    )
    .bind(&now_str)
    .execute(&st.db)
    .await
    .unwrap();

    // 1. Members without admin_audit cannot stream export.
    let (m_status, _, _) =
        call_raw(&app, Method::GET, "/api/admin/audit/export", Some(&member)).await;
    assert_eq!(m_status, StatusCode::FORBIDDEN);

    // 2. Admin streams full export.
    let (status, headers, body) =
        call_raw(&app, Method::GET, "/api/admin/audit/export", Some(&admin)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-type").unwrap().to_str().unwrap(),
        "text/csv; charset=utf-8"
    );
    assert!(headers
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("attachment; filename=\"audit_export_"));

    // Verify UTF-8 BOM is present.
    assert!(
        body.starts_with('\u{FEFF}'),
        "CSV export must begin with UTF-8 BOM"
    );
    assert!(body.contains("ID,Timestamp,Actor ID,Actor,Action,Entity,Entity ID,Detail"));
    assert!(body.contains("stream.test_event"));
    assert!(body.contains("system.routine"));
    assert!(body.contains("教会活动"));

    // 3. Admin streams filtered by action.
    let (f_status, _, f_body) = call_raw(
        &app,
        Method::GET,
        "/api/admin/audit/export?action=stream.test_event",
        Some(&admin),
    )
    .await;
    assert_eq!(f_status, StatusCode::OK);
    assert!(f_body.contains("stream.test_event"));
    assert!(!f_body.contains("system.routine"));

    // 4. Admin streams filtered by actor_id.
    let (a_status, _, a_body) = call_raw(
        &app,
        Method::GET,
        &format!("/api/admin/audit/export?actor_id={admin_uid}"),
        Some(&admin),
    )
    .await;
    assert_eq!(a_status, StatusCode::OK);
    assert!(a_body.contains("stream.test_event"));
    assert!(!a_body.contains("system.routine"));
}
