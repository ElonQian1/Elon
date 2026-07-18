use super::*;
use crate::node_agent_cli_sidecar_io::{append_output, CliSidecarOutputRecord};

#[test]
fn terminal_output_detection_survives_reopen_and_duplicate_scan() {
    let path = std::env::temp_dir().join(format!(
        "elon-sidecar-terminal-{}.jsonl",
        uuid::Uuid::new_v4().simple()
    ));
    append_output(&path, CliSidecarOutputRecord::chunk("stdout", "working\n")).unwrap();
    assert!(!output_contains_terminal_record(&path).unwrap());
    append_output(&path, CliSidecarOutputRecord::exit(true, false)).unwrap();
    assert!(output_contains_terminal_record(&path).unwrap());
    assert!(output_contains_terminal_record(&path).unwrap());
    let _ = std::fs::remove_file(path);
}

#[test]
fn replay_state_advances_without_skipping_checkpoint() {
    let path = std::env::temp_dir().join(format!(
        "elon-sidecar-receipt-{}.json",
        uuid::Uuid::new_v4().simple()
    ));
    let store = crate::node_agent_update_recovery::UpdateRecoveryStore::new(&path);
    let mut receipt = UpdateRecoveryReceipt::planned("update", "root", "task");
    receipt
        .transition(UpdateRecoveryState::Downloaded, None)
        .unwrap();
    receipt
        .transition(UpdateRecoveryState::CheckpointSaved, None)
        .unwrap();
    receipt
        .transition(UpdateRecoveryState::Applying, None)
        .unwrap();
    store.upsert(receipt).unwrap();
    let runtime_path = path;
    let mut saved = store.load().unwrap().receipts.remove(0);
    saved
        .transition(UpdateRecoveryState::RuntimeOnline, None)
        .unwrap();
    saved
        .transition(UpdateRecoveryState::Reattaching, None)
        .unwrap();
    saved
        .transition(UpdateRecoveryState::Resumed, None)
        .unwrap();
    assert_eq!(saved.state, UpdateRecoveryState::Resumed);
    let _ = std::fs::remove_file(runtime_path);
}
