use std::time::Duration;

use crate::compute_federation::external_pool_adapter_release_lifecycle::{
    canonical_external_pool_adapter_release_admission_terminal_request_digest,
    validate_external_pool_adapter_release_admission_terminal_receipt,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ADAPTER_EFFECT,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ARTIFACT_INTAKE_EFFECT,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_CURRENTNESS_EFFECT,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_EXISTING_ARTIFACT_SOURCE_EFFECT,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ROUTE_EFFECT,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN,
};
use rusqlite::params;

use super::{lifecycle_support::*, *};

#[test]
fn lifecycle_all_three_terminals_preserve_the_staged_root_and_exact_effects() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let withdrawn = stage_release(&store, "status-pool", "1.0.0", "status-withdrawn");
    let revoked = stage_release(&store, "status-pool", "2.0.0", "status-revoked");
    let superseded = stage_release(&store, "status-pool", "3.0.0", "status-superseded");
    let successor = stage_release(&store, "status-pool", "4.0.0", "status-successor");

    let withdrawn_receipt = store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &withdrawn,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN,
            "terminal-withdrawn",
        ))
        .expect("withdrawal should append");
    assert_terminal(
        &store,
        &withdrawn,
        &withdrawn_receipt,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN,
        None,
    );

    let revoked_receipt = store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &revoked,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "terminal-revoked",
        ))
        .expect("revocation should append");
    assert_terminal(
        &store,
        &revoked,
        &revoked_receipt,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
        None,
    );

    let superseded_receipt = store
        .create_external_pool_adapter_release_admission_terminal(supersession_input(
            &superseded,
            &successor,
            "terminal-superseded",
        ))
        .expect("exact current successor should permit supersession");
    assert_terminal(
        &store,
        &superseded,
        &superseded_receipt,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED,
        Some(&successor),
    );
    let successor_current = store
        .external_pool_adapter_release_admission_currentness(&successor.admission_id)
        .unwrap()
        .unwrap();
    assert_eq!(successor_current.current_status, "staged");
    assert!(successor_current.successor_admission.is_none());

    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[test]
fn lifecycle_terminal_replay_conflicts_and_survives_two_reopens() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(&store, "replay-pool", "1.0.0", "replay-base");
    let first = store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "replay-terminal",
        ))
        .expect("first terminal should append");
    assert!(!first.replayed);
    let replay = store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "replay-terminal",
        ))
        .expect("exact terminal should replay");
    assert!(replay.replayed);
    assert_eq!(
        replay.terminal_receipt.terminal_receipt_id,
        first.terminal_receipt.terminal_receipt_id
    );
    assert_eq!(
        replay.terminal_receipt.terminal_receipt_digest,
        first.terminal_receipt.terminal_receipt_digest
    );

    let mut changed = terminal_input(
        &release,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
        "replay-terminal",
    );
    changed.reason = "changed terminal replay material must conflict".to_string();
    assert!(store
        .create_external_pool_adapter_release_admission_terminal(changed)
        .is_err());
    assert!(store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN,
            "another-terminal-key",
        ))
        .is_err());

    let wrong_digest = stage_release(&store, "replay-pool", "2.0.0", "wrong-base-digest");
    assert!(store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &wrong_digest,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "replay-terminal",
        ))
        .is_err());
    let mut wrong = terminal_input(
        &wrong_digest,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
        "wrong-base-terminal",
    );
    wrong.expected_admission_digest = "f".repeat(64);
    assert!(store
        .create_external_pool_adapter_release_admission_terminal(wrong)
        .is_err());
    assert_eq!(
        store
            .external_pool_adapter_release_admission_currentness(&wrong_digest.admission_id)
            .unwrap()
            .unwrap()
            .current_status,
        "staged"
    );
    drop(store);

    let first_reopen = Store::open(&database_path).expect("terminal database should reopen");
    let reopened_replay = first_reopen
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "replay-terminal",
        ))
        .expect("terminal replay must not reject its terminal base");
    assert!(reopened_replay.replayed);
    drop(first_reopen);

    let second_reopen = Store::open(&database_path).expect("terminal database should reopen twice");
    let current = second_reopen
        .external_pool_adapter_release_admission_currentness(&release.admission_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.admission_status, "staged");
    assert_eq!(current.current_status, "revoked");
    let terminal_count: i64 = second_reopen
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM compute_external_pool_adapter_release_admission_terminal_receipts WHERE admission_id=?1",
            params![release.admission_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(terminal_count, 1);
    drop(second_reopen);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[test]
fn lifecycle_supersession_rejects_every_non_exact_or_non_current_successor() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let missing_base = stage_release(&store, "invalid-pool", "1.0.0", "missing-successor");
    let missing = terminal_input(
        &missing_base,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED,
        "missing-successor-terminal",
    );
    assert!(store
        .create_external_pool_adapter_release_admission_terminal(missing)
        .is_err());

    let digest_base = stage_release(&store, "invalid-pool", "2.0.0", "digest-base");
    let digest_successor = stage_release(&store, "invalid-pool", "3.0.0", "digest-successor");
    let mut wrong_digest =
        supersession_input(&digest_base, &digest_successor, "wrong-successor-digest");
    wrong_digest.expected_successor_admission_digest = Some("f".repeat(64));
    assert!(store
        .create_external_pool_adapter_release_admission_terminal(wrong_digest)
        .is_err());

    let lineage_base = stage_release(&store, "lineage-a", "4.0.0", "lineage-base");
    let wrong_lineage = stage_release(&store, "lineage-b", "4.0.0", "lineage-successor");
    assert!(store
        .create_external_pool_adapter_release_admission_terminal(supersession_input(
            &lineage_base,
            &wrong_lineage,
            "wrong-successor-lineage",
        ))
        .is_err());

    let earlier = stage_release(&store, "time-pool", "5.0.0", "earlier-successor");
    std::thread::sleep(Duration::from_millis(2));
    let later_base = stage_release(&store, "time-pool", "6.0.0", "later-base");
    assert!(earlier.applied_at < later_base.applied_at);
    assert!(store
        .create_external_pool_adapter_release_admission_terminal(supersession_input(
            &later_base,
            &earlier,
            "earlier-successor-terminal",
        ))
        .is_err());

    let terminal_base = stage_release(&store, "terminal-pool", "7.0.0", "terminal-base");
    let terminal_successor = stage_release(&store, "terminal-pool", "8.0.0", "terminal-successor");
    store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &terminal_successor,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "successor-revoked",
        ))
        .unwrap();
    assert!(store
        .create_external_pool_adapter_release_admission_terminal(supersession_input(
            &terminal_base,
            &terminal_successor,
            "terminal-successor-reference",
        ))
        .is_err());

    let self_base = stage_release(&store, "self-pool", "9.0.0", "self-base");
    assert!(store
        .create_external_pool_adapter_release_admission_terminal(supersession_input(
            &self_base,
            &self_base,
            "self-successor-terminal",
        ))
        .is_err());
    for base in [
        missing_base,
        digest_base,
        lineage_base,
        later_base,
        terminal_base,
        self_base,
    ] {
        assert_eq!(
            store
                .external_pool_adapter_release_admission_currentness(&base.admission_id)
                .unwrap()
                .unwrap()
                .current_status,
            "staged"
        );
    }

    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

