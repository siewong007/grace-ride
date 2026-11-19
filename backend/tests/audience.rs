//! Event audiences (see docs/TECHNICAL.md):
//! sub-group visibility, group-matched drivers, the audited admin override and the
//! masked event title.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::state::AppState;
use serde_json::json;

async fn id_of(st: &AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

/// Replace a member's groups, so each case starts from known memberships.
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

#[tokio::test]
async fn sub_groups_drivers_and_masked_titles() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let youth: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE name='青年团契'")
        .fetch_one(&st.db)
        .await
        .unwrap();

    // "Youth staff" is part of Youth.
    let r = call(
        &app,
        Method::POST,
        "/api/admin/groups",
        Some(&admin),
        Some(json!({"name": "青年同工", "parent_id": youth})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let staff: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE name='青年同工'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    // Youth cannot become part of its own sub-group.
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/groups/{youth}"),
        Some(&admin),
        Some(json!({"name": "青年团契", "parent_id": staff})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "INVALID_PARENT_GROUP");

    let member = id_of(&st, "member@example.org").await; // Youth only
    let sophie = id_of(&st, "sophie@example.org").await; // Youth staff only
    let amina = id_of(&st, "amina@example.org").await; // no group
    let driver = id_of(&st, "driver@example.org").await; // Youth
    let driver2 = id_of(&st, "driver2@example.org").await; // Youth staff
    only_in(&st, member, &[youth]).await;
    only_in(&st, sophie, &[staff]).await;
    only_in(&st, amina, &[]).await;
    only_in(&st, driver, &[youth]).await;
    only_in(&st, driver2, &[staff]).await;
    let member_c = login(&st, "member@example.org").await;
    let sophie_c = login(&st, "sophie@example.org").await;
    let amina_c = login(&st, "amina@example.org").await;
    let driver_c = login(&st, "driver@example.org").await;
    let driver2_c = login(&st, "driver2@example.org").await;

    let event = |title: &str, group: i64| json!({"title": title, "kind": "other", "starts_at": tomorrow("19:00"), "location": "教会", "group_ids": [group]});
    let youth_ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(event("青年营", youth)),
    )
    .await;
    assert_eq!(youth_ev.status, StatusCode::OK, "{}", youth_ev.json);
    let youth_ev = youth_ev.json["id"].as_i64().unwrap();
    let staff_ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(event("同工会", staff)),
    )
    .await;
    assert_eq!(staff_ev.status, StatusCode::OK, "{}", staff_ev.json);
    let staff_ev = staff_ev.json["id"].as_i64().unwrap();

    // Visibility runs down the tree, never up: Youth staff see the Youth event, Youth
    // members do not see the staff event, and someone in neither sees neither.
    let get = |cookie: &str, ev: i64| {
        let (app, cookie) = (app.clone(), cookie.to_string());
        async move {
            call(
                &app,
                Method::GET,
                &format!("/api/events/{ev}"),
                Some(&cookie),
                None,
            )
            .await
            .status
        }
    };
    assert_eq!(get(&sophie_c, youth_ev).await, StatusCode::OK);
    assert_eq!(get(&sophie_c, staff_ev).await, StatusCode::OK);
    assert_eq!(get(&member_c, youth_ev).await, StatusCode::OK);
    assert_eq!(get(&member_c, staff_ev).await, StatusCode::FORBIDDEN);
    assert_eq!(get(&amina_c, youth_ev).await, StatusCode::FORBIDDEN);

    // A Youth member's ride to the Youth event.
    let r = call(
        &app,
        Method::POST,
        &format!("/api/events/{youth_ev}/signup"),
        Some(&member_c),
        Some(json!({"need_ride": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let ride = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member_c),
        Some(json!({
            "pickup_at": tomorrow("18:00"), "pickup_area_id": 1, "pickup_address": "audience test",
            "destination": "教会", "seats": 1, "assistance": "", "event_id": youth_ev,
        })),
    )
    .await;
    assert_eq!(ride.status, StatusCode::OK, "{}", ride.json);
    let ride = ride.json["id"].as_i64().unwrap();
    let ride_row = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ride}"),
        Some(&member_c),
        None,
    )
    .await;
    assert_eq!(
        ride_row.json["as_group"]["id"], youth,
        "the ride keeps the signup's group"
    );

    // A Youth-staff driver may not drive a Youth passenger: not listed, and refused.
    let listed = |cookie: &str| {
        let (app, cookie) = (app.clone(), cookie.to_string());
        async move {
            let r = call(
                &app,
                Method::GET,
                "/api/driver/requests",
                Some(&cookie),
                None,
            )
            .await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.json);
            r.json.as_array().unwrap().iter().any(|x| x["id"] == ride)
        }
    };
    assert!(
        !listed(&driver2_c).await,
        "an out-of-group driver must not see the ride"
    );
    let claim = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride}/claim"),
        Some(&driver2_c),
        None,
    )
    .await;
    assert_eq!(claim.status, StatusCode::FORBIDDEN, "{}", claim.json);
    assert_eq!(claim.json["code"], "DRIVER_OUTSIDE_GROUP");
    assert!(
        listed(&driver_c).await,
        "a Youth driver sees the Youth ride"
    );

    // Transport staff can still assign an outside driver; that is audited. A driver in none
    // of the event's groups (Youth staff would see a Youth event) learns only that it is a
    // church event.
    only_in(&st, driver2, &[]).await;
    let r = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride}/reassign"),
        Some(&admin),
        Some(json!({"driver_id": driver2})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let (n,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action='ride.assign_outside_group' AND entity_id=$1",
    )
    .bind(ride)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(n, 1, "the out-of-group assignment is audited");
    let seen = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ride}"),
        Some(&driver2_c),
        None,
    )
    .await;
    assert_eq!(seen.status, StatusCode::OK, "{}", seen.json);
    assert_eq!(
        seen.json["event"]["private"], true,
        "{}",
        seen.json["event"]
    );
    assert!(seen.json["event"]["title"].is_null());
    let own = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ride}"),
        Some(&member_c),
        None,
    )
    .await;
    assert_eq!(own.json["event"]["title"], "青年营");

    // The planning board says which group each rider is matched on and whom each driver
    // may drive for, so staff can see who is outside the group.
    let board = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{youth_ev}/plan"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(board.status, StatusCode::OK, "{}", board.json);
    let row = board.json["rides"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == ride)
        .unwrap();
    assert_eq!(row["match_group_id"], youth);
    let drive_groups = |id: i64| {
        board.json["drivers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["id"] == id)
            .map(|d| d["drive_group_ids"].clone())
    };
    assert!(drive_groups(driver)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g == youth));
    if let Some(g) = drive_groups(driver2) {
        assert!(g.as_array().unwrap().is_empty(), "{g}");
    }
}

