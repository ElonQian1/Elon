use super::Fixture;
use crate::esk_asset::platform::game_rewards::tests;
use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn tls(f: &Fixture) -> Router {
    crate::node_endpoint_transport::asset_access::test_routes("https://main.example.test")
        .with_state(Arc::clone(&f.state))
}
async fn request(
    router: &Router,
    path: &str,
    token: &str,
    body: Option<Value>,
    origin: Option<&str>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .uri(path)
        .method(if body.is_some() {
            Method::POST
        } else {
            Method::GET
        })
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    if let Some(origin) = origin {
        req = req.header(header::ORIGIN, origin);
    }
    let req = if let Some(body) = body {
        req.header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    } else {
        req.body(Body::empty()).unwrap()
    };
    let response = router.clone().oneshot(req).await.unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 32768).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn game_rewards_real_store_tls_signed_settlement_and_read_are_account_bound() {
    let _guard = tests::enable();
    let f = Fixture::new();
    let router = tls(&f);
    let at = chrono::Utc::now().timestamp_millis();
    let proof = tests::report(&f.user_id, at);
    let value = serde_json::to_value(&proof).unwrap();
    let url = "/api/admin/game-rewards/v1/settlements";
    assert_eq!(
        request(&router, url, &f.user_token, Some(value.clone()), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(&f.router, url, &f.admin_token, Some(value.clone()), None)
            .await
            .0,
        StatusCode::UPGRADE_REQUIRED
    );
    assert_eq!(
        request(
            &router,
            url,
            &f.admin_token,
            Some(value.clone()),
            Some("https://evil.example.test")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, result) = request(
        &router,
        url,
        &f.admin_token,
        Some(value),
        Some("https://main.example.test"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["offchain_payment_authorized"], false);
    let intent = tests::http_intent(result["report_digest"].as_str().unwrap(), &f.user_id);
    let (status, prepared) = request(
        &router,
        "/api/admin/game-rewards/v1/budgets/prepare",
        &f.admin_token,
        Some(serde_json::to_value(&intent).unwrap()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{prepared}");
    assert_eq!(prepared["state"], "reserved_awaiting_funding");
    let (status, confirmed) = request(
        &router,
        "/api/admin/game-rewards/v1/budgets/confirm-funding",
        &f.admin_token,
        Some(serde_json::to_value(tests::http_funding(&intent, at)).unwrap()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{confirmed}");
    assert_eq!(confirmed["state"], "funding_attested");
    assert_eq!(confirmed["offchain_payment_authorized"], false);
    let (status, account) = request(
        &router,
        "/api/me/game-rewards/v1/account",
        &f.user_token,
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{account}");
    assert_eq!(account["available_profit_units"], "500");
    assert_eq!(account["reserved_profit_units"], "500");
    let (_, other) = request(
        &router,
        "/api/me/game-rewards/v1/account",
        &f.other_token,
        None,
        None,
    )
    .await;
    assert_eq!(other["available_profit_units"], "0");
    assert!(other["budgets"].as_array().unwrap().is_empty());
    assert_eq!(
        request(
            &router,
            "/api/me/game-rewards/v1/account?user_id=other",
            &f.user_token,
            None,
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    drop(router);
    f.cleanup();
}
#[tokio::test]
async fn game_rewards_live_routes_reject_fake_evidence_and_static_authority() {
    let _guard = tests::enable();
    let f = Fixture::new();
    let router = tls(&f);
    let url = "/api/admin/game-rewards/v1/settlements";
    let mut value = serde_json::to_value(tests::report(
        &f.user_id,
        chrono::Utc::now().timestamp_millis(),
    ))
    .unwrap();
    value["payload"]["cumulative_net_profit_units"] = "2000000".into();
    assert_eq!(
        request(&router, url, &f.admin_token, Some(value), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &router,
            url,
            "synthetic-static-owner-not-a-session",
            Some(json!({})),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &router,
            url,
            &f.admin_token,
            Some(json!({"profit":2000000})),
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let amount: i64 = f
        .state
        .store
        .conn()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM game_reward_reports", [], |r| r.get(0))
        .unwrap();
    assert_eq!(amount, 0);
    drop(router);
    f.cleanup();
}
