use super::*;
use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

pub(super) fn snapshot(f: &Fixture) -> PlatformReconciliationSnapshot {
    f.store
        .esk_platform_reconciliation_snapshot("admin-1", &token("admin-1"))
        .unwrap()
}

pub(super) fn cancel(f: &Fixture, p: &PlatformPolicy, r: &PlatformAllocationRecord) {
    f.store
        .cancel_esk_platform_allocation(
            p,
            &r.allocation_id,
            &r.input.request_digest,
            "admin-1",
            &token("admin-1"),
        )
        .unwrap();
}

pub(super) fn assert_counts(s: &PlatformReconciliationSnapshot, prepared: usize, recorded: usize) {
    let value = serde_json::to_value(s).unwrap();
    assert_eq!(value["prepared_count"], prepared.to_string());
    assert_eq!(value["recorded_count"], recorded.to_string());
    assert_eq!(value["key_count"], (prepared + recorded).to_string());
    assert_eq!(
        value["used_payment_keys"].as_array().unwrap().len(),
        prepared + recorded
    );
}

pub(super) fn seed_pending(f: &Fixture, p: &PlatformPolicy, count: usize) {
    prepare(f, p);
    let mut conn = f.store.conn().unwrap();
    let tx = conn.transaction().unwrap();
    for index in 1..count {
        let mut body = body();
        body.transfer_index = index as u32;
        let i = prepare_input(p, body).unwrap();
        // Synthetic scale fixture: use production-normalized input and real DDL constraints.
        tx.execute(
            "INSERT INTO esk_platform_allocations(allocation_id,payment_key,policy_digest,user_id,
            amount_base_units,request_digest,input_json,prepared_by,prepared_at)
            VALUES(?1,?2,?3,?4,?5,?6,?7,'admin-1','2026-09-05T00:00:00.000Z')",
            params![
                format!("synthetic-scale-{index}"),
                i.payment_key,
                i.policy_digest,
                i.user_id,
                i.amount_base_units,
                i.request_digest,
                serde_json::to_string(&i).unwrap()
            ],
        )
        .unwrap();
    }
    tx.commit().unwrap();
}

pub(super) fn cli_preview(s: &PlatformReconciliationSnapshot, expected_code: i32) -> Value {
    let mut reconciliation: Value = serde_json::from_str(include_str!(
        "../../../../../contracts/assets/esk-paid-reconciliation-v1.fixture.json"
    ))
    .unwrap();
    reconciliation["as_of"] = json!(s.observed_at);
    reconciliation["snapshot"]["observed_at"] = json!(s.observed_at);
    reconciliation["snapshot"]["source_fingerprint"] = json!(s.source_fingerprint);
    let input = json!({"schema":"yilong.esk.platform_reconciliation_input.v1",
        "reconciliation": reconciliation, "platform_snapshot": s});
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../scripts/preview-esk-platform-reconciliation.js");
    let mut child = Command::new("node")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(expected_code),
        "synthetic CLI exit mismatch"
    );
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["schema"],
        "yilong.esk.platform_reconciliation_preview.v1"
    );
    for field in [
        "funds_moved",
        "balances_written",
        "commit_eligible",
        "platform_snapshot_authenticity_verified",
    ] {
        assert_eq!(report[field], false);
    }
    report
}