/// Every "who can see this event" check must go through `audience::visible_events_sql`
/// (and every announcement check through `audience::visible_announcements_sql`).
#[test]
fn event_visibility_is_written_only_in_audience() {
    let mut offenders = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src"
    ))];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "rs") && !p.ends_with("audience.rs") {
                let src = std::fs::read_to_string(&p).unwrap();
                if src.contains("eg.event_id = e.id AND (g.is_all")
                    || src.contains("ag.announcement_id = a.id AND (g.is_all")
                    || src.contains("gm.group_id = g.id AND gm.user_id")
                {
                    offenders.push(p.display().to_string());
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "event visibility written outside audience.rs: {offenders:?}"
    );
}

/// Announcements for chosen groups: members of those groups (and their sub-groups) see it
/// and are notified; others neither see it nor hear about it. No groups: everyone.
#[tokio::test]
async fn announcements_reach_only_their_groups() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let youth: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE name='青年团契'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        "/api/admin/groups",
        Some(&admin),
        Some(json!({"name": "青年同工", "parent_id": youth})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let staff: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE name='青年同工'")
        .fetch_one(&st.db)
        .await
        .unwrap();

    let member = id_of(&st, "member@example.org").await;
    let sophie = id_of(&st, "sophie@example.org").await;
    let amina = id_of(&st, "amina@example.org").await;
    only_in(&st, member, &[youth]).await;
    only_in(&st, sophie, &[staff]).await;
    only_in(&st, amina, &[]).await;

    // A group that doesn't exist is a 400, and nothing is published.
    let r = call(
        &app,
        Method::POST,
        "/api/announcements",
        Some(&admin),
        Some(json!({"title": "Bad", "body": "x", "group_ids": [999_999]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "SELECT_VISIBLE_GROUP");

    let r = call(
        &app,
        Method::POST,
        "/api/announcements",
        Some(&admin),
        Some(json!({"title": "Youth retreat", "body": "Sign up", "notify_all": true, "group_ids": [youth]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let ann = r.json["id"].as_i64().unwrap();
    let r = call(
        &app,
        Method::POST,
        "/api/announcements",
        Some(&admin),
        Some(json!({"title": "For everyone", "body": "Hello"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    let notified = |user: i64| {
        let db = st.db.clone();
        async move {
            let n: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM notifications WHERE user_id=$1 AND body='Youth retreat'",
            )
            .bind(user)
            .fetch_one(&db)
            .await
            .unwrap();
            n > 0
        }
    };
    assert!(notified(member).await);
    assert!(notified(sophie).await, "sub-group members are notified");
    assert!(!notified(amina).await, "members outside the group are not");

    let titles = |cookie: String| {
        let app = app.clone();
        async move {
            let r = call(&app, Method::GET, "/api/announcements", Some(&cookie), None).await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.json);
            r.json
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a["title"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        }
    };
    let seen = titles(login(&st, "member@example.org").await).await;
    assert!(
        seen.contains(&"Youth retreat".into()) && seen.contains(&"For everyone".into()),
        "{seen:?}"
    );
    let seen = titles(login(&st, "sophie@example.org").await).await;
    assert!(seen.contains(&"Youth retreat".into()), "{seen:?}");
    let seen = titles(login(&st, "amina@example.org").await).await;
    assert!(!seen.contains(&"Youth retreat".into()), "{seen:?}");
    assert!(seen.contains(&"For everyone".into()), "{seen:?}");

    // The admin page lists every announcement with its groups.
    let r = call(
        &app,
        Method::GET,
        "/api/announcements?manage=true",
        Some(&admin),
        None,
    )
    .await;
    let row = r
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == ann)
        .cloned()
        .unwrap();
    assert_eq!(row["targeted"], true);
    assert_eq!(row["groups"][0]["id"], youth);

    // Pinning keeps the audience; sending groups changes it.
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/announcements/{ann}"),
        Some(&admin),
        Some(json!({"title": "Youth retreat", "body": "Sign up", "pinned": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let seen = titles(login(&st, "amina@example.org").await).await;
    assert!(!seen.contains(&"Youth retreat".into()), "{seen:?}");
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/announcements/{ann}"),
        Some(&admin),
        Some(json!({"title": "Youth retreat", "body": "Sign up", "pinned": true, "group_ids": []})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let seen = titles(login(&st, "amina@example.org").await).await;
    assert!(seen.contains(&"Youth retreat".into()), "{seen:?}");
}