fn assert_terminal(
    store: &Store,
    release: &StagedRelease,
    write: &crate::store::ExternalPoolAdapterReleaseAdmissionTerminalWriteReceipt,
    status: &str,
    successor: Option<&StagedRelease>,
) {
    assert!(!write.replayed);
    validate_external_pool_adapter_release_admission_terminal_receipt(&write.terminal_receipt)
        .expect("terminal receipt should be canonical");
    let terminal = &write.terminal_receipt.terminal;
    assert_eq!(
        write.terminal_receipt.request_digest,
        canonical_external_pool_adapter_release_admission_terminal_request_digest(terminal)
            .expect("terminal request digest should use the canonical request domain")
    );
    assert_ne!(
        write.terminal_receipt.request_digest,
        write.terminal_receipt.terminal_receipt_digest
    );
    assert_eq!(terminal.admission.admission_id, release.admission_id);
    assert_eq!(
        terminal.admission.admission_digest,
        release.admission_digest
    );
    assert_eq!(terminal.admission.adapter_id, release.adapter_id);
    assert_eq!(terminal.admission.release_version, release.release_version);
    assert_eq!(terminal.prior_status, "staged");
    assert_eq!(terminal.terminal_status, status);
    assert_eq!(
        terminal.currentness_effect,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_CURRENTNESS_EFFECT
    );
    assert_eq!(
        terminal.artifact_intake_effect,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ARTIFACT_INTAKE_EFFECT
    );
    assert_eq!(
        terminal.existing_artifact_source_effect,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_EXISTING_ARTIFACT_SOURCE_EFFECT
    );
    assert_eq!(
        terminal.adapter_effect,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ADAPTER_EFFECT
    );
    assert_eq!(
        terminal.route_effect,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ROUTE_EFFECT
    );
    assert_eq!(terminal.occurred_at, terminal.recorded_at);
    assert_eq!(
        terminal
            .successor_admission
            .as_ref()
            .map(|item| item.admission_id.as_str()),
        successor.map(|item| item.admission_id.as_str())
    );
    assert_eq!(
        terminal
            .successor_admission
            .as_ref()
            .map(|item| item.admission_digest.as_str()),
        successor.map(|item| item.admission_digest.as_str())
    );
    assert_eq!(
        terminal
            .successor_admission
            .as_ref()
            .map(|item| item.release_version.as_str()),
        successor.map(|item| item.release_version.as_str())
    );

    let current = store
        .external_pool_adapter_release_admission_currentness(&release.admission_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.admission_status, "staged");
    assert_eq!(current.current_status, status);
    assert_eq!(current.adapter_id, release.adapter_id);
    assert_eq!(current.release_version, release.release_version);
    assert_eq!(
        current.terminal_receipt_id.as_deref(),
        Some(write.terminal_receipt.terminal_receipt_id.as_str())
    );
    assert_eq!(
        current.terminal_receipt_digest.as_deref(),
        Some(write.terminal_receipt.terminal_receipt_digest.as_str())
    );
    assert_eq!(
        current
            .successor_admission
            .as_ref()
            .map(|item| item.admission_digest.as_str()),
        successor.map(|item| item.admission_digest.as_str())
    );
    assert_eq!(
        current
            .successor_admission
            .as_ref()
            .map(|item| item.admission_id.as_str()),
        successor.map(|item| item.admission_id.as_str())
    );
    assert_eq!(
        current
            .successor_admission
            .as_ref()
            .map(|item| item.release_version.as_str()),
        successor.map(|item| item.release_version.as_str())
    );
    let root_status: String = store
        .conn()
        .unwrap()
        .query_row(
            "SELECT status FROM compute_external_pool_adapter_release_admissions WHERE admission_id=?1",
            params![release.admission_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(root_status, "staged");
}
