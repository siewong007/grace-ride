//! Fixed fares between service areas: the traffic lead and admins price each pair of
//! areas, the same in both directions; drivers see the fare for a ride's route.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

async fn area_id(st: &grace_ride::state::AppState, name: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM areas WHERE name=$1")
        .bind(name)
        .fetch_one(&st.db)
        .await
        .unwrap_or_else(|_| panic!("area {name} missing"))
}

async fn user_id(st: &grace_ride::state::AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn default_service_areas_exist_without_invented_fares() {
    let (_app, st) = setup().await;
    for name in ["UT", "Sec 17", "UM", "KK", "Pacific/RM"] {
        let active: i64 = sqlx::query_scalar("SELECT active FROM areas WHERE name=$1")
            .bind(name)
            .fetch_one(&st.db)
            .await
            .unwrap_or_else(|_| panic!("area {name} missing"));
        assert_eq!(active, 1, "{name} should be open");
    }
    let fares: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM area_fares f JOIN areas a ON a.id IN (f.area_a_id, f.area_b_id)
         WHERE a.name IN ('UT','Sec 17','UM','KK','Pacific/RM')",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(fares, 0, "rates are left for the traffic lead to fill in");
}

#[tokio::test]
async fn only_traffic_lead_and_admin_can_set_fares() {
    let (app, st) = setup().await;
    let (ut, kk) = (area_id(&st, "UT").await, area_id(&st, "KK").await);
    let body = |cents: i64| json!({"area_a_id": kk, "area_b_id": ut, "amount_cents": cents});

    // Head of Traffic sets and changes a fare; the pair is stored once.
    let head = login(&st, "traffichead@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares",
        Some(&head),
        Some(body(1200)),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let admin = login(&st, "admin@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares",
        Some(&admin),
        Some(body(1300)),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    let list = call(
        &app,
        Method::GET,
        "/api/admin/settings/fares",
        Some(&head),
        None,
    )
    .await;
    assert_eq!(list.status, StatusCode::OK);
    let fares = list.json["fares"].as_array().unwrap();
    assert_eq!(fares.len(), 1);
    assert_eq!(fares[0]["amount_cents"], 1300);
    let fare_id = fares[0]["id"].as_i64().unwrap();

    // Leadership can read fares but not change them.
    for email in ["president@example.org", "vicepresident@example.org"] {
        let who = login(&st, email).await;
        let r = call(
            &app,
            Method::GET,
            "/api/admin/settings/fares",
            Some(&who),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{email} reads fares");
        let r = call(
            &app,
            Method::PUT,
            "/api/admin/settings/fares",
            Some(&who),
            Some(body(1)),
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not set fares"
        );
        let r = call(
            &app,
            Method::DELETE,
            &format!("/api/admin/settings/fares/{fare_id}"),
            Some(&who),
            None,
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not clear fares"
        );
    }
    // Everyone else can't even see them.
    for email in [
        "secretary@example.org",
        "transport@example.org",
        "cellleader@example.org",
        "finance@example.org",
        "driver@example.org",
        "member@example.org",
    ] {
        let who = login(&st, email).await;
        let r = call(
            &app,
            Method::PUT,
            "/api/admin/settings/fares",
            Some(&who),
            Some(body(1)),
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not set fares"
        );
        let r = call(
            &app,
            Method::DELETE,
            &format!("/api/admin/settings/fares/{fare_id}"),
            Some(&who),
            None,
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not clear fares"
        );
    }
    let stored: i64 = sqlx::query_scalar("SELECT amount_cents FROM area_fares WHERE id=$1")
        .bind(fare_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(stored, 1300);

    // A same-area fare is allowed (pickup and event in the same area).
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares",
        Some(&head),
        Some(json!({"area_a_id": ut, "area_b_id": ut, "amount_cents": 500})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // The traffic lead clears a fare.
    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/admin/settings/fares/{fare_id}"),
        Some(&head),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);

    // The traffic lead adds, renames and closes areas.
    let r = call(
        &app,
        Method::POST,
        "/api/admin/areas",
        Some(&head),
        Some(json!({"name": "Damansara"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let new_id = area_id(&st, "Damansara").await;
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/areas/{new_id}"),
        Some(&head),
        Some(json!({"name": "Damansara Utama", "active": false})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let active: i64 = sqlx::query_scalar("SELECT active FROM areas WHERE name='Damansara Utama'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(active, 0);
}

#[tokio::test]
async fn ride_shows_its_route_fare_to_the_driver_not_the_rider() {
    let (app, st) = setup().await;
    let (ut, kk) = (area_id(&st, "UT").await, area_id(&st, "KK").await);
    let passenger = user_id(&st, "member@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let admin_id = user_id(&st, "admin@example.org").await;

    // A mileage-claimable event held in KK, open to everyone.
    let event: i64 = sqlx::query_scalar(
        "INSERT INTO events (title, starts_at, created_by, mileage_claimable, area_id)
         VALUES ('Fare test', $1, $2, 1, $3) RETURNING id",
    )
    .bind(tomorrow("19:00"))
    .bind(admin_id)
    .bind(kk)
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO event_groups (event_id, group_id) SELECT $1, id FROM groups WHERE is_all = 1",
    )
    .bind(event)
    .execute(&st.db)
    .await
    .unwrap();
    let ride: i64 = sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, notes, status)
         VALUES ($1, $2, $3, $4, $5, 'Jalan 1', 'Hall', 1, '', '', 'confirmed') RETURNING id",
    )
    .bind(passenger)
    .bind(driver_id)
    .bind(event)
    .bind(tomorrow("18:30"))
    .bind(ut)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let driver = login(&st, "driver@example.org").await;
    let rider = login(&st, "member@example.org").await;
    let path = format!("/api/rides/{ride}");

    // No fare yet: the driver sees the route with an unset amount.
    let r = call(&app, Method::GET, &path, Some(&driver), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["route_fare"]["from"], "UT");
    assert_eq!(r.json["route_fare"]["to"], "KK");
    assert!(r.json["route_fare"]["amount_cents"].is_null());

    // Priced KK -> UT; the UT -> KK ride picks it up.
    let head = login(&st, "traffichead@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares",
        Some(&head),
        Some(json!({"area_a_id": kk, "area_b_id": ut, "amount_cents": 1500})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let r = call(&app, Method::GET, &path, Some(&driver), None).await;
    assert_eq!(r.json["route_fare"]["amount_cents"], 1500);

    // Riders pay no fare, so they aren't shown it.
    let r = call(&app, Method::GET, &path, Some(&rider), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.json["route_fare"].is_null());

    // Completing the ride pays the driver that fare.
    sqlx::query("UPDATE rides SET status='in_progress', started_at=$2 WHERE id=$1")
        .bind(ride)
        .bind(tomorrow("18:31"))
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride}/complete"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let paid: Option<i64> =
        sqlx::query_scalar("SELECT reimbursement_amount_cents FROM rides WHERE id=$1")
            .bind(ride)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(paid, Some(1500));

    // A non-claimable event shows no fare.
    sqlx::query("UPDATE events SET mileage_claimable=0 WHERE id=$1")
        .bind(event)
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(&app, Method::GET, &path, Some(&driver), None).await;
    assert!(r.json["route_fare"].is_null());
}

#[tokio::test]
async fn bulk_fares_upsert_deduplicates_rolls_back_and_creates_missing_areas() {
    let (app, st) = setup().await;
    let head = login(&st, "traffichead@example.org").await;

    // Bulk import: deduplicates case/whitespace variants and symmetric pairs, supports same-area fare.
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares/bulk",
        Some(&head),
        Some(json!({
            "fares": [
                {"area_a": "UT", "area_b": "KK", "amount_cents": 1200},
                {"area_a": "kk", "area_b": "  ut ", "amount_cents": 1200},
                {"area_a": "  Sec   17 ", "area_b": "UM", "amount_cents": 850},
                {"area_a": "UT", "area_b": "UT", "amount_cents": 500}
            ],
            "create_missing_areas": false
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["created"], 3);
    assert_eq!(r.json["updated"], 0);
    assert_eq!(r.json["unchanged"], 0);
    assert_eq!(r.json["total"], 3);
    assert_eq!(r.json["areas_created"], json!([]));

    let audit_detail: Option<String> = sqlx::query_scalar(
        "SELECT detail FROM audit_logs WHERE action='settings.fare_bulk' ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(&st.db)
    .await
    .unwrap();
    assert_eq!(
        audit_detail.as_deref(),
        Some("3 routes (3 added, 0 updated, 0 unchanged)")
    );

    // Second import: 1 unchanged, 1 updated, 1 created (including area named Pacific/RM).
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares/bulk",
        Some(&head),
        Some(json!({
            "fares": [
                {"area_a": "UT", "area_b": "KK", "amount_cents": 1200},
                {"area_a": "Sec 17", "area_b": "UM", "amount_cents": 900},
                {"area_a": "Pacific/RM", "area_b": "UT", "amount_cents": 700}
            ]
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["created"], 1);
    assert_eq!(r.json["updated"], 1);
    assert_eq!(r.json["unchanged"], 1);
    assert_eq!(r.json["total"], 3);

    // Empty batch is rejected.
    let empty = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares/bulk",
        Some(&head),
        Some(json!({"fares": []})),
    )
    .await;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
    assert_eq!(empty.json["code"], "FARE_BATCH_EMPTY");

    // Conflicting symmetric routes roll back the entire transaction (valid new route not saved).
    let conflict = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares/bulk",
        Some(&head),
        Some(json!({
            "fares": [
                {"area_a": "KK", "area_b": "UM", "amount_cents": 1100},
                {"area_a": "UT", "area_b": "KK", "amount_cents": 1000},
                {"area_a": "KK", "area_b": "UT", "amount_cents": 1400}
            ]
        })),
    )
    .await;
    assert_eq!(conflict.status, StatusCode::BAD_REQUEST);
    assert_eq!(conflict.json["code"], "FARE_DUPLICATE_CONFLICT");

    let (kk, um) = (area_id(&st, "KK").await, area_id(&st, "UM").await);
    let (lo, hi) = (kk.min(um), kk.max(um));
    let kk_um_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM area_fares WHERE area_a_id=$1 AND area_b_id=$2)",
    )
    .bind(lo)
    .bind(hi)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert!(
        !kk_um_exists,
        "valid route in failed batch must be rolled back"
    );

    // Out-of-range fare is rejected.
    let too_high = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares/bulk",
        Some(&head),
        Some(json!({
            "fares": [{"area_a": "UT", "area_b": "KK", "amount_cents": 60000}]
        })),
    )
    .await;
    assert_eq!(too_high.status, StatusCode::BAD_REQUEST);
    assert_eq!(too_high.json["code"], "AMOUNT_RANGE_INVALID");

    // Unknown area is rejected unless create_missing_areas is true.
    let unknown = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares/bulk",
        Some(&head),
        Some(json!({
            "fares": [{"area_a": "Subang", "area_b": "UT", "amount_cents": 1800}],
            "create_missing_areas": false
        })),
    )
    .await;
    assert_eq!(unknown.status, StatusCode::BAD_REQUEST);
    assert_eq!(unknown.json["code"], "FARE_UNKNOWN_AREA");
    let subang_exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM areas WHERE LOWER(name)='subang')")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(!subang_exists);

    // Opting into create_missing_areas creates new areas (deduped by case) + fares atomically.
    let with_create = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares/bulk",
        Some(&head),
        Some(json!({
            "fares": [
                {"area_a": "Subang", "area_b": "UT", "amount_cents": 1800},
                {"area_a": "subang", "area_b": "Kepong", "amount_cents": 2400}
            ],
            "create_missing_areas": true
        })),
    )
    .await;
    assert_eq!(with_create.status, StatusCode::OK, "{}", with_create.json);
    assert_eq!(with_create.json["created"], 2);
    assert_eq!(
        with_create.json["areas_created"],
        json!(["Subang", "Kepong"])
    );
    for name in ["Subang", "Kepong"] {
        let active: i64 = sqlx::query_scalar("SELECT active FROM areas WHERE name=$1")
            .bind(name)
            .fetch_one(&st.db)
            .await
            .unwrap();
        assert_eq!(active, 1);
    }
    let created_area_audits: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs WHERE action='area.create' AND detail IN ('Subang', 'Kepong')")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(created_area_audits, 2);
}

#[tokio::test]
async fn bulk_fares_permissions_and_csv_export() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let head = login(&st, "traffichead@example.org").await;

    // Seed one fare and one formula-like area name to test CSV export & neutralization.
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares/bulk",
        Some(&head),
        Some(json!({
            "fares": [{"area_a": "UT", "area_b": "KK", "amount_cents": 1500}]
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    sqlx::query("INSERT INTO areas (name, active) VALUES ('=CMD', 1)")
        .execute(&st.db)
        .await
        .unwrap();

    // Roles without admin_fares.update are forbidden from bulk import.
    for email in [
        "president@example.org",
        "vicepresident@example.org",
        "transport@example.org",
        "driver@example.org",
        "member@example.org",
    ] {
        let who = login(&st, email).await;
        let denied = call(
            &app,
            Method::PUT,
            "/api/admin/settings/fares/bulk",
            Some(&who),
            Some(json!({
                "fares": [{"area_a": "UT", "area_b": "KK", "amount_cents": 1600}]
            })),
        )
        .await;
        assert_eq!(
            denied.status,
            StatusCode::FORBIDDEN,
            "{email} must not bulk import fares"
        );
    }

    // Custom role with admin_fares.update but WITHOUT admin_areas.create:
    // can update fares for existing areas, cannot create missing areas.
    let made = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&admin),
        Some(json!({
            "name": "Fare editor",
            "description": "Can edit fares only",
            "permissions": ["admin_fares.view", "admin_fares.update"]
        })),
    )
    .await;
    assert_eq!(made.status, StatusCode::OK, "{}", made.json);
    let role_key = made.json["key"].as_str().unwrap().to_string();
    let member_id = user_id(&st, "member@example.org").await;
    let grant = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{member_id}"),
        Some(&admin),
        Some(json!({"roles": [role_key], "status": "active"})),
    )
    .await;
    assert_eq!(grant.status, StatusCode::OK, "{}", grant.json);
    let editor = login(&st, "member@example.org").await;

    let ok_existing = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares/bulk",
        Some(&editor),
        Some(json!({
            "fares": [{"area_a": "UT", "area_b": "KK", "amount_cents": 1650}],
            "create_missing_areas": true
        })),
    )
    .await;
    assert_eq!(ok_existing.status, StatusCode::OK, "{}", ok_existing.json);
    assert_eq!(ok_existing.json["updated"], 1);

    let denied_create = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares/bulk",
        Some(&editor),
        Some(json!({
            "fares": [{"area_a": "Cyberjaya", "area_b": "UT", "amount_cents": 3000}],
            "create_missing_areas": true
        })),
    )
    .await;
    assert_eq!(denied_create.status, StatusCode::FORBIDDEN);
    let cyber_exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM areas WHERE LOWER(name)='cyberjaya')")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(!cyber_exists);

    // Export CSV: allowed for traffic head and read-only leadership, denied for driver.
    let pres = login(&st, "president@example.org").await;
    for cookie in [&head, &pres] {
        let (status, headers, body) = call_raw(
            &app,
            Method::GET,
            "/api/admin/settings/fares/export",
            Some(cookie),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(headers[axum::http::header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .contains("text/csv"));
        assert!(headers[axum::http::header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .contains("grace-ride-area-fares.csv"));
        assert!(body.starts_with('\u{FEFF}'), "UTF-8 BOM expected");
        assert!(body.contains("16.50"), "updated fare expected in CSV");
        assert!(
            body.contains("'=CMD"),
            "formula trigger must be neutralized in CSV: {body}"
        );
    }

    let driver = login(&st, "driver@example.org").await;
    let (status, _, _) = call_raw(
        &app,
        Method::GET,
        "/api/admin/settings/fares/export",
        Some(&driver),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
