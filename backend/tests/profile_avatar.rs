//! Custom avatars: a member picks a colour and icon from fixed lists, can remove their
//! photo, and anything outside the lists is refused.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn avatar_style_and_photo_removal() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let me = || call(&app, Method::GET, "/api/auth/me", Some(&member), None);
    let put =
        |body: serde_json::Value| call(&app, Method::PUT, "/api/me", Some(&member), Some(body));

    // Colour and icon are saved and come back on /me.
    let r = put(json!({"avatar_color": "teal", "avatar_icon": "car"})).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let m = me().await;
    assert_eq!(m.json["avatar_color"], "teal", "{}", m.json);
    assert_eq!(m.json["avatar_icon"], "car");

    // Fields not sent are left alone.
    let r = put(json!({"name": "Grace Member"})).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(me().await.json["avatar_icon"], "car");

    // Keys outside the fixed lists, including raw colours, are refused.
    for body in [
        json!({"avatar_color": "#ff0000"}),
        json!({"avatar_color": "red; background:url(x)"}),
        json!({"avatar_icon": "skull"}),
    ] {
        let r = put(body.clone()).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{body} {}", r.json);
    }
    assert_eq!(me().await.json["avatar_color"], "teal");

    // An empty string resets the icon to the initial.
    let r = put(json!({"avatar_icon": ""})).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let m = me().await;
    assert!(m.json["avatar_icon"].is_null(), "{}", m.json);
    assert_eq!(m.json["avatar_color"], "teal");

    // A photo can be set and then removed.
    let photo = upload_png(&app, &member).await;
    let r = put(json!({"photo_file_id": photo})).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(me().await.json["photo_file_id"], photo);
    let r = put(json!({"clear_photo": true})).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert!(me().await.json["photo_file_id"].is_null());

    // All 10 animated avatars are supported.
    for (anim, color) in [
        ("grace_car", "blue"),
        ("peace_dove", "teal"),
        ("sunny_cloud", "amber"),
        ("happy_bear", "coral"),
        ("fellowship_cat", "pink"),
        ("joy_puppy", "green"),
        ("star_lantern", "purple"),
        ("blossom_sprout", "pink"),
        ("grok", "purple"),
        ("muse", "teal"),
        ("grok", "white"),
        ("muse", "white"),
    ] {
        let r = put(json!({"avatar_icon": anim, "avatar_color": color})).await;
        assert_eq!(r.status, StatusCode::OK, "{anim} ({color}): {}", r.json);
        let m = me().await;
        assert_eq!(m.json["avatar_icon"], anim);
        assert_eq!(m.json["avatar_color"], color);
    }

    // White also works with static icons and initials.
    let r = put(json!({"avatar_icon": "", "avatar_color": "white"})).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let m = me().await;
    assert!(m.json["avatar_icon"].is_null());
    assert_eq!(m.json["avatar_color"], "white");

    // Independent avatar + background shape and colour combinations validate and persist.
    for (anim, bg) in [
        ("grok:circle:coral", "white:hexagon"),
        ("muse:droplet:pink", "blue:cloud"),
        ("grok::blue", ":squircle"),
        ("car::purple", "amber:pebble"),
        ("initials::teal", "white:rounded"),
    ] {
        let r = put(json!({"avatar_icon": anim, "avatar_color": bg})).await;
        assert_eq!(r.status, StatusCode::OK, "{anim} on {bg}: {}", r.json);
        let m = me().await;
        assert_eq!(m.json["avatar_icon"], anim);
        assert_eq!(m.json["avatar_color"], bg);
    }

    // Invalid shape / foreground / injected compound values are rejected.
    for bad_body in [
        json!({"avatar_icon": "grok:star"}),
        json!({"avatar_icon": "grok:circle:#ff0000"}),
        json!({"avatar_icon": "grok:circle:blue:extra"}),
        json!({"avatar_color": "blue:diamond"}),
        json!({"avatar_color": ":badshape"}),
        json!({"avatar_color": ":"}),
    ] {
        let r = put(bad_body.clone()).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{bad_body}: {}", r.json);
    }
}

