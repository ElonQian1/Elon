use super::*;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request},
};
use tower::ServiceExt;

pub(super) async fn send(
    fixture: &Fixture,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    send_raw(fixture, method, path, token, &body.to_string()).await
}

pub(super) async fn send_raw(
    fixture: &Fixture,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: &str,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = fixture
        .router
        .clone()
        .oneshot(request.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.headers()[header::PRAGMA], "no-cache");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

pub(super) fn configure(fixture: &Fixture) -> (ConfigurationGuard, SellbackPolicy) {
    let source = crate::esk_asset::platform::payment_identity::source_fingerprint(
        &crate::esk_asset::platform::PaymentSource {
            namespace: "synthetic-ledger".into(),
            network: "synthetic-test".into(),
            asset_symbol: "USDT".into(),
            asset_reference: "0xA1".into(),
            decimals: 6,
            reference_format: "hex32".into(),
        },
    )
    .unwrap();
    let policy = crate::esk_asset::platform::sellback::domain::tests::fixture_policy(
        &fixture.user_id,
        &source,
    );
    (
        override_configuration(SellbackConfiguration::Enabled(policy.clone())),
        policy,
    )
}

pub(super) async fn credit(fixture: &Fixture) {
    let (status, prepared) = request(
        &fixture.router,
        "POST",
        "/api/admin/assets/esk/platform-allocations/prepare",
        Some(&fixture.admin_token),
        fixture.body(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let path = format!(
        "/api/admin/assets/esk/platform-allocations/{}/record",
        prepared["allocation_id"].as_str().unwrap()
    );
    let (status, _) = request(
        &fixture.router,
        "POST",
        &path,
        Some(&fixture.admin_token),
        json!({
            "expected_request_digest":prepared["request_digest"],
            "confirmation":crate::esk_asset::platform::RECORD_CONFIRMATION,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

pub(super) async fn page(fixture: &Fixture) -> Value {
    let (status, page) = send(fixture, "GET", BASE, Some(&fixture.user_token), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    page
}

pub(super) fn submit_body(page: &Value, key: &str, amount: &str) -> Value {
    json!({"schema":SUBMIT_SCHEMA, "idempotency_key":key, "amount_base_units":amount,
        "expected_snapshot_digest":page["summary"]["snapshot_digest"],
        "policy_digest":page["summary"]["policy"]["policy_digest"],
        "terms_digest":page["summary"]["policy"]["terms_digest"], "confirmation":SUBMIT_CONFIRMATION})
}

pub(super) fn cancel_body() -> Value {
    json!({"schema":CANCEL_SCHEMA,"confirmation":CANCEL_CONFIRMATION})
}

pub(super) fn keys(value: &Value, expected: &[&str]) {
    let actual: std::collections::BTreeSet<_> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(actual, expected.iter().copied().collect());
}

pub(super) fn assert_summary(
    summary: &Value,
    total: &str,
    held: &str,
    available: &str,
    count: &str,
) {
    keys(
        summary,
        &[
            "snapshot_digest",
            "total_base_units",
            "reserved_base_units",
            "available_base_units",
            "open_request_count",
            "request_count",
            "new_requests_enabled",
            "unavailable_reason",
            "policy",
        ],
    );
    assert_eq!(summary["total_base_units"], total);
    assert_eq!(summary["reserved_base_units"], held);
    assert_eq!(summary["available_base_units"], available);
    assert_eq!(summary["request_count"], count);
}

pub(super) fn assert_base(value: &Value, list: bool) {
    let mut names = vec![
        "schema",
        "asset_id",
        "symbol",
        "decimals",
        "source",
        "chain_status",
        "simulated",
        "funds_moved",
        "verification_basis",
        "external_payment_verified",
        "sellback_settlement",
        "summary",
    ];
    names.extend(if list {
        vec![
            "requests",
            "range_start",
            "range_end",
            "has_more",
            "next_cursor",
        ]
    } else {
        vec!["request", "replayed"]
    });
    keys(value, &names);
    assert_eq!(
        value["schema"],
        if list {
            "yilong.esk.platform_sellback_page.v1"
        } else {
            "yilong.esk.platform_sellback_result.v1"
        }
    );
    assert_eq!(value["asset_id"], "esk");
    assert_eq!(value["symbol"], "ESK");
    assert_eq!(value["decimals"], 6);
    assert_eq!(value["source"], "platform_recorded");
    assert_eq!(value["chain_status"], "not_deployed");
    assert_eq!(value["verification_basis"], "authenticated_operator_review");
    for name in [
        "simulated",
        "funds_moved",
        "external_payment_verified",
        "sellback_settlement",
    ] {
        assert_eq!(value[name], false);
    }
}

pub(super) fn assert_record(record: &Value) {
    keys(
        record,
        &[
            "request_id",
            "idempotency_key",
            "amount_base_units",
            "expected_snapshot_digest",
            "request_digest",
            "policy_revision",
            "policy_digest",
            "terms_digest",
            "created_at",
            "canceled_at",
            "cancel_event_id",
            "status",
        ],
    );
}
