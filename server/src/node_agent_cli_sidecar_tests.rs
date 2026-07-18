use super::{now_ms, CliSidecarRegistry, CliSidecarSessionRecord};
use std::{fs, io::Write, path::PathBuf, process::Command};

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
        .record_cancel_command_with_audit(
            "task-1",
            &homecli_proto::CancelRequestAudit {
                requested_by: Some("owner-1".to_string()),
                source: Some("pc_ui".to_string()),
                reason: Some("user_stop_button".to_string()),
                requested_at_ms: Some(5678),
                interruption_source: Some(
                    homecli_proto::InterruptionSource::SupervisorIntervention,
                ),
            },
        )
        .expect("sidecar cancel command should persist"));
    let commands =
        fs::read_to_string(dir.join("commands-task-1.jsonl")).expect("mailbox should exist");
    assert!(commands.contains(r#""command":"tool_approval_decision""#));
    assert!(commands.contains(r#""approval_id":"tap_1_1""#));
    assert!(commands.contains(r#""command":"cancel""#));
    assert!(commands.contains(r#""requested_by":"owner-1""#));
    assert!(commands.contains(r#""source":"pc_ui""#));
    assert!(commands.contains(r#""reason":"user_stop_button""#));
    assert!(commands.contains(r#""requested_at_ms":5678"#));
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

#[test]
fn sidecar_registry_recovers_real_excess_tail_brace_from_valid_backup() {
    let dir = unique_test_dir("tail-brace-recovery");
    let _ = fs::remove_dir_all(&dir);
    let registry = CliSidecarRegistry::new(&dir);
    registry
        .upsert_session(session("session-a", "task-a"))
        .unwrap();
    registry
        .upsert_session(session("session-b", "task-b"))
        .unwrap();
    assert!(dir.join("sessions.json.bak").exists());

    let mut primary = fs::OpenOptions::new()
        .append(true)
        .open(dir.join("sessions.json"))
        .unwrap();
    writeln!(primary, "}}").unwrap();
    primary.sync_all().unwrap();
    drop(primary);

    let recovered = CliSidecarRegistry::new(&dir).all_sessions().unwrap();
    assert_eq!(
        recovered.len(),
        1,
        "backup should be the prior valid generation"
    );
    assert_eq!(recovered[0].session_id, "session-a");
    let rebuilt = fs::read_to_string(dir.join("sessions.json")).unwrap();
    serde_json::from_str::<serde_json::Value>(&rebuilt)
        .expect("primary must be rebuilt as valid JSON");

    registry
        .upsert_session(session("session-c", "task-c"))
        .unwrap();
    assert_eq!(registry.all_sessions().unwrap().len(), 2);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn corrupt_sessions_without_a_valid_backup_fails_closed() {
    let dir = unique_test_dir("corrupt-without-backup");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("test registry directory should exist");
    fs::write(dir.join("sessions.json"), b"{\"broken\":").unwrap();
    fs::write(dir.join("sessions.json.bak"), b"{\"also-broken\":").unwrap();

    let registry = CliSidecarRegistry::new(&dir);
    let error = registry
        .all_sessions()
        .expect_err("corruption without a valid backup must not be swallowed");
    let detail = format!("{error:#}");
    assert!(detail.contains("主文件损坏且备份不可恢复"));
    assert!(detail.contains("sessions.json.bak"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn sidecar_registry_cross_process_writer_stress_preserves_every_session() {
    let dir = unique_test_dir("cross-process-writers");
    let _ = fs::remove_dir_all(&dir);
    let writers = 6;
    let per_writer = 20;
    let mut children = Vec::new();
    for writer in 0..writers {
        children.push(
            Command::new(std::env::current_exe().unwrap())
                .args(["sidecar_registry_process_writer_helper", "--nocapture"])
                .env("ELON_SIDECAR_TEST_WRITER_DIR", &dir)
                .env("ELON_SIDECAR_TEST_WRITER_ID", writer.to_string())
                .env("ELON_SIDECAR_TEST_WRITER_COUNT", per_writer.to_string())
                .spawn()
                .expect("spawn registry writer process"),
        );
    }
    for child in children {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "writer process failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let sessions = CliSidecarRegistry::new(&dir).all_sessions().unwrap();
    assert_eq!(sessions.len(), writers * per_writer);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn sidecar_registry_process_writer_helper() {
    let Ok(dir) = std::env::var("ELON_SIDECAR_TEST_WRITER_DIR") else {
        return;
    };
    let writer: usize = std::env::var("ELON_SIDECAR_TEST_WRITER_ID")
        .unwrap()
        .parse()
        .unwrap();
    let count: usize = std::env::var("ELON_SIDECAR_TEST_WRITER_COUNT")
        .unwrap()
        .parse()
        .unwrap();
    let registry = CliSidecarRegistry::new(dir);
    for index in 0..count {
        let id = format!("writer-{writer}-session-{index}");
        registry
            .upsert_session(session(&id, &format!("writer-{writer}-task-{index}")))
            .unwrap();
    }
}

fn session(session_id: &str, task_id: &str) -> CliSidecarSessionRecord {
    CliSidecarSessionRecord::managed_pipe_json(
        session_id,
        task_id,
        "codex",
        "route_a_external_cli",
        None,
        None,
        Some(std::process::id()),
        None,
        now_ms(),
    )
}

fn unique_test_dir(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-cli-sidecar-test-{}-{}",
        std::process::id(),
        suffix
    ))
}
