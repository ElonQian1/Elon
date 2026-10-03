use super::{model, storage};
use rusqlite::Connection;
use serde_json::json;

fn snapshot(platform: &str, sequence: u64) -> model::Snapshot {
    serde_json::from_value(json!({
        "schema":"yilong.grid_device_snapshot.v1","platform":platform,
        "device_id":"00000000-0000-4000-8000-000000000001","sequence":sequence,
        "account":"a".repeat(64),"account_kind":"primary","observed_at_ms":1000,
        "fresh_until_ms":301000,"status":"fresh","rows":[{
            "id":"7","symbol":"BTCUSDT","status":"WORKING","direction":"LONG","spacing":"ARITH",
            "lower":"10","upper":"20","count":"10","leverage":"2","profit":"-0.1","created":"900",
            "detail":false,"metrics":{"investment":"12.30","autoAddMargin":false}
        }]
    }))
    .unwrap()
}
#[test]
fn both_platforms_are_independent_and_owners_cannot_see_each_other() {
    let mut conn = Connection::open_in_memory().unwrap();
    for platform in ["android", "windows"] {
        storage::put(&mut conn, "alice", &snapshot(platform, 1), 1000).unwrap();
    }
    let sources = storage::read(&conn, "alice", 1001).unwrap();
    assert_eq!(sources.len(), 2);
    assert_ne!(sources[0].source_id, sources[1].source_id);
    assert_eq!(
        sources[0].snapshot.rows[0].id,
        sources[1].snapshot.rows[0].id
    );
    assert!(storage::read(&conn, "bob", 1001).unwrap().is_empty());
    storage::put(&mut conn, "bob", &snapshot("android", 1), 1001).unwrap();
    assert_eq!(storage::read(&conn, "alice", 1001).unwrap().len(), 2);
}
#[test]
fn replay_is_idempotent_without_renewing_observation_and_conflicts_are_rejected() {
    let mut conn = Connection::open_in_memory().unwrap();
    let value = snapshot("windows", 2);
    assert_eq!(
        storage::put(&mut conn, "a", &value, 1000).unwrap(),
        "accepted"
    );
    assert_eq!(
        storage::put(&mut conn, "a", &value, 2000).unwrap(),
        "unchanged"
    );
    assert!(storage::put(&mut conn, "a", &snapshot("windows", 1), 2000).is_err());
    let mut changed = value.clone();
    changed.rows.clear();
    assert!(storage::put(&mut conn, "a", &changed, 2000).is_err());
    let current = storage::read(&conn, "a", 400000).unwrap();
    assert_eq!(current[0].snapshot.observed_at_ms, 1000);
    assert_eq!(current[0].snapshot.fresh_until_ms, 301000);
    assert!(storage::read(&conn, "a", 1001 + model::RETENTION_MS)
        .unwrap()
        .is_empty());
}
#[test]
fn account_replacement_and_unavailable_never_leave_old_rows_or_resurrect_on_replay() {
    let mut conn = Connection::open_in_memory().unwrap();
    let mut value = snapshot("android", 1);
    storage::put(&mut conn, "a", &value, 1000).unwrap();
    value.sequence = 2;
    value.account = Some("b".repeat(64));
    value.rows[0].id = "9".into();
    storage::put(&mut conn, "a", &value, 1000).unwrap();
    let current = storage::read(&conn, "a", 1001).unwrap();
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].snapshot.rows[0].id, "9");
    value.sequence = 3;
    value.status = "unavailable".into();
    value.account = None;
    value.rows.clear();
    storage::put(&mut conn, "a", &value, 1001).unwrap();
    assert!(storage::read(&conn, "a", 1001).unwrap()[0]
        .snapshot
        .rows
        .is_empty());
    assert!(storage::put(&mut conn, "a", &snapshot("android", 2), 1002).is_err());
}
#[test]
fn contract_rejects_credentials_unknown_fields_duplicate_keys_and_duplicate_rows() {
    let value = serde_json::to_value(snapshot("android", 1)).unwrap();
    for path in ["cookie", "owner", "url", "token"] {
        let mut bad = value.clone();
        bad[path] = json!("not-allowed");
        assert!(model::parse(&serde_json::to_vec(&bad).unwrap(), 1000).is_err());
        let mut bad = value.clone();
        bad["rows"][0]["metrics"][path] = json!(null);
        assert!(model::parse(&serde_json::to_vec(&bad).unwrap(), 1000).is_err());
    }
    let raw = serde_json::to_string(&value).unwrap();
    let duplicate = raw.replacen('{', "{\"sequence\":1,", 1);
    assert!(model::parse(duplicate.as_bytes(), 1000).is_err());
    let mut bad = snapshot("android", 1);
    bad.rows.push(bad.rows[0].clone());
    assert!(bad.validate(1000).is_err());
    assert!(snapshot("android", 1).validate(301000).is_err());
}
#[test]
fn exact_decimals_and_missing_metrics_are_preserved() {
    let mut value = snapshot("android", 1);
    value.rows[0].profit = Some("-0.00000000000000000001".into());
    value.rows[0]
        .metrics
        .insert("fundingFee".into(), json!(null));
    let parsed = model::parse(&serde_json::to_vec(&value).unwrap(), 1000).unwrap();
    assert_eq!(parsed, value);
    value.rows[0].metrics.insert("fee".into(), json!(0.1));
    assert!(value.validate(1000).is_err());
}

#[test]
fn device_price_and_capacity_limits_are_enforced() {
    let mut value = snapshot("windows", 1);
    value.device_id = "-".repeat(36);
    assert!(value.validate(1000).is_err());
    for (lower, upper) in [("0", "20"), ("20", "10"), ("1.01", "1.010"), ("-1", "20")] {
        let mut value = snapshot("windows", 1);
        value.rows[0].lower = Some(lower.into());
        value.rows[0].upper = Some(upper.into());
        assert!(value.validate(1000).is_err());
    }
    let mut conn = Connection::open_in_memory().unwrap();
    for i in 0..model::MAX_SOURCES {
        let mut value = snapshot("windows", 1);
        value.device_id = format!("00000000-0000-4000-8000-{i:012x}");
        storage::put(&mut conn, "a", &value, 1000).unwrap();
    }
    let mut extra = snapshot("windows", 1);
    extra.device_id = "10000000-0000-4000-8000-000000000001".into();
    assert!(storage::put(&mut conn, "a", &extra, 1000).is_err());
    assert_eq!(storage::read(&conn, "a", 1000).unwrap().len(), 16);
    // Another owner's quota is independent.
    storage::put(&mut conn, "b", &extra, 1000).unwrap();
}
