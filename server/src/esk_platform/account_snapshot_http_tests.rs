//! Production Router/Store regression tests using only temporary synthetic accounts.
use std::collections::BTreeSet;

use axum::http::HeaderMap;
use rusqlite::types::Value as SqlValue;

use super::*;

const ACCOUNT: &str = "/api/me/assets/esk/platform";

async fn get(fixture: &Fixture, path: &str, token: &str) -> (StatusCode, Value) {
    let response = fixture
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.headers()[header::PRAGMA], "no-cache");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

fn assert_account_contract(account: &Value) {
    let expected = [
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
        "total",
        "total_base_units",
        "entry_count",
        "updated_at",
        "history_has_more",
        "entries",
        "capabilities",
        "status_message",
    ];
    assert_eq!(
        account
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        expected.into_iter().collect::<BTreeSet<_>>()
    );
    assert_eq!(account["schema"], "yilong.esk.platform_account.v1");
    assert_eq!(account["asset_id"], "esk");
    assert_eq!(account["symbol"], "ESK");
    assert_eq!(account["decimals"], 6);
    assert_eq!(account["source"], "platform_recorded");
    assert_eq!(account["chain_status"], "not_deployed");
    assert_eq!(
        account["verification_basis"],
        "authenticated_operator_review"
    );
    for field in ["simulated", "funds_moved", "external_payment_verified"] {
        assert_eq!(account[field], false);
    }
    assert_eq!(
        account["capabilities"],
        json!({
            "service_spending": false, "quant_subscription": false,
            "sellback_settlement": false, "onchain_transfer": false, "chain_migration": false,
        })
    );
    assert_eq!(account["status_message"], "经管理员审核的 ESK 平台登记；未上链，不代表可提现、固定价格或已产生收益。模拟余额另行显示。");
    for entry in account["entries"].as_array().unwrap() {
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
    }
}

// Compare complete business rows, not just counts. Existing authentication may update last_seen_at.
fn business_state(fixture: &Fixture) -> Vec<Vec<Vec<SqlValue>>> {
    let conn = fixture.state.store.conn().unwrap();
    [
        "SELECT * FROM esk_platform_policy ORDER BY rowid",
        "SELECT * FROM esk_platform_allocations ORDER BY rowid",
        "SELECT * FROM esk_platform_approvals ORDER BY rowid",
        "SELECT * FROM esk_platform_ledger_entries ORDER BY rowid",
        "SELECT * FROM esk_platform_cancellations ORDER BY rowid",
        "SELECT * FROM esk_asset_ledger_entries ORDER BY rowid",
        "SELECT * FROM users ORDER BY rowid",
        "SELECT id,user_id,token_hash,expires_at,revoked_at,revocation_reason FROM sessions ORDER BY rowid",
    ].into_iter().map(|sql| {
        let mut statement = conn.prepare(sql).unwrap();
        let columns = statement.column_count();
        let rows = statement.query_map([], |row| {
            (0..columns).map(|index| row.get(index)).collect::<rusqlite::Result<Vec<SqlValue>>>()
        }).unwrap();
        rows.collect::<rusqlite::Result<Vec<_>>>().unwrap()
    }).collect()
}

fn change_count(fixture: &Fixture) -> i64 {
    fixture
        .state
        .store
        .conn()
        .unwrap()
        .query_row("SELECT total_changes()", [], |row| row.get(0))
        .unwrap()
}

