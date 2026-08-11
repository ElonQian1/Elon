use std::{
    sync::{Arc, Barrier},
    thread,
};

use crate::{
    compute_federation::{
        external_pool_adapter_artifact_source::require_current_quarantined_artifact_bytes,
        external_pool_adapter_release_lifecycle::{
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED,
        },
    },
    store::Store,
};

use super::lifecycle_support::*;

#[test]
fn lifecycle_two_connections_linearize_exact_terminal_as_one_append_and_one_replay() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(&store, "same-terminal-pool", "1.0.0", "same-terminal");
    let store_a = Store::open(&database_path).expect("first race connection should open");
    let store_b = Store::open(&database_path).expect("second race connection should open");
    let barrier = Arc::new(Barrier::new(3));

    let release_a = release.clone();
    let barrier_a = Arc::clone(&barrier);
    let write_a = thread::spawn(move || {
        barrier_a.wait();
        store_a.create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release_a,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "same-terminal-key",
        ))
    });
    let release_b = release.clone();
    let barrier_b = Arc::clone(&barrier);
    let write_b = thread::spawn(move || {
        barrier_b.wait();
        store_b.create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release_b,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "same-terminal-key",
        ))
    });

    barrier.wait();
    let writes = [
        write_a
            .join()
            .expect("first race thread should finish")
            .unwrap(),
        write_b
            .join()
            .expect("second race thread should finish")
            .unwrap(),
    ];
    assert_eq!(writes.iter().filter(|write| write.replayed).count(), 1);
    assert_eq!(
        writes[0].terminal_receipt.terminal_receipt_id,
        writes[1].terminal_receipt.terminal_receipt_id
    );
    assert_eq!(
        writes[0].terminal_receipt.terminal_receipt_digest,
        writes[1].terminal_receipt.terminal_receipt_digest
    );
    assert_eq!(terminal_count(&store), 1);

    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[test]
fn lifecycle_two_connections_reject_competing_terminal_keys_for_one_admission() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(
        &store,
        "different-terminal-pool",
        "1.0.0",
        "different-terminal",
    );
    let store_a = Store::open(&database_path).expect("first race connection should open");
    let store_b = Store::open(&database_path).expect("second race connection should open");
    let barrier = Arc::new(Barrier::new(3));

    let release_a = release.clone();
    let barrier_a = Arc::clone(&barrier);
    let write_a = thread::spawn(move || {
        barrier_a.wait();
        store_a.create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release_a,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "competing-terminal-a",
        ))
    });
    let release_b = release.clone();
    let barrier_b = Arc::clone(&barrier);
    let write_b = thread::spawn(move || {
        barrier_b.wait();
        store_b.create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release_b,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "competing-terminal-b",
        ))
    });

    barrier.wait();
    let outcomes = [
        write_a.join().expect("first race thread should finish"),
        write_b.join().expect("second race thread should finish"),
    ];
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_err()).count(),
        1
    );
    assert_eq!(terminal_count(&store), 1);
    assert_eq!(
        store
            .external_pool_adapter_release_admission_currentness(&release.admission_id)
            .unwrap()
            .unwrap()
            .current_status,
        "revoked"
    );

    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[test]
