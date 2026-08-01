use super::*;
use crate::task_settlement::{
    ledger::LedgerPosting,
    model::{
        CreateSettlementIntent, CreateSettlementReceipt, CreateUsageReceipt, RECEIPT_RECONCILED,
    },
};

fn fixture() -> (Store, std::path::PathBuf, String, String, String) {
    let path = std::env::temp_dir().join(format!(
        "elon-sui-projection-package-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let user = store
        .create_user(
            &format!("sui-package-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let project = store
        .create_project(&user.id, "Sui projection fixture", None, None)
        .unwrap()
        .project;
    let usage = store
        .insert_task_usage_receipt(CreateUsageReceipt {
            project_id: &project.id,
            subject_type: "task_assignment",
            subject_id: "assignment-sui",
            source_type: "test",
            source_id: "source-sui",
            source_digest: "source-digest",
            consumer_user_id: &user.id,
            provider_user_id: Some(&user.id),
            units: 0,
            amount_micros: 0,
            provider_amount_micros: 0,
            currency: "CNY",
            billing_source: "test",
            source_status: "settled",
            occurred_at: "2026-08-01T00:00:00Z",
        })
        .unwrap();
    let intent = store
        .create_task_settlement_intent(CreateSettlementIntent {
            project_id: &project.id,
            matter_id: Some("matter-sui"),
            assignment_id: Some("assignment-sui"),
            payer_user_id: &user.id,
            payee_user_id: Some(&user.id),
            idempotency_key: "sui-package-intent",
            policy_version: "test.v1",
            policy_digest: "policy-digest",
            usage_receipt_id: &usage.id,
        })
        .unwrap();
    let receipt = store
        .post_task_shadow_settlement(
            CreateSettlementReceipt {
                project_id: &project.id,
                intent_id: &intent.id,
                posting_key: "sui-package-posting",
                status: RECEIPT_RECONCILED,
                compute_amount_micros: 0,
                provider_amount_micros: 0,
                platform_amount_micros: 0,
                outcome_reward_micros: 0,
                review_reward_micros: 0,
                currency: "CNY",
                accepted_matter_id: Some("matter-sui"),
                reason: "projection package test",
            },
            &Vec::<LedgerPosting>::new(),
        )
        .unwrap();
    (store, path, user.id, project.id, receipt.id)
}

#[test]
fn preparation_is_idempotent_network_scoped_and_never_submitted() {
    let (store, path, user_id, project_id, receipt_id) = fixture();
    let first = prepare(&store, &project_id, &receipt_id, &user_id, "testnet").unwrap();
    let replay = prepare(&store, &project_id, &receipt_id, &user_id, "TESTNET").unwrap();
    assert_eq!(first.id, replay.id);
    assert_eq!(first.integrity_status, "verified");
    assert_eq!(first.submission_readiness, "adapter_required");
    assert_eq!(first.network_submission, "not_submitted");
    assert_eq!(first.submission_attempts, 0);

    let mainnet = prepare(&store, &project_id, &receipt_id, &user_id, "mainnet").unwrap();
    assert_ne!(mainnet.id, first.id);
    assert_ne!(mainnet.projection_digest, first.projection_digest);
    assert_eq!(list(&store, &project_id).unwrap().len(), 2);
    assert!(prepare(&store, &project_id, &receipt_id, &user_id, "localnet").is_err());
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn verification_persists_conflict_and_blocks_adapter_readiness() {
    let (store, path, user_id, project_id, receipt_id) = fixture();
    let package = prepare(&store, &project_id, &receipt_id, &user_id, "devnet").unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute(
        "UPDATE task_sui_projection_packages
            SET projection_digest='tampered'
          WHERE id=?1",
        [&package.id],
    )
    .unwrap();
    drop(conn);

    let verified = verify(&store, &project_id, &package.id).unwrap();
    assert_eq!(verified.integrity_status, "conflict");
    assert_eq!(verified.submission_readiness, "integrity_conflict");
    assert_eq!(verified.network_submission, "not_submitted");
    assert!(verified.last_error.as_deref().unwrap().contains("禁止"));
    assert!(prepare(&store, &project_id, &receipt_id, &user_id, "devnet").is_err());
    drop(store);
    let _ = std::fs::remove_file(path);
}
