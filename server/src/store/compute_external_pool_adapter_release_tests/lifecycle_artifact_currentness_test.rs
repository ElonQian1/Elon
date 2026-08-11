use crate::compute_federation::{
    external_pool_adapter_artifact_source::require_current_quarantined_artifact_bytes,
    external_pool_adapter_release_lifecycle::EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
};

use super::{lifecycle_support::*, *};

#[test]
fn lifecycle_terminal_first_removes_artifact_intake_authority() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(&store, "terminal-first-pool", "1.0.0", "terminal-first");

    store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "terminal-first-revoked",
        ))
        .expect("terminal should append before artifact intake");

    assert!(store
        .external_pool_adapter_artifact_intake_authority(
            &release.admission_id,
            &release.admission_digest,
        )
        .is_err());
    assert!(store
        .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
        .expect("artifact history should be readable")
        .is_none());

    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[tokio::test]
async fn lifecycle_cas_first_terminal_before_db_second_leaves_only_unreferenced_bytes() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(&store, "cas-first-pool", "1.0.0", "cas-first");
    let input = artifact_record_input(&data_dir, &release, "cas-first-artifact").await;

    store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "cas-first-terminal",
        ))
        .expect("terminal should linearize after CAS and before the Store receipt");
    assert!(store
        .record_external_pool_adapter_artifact_source(input)
        .is_err());
    assert!(store
        .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
        .expect("artifact history should be readable")
        .is_none());

    require_current_quarantined_artifact_bytes(
        &data_dir,
        &release.declared_sha256,
        release.artifact_bytes.len() as u64,
    )
    .await
    .expect("CAS-first bytes should remain rehashable but unreferenced");

    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[tokio::test]
async fn lifecycle_receipt_first_is_historical_and_replay_fails_after_terminal_and_reopen() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(&store, "receipt-first-pool", "1.0.0", "receipt-first");
    let receipt = record_artifact(&store, &data_dir, &release, "receipt-first-artifact").await;

    store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "receipt-first-terminal",
        ))
        .expect("terminal should preserve an earlier receipt as history");
    let historical = store
        .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
        .expect("historical receipt should remain readable")
        .expect("receipt-first history should remain present");
    assert_eq!(historical.source_receipt_id, receipt.source_receipt_id);
    assert!(!historical.replayed);

    let replay_input = artifact_record_input(&data_dir, &release, "receipt-first-artifact").await;
    assert!(store
        .record_external_pool_adapter_artifact_source(replay_input)
        .is_err());
    drop(store);

    let first_reopen = Store::open(&database_path).expect("terminal database should reopen");
    let reopened = first_reopen
        .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
        .unwrap()
        .unwrap();
    assert_eq!(reopened.source_receipt_id, receipt.source_receipt_id);
    assert!(first_reopen
        .external_pool_adapter_artifact_intake_authority(
            &release.admission_id,
            &release.admission_digest,
        )
        .is_err());
    drop(first_reopen);

    let second_reopen = Store::open(&database_path).expect("terminal database should reopen twice");
    let current = second_reopen
        .external_pool_adapter_release_admission_currentness(&release.admission_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.current_status, "revoked");
    assert_eq!(
        second_reopen
            .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
            .unwrap()
            .unwrap()
            .source_receipt_id,
        receipt.source_receipt_id
    );

    require_current_quarantined_artifact_bytes(
        &data_dir,
        receipt.content_address_digest(),
        receipt.artifact_size_bytes(),
    )
    .await
    .expect("historical receipt bytes should still reopen and rehash exactly");

    drop(second_reopen);
    cleanup_lifecycle_files(&database_path, &data_dir);
}
