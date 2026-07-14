use super::{now_ms, CliSidecarRegistry, CliSidecarSessionRecord};
use std::{fs, path::PathBuf};

#[test]
fn sidecar_session_survives_registry_reload_and_accepts_recovered_approval() {
    let dir = unique_test_dir("approval");
    let _ = fs::remove_dir_all(&dir);
    let registry = CliSidecarRegistry::new(&dir);
    registry
        .upsert_session(CliSidecarSessionRecord::managed_conpty(
            "sidecar-1",
            "task-1",
            "codex",
            "route_a_external_cli",
            Some("D:/demo".to_string()),
            Some("npipe://elon/sidecar-1".to_string()),
            Some(100),
            Some(200),
            now_ms(),
        ))
        .expect("sidecar session should persist");

    let reloaded = CliSidecarRegistry::new(&dir)
        .session_for_task("task-1")
        .expect("session lookup should read")
        .expect("session should exist");
    assert!(reloaded.is_attachable_at(now_ms()));
    assert!(reloaded.can_recover_tool_approval_after_restart(now_ms()));
    assert!(CliSidecarRegistry::new(&dir)
        .record_tool_approval_decision("task-1", "tap_1_1", "approve")
        .expect("sidecar command should persist"));
    assert!(CliSidecarRegistry::new(&dir)
        .record_cancel_command("task-1")
        .expect("sidecar cancel command should persist"));
    let commands =
        fs::read_to_string(dir.join("commands-task-1.jsonl")).expect("mailbox should exist");
    assert!(commands.contains(r#""command":"tool_approval_decision""#));
    assert!(commands.contains(r#""approval_id":"tap_1_1""#));
    assert!(commands.contains(r#""command":"cancel""#));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stale_sidecar_session_does_not_claim_restart_approval() {
    let now = now_ms();
    let mut session = CliSidecarSessionRecord::managed_conpty(
        "sidecar-1",
        "task-1",
        "codex",
        "route_a_external_cli",
        None,
        None,
        None,
        None,
        now.saturating_sub(10 * 60 * 1_000),
    );
    session.last_seen_at_ms = now.saturating_sub(10 * 60 * 1_000);

    assert!(!session.is_attachable_at(now));
    assert!(!session.can_recover_tool_approval_after_restart(now));
}

#[test]
fn sidecar_commands_fail_closed_without_attachable_session_or_valid_decision() {
    let dir = unique_test_dir("fail-closed");
    let _ = fs::remove_dir_all(&dir);
    let registry = CliSidecarRegistry::new(&dir);

    assert!(!registry
        .record_cancel_command("missing-task")
        .expect("missing sidecar should be handled"));
    assert!(!registry
        .record_tool_approval_decision("missing-task", "tap_1_1", "approve")
        .expect("missing sidecar should be handled"));

    registry
        .upsert_session(CliSidecarSessionRecord::managed_conpty(
            "sidecar-1",
            "task-1",
            "codex",
            "route_a_external_cli",
            None,
            None,
            None,
            None,
            now_ms(),
        ))
        .expect("sidecar session should persist");
    assert!(!registry
        .record_tool_approval_decision("task-1", "tap_1_1", "maybe")
        .expect("invalid decision should be rejected"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn pipe_json_sidecar_can_cancel_without_terminal_attach() {
    let dir = unique_test_dir("pipe-cancel");
    let _ = fs::remove_dir_all(&dir);
    let registry = CliSidecarRegistry::new(&dir);
    registry
        .upsert_session(CliSidecarSessionRecord::managed_pipe_json(
            "sidecar-pipe-1",
            "task-pipe-1",
            "codex",
            "route_a_external_cli",
            Some("D:/demo".to_string()),
            Some("D:/state/output.jsonl".to_string()),
            Some(100),
            Some(200),
            now_ms(),
        ))
        .expect("pipe sidecar session should persist");

    let session = registry
        .session_for_task("task-pipe-1")
        .expect("session lookup should read")
        .expect("session should exist");
    assert!(!session.is_attachable_at(now_ms()));
    assert!(session.can_replay_output_at(now_ms()));
    assert!(session.can_cancel_at(now_ms()));
    assert!(!session.can_recover_tool_approval_after_restart(now_ms()));
    assert!(registry
        .record_cancel_command("task-pipe-1")
        .expect("pipe sidecar cancel command should persist"));
    let commands =
        fs::read_to_string(dir.join("commands-task-pipe-1.jsonl")).expect("mailbox should exist");
    assert!(commands.contains(r#""command":"cancel""#));
    let _ = fs::remove_dir_all(dir);
}

fn unique_test_dir(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-cli-sidecar-test-{}-{}",
        std::process::id(),
        suffix
    ))
}