#[tokio::test]
async fn role_achievement_badges_cover_member_driver_and_traffic_lead() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let traffic_lead = login(&st, "traffichead@example.org").await;

    let count_cat = |badges: &[serde_json::Value], cat: &str| {
        badges.iter().filter(|b| b["category"] == cat).count()
    };
    let any_role_match = |badges: &[serde_json::Value], cat: &str| {
        badges
            .iter()
            .filter(|b| b["category"] == cat)
            .all(|b| b["role_match"] == true)
    };
    let none_role_match = |badges: &[serde_json::Value], cat: &str| {
        badges
            .iter()
            .filter(|b| b["category"] == cat)
            .all(|b| b["role_match"] == false)
    };

    // Normal member sees all 12 badges across 3 categories; member category matches.
    let m_res = call(&app, Method::GET, "/api/badges", Some(&member), None).await;
    assert_eq!(m_res.status, StatusCode::OK, "{}", m_res.json);
    let m_badges = m_res.json.as_array().expect("member badges array");
    assert_eq!(m_badges.len(), 12);
    assert_eq!(count_cat(m_badges, "member"), 4);
    assert_eq!(count_cat(m_badges, "driver"), 4);
    assert_eq!(count_cat(m_badges, "traffic_lead"), 4);
    assert!(any_role_match(m_badges, "member"));
    assert!(none_role_match(m_badges, "driver"));
    assert!(none_role_match(m_badges, "traffic_lead"));

    // Every badge exposes ordered bronze -> silver -> gold -> platinum tier definitions.
    for b in m_badges {
        assert!(b["tier"].is_string(), "{b}");
        assert!(b["tier_level"].is_number(), "{b}");
        let tiers = b["tiers"].as_array().expect("badge tiers array");
        let names: Vec<_> = tiers.iter().map(|t| t["tier"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["bronze", "silver", "gold", "platinum"]);
        let targets: Vec<_> = tiers
            .iter()
            .map(|t| t["target"].as_i64().unwrap())
            .collect();
        assert!(
            targets.windows(2).all(|w| w[0] < w[1]),
            "non-ascending tiers: {targets:?}"
        );
    }

    // Driver matches both member and driver badges.
    let d_res = call(&app, Method::GET, "/api/badges", Some(&driver), None).await;
    assert_eq!(d_res.status, StatusCode::OK, "{}", d_res.json);
    let d_badges = d_res.json.as_array().expect("driver badges array");
    assert_eq!(d_badges.len(), 12);
    assert!(any_role_match(d_badges, "member"));
    assert!(any_role_match(d_badges, "driver"));
    assert!(none_role_match(d_badges, "traffic_lead"));

    // Traffic lead matches member and traffic_lead badges.
    let tl_res = call(&app, Method::GET, "/api/badges", Some(&traffic_lead), None).await;
    assert_eq!(tl_res.status, StatusCode::OK, "{}", tl_res.json);
    let tl_badges = tl_res.json.as_array().expect("traffic lead badges array");
    assert_eq!(tl_badges.len(), 12);
    assert!(any_role_match(tl_badges, "member"));
    assert!(any_role_match(tl_badges, "traffic_lead"));

    // Admin can customize a traffic lead badge to a custom adorable animated icon and reset it.
    let tl_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE email = 'traffichead@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{tl_id}/badges/traffic_dispatcher/icon"),
        Some(&admin),
        Some(json!({ "icon": "badge_route_compass" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["icon"], "badge_route_compass");

    let tl_res2 = call(&app, Method::GET, "/api/badges", Some(&traffic_lead), None).await;
    let tl_badges2 = tl_res2.json.as_array().unwrap();
    let dispatcher = tl_badges2
        .iter()
        .find(|b| b["key"] == "traffic_dispatcher")
        .expect("traffic_dispatcher badge");
    assert_eq!(dispatcher["icon"], "badge_route_compass");

    let reset = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{tl_id}/badges/traffic_dispatcher/icon"),
        Some(&admin),
        Some(json!({ "icon": "" })),
    )
    .await;
    assert_eq!(reset.status, StatusCode::OK);
    let tl_res3 = call(&app, Method::GET, "/api/badges", Some(&traffic_lead), None).await;
    let dispatcher3 = tl_res3
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["key"] == "traffic_dispatcher")
        .unwrap();
    assert_eq!(dispatcher3["icon"], "badge_traffic_light");
}

