//! Real production asset Router + Store/session tests, with temporary synthetic accounts only.
use std::collections::BTreeSet;

use axum::http::HeaderMap;

use super::*;

const HISTORY: &str = "/api/me/assets/esk/platform/history";
const ROOT_KEYS: [&str; 20] = [
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
    "snapshot_digest",
    "total",
    "total_base_units",
    "entry_count",
    "range_start",
    "range_end",
    "updated_at",
    "entries",
    "has_more",
    "next_cursor",
];

async fn raw_history(fixture: &Fixture, suffix: &str, headers: HeaderMap) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .uri(format!("{HISTORY}{suffix}"))
        .body(Body::empty())
        .unwrap();
    *request.headers_mut() = headers;
    let response = fixture.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.headers()[header::PRAGMA], "no-cache");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn history(fixture: &Fixture, suffix: &str, token: Option<&str>) -> (StatusCode, Value) {
    let mut headers = HeaderMap::new();
    if let Some(token) = token {
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );
    }
    raw_history(fixture, suffix, headers).await
}

fn assert_contract(value: &Value) {
    assert_eq!(
        value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        ROOT_KEYS.into_iter().collect::<BTreeSet<_>>()
    );
    assert_eq!(value["schema"], "yilong.esk.platform_history.v1");
    assert_eq!(value["asset_id"], "esk");
    assert_eq!(value["symbol"], "ESK");
    assert_eq!(value["decimals"], 6);
    assert_eq!(value["source"], "platform_recorded");
    assert_eq!(value["chain_status"], "not_deployed");
    assert_eq!(value["simulated"], false);
    assert_eq!(value["funds_moved"], false);
    assert_eq!(value["verification_basis"], "authenticated_operator_review");
    assert_eq!(value["external_payment_verified"], false);
    let digest = value["snapshot_digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64);
    assert!(digest
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    for field in [
        "total_base_units",
        "entry_count",
        "range_start",
        "range_end",
    ] {
        let text = value[field].as_str().unwrap();
        assert_eq!(text.parse::<i64>().unwrap().to_string(), text);
    }
    for entry in value["entries"].as_array().unwrap() {
        assert_eq!(
            entry
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            [
                "entry_id",
                "allocation_id",
                "amount",
                "amount_base_units",
                "created_at",
                "kind"
            ]
            .into_iter()
            .collect::<BTreeSet<_>>()
        );
        assert_eq!(entry["kind"], "approved_payment_allocation");
        assert!(
            entry["amount_base_units"]
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap()
                > 0
        );
    }
}

async fn add_entry(fixture: &Fixture, transfer_index: u32) {
    let mut body = fixture.body();
    body["transfer_index"] = json!(transfer_index);
    let (status, prepared) = request(
        &fixture.router,
        "POST",
        "/api/admin/assets/esk/platform-allocations/prepare",
        Some(&fixture.admin_token),
        body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let path = format!(
        "/api/admin/assets/esk/platform-allocations/{}/record",
        prepared["allocation_id"].as_str().unwrap()
    );
    let (status, receipt) = request(&fixture.router, "POST", &path, Some(&fixture.admin_token),
        json!({ "expected_request_digest": prepared["request_digest"], "confirmation": RECORD_CONFIRMATION })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(receipt["balance_written"], true);
}

#[tokio::test]
async fn empty_history_is_exact_private_and_distinct_from_account_v1_and_paper() {
    let fixture = Fixture::new();
    for suffix in ["", "?limit=1", "?limit=100"] {
        let (status, page) = history(&fixture, suffix, Some(&fixture.user_token)).await;
        assert_eq!(status, StatusCode::OK);
        assert_contract(&page);
        assert_eq!(page["total"], "0.000000");
        assert_eq!(page["total_base_units"], "0");
        assert_eq!(page["entry_count"], "0");
        assert_eq!(page["range_start"], "0");
        assert_eq!(page["range_end"], "0");
        assert_eq!(page["entries"], json!([]));
        assert_eq!(page["updated_at"], Value::Null);
        assert_eq!(page["has_more"], false);
        assert_eq!(page["next_cursor"], Value::Null);
    }
    let (status, account) = request(
        &fixture.router,
        "GET",
        "/api/me/assets/esk/platform",
        Some(&fixture.user_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(account["schema"], "yilong.esk.platform_account.v1");
    assert_eq!(account.as_object().unwrap().len(), 18);
    assert!(account.get("snapshot_digest").is_none());
    assert!(account.get("capabilities").is_some());
    assert_eq!(
        fixture
            .state
            .store
            .esk_account_ledger(&fixture.user_id)
            .unwrap()
            .total_base_units,
        9_000_000
    );
    fixture.cleanup();
}

#[tokio::test]
async fn default_twenty_then_changed_page_size_preserves_whole_snapshot_and_range() {
    let fixture = Fixture::new();
    let _policy = enable_fixture_policy();
    for index in 0..21 {
        add_entry(&fixture, index).await;
    }
    let (status, first) = history(&fixture, "", Some(&fixture.user_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_contract(&first);
    assert_eq!(first["total"], "525.000000");
    assert_eq!(first["total_base_units"], "525000000");
    assert_eq!(first["entry_count"], "21");
    assert_eq!(first["range_start"], "1");
    assert_eq!(first["range_end"], "20");
    assert_eq!(first["entries"].as_array().unwrap().len(), 20);
    assert_eq!(first["has_more"], true);
    let cursor = first["next_cursor"].as_str().unwrap();
    let (status, last) = history(
        &fixture,
        &format!("?limit=100&cursor={cursor}"),
        Some(&fixture.user_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_contract(&last);
    for field in [
        "total",
        "total_base_units",
        "entry_count",
        "updated_at",
        "snapshot_digest",
    ] {
        assert_eq!(first[field], last[field]);
    }
    assert_eq!(last["range_start"], "21");
    assert_eq!(last["range_end"], "21");
    assert_eq!(last["entries"].as_array().unwrap().len(), 1);
    assert_eq!(last["has_more"], false);
    assert_eq!(last["next_cursor"], Value::Null);
    let ids = first["entries"]
        .as_array()
        .unwrap()
        .iter()
        .chain(last["entries"].as_array().unwrap())
        .map(|entry| entry["entry_id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 21);
    let (status, replay) = history(&fixture, "", Some(&fixture.user_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first, replay);
    let serialized = first.to_string();
    for private in [
        &fixture.user_id,
        &fixture.user_token,
        &fixture.admin_token,
        &"a".repeat(64),
        &"2".repeat(64),
    ] {
        assert!(!serialized.contains(private));
    }
    assert_eq!(
        fixture
            .state
            .store
            .esk_account_ledger(&fixture.user_id)
            .unwrap()
            .total_base_units,
        9_000_000
    );
    fixture.cleanup();
}

#[tokio::test]
async fn real_bearer_auth_precedes_invalid_query_and_cookie_or_duplicate_headers_never_authenticate(
) {
    let fixture = Fixture::new();
    for token in [
        None,
        Some("synthetic-static-owner-not-a-session"),
        Some(fixture.state.admin_token.as_str()),
        Some("synthetic-unknown-session"),
    ] {
        for suffix in [
            "",
            "?limit=0&user_id=untrusted",
            "?cursor=invalid&cursor=invalid",
        ] {
            let (status, value) = history(&fixture, suffix, token).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(value.as_object().unwrap().len(), 1);
        }
    }
    let mut cookie = HeaderMap::new();
    cookie.insert(
        header::COOKIE,
        format!("session_token={}", fixture.user_token)
            .parse()
            .unwrap(),
    );
    assert_eq!(
        raw_history(&fixture, "?limit=0", cookie).await.0,
        StatusCode::UNAUTHORIZED
    );
    let mut duplicate = HeaderMap::new();
    for _ in 0..2 {
        duplicate.append(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.user_token).parse().unwrap(),
        );
    }
    assert_eq!(
        raw_history(&fixture, "?limit=0", duplicate).await.0,
        StatusCode::UNAUTHORIZED
    );
    fixture.cleanup();
}

#[tokio::test]
async fn invalid_limits_unknown_duplicate_and_malformed_cursor_parameters_are_fixed_400() {
    let fixture = Fixture::new();
    for suffix in [
        "?limit=0",
        "?limit=101",
        "?limit=-1",
        "?limit=1.5",
        "?limit=wrong",
        "?limit=18446744073709551616",
        "?limit=",
        "?limit=1&limit=2",
        "?limit=1&limit=1",
        "?cursor=x&cursor=y",
        "?cursor=x&cursor=x",
        "?user_id=never-echo",
        "?unknown=never-echo",
        "?cursor=",
        "?cursor=never-echo",
        "?cursor=ephp1.bad.eskp_entry_bad",
    ] {
        let (status, value) = history(&fixture, suffix, Some(&fixture.user_token)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{suffix}");
        assert_eq!(value, json!({ "error": "ESK_PLATFORM_INVALID_INPUT" }));
    }
    fixture.cleanup();
}

#[tokio::test]
async fn cross_user_unknown_last_and_changed_snapshot_cursors_share_one_private_409() {
    let fixture = Fixture::new();
    let _policy = enable_fixture_policy();
    add_entry(&fixture, 0).await;
    add_entry(&fixture, 1).await;
    let (status, first) = history(&fixture, "?limit=1", Some(&fixture.user_token)).await;
    assert_eq!(status, StatusCode::OK);
    let cursor = first["next_cursor"].as_str().unwrap();
    let digest = first["snapshot_digest"].as_str().unwrap();
    let (status, last) = history(
        &fixture,
        &format!("?cursor={cursor}"),
        Some(&fixture.user_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let last_cursor = format!(
        "ephp1.{digest}.{}",
        last["entries"][0]["entry_id"].as_str().unwrap()
    );
    let unknown = format!("ephp1.{digest}.eskp_entry_{}", "0".repeat(32));
    let mismatched = format!(
        "ephp1.{}.{}",
        "f".repeat(64),
        first["entries"][0]["entry_id"].as_str().unwrap()
    );
    for (token, cursor) in [
        (&fixture.other_token, cursor),
        (&fixture.user_token, last_cursor.as_str()),
        (&fixture.user_token, unknown.as_str()),
        (&fixture.user_token, mismatched.as_str()),
    ] {
        let (status, value) = history(&fixture, &format!("?cursor={cursor}"), Some(token)).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(value, json!({ "error": "ESK_PLATFORM_HISTORY_CHANGED" }));
    }
    add_entry(&fixture, 2).await;
    let (status, changed) = history(
        &fixture,
        &format!("?cursor={cursor}"),
        Some(&fixture.user_token),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(changed, json!({ "error": "ESK_PLATFORM_HISTORY_CHANGED" }));
    let (status, refreshed) = history(&fixture, "", Some(&fixture.user_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(refreshed["entry_count"], "3");
    assert_eq!(refreshed["total"], "75.000000");
    assert_ne!(refreshed["snapshot_digest"], first["snapshot_digest"]);
    let (status, other) = history(&fixture, "", Some(&fixture.other_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(other["entry_count"], "0");
    fixture.cleanup();
}

#[tokio::test]
async fn expired_malformed_revoked_and_disabled_real_sessions_cannot_read_history() {
    let fixture = Fixture::new();
    for expiry in [
        "not-a-date",
        "2000-01-01T00:00:00Z",
        "2000-01-01T08:00:00+08:00",
    ] {
        fixture
            .state
            .store
            .conn()
            .unwrap()
            .execute(
                "UPDATE sessions SET expires_at=?1 WHERE user_id=?2",
                rusqlite::params![expiry, fixture.user_id],
            )
            .unwrap();
        let (status, _) = history(&fixture, "?limit=0", Some(&fixture.user_token)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    fixture.state.store.conn().unwrap().execute(
        "UPDATE sessions SET expires_at='2099-01-01T00:00:00Z', revoked_at='synthetic-revocation' WHERE user_id=?1",
        rusqlite::params![fixture.user_id]).unwrap();
    assert_eq!(
        history(&fixture, "", Some(&fixture.user_token)).await.0,
        StatusCode::UNAUTHORIZED
    );
    fixture
        .state
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE sessions SET revoked_at=NULL WHERE user_id=?1",
            rusqlite::params![fixture.user_id],
        )
        .unwrap();
    fixture
        .state
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE users SET status='disabled' WHERE id=?1",
            rusqlite::params![fixture.user_id],
        )
        .unwrap();
    assert_eq!(
        history(&fixture, "", Some(&fixture.user_token)).await.0,
        StatusCode::UNAUTHORIZED
    );
    fixture.cleanup();
}

#[tokio::test]
async fn corrupt_stored_policy_returns_sanitized_uncacheable_failure() {
    let fixture = Fixture::new();
    let _policy = enable_fixture_policy();
    add_entry(&fixture, 0).await;
    // Deliberate corruption is confined to this temporary fixture; no production mutation.
    fixture
        .state
        .store
        .conn()
        .unwrap()
        .execute_batch(
            "DROP TRIGGER trg_esk_platform_policy_no_update;
         UPDATE esk_platform_policy SET source_json='synthetic-private-never-echo';",
        )
        .unwrap();
    let (status, value) = history(&fixture, "", Some(&fixture.user_token)).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        value,
        json!({ "error": "ESK_PLATFORM_LEDGER_INCONSISTENT" })
    );
    fixture.cleanup();
}
