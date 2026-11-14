mod common;

use axum::http::{Method, StatusCode};
use common::*;

fn pickup_times(body: &serde_json::Value) -> Vec<String> {
    body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["pickup_at"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn admin_rides_sort_uses_allow_listed_fields() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;

    // Give the seeded rides names whose order is independent of database locale.
    sqlx::query("UPDATE users SET name = 'User ' || lpad(id::text, 20, '0')")
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query("UPDATE areas SET name = 'Area ' || lpad(id::text, 20, '0')")
        .execute(&st.db)
        .await
        .unwrap();

    // This rider sorts after the other passengers, while the booker sorts first.
    let proxy_ride =
        sqlx::query("UPDATE rides SET is_proxy = 1, rider_name = 'Zoe' WHERE id = 20004")
            .execute(&st.db)
            .await
            .unwrap();
    assert_eq!(proxy_ride.rows_affected(), 1);

    let desc = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=200&sort=pickup_at&dir=desc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(desc.status, StatusCode::OK, "{}", desc.json);
    let times = pickup_times(&desc.json);
    assert!(times.windows(2).all(|w| w[0] >= w[1]), "{times:?}");

    let asc = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=200&sort=pickup_at&dir=asc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(asc.status, StatusCode::OK, "{}", asc.json);
    let times = pickup_times(&asc.json);
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "{times:?}");

    for (sort, field) in [
        ("passenger", "/passenger/name"),
        ("area", "/area/name"),
        ("driver", "/driver/name"),
        ("status", "/status"),
    ] {
        let sorted = call(
            &app,
            Method::GET,
            &format!("/api/admin/rides?page=1&limit=200&sort={sort}&dir=asc"),
            Some(&root),
            None,
        )
        .await;
        assert_eq!(sorted.status, StatusCode::OK, "{}", sorted.json);
        let values: Vec<_> = sorted.json["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|ride| {
                let value = ride.pointer(field).and_then(serde_json::Value::as_str);
                (value.is_none(), value)
            })
            .collect();
        assert!(
            values.windows(2).all(|w| w[0] <= w[1]),
            "{sort}: {values:?}"
        );
    }

    let injected = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=200&sort=passenger%3B+DROP+TABLE+rides&dir=sideways",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(injected.status, StatusCode::OK, "{}", injected.json);
    assert_eq!(pickup_times(&injected.json), pickup_times(&asc.json));

    let unsupported = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=200&sort=id&dir=desc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(unsupported.status, StatusCode::OK, "{}", unsupported.json);
    assert_eq!(pickup_times(&unsupported.json), pickup_times(&asc.json));
}
