//! Full production Store/session/Router tests; no production accounts or network.
#[path = "history_http_tests.rs"]
mod history_http_tests;

use std::{cell::RefCell, path::PathBuf, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use super::model::*;
use crate::{open_commerce_developer_production_test_support::test_app_state, store::Store};

thread_local! {
    static POLICY_OVERRIDE: RefCell<Option<Option<PolicyBody>>> = const { RefCell::new(None) };
}

pub(super) fn policy_override() -> Option<anyhow::Result<PlatformPolicy>> {
    POLICY_OVERRIDE
        .with(|value| value.borrow().clone())
        .map(|value| match value {
            Some(policy) => super::validation::validate_policy(policy),
            None => Err(PlatformError::Disabled.into()),
        })
}

struct PolicyGuard(Option<Option<PolicyBody>>);
impl Drop for PolicyGuard {
    fn drop(&mut self) {
        POLICY_OVERRIDE.with(|value| {
            value.replace(self.0.take());
        });
    }
}

fn enable_fixture_policy() -> PolicyGuard {
    PolicyGuard(POLICY_OVERRIDE.with(|value| {
        value.replace(Some(Some(PolicyBody {
            source: PaymentSource {
                namespace: "synthetic-ledger".into(),
                network: "synthetic-test".into(),
                asset_symbol: "USDT".into(),
                asset_reference: "0xA1".into(),
                decimals: 6,
                reference_format: "hex32".into(),
            },
            issuance_limit_base_units: "1000000000".into(),
        })))
    }))
}

struct Fixture {
    router: Router,
    state: Arc<crate::types::AppState>,
    root: PathBuf,
    user_id: String,
    admin_token: String,
    user_token: String,
    other_token: String,
}

impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("esk_platform_http_{}", Uuid::new_v4().simple()));
        std::fs::create_dir(&root).unwrap();
        let store = Store::open(&root.join("fixture.db")).unwrap();
        let admin = store
            .create_user("admin@example.test", "secret1", None, Some("admin"))
            .unwrap();
        let user = store
            .create_user("holder@example.test", "secret1", None, None)
            .unwrap();
        let other = store
            .create_user("other@example.test", "secret1", None, None)
            .unwrap();
        let admin_token = store
            .create_session(&admin.id, Some("fixture"), None)
            .unwrap()
            .0;
        let user_token = store
            .create_session(&user.id, Some("fixture"), None)
            .unwrap()
            .0;
        let other_token = store
            .create_session(&other.id, Some("fixture"), None)
            .unwrap()
            .0;
        store
            .create_esk_paper_allocation(&crate::esk_asset::EskAllocationInput {
                user_id: user.id.clone(),
                amount_base_units: 9_000_000,
                reference: "synthetic-paper".into(),
                idempotency_key: "synthetic-paper".into(),
            })
            .unwrap();
        let mut state = test_app_state(store, &root);
        state.owner_token = Some("synthetic-static-owner-not-a-session".into());
        let state = Arc::new(state);
        let router = crate::esk_asset::routes().with_state(Arc::clone(&state));
        Self {
            router,
            state,
            root,
            user_id: user.id,
            admin_token,
            user_token,
            other_token,
        }
    }

    fn body(&self) -> Value {
        json!({
            "schema": PREPARE_SCHEMA, "user_id": self.user_id,
            "external_payment_reference": "a".repeat(64), "transfer_index": 0,
            "payment_amount": "12.5", "amount": "25.000000", "commercial_purpose": "esk_purchase",
            "sale": { "sale_batch_id": "synthetic-sale", "payment_base_units_per_lot": "1000000",
                "esk_base_units_per_lot": "2000000", "disclosure_revision": "synthetic-v1",
                "terms_digest": "1".repeat(64) },
            "payment_evidence_digest": "2".repeat(64), "consent_digest": "3".repeat(64),
            "history_evidence_digest": "4".repeat(64), "history_complete": true,
            "review_reference": "synthetic-review",
        })
    }

    fn cleanup(self) {
        let root = self.root;
        drop(self.router);
        drop(self.state);
        assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
        assert!(root
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("esk_platform_http_"));
        std::fs::remove_dir_all(root).unwrap();
    }
}

