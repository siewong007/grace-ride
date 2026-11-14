mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::Value;

fn names(body: &Value) -> Vec<String> {
    body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["name"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn admin_users_sort_uses_allow_listed_fields() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;

    for (name, email) in [
        ("AAA Sort", "aaa-sort@example.org"),
        ("BBB Sort", "bbb-sort@example.org"),
        ("CCC Sort", "ccc-sort@example.org"),
    ] {
        sqlx::query("INSERT INTO users (name, email, webauthn_uuid) VALUES ($1,$2,$3)")
            .bind(name)
            .bind(email)
            .bind(format!("uuid-{email}"))
            .execute(&st.db)
            .await
            .unwrap();
    }

    let asc = call(
        &app,
        Method::GET,
        "/api/admin/users?q=Sort&page=1&limit=50&sort=name&dir=asc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(asc.status, StatusCode::OK, "{}", asc.json);
    assert_eq!(names(&asc.json), ["AAA Sort", "BBB Sort", "CCC Sort"]);

    let desc = call(
        &app,
        Method::GET,
        "/api/admin/users?q=Sort&page=1&limit=50&sort=name&dir=desc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(desc.status, StatusCode::OK, "{}", desc.json);
    assert_eq!(names(&desc.json), ["CCC Sort", "BBB Sort", "AAA Sort"]);

    let injected = call(
        &app,
        Method::GET,
        "/api/admin/users?q=Sort&page=1&limit=50&sort=name%3B+DROP+TABLE+users&dir=sideways",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(injected.status, StatusCode::OK, "{}", injected.json);
    assert_eq!(names(&injected.json), ["AAA Sort", "BBB Sort", "CCC Sort"]);
}