async fn allocation(fixture: &Fixture, user_id: &str, index: u32, confirm: bool) {
    let mut body = fixture.body();
    body["user_id"] = json!(user_id);
    body["transfer_index"] = json!(index);
    let (status, prepared) = request(
        &fixture.router,
        "POST",
        "/api/admin/assets/esk/platform-allocations/prepare",
        Some(&fixture.admin_token),
        body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    if confirm {
        let path = format!(
            "/api/admin/assets/esk/platform-allocations/{}/record",
            prepared["allocation_id"].as_str().unwrap()
        );
        let (status, recorded) = request(&fixture.router, "POST", &path, Some(&fixture.admin_token),
            json!({ "expected_request_digest": prepared["request_digest"], "confirmation": RECORD_CONFIRMATION })).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(recorded["balance_written"], true);
    }
}

#[tokio::test]
async fn empty_account_retains_exact_v1_fields_and_reads_never_pin_policy() {
    let fixture = Fixture::new();
    let before = business_state(&fixture);
    for suffix in ["", "?limit=1", "?limit=100"] {
        let (status, account) =
            get(&fixture, &format!("{ACCOUNT}{suffix}"), &fixture.user_token).await;
        assert_eq!(status, StatusCode::OK);
        assert_account_contract(&account);
        assert_eq!(account["total"], "0.000000");
        assert_eq!(account["total_base_units"], "0");
        assert_eq!(account["entry_count"], "0");
        assert_eq!(account["updated_at"], Value::Null);
        assert_eq!(account["entries"], json!([]));
        assert_eq!(account["history_has_more"], false);
    }
    assert_eq!(before, business_state(&fixture));
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
async fn old_account_projects_history_first_page_without_mixing_users_or_writing_records() {
    let fixture = Fixture::new();
    let _policy = enable_fixture_policy();
    let other = fixture
        .state
        .store
        .authenticate_token(&fixture.other_token)
        .unwrap();
    for index in 0..3 {
        allocation(&fixture, &fixture.user_id, index, true).await;
    }
    allocation(&fixture, &other.id, 3, true).await;
    let before = business_state(&fixture);
    for limit in [1, 2, 100] {
        let (status, account) = get(
            &fixture,
            &format!("{ACCOUNT}?limit={limit}"),
            &fixture.user_token,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_account_contract(&account);
        assert_eq!(account["total"], "75.000000");
        assert_eq!(account["total_base_units"], "75000000");
        assert_eq!(account["entry_count"], "3");
        assert_eq!(account["entries"].as_array().unwrap().len(), limit.min(3));
        assert_eq!(account["history_has_more"], limit < 3);
        let (status, history) = get(
            &fixture,
            &format!("{ACCOUNT}/history?limit={limit}"),
            &fixture.user_token,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        for field in [
            "total",
            "total_base_units",
            "entry_count",
            "updated_at",
            "entries",
        ] {
            assert_eq!(account[field], history[field]);
        }
    }
    let (status, account) = get(&fixture, ACCOUNT, &fixture.other_token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(account["total"], "25.000000");
    assert_eq!(account["entry_count"], "1");
    let (status, failure) = get(
        &fixture,
        &format!("{ACCOUNT}?user_id={}", fixture.user_id),
        &fixture.other_token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(failure, json!({ "error": "ESK_PLATFORM_INVALID_INPUT" }));
    assert_eq!(before, business_state(&fixture));
    let changes = change_count(&fixture);
    for _ in 0..3 {
        let account = fixture
            .state
            .store
            .esk_platform_account(&fixture.user_id, &fixture.user_token, 1)
            .unwrap();
        assert_eq!(account.total_base_units, 75_000_000);
        assert_eq!(account.entries.len(), 1);
    }
    assert_eq!(changes, change_count(&fixture));
    assert_eq!(before, business_state(&fixture));
    fixture.cleanup();
}

#[tokio::test]
async fn successful_real_user_precheck_cannot_authorize_a_later_revoked_expired_or_rebound_snapshot(
) {
    for change in ["revoke", "expire", "rebind"] {
        let fixture = Fixture::new();
        let other = fixture
            .state
            .store
            .authenticate_token(&fixture.other_token)
            .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.user_token).parse().unwrap(),
        );
        let (checked, token) = super::super::api::real_user(&fixture.state, &headers).unwrap();
        assert_eq!(checked.id, fixture.user_id);
        assert!(fixture
            .state
            .store
            .esk_platform_account(&checked.id, token, 20)
            .is_ok());
        // Deterministic production precheck -> independent committed session change -> snapshot.
        // This tests the authorization boundary, not an artificial sleep or source-text ordering.
        let sql = match change {
            "revoke" => "UPDATE sessions SET revoked_at='synthetic-revoked' WHERE user_id=?1",
            "expire" => "UPDATE sessions SET expires_at='2000-01-01T00:00:00Z' WHERE user_id=?1",
            _ => "UPDATE sessions SET user_id=?2 WHERE user_id=?1",
        };
        {
            let conn = fixture.state.store.conn().unwrap();
            if change == "rebind" {
                conn.execute(sql, rusqlite::params![checked.id, other.id])
                    .unwrap();
            } else {
                conn.execute(sql, rusqlite::params![checked.id]).unwrap();
            }
        }
        let before = business_state(&fixture);
        let changes = change_count(&fixture);
        let error = fixture
            .state
            .store
            .esk_platform_account(&checked.id, token, 20)
            .unwrap_err();
        assert_eq!(
            error.downcast_ref::<PlatformError>(),
            Some(&PlatformError::Unauthorized)
        );
        assert_eq!(changes, change_count(&fixture));
        assert_eq!(before, business_state(&fixture));
        if change != "rebind" {
            let (status, value) = get(&fixture, ACCOUNT, &fixture.user_token).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(value.as_object().unwrap().len(), 1);
            assert!(value.get("total").is_none());
        }
        fixture.cleanup();
    }
}

#[tokio::test]
async fn pinned_policy_digest_mismatch_rejects_an_empty_account_with_private_fixed_failure() {
    let fixture = Fixture::new();
    let _policy = enable_fixture_policy();
    allocation(&fixture, &fixture.user_id, 0, false).await;
    let (status, account) = get(&fixture, ACCOUNT, &fixture.user_token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(account["total"], "0.000000");
    {
        let conn = fixture.state.store.conn().unwrap();
        conn.execute_batch("DROP TRIGGER trg_esk_platform_policy_no_update")
            .unwrap();
        conn.execute(
            "UPDATE esk_platform_policy SET source_fingerprint=?1",
            ["f".repeat(64)],
        )
        .unwrap();
    }
    let before = business_state(&fixture);
    let (status, value) = get(&fixture, ACCOUNT, &fixture.user_token).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        value,
        json!({ "error": "ESK_PLATFORM_LEDGER_INCONSISTENT" })
    );
    assert_eq!(before, business_state(&fixture));
    fixture.cleanup();
}

#[tokio::test]
async fn off_page_corrupt_allocation_cannot_be_hidden_by_summary_limit() {
    let fixture = Fixture::new();
    let _policy = enable_fixture_policy();
    for index in 0..3 {
        allocation(&fixture, &fixture.user_id, index, true).await;
    }
    {
        let conn = fixture.state.store.conn().unwrap();
        let oldest: String = conn.query_row(
            "SELECT allocation_id FROM esk_platform_ledger_entries WHERE user_id=?1 ORDER BY created_at,entry_id LIMIT 1",
            [&fixture.user_id], |row| row.get(0)).unwrap();
        conn.execute_batch("DROP TRIGGER trg_esk_platform_allocations_no_update")
            .unwrap();
        conn.execute("UPDATE esk_platform_allocations SET input_json='synthetic-private-invalid' WHERE allocation_id=?1", [oldest]).unwrap();
    }
    let before = business_state(&fixture);
    for path in [
        format!("{ACCOUNT}?limit=1"),
        format!("{ACCOUNT}/history?limit=1"),
    ] {
        let (status, value) = get(&fixture, &path, &fixture.user_token).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            value,
            json!({ "error": "ESK_PLATFORM_LEDGER_INCONSISTENT" })
        );
    }
    assert_eq!(before, business_state(&fixture));
    fixture.cleanup();
}
