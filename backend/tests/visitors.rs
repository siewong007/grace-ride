//! Visitor passes: a one-time link for a visitor's journey to an event, which closes when
//! the journey ends (spec section 4).

mod common;

use axum::http::{header, Method, StatusCode};
use common::*;
use grace_ride::state::AppState;
use serde_json::{json, Value};

fn in_hours(h: i64) -> String {
    (grace_ride::util::local_now() + chrono::Duration::hours(h))
        .format("%Y-%m-%dT%H:%M")
        .to_string()
}

async fn group_id(st: &AppState, name: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM groups WHERE name=$1")
        .bind(name)
        .fetch_one(&st.db)
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

async fn only_in(st: &AppState, user: i64, groups: &[i64]) {
    sqlx::query("DELETE FROM group_members WHERE user_id=$1")
        .bind(user)
        .execute(&st.db)
        .await
        .unwrap();
    for g in groups {
        sqlx::query("INSERT INTO group_members (group_id, user_id) VALUES ($1,$2)")
            .bind(g)
            .bind(user)
            .execute(&st.db)
            .await
            .unwrap();
    }
}

/// The token at the end of a visitor link.
fn token(r: &Value) -> String {
    r["url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .to_string()
}

/// The `gr_visit=...` cookie a claim sets.
fn visit_cookie(res: &Res) -> String {
    res.headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with("gr_visit="))
        .map(|v| v.split(';').next().unwrap().to_string())
        .expect("claim sets gr_visit")
}

#[tokio::test]
async fn a_visitor_link_is_one_time_and_closes_with_the_journey() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let youth = group_id(&st, "青年团契").await;
    let staff = {
        let r = call(
            &app,
            Method::POST,
            "/api/admin/groups",
            Some(&admin),
            Some(json!({"name": "青年同工", "parent_id": youth})),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
        group_id(&st, "青年同工").await
    };
    let member = user_id(&st, "member@example.org").await;
    let sophie = user_id(&st, "sophie@example.org").await;
    let driver = user_id(&st, "driver@example.org").await;
    let driver2 = user_id(&st, "driver2@example.org").await;
    only_in(&st, member, &[youth]).await;
    only_in(&st, sophie, &[youth]).await;
    only_in(&st, driver, &[youth]).await;
    only_in(&st, driver2, &[staff]).await;
    let host = login(&st, "member@example.org").await;
    let not_going = login(&st, "sophie@example.org").await;
    let driver_c = login(&st, "driver@example.org").await;
    let driver2_c = login(&st, "driver2@example.org").await;

    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({"title": "青年营", "kind": "other", "starts_at": in_hours(5), "location": "教会", "group_ids": [youth]})),
    )
    .await;
    assert_eq!(ev.status, StatusCode::OK, "{}", ev.json);
    let ev = ev.json["id"].as_i64().unwrap();
    let invite = |pickup: String, name: &str| json!({"name": name, "phone": "012-345 6789", "pickup_at": pickup, "pickup_area_id": 1, "pickup_address": "Jalan Visitor 1"});
    let path = format!("/api/events/{ev}/visitors");

    // Only someone going (or the organiser) may bring a visitor.
    let r = call(
        &app,
        Method::POST,
        &path,
        Some(&not_going),
        Some(invite(in_hours(3), "Ann")),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "ONLY_GOING_CAN_INVITE_VISITOR");
    let r = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev}/signup"),
        Some(&host),
        Some(json!({})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let r = call(
        &app,
        Method::POST,
        &path,
        Some(&host),
        Some(invite(in_hours(3), " ")),
    )
    .await;
    assert_eq!(r.json["code"], "VISITOR_NAME_REQUIRED");

    let pass = call(
        &app,
        Method::POST,
        &path,
        Some(&host),
        Some(invite(in_hours(3), "Ann")),
    )
    .await;
    assert_eq!(pass.status, StatusCode::OK, "{}", pass.json);
    let tok = token(&pass.json);
    let ride = pass.json["ride_id"].as_i64().unwrap();
    let link = format!("/api/visit/{tok}");

    // The visitor's ride is booked in the host's name but is not the host's own ride.
    let ev_view = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev}"),
        Some(&host),
        None,
    )
    .await;
    assert!(
        ev_view.json["my_ride"].is_null(),
        "{}",
        ev_view.json["my_ride"]
    );

    // Opening the link uses nothing up; claiming does, once.
    let r = call(&app, Method::GET, &link, None, None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["state"], "open");
    assert_eq!(r.json["event"]["title"], "青年营");
    let claimed = call(&app, Method::POST, &format!("{link}/claim"), None, None).await;
    assert_eq!(claimed.status, StatusCode::OK, "{}", claimed.json);
    let visitor = visit_cookie(&claimed);
    let r = call(&app, Method::GET, &link, Some(&visitor), None).await;
    assert_eq!(r.json["state"], "mine");
    let again = call(
        &app,
        Method::POST,
        &format!("{link}/claim"),
        Some(&visitor),
        None,
    )
    .await;
    assert_eq!(
        again.status,
        StatusCode::OK,
        "the same browser may open it again"
    );
    let other = call(&app, Method::POST, &format!("{link}/claim"), None, None).await;
    assert_eq!(other.status, StatusCode::CONFLICT, "{}", other.json);
    assert_eq!(other.json["code"], "VISITOR_LINK_USED");
    let r = call(&app, Method::GET, &link, None, None).await;
    assert_eq!(r.json["state"], "used");

    // The visitor sees their ride and nothing else.
    let me = call(&app, Method::GET, "/api/visit/me", Some(&visitor), None).await;
    assert_eq!(me.status, StatusCode::OK, "{}", me.json);
    assert_eq!(me.json["name"], "Ann");
    assert_eq!(me.json["ride"]["status"], "pending_driver");
    assert!(me.json["ride"]["driver"].is_null());
    for member_api in [
        "/api/rides",
        "/api/auth/me",
        "/api/events",
        &format!("/api/rides/{ride}"),
    ] {
        let r = call(&app, Method::GET, member_api, Some(&visitor), None).await;
        assert_eq!(
            r.status,
            StatusCode::UNAUTHORIZED,
            "{member_api} must refuse a visitor: {}",
            r.json
        );
    }
    let r = call(&app, Method::GET, "/api/visit/me", Some(&host), None).await;
    assert_eq!(
        r.status,
        StatusCode::UNAUTHORIZED,
        "a member session is not a visitor pass"
    );

    // Drivers are matched on the host's group (Youth), not "Visitor".
    let listed = |cookie: String| {
        let app = app.clone();
        async move {
            let r = call(
                &app,
                Method::GET,
                "/api/driver/requests",
                Some(&cookie),
                None,
            )
            .await;
            r.json.as_array().unwrap().iter().any(|x| x["id"] == ride)
        }
    };
    assert!(
        listed(driver_c.clone()).await,
        "a Youth driver sees the visitor's ride"
    );
    assert!(
        !listed(driver2_c.clone()).await,
        "a Youth-staff driver does not"
    );
    let r = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ride}"),
        Some(&host),
        None,
    )
    .await;
    assert_eq!(r.json["as_group"]["name"], "访客", "reported as a visitor");

    // The host withdrawing cancels only their own ride; their visitor may still come.
    let r = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev}/withdraw"),
        Some(&host),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let still = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ride}"),
        Some(&host),
        None,
    )
    .await;
    assert_eq!(still.json["status"], "pending_driver");
    assert_eq!(
        call(&app, Method::GET, "/api/visit/me", Some(&visitor), None)
            .await
            .status,
        StatusCode::OK
    );
    let r = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev}/signup"),
        Some(&host),
        Some(json!({})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // The organiser, in none of the event's groups, can still join as one of them and
    // bring a visitor.
    let admin_id = user_id(&st, "admin@example.org").await;
    only_in(&st, admin_id, &[]).await;
    let as_admin = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(
        as_admin.json["join_as"][0]["id"], youth,
        "{}",
        as_admin.json["join_as"]
    );
    let r = call(
        &app,
        Method::POST,
        &path,
        Some(&admin),
        Some(invite(in_hours(3), "Eve")),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let eve_ride = r.json["ride_id"].as_i64().unwrap();
    let eve = call(
        &app,
        Method::GET,
        "/api/driver/requests",
        Some(&driver_c),
        None,
    )
    .await;
    assert!(
        eve.json
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["id"] == eve_ride),
        "matched on the organiser's Youth fit"
    );

    // Completing the ride ends the journey: the page closes and the cookie stops working.
    sqlx::query("UPDATE rides SET status='completed' WHERE id=$1")
        .bind(ride)
        .execute(&st.db)
        .await
        .unwrap();
    let me = call(&app, Method::GET, "/api/visit/me", Some(&visitor), None).await;
    assert_eq!(me.status, StatusCode::CONFLICT, "{}", me.json);
    assert_eq!(me.json["code"], "VISITOR_JOURNEY_ENDED");
    let r = call(&app, Method::GET, &link, None, None).await;
    assert_eq!(
        (r.json["state"].as_str(), r.json["closed_reason"].as_str()),
        (Some("closed"), Some("completed"))
    );
    let me = call(&app, Method::GET, "/api/visit/me", Some(&visitor), None).await;
    assert_eq!(
        me.status,
        StatusCode::UNAUTHORIZED,
        "the session is gone once closed"
    );

    // A link opens 24 hours before pickup.
    let later = call(
        &app,
        Method::POST,
        &path,
        Some(&host),
        Some(invite(in_hours(60), "Ben")),
    )
    .await;
    assert_eq!(later.status, StatusCode::OK, "{}", later.json);
    let later_link = format!("/api/visit/{}", token(&later.json));
    let r = call(&app, Method::GET, &later_link, None, None).await;
    assert_eq!(r.json["state"], "not_yet");
    let r = call(
        &app,
        Method::POST,
        &format!("{later_link}/claim"),
        None,
        None,
    )
    .await;
    assert_eq!(r.json["code"], "VISITOR_NOT_OPEN_YET");

    // "Can't make it" from the visitor cancels the ride and closes the link.
    let third = call(
        &app,
        Method::POST,
        &path,
        Some(&host),
        Some(invite(in_hours(2), "Cat")),
    )
    .await;
    let third_link = format!("/api/visit/{}", token(&third.json));
    let third_ride = third.json["ride_id"].as_i64().unwrap();
    let c = visit_cookie(
        &call(
            &app,
            Method::POST,
            &format!("{third_link}/claim"),
            None,
            None,
        )
        .await,
    );
    sqlx::query("UPDATE rides SET status='confirmed', driver_id=$1 WHERE id=$2")
        .bind(driver)
        .bind(third_ride)
        .execute(&st.db)
        .await
        .unwrap();
    let me = call(&app, Method::GET, "/api/visit/me", Some(&c), None).await;
    assert!(
        me.json["ride"]["driver"]["name"].is_string(),
        "driver shown once approved"
    );
    let r = call(
        &app,
        Method::POST,
        "/api/visit/me/readiness",
        Some(&c),
        Some(json!({"status": "unable_to_join", "notes": "  flight delayed  "})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let ride_now = call(
        &app,
        Method::GET,
        &format!("/api/rides/{third_ride}"),
        Some(&host),
        None,
    )
    .await;
    assert_eq!(ride_now.json["status"], "cancelled");
    assert!(ride_now.json["attention_reason"]
        .as_str()
        .unwrap()
        .contains("flight delayed"));
    let r = call(&app, Method::GET, &third_link, None, None).await;
    assert_eq!(r.json["closed_reason"], "cancelled");

    // Reissue stops the old link; revoke stops the new one.
    let fourth = call(
        &app,
        Method::POST,
        &path,
        Some(&host),
        Some(invite(in_hours(4), "Dan")),
    )
    .await;
    let fourth_id = fourth.json["id"].as_i64().unwrap();
    let fourth_link = format!("/api/visit/{}", token(&fourth.json));
    let re = call(
        &app,
        Method::POST,
        &format!("/api/visitor-passes/{fourth_id}/reissue"),
        Some(&host),
        None,
    )
    .await;
    assert_eq!(re.status, StatusCode::OK, "{}", re.json);
    let r = call(&app, Method::GET, &fourth_link, None, None).await;
    assert_eq!(r.json["closed_reason"], "reissued");
    let new_link = format!("/api/visit/{}", token(&re.json));
    assert_eq!(
        call(&app, Method::GET, &new_link, None, None).await.json["state"],
        "open"
    );
    let r = call(
        &app,
        Method::POST,
        &format!("/api/visitor-passes/{fourth_id}/revoke"),
        Some(&not_going),
        None,
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::FORBIDDEN,
        "only the host, organiser or staff"
    );
    let new_id = re.json["id"].as_i64().unwrap();
    let r = call(
        &app,
        Method::POST,
        &format!("/api/visitor-passes/{new_id}/revoke"),
        Some(&host),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(
        call(&app, Method::GET, &new_link, None, None).await.json["closed_reason"],
        "revoked"
    );

    // The host's list shows each visitor once, with where their link stands.
    let list = call(&app, Method::GET, &path, Some(&host), None).await;
    assert_eq!(list.status, StatusCode::OK, "{}", list.json);
    let names: Vec<_> = list
        .json
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names.iter().filter(|n| **n == "Dan").count(),
        1,
        "{names:?}"
    );

    // The background sweep closes links whose time has run out.
    sqlx::query(
        "UPDATE visitor_passes SET valid_from='2000-01-01T00:00:00', valid_until='2000-01-01T12:00:00' WHERE closed_at IS NULL",
    )
    .execute(&st.db)
    .await
    .unwrap();
    grace_ride::jobs::run_once(&st.db, &st.cfg).await.unwrap();
    let open: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM visitor_passes WHERE closed_at IS NULL")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(open, 0);
    assert_eq!(
        call(&app, Method::GET, &later_link, None, None).await.json["closed_reason"],
        "expired"
    );
}
