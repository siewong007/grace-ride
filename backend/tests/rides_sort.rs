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

    let injected = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=200&sort=passenger%3B+DROP+TABLE+rides&dir=sideways",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(injected.status, StatusCode::OK, "{}", injected.json);
    let times = pickup_times(&injected.json);
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "{times:?}");
}