fn lifecycle_supersession_and_successor_terminal_race_has_only_linearizable_outcomes() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let base = stage_release(
        &store,
        "successor-race-pool",
        "1.0.0",
        "successor-race-base",
    );
    std::thread::sleep(std::time::Duration::from_millis(2));
    let successor = stage_release(
        &store,
        "successor-race-pool",
        "2.0.0",
        "successor-race-next",
    );
    let supersede_store = Store::open(&database_path).expect("supersede connection should open");
    let successor_store = Store::open(&database_path).expect("successor connection should open");
    let barrier = Arc::new(Barrier::new(3));

    let supersede_input = supersession_input(&base, &successor, "successor-race-supersede");
    let supersede_barrier = Arc::clone(&barrier);
    let supersede = thread::spawn(move || {
        supersede_barrier.wait();
        supersede_store.create_external_pool_adapter_release_admission_terminal(supersede_input)
    });
    let successor_input = terminal_input(
        &successor,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
        "successor-race-revoke",
    );
    let successor_barrier = Arc::clone(&barrier);
    let revoke_successor = thread::spawn(move || {
        successor_barrier.wait();
        successor_store.create_external_pool_adapter_release_admission_terminal(successor_input)
    });

    barrier.wait();
    let supersede_outcome = supersede
        .join()
        .expect("supersede race thread should finish");
    let successor_outcome = revoke_successor
        .join()
        .expect("successor race thread should finish");
    assert!(successor_outcome.is_ok());

    let base_current = store
        .external_pool_adapter_release_admission_currentness(&base.admission_id)
        .unwrap()
        .unwrap();
    let successor_current = store
        .external_pool_adapter_release_admission_currentness(&successor.admission_id)
        .unwrap()
        .unwrap();
    assert_eq!(successor_current.current_status, "revoked");
    if supersede_outcome.is_ok() {
        assert_eq!(
            base_current.current_status,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED
        );
        assert_eq!(
            base_current
                .successor_admission
                .as_ref()
                .map(|binding| binding.admission_id.as_str()),
            Some(successor.admission_id.as_str())
        );
        assert_eq!(terminal_count(&store), 2);
    } else {
        assert_eq!(base_current.current_status, "staged");
        assert!(base_current.successor_admission.is_none());
        assert_eq!(terminal_count(&store), 1);
    }

    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[tokio::test]
async fn lifecycle_artifact_receipt_and_terminal_race_preserves_currentness_and_history() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(&store, "artifact-race-pool", "1.0.0", "artifact-race");
    let artifact_input = artifact_record_input(&data_dir, &release, "artifact-race-key").await;
    let artifact_store = Store::open(&database_path).expect("artifact connection should open");
    let terminal_store = Store::open(&database_path).expect("terminal connection should open");
    let barrier = Arc::new(Barrier::new(3));

    let artifact_barrier = Arc::clone(&barrier);
    let artifact_write = thread::spawn(move || {
        artifact_barrier.wait();
        artifact_store.record_external_pool_adapter_artifact_source(artifact_input)
    });
    let terminal_input = terminal_input(
        &release,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
        "artifact-race-terminal",
    );
    let terminal_barrier = Arc::clone(&barrier);
    let terminal_write = thread::spawn(move || {
        terminal_barrier.wait();
        terminal_store.create_external_pool_adapter_release_admission_terminal(terminal_input)
    });

    barrier.wait();
    let artifact_outcome = artifact_write
        .join()
        .expect("artifact race thread should finish");
    terminal_write
        .join()
        .expect("terminal race thread should finish")
        .expect("terminal should always linearize");

    let current = store
        .external_pool_adapter_release_admission_currentness(&release.admission_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.current_status, "revoked");
    assert_eq!(terminal_count(&store), 1);
    let historical = store
        .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
        .unwrap();
    match artifact_outcome {
        Ok(receipt) => {
            assert_eq!(
                historical
                    .as_ref()
                    .map(|value| value.source_receipt_id.as_str()),
                Some(receipt.source_receipt_id.as_str())
            );
        }
        Err(_) => assert!(historical.is_none()),
    }
    require_current_quarantined_artifact_bytes(
        &data_dir,
        &release.declared_sha256,
        release.artifact_bytes.len() as u64,
    )
    .await
    .expect("the pre-race CAS result should remain exact in either DB outcome");

    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

fn terminal_count(store: &Store) -> i64 {
    let connection = store.conn().expect("Store connection should open");
    connection
        .query_row(
            "SELECT COUNT(*) FROM compute_external_pool_adapter_release_admission_terminal_receipts",
            [],
            |row| row.get(0),
        )
        .expect("terminal count should be readable")
}