#[tokio::test]
async fn admin_can_change_user_achievement_icon() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;

    // Member gets their badges. Initially defaults are used (e.g. night => moon).
    let r = call(&app, Method::GET, "/api/badges", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let badges = r.json.as_array().expect("array of badges");
    let night = badges
        .iter()
        .find(|b| b["key"] == "night")
        .expect("night badge");
    assert_eq!(night["icon"], "moon");

    // Regular member cannot change achievement icons.
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/users/1/badges/night/icon",
        Some(&member),
        Some(json!({ "icon": "owl" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // Admin updates the achievement icon for the member (user id 2 in seed is member).
    let member_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE email = 'member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();

    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{member_id}/badges/night/icon"),
        Some(&admin),
        Some(json!({ "icon": "trophy" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["icon"], "trophy");

    // Member checks their badges again; night badge now has custom icon 'trophy'.
    let r = call(&app, Method::GET, "/api/badges", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let badges = r.json.as_array().expect("array of badges");
    let night = badges
        .iter()
        .find(|b| b["key"] == "night")
        .expect("night badge");
    assert_eq!(night["icon"], "trophy");

    // Resetting to empty restores default icon.
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{member_id}/badges/night/icon"),
        Some(&admin),
        Some(json!({ "icon": "" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    let r = call(&app, Method::GET, "/api/badges", Some(&member), None).await;
    let badges = r.json.as_array().expect("array of badges");
    let night = badges
        .iter()
        .find(|b| b["key"] == "night")
        .expect("night badge");
    assert_eq!(night["icon"], "moon");
}

#[tokio::test]
async fn achievement_badges_advance_through_bronze_silver_gold_and_platinum() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let member_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE email = 'member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    let group_id: i64 = sqlx::query_scalar("SELECT MIN(id) FROM groups")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let event_id: i64 = sqlx::query_scalar(
        "INSERT INTO events (title, starts_at, location, created_by, reminder_sent)
         VALUES ('Tier Test Event', '2026-11-29T09:00', 'Hall', $1, 1) RETURNING id",
    )
    .bind(member_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let warm_badge = || async {
        let r = call(&app, Method::GET, "/api/badges", Some(&member), None).await;
        assert_eq!(r.status, StatusCode::OK);
        r.json
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["key"] == "member_warm_welcome")
            .cloned()
            .unwrap()
    };

    // Start at none (0 / 2 -> bronze).
    let b0 = warm_badge().await;
    assert_eq!(b0["tier"], "none");
    assert_eq!(b0["tier_level"], 0);
    assert_eq!(b0["next_tier"], "bronze");
    assert_eq!(b0["next_target"], 2);
    assert_eq!(b0["earned"], false);

    let add_passes = |from: usize, to: usize| {
        let db = st.db.clone();
        async move {
            for idx in from..to {
                sqlx::query(
                    "INSERT INTO visitor_passes (token_hash, event_id, host_id, host_group_id, name, valid_from, valid_until)
                     VALUES ($1, $2, $3, $4, $5, '2026-11-01T00:00', '2026-12-31T23:59')",
                )
                .bind(format!("tier-pass-{idx}"))
                .bind(event_id)
                .bind(member_id)
                .bind(group_id)
                .bind(format!("Visitor {idx}"))
                .execute(&db)
                .await
                .unwrap();
            }
        }
    };

    // Thresholds for member_warm_welcome are [2, 5, 12, 25].
    add_passes(0, 2).await;
    let b1 = warm_badge().await;
    assert_eq!(b1["tier"], "bronze");
    assert_eq!(b1["tier_level"], 1);
    assert_eq!(b1["next_tier"], "silver");
    assert_eq!(b1["next_target"], 5);
    assert_eq!(b1["target"], 5);
    assert_eq!(b1["earned"], true);

    add_passes(2, 5).await;
    let b2 = warm_badge().await;
    assert_eq!(b2["tier"], "silver");
    assert_eq!(b2["tier_level"], 2);
    assert_eq!(b2["next_tier"], "gold");
    assert_eq!(b2["next_target"], 12);

    add_passes(5, 12).await;
    let b3 = warm_badge().await;
    assert_eq!(b3["tier"], "gold");
    assert_eq!(b3["tier_level"], 3);
    assert_eq!(b3["next_tier"], "platinum");
    assert_eq!(b3["next_target"], 25);

    add_passes(12, 25).await;
    let b4 = warm_badge().await;
    assert_eq!(b4["tier"], "platinum");
    assert_eq!(b4["tier_level"], 4);
    assert!(b4["next_tier"].is_null());
    assert!(b4["next_target"].is_null());
    assert_eq!(b4["target"], 25);
    assert!(b4["tiers"]
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t["reached"] == true));
}

#[tokio::test]
async fn grokbot_studio_combinations_validate_and_persist() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let me = || call(&app, Method::GET, "/api/auth/me", Some(&member), None);
    let put =
        |body: serde_json::Value| call(&app, Method::PUT, "/api/me", Some(&member), Some(body));

    // Bare "grokbot" and full "grokbot:<shape>:<face>:<color>" studio specs persist cleanly.
    for valid_icon in [
        "grokbot",
        "grokbot:circle:neutral:blue",
        "grokbot:cloud:excited:brown",
        "grokbot:pebble:suspicious:ink",
        "grokbot:squircle:happy:cream",
        "grokbot:capsule:laughing:turquoise",
        "grokbot:triangle:angry:red",
        "grokbot:hexagon:curious:orange",
        "grokbot:droplet:sleepy:grey",
    ] {
        let r = put(json!({
            "clear_photo": true,
            "avatar_icon": valid_icon,
            "avatar_color": "",
        }))
        .await;
        assert_eq!(r.status, StatusCode::OK, "{valid_icon}: {}", r.json);
        let m = me().await;
        assert_eq!(m.json["avatar_icon"], valid_icon, "{}", m.json);
        assert!(m.json["avatar_color"].is_null(), "{}", m.json);
    }

    // Invalid shape, face, colour, missing segment, or extra segment is rejected with 400.
    for bad_icon in [
        "grokbot:",
        "grokbot:circle:neutral",
        "grokbot:octagon:neutral:blue",
        "grokbot:circle:winking:blue",
        "grokbot:circle:neutral:neon",
        "grokbot:circle:neutral:#3b93f0",
        "grokbot:circle:neutral:blue:extra",
    ] {
        let r = put(json!({"avatar_icon": bad_icon})).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{bad_icon}: {}", r.json);
    }
}
