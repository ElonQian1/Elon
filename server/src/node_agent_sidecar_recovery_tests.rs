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

#[test]
fn multi_generation_sidecar_recovery_replaces_stale_target_without_manual_resume() {
    let mut receipt = UpdateRecoveryReceipt::planned("update-old", "root", "task");
    receipt.to_release = ReleaseIdentity {
        version: "0.3.69".to_string(),
        git_sha: "old-sha".to_string(),
    };
    assert!(!receipt_targets_release(&receipt, "0.3.69+new-sha"));
    assert!(receipt_targets_release(&receipt, "0.3.69+old-sha"));
    assert!(recoverable_sidecar_task_status("resume_required"));
    assert!(recoverable_sidecar_task_status("cancel_requested"));
    assert!(!recoverable_sidecar_task_status("done"));
    assert_eq!(
        release_identity("0.3.69+new-sha"),
        ReleaseIdentity {
            version: "0.3.69".to_string(),
            git_sha: "new-sha".to_string(),
        }
    );
}

#[test]
fn codex_process_exit_without_turn_terminal_is_not_completion() {
    let interrupted = concat!(
        "{\"type\":\"turn.started\"}\n",
        "{\"type\":\"item.completed\",\"item\":{\"id\":\"msg\",\"type\":\"agent_message\",\"text\":\"still working\"}}\n"
    );
    assert_eq!(codex_terminal_outcome(interrupted), None);
    assert_eq!(
        codex_terminal_outcome(concat!(
            "{\"type\":\"item.completed\",\"item\":{\"id\":\"msg\",\"type\":\"agent_message\",\"text\":\"done\"}}\n",
            "{\"type\":\"turn.completed\"}\n"
        )),
        Some(true)
    );
    assert_eq!(
        codex_terminal_outcome("{\"type\":\"turn.failed\"}\n"),
        Some(false)
    );
}

#[test]
fn only_complete_evidence_with_no_live_handle_allows_resume_required() {
    let mut receipt = UpdateRecoveryReceipt::planned("update", "root", "task");
    assert!(!stale_transition_evidence_complete(
        false,
        false,
        Some(&receipt)
    ));
    receipt.safety.evidence_complete = true;
    assert!(stale_transition_evidence_complete(
        false,
        false,
        Some(&receipt)
    ));
    assert!(!stale_transition_evidence_complete(
        true,
        false,
        Some(&receipt)
    ));
    assert!(!stale_transition_evidence_complete(
        false,
        true,
        Some(&receipt)
    ));
}
