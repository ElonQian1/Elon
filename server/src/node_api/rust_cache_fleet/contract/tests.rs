//! Contract boundary tests for untrusted node reports.

use super::*;
use serde_json::{json, Value};

const NODE_ID: &str = "pc-node-a";

#[test]
fn accepts_a_valid_read_only_envelope() {
    let validated = validate_envelope(NODE_ID, fixture_envelope()).expect("valid envelope");

    assert_eq!(validated.input.node_id, NODE_ID);
    assert_eq!(validated.input.platform_health, "healthy");
    assert!(!validated.input.gc_review_recommended);
    assert_eq!(validated.input.active_writer_count, 1);
    assert_eq!(validated.input.managed_size_bytes, Some(42));
}

#[test]
fn rejects_tampered_report_content() {
    let mut envelope = fixture_envelope_value();
    envelope["report"]["json"] = Value::String("{}".into());

    let error = validate_envelope(NODE_ID, deserialize(envelope)).unwrap_err();
    assert!(error.to_string().contains("byte length mismatch"));
}

#[test]
fn rejects_route_and_embedded_node_mismatch() {
    let error = validate_envelope("pc-node-b", fixture_envelope()).unwrap_err();

    assert!(error.to_string().contains("identity mismatch"));
}

#[test]
fn rejects_destructive_authority() {
    let mut envelope = fixture_envelope_value();
    envelope["security"]["destructive_actions_authorized"] = Value::Bool(true);

    let error = validate_envelope(NODE_ID, deserialize(envelope)).unwrap_err();
    assert!(error.to_string().contains("unsafe"));
}

#[test]
fn rejects_unknown_fields_that_could_hide_local_paths() {
    let mut envelope = fixture_envelope_value();
    let report_json = envelope["report"]["json"].as_str().expect("report json");
    let mut report: Value = serde_json::from_str(report_json).expect("report value");
    report["project_root"] = Value::String(r"C:\users\owner\project".into());
    replace_report(&mut envelope, report);

    let error = validate_envelope(NODE_ID, deserialize(envelope)).unwrap_err();
    assert!(error.to_string().contains("forbidden local identity"));
}

fn fixture_envelope() -> FleetEnvelopeV1 {
    deserialize(fixture_envelope_value())
}

fn fixture_envelope_value() -> Value {
    let report = json!({
        "schema": REPORT_SCHEMA,
        "generated_at_utc": "2026-08-16T00:00:00Z",
        "node": {
            "node_id": NODE_ID,
            "os": "windows",
            "powershell_edition": "Core",
            "powershell_version": "7.5.2"
        },
        "project": {
            "project_id": "elon-cli",
            "registered": true,
            "default_domain": "server",
            "allowed_domains": ["server", "android"],
            "shared_partition_count": 1
        },
        "platform": {
            "health": "healthy",
            "source_mode": "managed",
            "source_hash": "1".repeat(64),
            "actionable_checks": [{"id": "free-space", "status": "ok"}]
        },
        "cache": {
            "toolchain_epoch": "rust-1.89",
            "include_sizes": true,
            "partition_count": 1,
            "managed_size_bytes": 42,
            "locked_partition_count": 0,
            "invalid_marker_count": 0,
            "quarantine_partition_count": 0,
            "retired_shared_alias_count": 0,
            "by_scope": [{"name": "project", "count": 1, "size_bytes": 42}],
            "by_domain": [{"name": "server", "count": 1, "size_bytes": 42}],
            "legacy_cache_count": 0,
            "retired_legacy_cache_count": 0,
            "legacy_size_bytes": 0
        },
        "volume": {
            "total_bytes": 1000,
            "free_bytes": 800,
            "free_percent": 80.0,
            "warning_free_percent": 15.0,
            "gc_review_recommended": false
        },
        "activity": {
            "active_writer_count": 1,
            "active_writers": [{"process_name": "cargo", "count": 1}]
        },
        "privacy": {
            "absolute_paths_included": false,
            "host_name_included": false,
            "user_name_included": false
        },
        "destructive_actions_taken": false
    });
    let report_json = serde_json::to_string(&report).expect("compact report");
    let report_hash = hex::encode(Sha256::digest(report_json.as_bytes()));
    json!({
        "schema": ENVELOPE_SCHEMA,
        "envelope_id": "a".repeat(32),
        "created_at_utc": "2026-08-16T00:00:01Z",
        "node_id": NODE_ID,
        "report": {
            "schema": REPORT_SCHEMA,
            "content_type": "application/json",
            "content_sha256": report_hash,
            "byte_length": report_json.len(),
            "json": report_json
        },
        "security": {
            "receiver_must_authenticate_node": true,
            "destructive_actions_authorized": false,
            "absolute_paths_included": false,
            "secrets_included": false
        }
    })
}

fn replace_report(envelope: &mut Value, report: Value) {
    let report_json = serde_json::to_string(&report).expect("compact report");
    envelope["report"]["content_sha256"] =
        Value::String(hex::encode(Sha256::digest(report_json.as_bytes())));
    envelope["report"]["byte_length"] = json!(report_json.len());
    envelope["report"]["json"] = Value::String(report_json);
}

fn deserialize(value: Value) -> FleetEnvelopeV1 {
    serde_json::from_value(value).expect("fleet envelope")
}
