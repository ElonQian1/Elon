use crate::{
    compute_federation::external_pool_adapter_artifact_source::require_current_quarantined_artifact_bytes,
    store::Store,
};

use super::lifecycle_support::*;

#[tokio::test]
async fn lifecycle_retry_after_sealed_evidence_loss_reopens_existing_cas_and_writes_receipt() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(
        &store,
        "sealed-evidence-loss-pool",
        "1.0.0",
        "sealed-evidence-loss",
    );

    let discarded = sealed_artifact(&data_dir, &release).await;
    assert_eq!(discarded.content_address_digest(), release.declared_sha256);
    assert_eq!(
        discarded.artifact_size_bytes(),
        release.artifact_bytes.len() as u64
    );
    drop(discarded);

    assert!(store
        .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
        .expect("discarded sealed evidence must leave Store history readable")
        .is_none());
    require_current_quarantined_artifact_bytes(
        &data_dir,
        &release.declared_sha256,
        release.artifact_bytes.len() as u64,
    )
    .await
    .expect("the CAS commit must survive loss of its sealed evidence handle");

    let retry = artifact_record_input(&data_dir, &release, "sealed-evidence-loss-key").await;
    let receipt = store
        .record_external_pool_adapter_artifact_source(retry)
        .expect("retry must reopen the existing CAS blob and persist its receipt");
    assert!(!receipt.replayed);
    assert_eq!(receipt.admission_id, release.admission_id);
    assert_eq!(receipt.intake_sha256, release.declared_sha256);
    assert_eq!(receipt.reopened_sha256, release.declared_sha256);
    assert_eq!(receipt.content_address_digest(), release.declared_sha256);

    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[tokio::test]
async fn lifecycle_retry_after_receipt_response_loss_replays_same_id_after_reopen() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(
        &store,
        "receipt-response-loss-pool",
        "1.0.0",
        "receipt-response-loss",
    );
    let receipt = record_artifact(&store, &data_dir, &release, "receipt-response-loss-key").await;
    assert!(!receipt.replayed);
    let source_receipt_id = receipt.source_receipt_id.clone();
    let source_receipt_digest = receipt.source_receipt_digest.clone();
    drop(receipt);
    drop(store);

    let reopened = Store::open(&database_path).expect("database must reopen after response loss");
    let persisted = reopened
        .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
        .expect("receipt history must remain readable after reopen")
        .expect("committed receipt must survive response loss");
    assert_eq!(persisted.source_receipt_id, source_receipt_id);
    assert!(!persisted.replayed);

    let retry = artifact_record_input(&data_dir, &release, "receipt-response-loss-key").await;
    let replay = reopened
        .record_external_pool_adapter_artifact_source(retry)
        .expect("same-key exact retry must replay after reopen");
    assert!(replay.replayed);
    assert_eq!(replay.source_receipt_id, source_receipt_id);
    assert_eq!(replay.source_receipt_digest, source_receipt_digest);

    drop(reopened);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[tokio::test]
async fn lifecycle_store_material_rejections_leave_cas_rehashable_for_correct_retry() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(&store, "material-retry-pool", "1.0.0", "material-retry");
    let idempotency_key = "material-retry-key";

    let mut wrong_digest = artifact_record_input(&data_dir, &release, idempotency_key).await;
    wrong_digest.expected_admission_digest = "f".repeat(64);
    assert!(store
        .record_external_pool_adapter_artifact_source(wrong_digest)
        .err()
        .expect("wrong admission digest must fail after CAS")
        .to_string()
        .contains("currentness digest is not exact"));
    assert!(store
        .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
        .expect("failed digest write must leave history readable")
        .is_none());
    require_release_blob(&data_dir, &release).await;

    let mut wrong_confirmation = artifact_record_input(&data_dir, &release, idempotency_key).await;
    wrong_confirmation.intake_confirmation = "confirm-a-different-operation".to_string();
    assert!(store
        .record_external_pool_adapter_artifact_source(wrong_confirmation)
        .err()
        .expect("wrong confirmation must fail after CAS")
        .to_string()
        .contains("intake confirmation is not exact"));
    assert!(store
        .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
        .expect("failed confirmation write must leave history readable")
        .is_none());
    require_release_blob(&data_dir, &release).await;

    let correct = artifact_record_input(&data_dir, &release, idempotency_key).await;
    let receipt = store
        .record_external_pool_adapter_artifact_source(correct)
        .expect("correct material must safely write after rejected attempts");
    assert!(!receipt.replayed);
    assert_eq!(receipt.admission_id, release.admission_id);

    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[tokio::test]
async fn lifecycle_cross_admission_key_conflict_keeps_second_blob_for_fresh_key() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let first = stage_release(
        &store,
        "cross-admission-key-pool-a",
        "1.0.0",
        "cross-admission-key-a",
    );
    let second = stage_release(
        &store,
        "cross-admission-key-pool-b",
        "1.0.0",
        "cross-admission-key-b",
    );
    let shared_key = "cross-admission-shared-key";
    let first_input = artifact_record_input(&data_dir, &first, shared_key).await;
    let shared_scope = first_input.idempotency_scope.clone();
    let first_receipt = store
        .record_external_pool_adapter_artifact_source(first_input)
        .expect("first admission must bind the shared scope and key");

    let conflicting_second = artifact_record_input(&data_dir, &second, shared_key).await;
    assert_eq!(conflicting_second.idempotency_scope, shared_scope);
    assert!(store
        .record_external_pool_adapter_artifact_source(conflicting_second)
        .err()
        .expect("same scope and key must not cross admission lineage")
        .to_string()
        .contains("idempotency material conflicts"));
    assert!(store
        .external_pool_adapter_artifact_source_for_admission(&second.admission_id)
        .expect("second admission history must remain readable")
        .is_none());
    require_release_blob(&data_dir, &second).await;
    assert_eq!(
        store
            .external_pool_adapter_artifact_source_for_admission(&first.admission_id)
            .expect("first admission history must remain readable")
            .expect("first receipt must remain immutable")
            .source_receipt_id,
        first_receipt.source_receipt_id
    );

    let fresh = artifact_record_input(&data_dir, &second, "cross-admission-fresh-key").await;
    let second_receipt = store
        .record_external_pool_adapter_artifact_source(fresh)
        .expect("fresh key must safely bind the retained second blob");
    assert!(!second_receipt.replayed);
    assert_eq!(second_receipt.admission_id, second.admission_id);
    assert_ne!(
        second_receipt.source_receipt_id,
        first_receipt.source_receipt_id
    );

    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

async fn require_release_blob(data_dir: &std::path::Path, release: &StagedRelease) {
    require_current_quarantined_artifact_bytes(
        data_dir,
        &release.declared_sha256,
        release.artifact_bytes.len() as u64,
    )
    .await
    .expect("a rejected Store write must leave the CAS blob exact and rehashable");
}
