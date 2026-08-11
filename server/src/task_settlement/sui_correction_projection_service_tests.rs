use super::*;
use crate::task_settlement::{
    dispute_service, model::OpenSettlementDisputeRequest,
    sui_correction_projection_test_support::fixture, sui_projection_service,
};

#[test]
fn package_requires_posted_correction_and_binds_both_legs_idempotently() {
    let pending = fixture(false);
    assert!(prepare(
        &pending.store,
        &pending.project_id,
        &pending.correction.correction.id,
        &pending.user_id,
        "testnet",
    )
    .is_err());
    drop(pending.store);
    let _ = std::fs::remove_file(pending.path);

    let fixture = fixture(true);
    let first = prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "testnet",
    )
    .unwrap();
    let replay = prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "TESTNET",
    )
    .unwrap();
    assert_eq!(first.id, replay.id);
    assert!(first.envelope.atomic_bundle);
    assert_eq!(first.envelope.reversal.receipt_kind, "correction_reversal");
    assert_eq!(
        first.envelope.replacement.receipt_kind,
        "correction_replacement"
    );
    assert_eq!(first.integrity_status, "verified");
    assert_eq!(first.submission_readiness, "adapter_required");
    assert_eq!(first.network_submission, "not_submitted");
    assert_eq!(first.submission_attempts, 0);
    let mainnet = prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "mainnet",
    )
    .unwrap();
    assert_ne!(mainnet.id, first.id);
    assert_ne!(mainnet.projection_digest, first.projection_digest);
    assert_eq!(list(&fixture.store, &fixture.project_id).unwrap().len(), 2);
    assert!(sui_projection_service::prepare(
        &fixture.store,
        &fixture.project_id,
        &first.reversal_receipt_id,
        &fixture.user_id,
        "testnet",
    )
    .is_err());
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}

#[test]
fn replacement_dispute_blocks_readiness_and_tampering_persists_conflict() {
    let fixture = fixture(true);
    let package = prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "devnet",
    )
    .unwrap();
    dispute_service::open(
        &fixture.store,
        &fixture.project_id,
        &package.replacement_receipt_id,
        &fixture.user_id,
        &OpenSettlementDisputeRequest {
            reason_code: "source_evidence".into(),
            summary: "替换凭证出现新的来源证据疑问".into(),
            evidence_ref: Some("artifact:replacement-dispute".into()),
        },
    )
    .unwrap();
    assert_eq!(
        detail(&fixture.store, &fixture.project_id, &package.id)
            .unwrap()
            .submission_readiness,
        "dispute_blocked"
    );
    let conn = rusqlite::Connection::open(&fixture.path).unwrap();
    conn.execute(
        "UPDATE task_sui_correction_projection_packages
            SET projection_digest='tampered'
          WHERE id=?1",
        [&package.id],
    )
    .unwrap();
    drop(conn);
    let verified = verify(&fixture.store, &fixture.project_id, &package.id).unwrap();
    assert_eq!(verified.integrity_status, "conflict");
    assert_eq!(verified.submission_readiness, "integrity_conflict");
    assert!(verified.last_error.as_deref().unwrap().contains("禁止"));
    assert!(prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "devnet",
    )
    .is_err());
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}
