mod common;

use axum::{
    http::{Method, StatusCode},
    Router,
};
use common::*;
use grace_ride::state::AppState;

async fn seed_claims(app: &Router, st: &AppState, root: &str) {
    let receipt = upload_png(app, root).await;
    let member: i64 = sqlx::query_scalar("SELECT id FROM users WHERE email='member@example.org'")
        .fetch_one(&st.db)
        .await
        .unwrap();

    for (amount, description, status, created) in [
        (1000, "Needle low", "pending", "2026-01-01T10:00:00"),
        (3000, "Needle high", "approved", "2026-01-02T10:00:00"),
        (2000, "Other", "pending", "2026-01-03T10:00:00"),
    ] {
        sqlx::query(
            "INSERT INTO reimbursements
             (user_id, amount_cents, description, receipt_file_id, payee_name, status, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$7)",
        )
        .bind(member)
        .bind(amount)
        .bind(description)
        .bind(receipt)
        .bind("Payee")
        .bind(status)
        .bind(created)
        .execute(&st.db)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn reimbursement_review_sorts_and_filters() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;
    seed_claims(&app, &st, &root).await;

    let sorted = call(
        &app,
        Method::GET,
        "/api/reimbursements?scope=review&page=1&limit=100&sort=amount_cents&dir=desc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(sorted.status, StatusCode::OK, "{}", sorted.json);
    let amounts: Vec<i64> = sorted.json["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["amount_cents"].as_i64().unwrap())
        .collect();
    assert_eq!(amounts, [3000, 2000, 1000]);

    let searched = call(
        &app,
        Method::GET,
        "/api/reimbursements?scope=review&page=1&limit=100&q=Needle",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(searched.status, StatusCode::OK, "{}", searched.json);
    assert_eq!(searched.json["total"].as_i64(), Some(2));
    assert!(searched.json["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["description"].as_str().unwrap().contains("Needle")));

    let filtered = call(
        &app,
        Method::GET,
        "/api/reimbursements?scope=review&page=1&limit=100&status=approved",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(filtered.status, StatusCode::OK, "{}", filtered.json);
    assert_eq!(filtered.json["total"].as_i64(), Some(1));
    assert_eq!(filtered.json["items"][0]["status"], "approved");

    let default_order = call(
        &app,
        Method::GET,
        "/api/reimbursements?scope=review&page=1&limit=100",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(
        default_order.status,
        StatusCode::OK,
        "{}",
        default_order.json
    );
    assert_eq!(default_order.json["items"][0]["status"], "pending");
}

#[tokio::test]
async fn payee_search_requires_claim_ownership_or_finance_permission() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;
    seed_claims(&app, &st, &root).await;
    let reviewer = login(&st, "admin@example.org").await;
    let claimant = login(&st, "member@example.org").await;

    for (cookie, scope, expected) in [
        (&reviewer, "review", 0),
        (&root, "review", 3),
        (&root, "payout", 1),
        (&claimant, "mine", 3),
    ] {
        let response = call(
            &app,
            Method::GET,
            &format!("/api/reimbursements?scope={scope}&page=1&limit=100&q=Payee"),
            Some(cookie),
            None,
        )
        .await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.json);
        assert_eq!(response.json["total"], expected);
        let items = response.json["items"].as_array().unwrap();
        assert_eq!(items.len(), expected as usize);
        assert!(items.iter().all(|row| row["payee_name"] == "Payee"));
    }
}