async fn request(
    router: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn authenticated_prepare_confirm_and_private_read_preserve_paper() {
    let fixture = Fixture::new();
    let policy = enable_fixture_policy();
    let (status, prepared) = request(
        &fixture.router,
        "POST",
        "/api/admin/assets/esk/platform-allocations/prepare",
        Some(&fixture.admin_token),
        fixture.body(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(prepared["status"], "prepared");
    assert_eq!(prepared["balance_written"], false);
    let (_, empty) = request(
        &fixture.router,
        "GET",
        "/api/me/assets/esk/platform",
        Some(&fixture.user_token),
        Value::Null,
    )
    .await;
    assert_eq!(empty["total"], "0.000000");
    let path = format!(
        "/api/admin/assets/esk/platform-allocations/{}/record",
        prepared["allocation_id"].as_str().unwrap()
    );
    let approval = json!({ "expected_request_digest": prepared["request_digest"], "confirmation": RECORD_CONFIRMATION });
    let (status, recorded) = request(
        &fixture.router,
        "POST",
        &path,
        Some(&fixture.admin_token),
        approval.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(recorded["balance_written"], true);
    let (_, replay) = request(
        &fixture.router,
        "POST",
        &path,
        Some(&fixture.admin_token),
        approval,
    )
    .await;
    assert_eq!(replay["balance_written"], false);
    assert_eq!(replay["replayed"], true);
    drop(policy);
    let (status, account) = request(
        &fixture.router,
        "GET",
        "/api/me/assets/esk/platform",
        Some(&fixture.user_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(account["total"], "25.000000");
    assert_eq!(account["source"], "platform_recorded");
    assert_eq!(account["chain_status"], "not_deployed");
    assert_eq!(account["simulated"], false);
    assert_eq!(account["funds_moved"], false);
    assert_eq!(account["entry_count"], "1");
    let serialized = account.to_string();
    for private in [
        &fixture.user_id,
        &fixture.admin_token,
        &"a".repeat(64),
        &"2".repeat(64),
    ] {
        assert!(!serialized.contains(private));
    }
    let (_, other) = request(
        &fixture.router,
        "GET",
        "/api/me/assets/esk/platform",
        Some(&fixture.other_token),
        Value::Null,
    )
    .await;
    assert_eq!(other["total"], "0.000000");
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
async fn private_routes_reject_anonymous_static_owner_and_user_supplied_identity() {
    let fixture = Fixture::new();
    let _policy = enable_fixture_policy();
    for token in [
        None,
        Some("synthetic-static-owner-not-a-session"),
        Some(fixture.state.admin_token.as_str()),
    ] {
        let (status, _) = request(
            &fixture.router,
            "GET",
            "/api/me/assets/esk/platform",
            token,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, _) = request(
        &fixture.router,
        "POST",
        "/api/admin/assets/esk/platform-allocations/prepare",
        Some(&fixture.user_token),
        fixture.body(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    for suffix in [
        "?user_id=other",
        "?limit=0",
        "?limit=101",
        "?limit=not-a-number",
    ] {
        let (status, _) = request(
            &fixture.router,
            "GET",
            &format!("/api/me/assets/esk/platform{suffix}"),
            Some(&fixture.user_token),
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let mut body = fixture.body();
    body["untrusted_secret_field"] = json!("never-echo-this-value");
    let (status, response) = request(
        &fixture.router,
        "POST",
        "/api/admin/assets/esk/platform-allocations/prepare",
        Some(&fixture.admin_token),
        body,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response, json!({"error": "ESK_PLATFORM_INVALID_INPUT"}));
    fixture.cleanup();
}

#[tokio::test]
async fn write_gate_and_cancellation_restore_preparation_without_double_credit() {
    let fixture = Fixture::new();
    let disabled = PolicyGuard(POLICY_OVERRIDE.with(|value| value.replace(Some(None))));
    let (status, body) = request(
        &fixture.router,
        "POST",
        "/api/admin/assets/esk/platform-allocations/prepare",
        Some(&fixture.admin_token),
        fixture.body(),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "ESK_PLATFORM_WRITES_DISABLED");
    drop(disabled);
    let _enabled = enable_fixture_policy();
    let (_, prepared) = request(
        &fixture.router,
        "POST",
        "/api/admin/assets/esk/platform-allocations/prepare",
        Some(&fixture.admin_token),
        fixture.body(),
    )
    .await;
    let base = format!(
        "/api/admin/assets/esk/platform-allocations/{}",
        prepared["allocation_id"].as_str().unwrap()
    );
    let (status, canceled) = request(&fixture.router, "POST", &format!("{base}/cancel"), Some(&fixture.admin_token), json!({
        "expected_request_digest": prepared["request_digest"], "confirmation": CANCEL_CONFIRMATION,
    })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(canceled["status"], "canceled");
    assert_eq!(canceled["balance_written"], false);
    let (status, _) = request(&fixture.router, "POST", &format!("{base}/record"), Some(&fixture.admin_token), json!({
        "expected_request_digest": prepared["request_digest"], "confirmation": RECORD_CONFIRMATION,
    })).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_, replacement) = request(
        &fixture.router,
        "POST",
        "/api/admin/assets/esk/platform-allocations/prepare",
        Some(&fixture.admin_token),
        fixture.body(),
    )
    .await;
    assert_ne!(replacement["allocation_id"], prepared["allocation_id"]);
    let (_, account) = request(
        &fixture.router,
        "GET",
        "/api/me/assets/esk/platform",
        Some(&fixture.user_token),
        Value::Null,
    )
    .await;
    assert_eq!(account["total"], "0.000000");
    fixture.cleanup();
}

#[tokio::test]
async fn malformed_expired_and_revoked_real_sessions_cannot_read_assets() {
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
        let (status, _) = request(
            &fixture.router,
            "GET",
            "/api/me/assets/esk/platform",
            Some(&fixture.user_token),
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "expiry: {expiry}");
    }
    fixture.state.store.conn().unwrap().execute(
        "UPDATE sessions SET expires_at='2099-01-01T00:00:00Z', revoked_at='synthetic-revocation' WHERE user_id=?1",
        rusqlite::params![fixture.user_id],
    ).unwrap();
    let (status, _) = request(
        &fixture.router,
        "GET",
        "/api/me/assets/esk/platform",
        Some(&fixture.user_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    fixture.cleanup();
}
