//! Synthetic-only fixtures for the production reconciliation HTTP contract.
use std::collections::{BTreeMap, BTreeSet};

use axum::http::HeaderMap;
use rusqlite::types::Value as SqlValue;
use sha2::{Digest, Sha256};

use super::*;

pub(super) const SNAPSHOT: &str = "/api/admin/assets/esk/platform-reconciliation-snapshot";

pub(super) fn auth(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );
    headers
}

pub(super) fn private(headers: &HeaderMap) {
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(headers[header::PRAGMA], "no-cache");
    assert_eq!(headers["referrer-policy"], "no-referrer");
}

pub(super) async fn raw(
    fixture: &Fixture,
    suffix: &str,
    headers: HeaderMap,
    body: Body,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .uri(format!("{SNAPSHOT}{suffix}"))
        .body(body)
        .unwrap();
    *request.headers_mut() = headers;
    let response = fixture.router.clone().oneshot(request).await.unwrap();
    private(response.headers());
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

pub(super) async fn get(fixture: &Fixture) -> Value {
    let (status, snapshot) = raw(fixture, "", auth(&fixture.admin_token), Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    assert_contract(fixture, &snapshot);
    snapshot
}

pub(super) fn assert_contract(fixture: &Fixture, snapshot: &Value) {
    let fields = [
        "schema",
        "scope",
        "source_fingerprint",
        "policy_digest",
        "observed_at",
        "used_payment_keys",
        "prepared_count",
        "recorded_count",
        "key_count",
        "platform_history_complete",
        "external_history_complete",
        "funds_moved",
        "balances_written",
        "external_payment_verified",
        "snapshot_digest",
    ];
    let object = snapshot.as_object().unwrap();
    assert_eq!(
        object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        fields.into_iter().collect()
    );
    assert_eq!(
        snapshot["schema"],
        "yilong.esk.platform_payment_snapshot.v1"
    );
    assert_eq!(snapshot["scope"], "platform_recorded_allocations_only");
    assert_eq!(snapshot["platform_history_complete"], true);
    for field in [
        "external_history_complete",
        "funds_moved",
        "balances_written",
        "external_payment_verified",
    ] {
        assert_eq!(snapshot[field], false);
    }
    let keys = snapshot["used_payment_keys"].as_array().unwrap();
    let keys: Vec<_> = keys.iter().map(|key| key.as_str().unwrap()).collect();
    assert!(keys.len() <= 10_000);
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
    for digest in keys.iter().copied().chain(
        ["source_fingerprint", "policy_digest", "snapshot_digest"]
            .map(|field| snapshot[field].as_str().unwrap()),
    ) {
        assert!(
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
    }
    let count = |field: &str| {
        let text = snapshot[field].as_str().unwrap();
        let value = text.parse::<usize>().unwrap();
        assert_eq!(value.to_string(), text);
        value
    };
    assert_eq!(
        count("prepared_count") + count("recorded_count"),
        count("key_count")
    );
    assert_eq!(count("key_count"), keys.len());
    let observed = snapshot["observed_at"].as_str().unwrap();
    let parsed = chrono::DateTime::parse_from_rfc3339(observed).unwrap();
    assert_eq!(
        parsed
            .with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        observed
    );
    // Flat exact schema: independently sort every root key before hashing the null digest.
    let mut canonical: BTreeMap<_, _> = object
        .iter()
        .map(|(key, value)| (key, value.clone()))
        .collect();
    canonical.insert(
        object.get_key_value("snapshot_digest").unwrap().0,
        Value::Null,
    );
    assert_eq!(
        snapshot["snapshot_digest"],
        hex::encode(Sha256::digest(serde_json::to_vec(&canonical).unwrap()))
    );
    let serialized = snapshot.to_string();
    for secret in [
        &fixture.user_id,
        &fixture.user_token,
        &fixture.admin_token,
        &"a".repeat(64),
        &"2".repeat(64),
        &"synthetic-review".into(),
        &"synthetic-ledger".into(),
    ] {
        assert!(!serialized.contains(secret));
    }
}

pub(super) async fn prepare(fixture: &Fixture, index: u32) -> Value {
    let mut body = fixture.body();
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
    prepared
}

pub(super) async fn transition(fixture: &Fixture, prepared: &Value, action: &str) {
    let path = format!(
        "/api/admin/assets/esk/platform-allocations/{}/{action}",
        prepared["allocation_id"].as_str().unwrap()
    );
    let confirmation = if action == "record" {
        RECORD_CONFIRMATION
    } else {
        CANCEL_CONFIRMATION
    };
    let (status, _) = request(&fixture.router, "POST", &path, Some(&fixture.admin_token), json!({"expected_request_digest": prepared["request_digest"], "confirmation": confirmation})).await;
    assert_eq!(status, StatusCode::OK);
}

// Compare every ESK/user row. Only the existing general authenticator's last_seen_at is excluded.
pub(super) fn business_state(fixture: &Fixture) -> Vec<Vec<Vec<SqlValue>>> {
    let conn = fixture.state.store.conn().unwrap();
    let mut names = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND (name GLOB 'esk_*' OR name IN ('users','sessions')) ORDER BY name").unwrap();
    let names = names
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    names
        .into_iter()
        .map(|name| {
            let quoted = format!("\"{}\"", name.replace('"', "\"\""));
            let mut info = conn
                .prepare(&format!("PRAGMA table_info({quoted})"))
                .unwrap();
            let columns = info
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            let columns: Vec<_> = columns
                .into_iter()
                .filter(|column| name != "sessions" || column != "last_seen_at")
                .map(|column| format!("\"{}\"", column.replace('"', "\"\"")))
                .collect();
            let order = (1..=columns.len())
                .map(|index| index.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let mut statement = conn
                .prepare(&format!(
                    "SELECT {} FROM {quoted} ORDER BY {order}",
                    columns.join(",")
                ))
                .unwrap();
            let rows = statement
                .query_map([], |row| {
                    (0..columns.len())
                        .map(|index| row.get(index))
                        .collect::<rusqlite::Result<Vec<SqlValue>>>()
                })
                .unwrap();
            rows.collect::<rusqlite::Result<Vec<_>>>().unwrap()
        })
        .collect()
}

pub(super) struct AbortServer(pub tokio::task::JoinHandle<()>);
impl Drop for AbortServer {
    fn drop(&mut self) {
        self.0.abort();
    }
}
