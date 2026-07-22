use super::{status_payload, status_payload_for};
use crate::{
    node_agent_active_task::ActiveCliPromptView,
    node_agent_cli_sidecar::{now_ms, CliSidecarSessionRecord},
    node_agent_task_journal::TaskJournalRecord,
};

#[test]
fn static_status_declares_recoverable_bridge_without_tty_takeover() {
    let status = status_payload();

    assert_eq!(status["tty_takeover_supported"], false);
    assert_eq!(status["status"], "recoverable_continuity");
    assert_eq!(status["current_state"], "ready_no_session");
    assert_eq!(status["restart_recovery_supported"], true);
    assert_eq!(status["post_restart_approval_supported"], false);
    assert_eq!(status["post_restart_approval_capability_supported"], true);
    assert_eq!(status["post_restart_approval_currently_available"], false);
    assert_eq!(status["can_resume_after_node_restart"], false);
    assert_eq!(status["json_stream_supported"], true);
    assert_eq!(status["codex_resume_supported"], true);
    assert!(status["display_summary"]
        .as_str()
        .unwrap_or_default()
        .contains("journal"));
    assert!(status["summary"]
        .as_str()
        .unwrap_or_default()
        .contains("当前没有可重接 sidecar"));
    assert!(status["continuity_modes"]
        .as_array()
        .is_some_and(|items| items.len() >= 3));
    assert!(status["not_supported"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item.as_str() == Some("attach_external_cli_tty_not_started_by_elon_sidecar")
        }));
    assert!(status["resume_order"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["kind"].as_str() == Some("codex_session_resume")
                && item["requires_new_task"].as_bool() == Some(true)
                && item["currently_available"].as_bool() == Some(false)
        }));
    assert!(status["recommended_next_actions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item.as_str()
                .unwrap_or_default()
                .contains("不再批准非 sidecar")
        }));
    assert_eq!(
        status["sidecar_protocol_mode"],
        "managed_pipe_json_or_pty_conpty"
    );
    assert_eq!(status["managed_conpty_sidecar_active"], false);
    assert_eq!(status["managed_pipe_json_sidecar_active"], false);
    assert_eq!(status["managed_tty_reattach_capability_supported"], true);
    assert_eq!(status["managed_tty_reattach_currently_available"], false);
    assert_eq!(status["sidecar_tool_approval_recovery_supported"], true);
    assert_eq!(
        status["sidecar_tool_approval_recovery_currently_available"],
        false
    );
    assert_eq!(
        status["restart_recovery"]["restores_tool_approval_waiter_supported"],
        true
    );
    assert_eq!(
        status["restart_recovery"]["restores_tool_approval_waiter_currently_available"],
        false
    );
    assert_eq!(
        status["capability_summary"]["managed_pty_conpty_sidecar"]["supported"],
        true
    );
    assert_eq!(
        status["capability_summary"]["managed_pty_conpty_sidecar"]["currently_available"],
        false
    );
    assert_eq!(
        status["capability_summary"]["managed_pipe_json_sidecar"]["supported"],
        true
    );
    assert_eq!(
        status["capability_summary"]["managed_pipe_json_sidecar"]["currently_available"],
        false
    );
    assert_eq!(
        status["capability_summary"]["post_restart_tool_approval"]["supported"],
        true
    );
    assert_eq!(
        status["capability_summary"]["post_restart_tool_approval"]["currently_available"],
        false
    );
    assert_eq!(
        status["capability_summary"]["external_tty_takeover"]["supported"],
        false
    );
    assert_eq!(
        status["sidecar_attach_api"]["write"],
        "/api/cli-sidecars/:task_id/input"
    );
}

#[test]
fn live_active_control_is_exposed_as_current_reconnect_path() {
    let active = vec![active_control("req-live")];
    let status = status_payload_for(&active, &[], &[]);

    assert_eq!(status["current_state"], "live_control_available");
    assert_eq!(status["can_reconnect_live_control"], true);
    assert_eq!(status["can_resume_after_node_restart"], false);
    assert_eq!(
        status["latest_recoverable_task"]["recovery_kind"],
        "live_control_handle"
    );
    assert_eq!(status["latest_recoverable_task"]["can_cancel"], true);
    assert_eq!(status["state_summary"]["active_control_count"], 1);
}

#[test]
fn detached_codex_record_makes_restart_resume_available() {
    let records = vec![codex_record("req-codex", "running", 200)];
    let status = status_payload_for(&[], &records, &[]);

    assert_eq!(status["current_state"], "codex_session_resumable");
    assert_eq!(status["can_resume_after_node_restart"], true);
    assert_eq!(status["can_resume_codex_session"], true);
    assert_eq!(
        status["restart_recovery"]["next_action"],
        "codex_exec_resume_or_snapshot_continue"
    );
    assert_eq!(
        status["latest_recoverable_task"]["recovery_kind"],
        "codex_session_resume"
    );
    assert_eq!(
        status["latest_recoverable_task"]["can_resume_codex_session"],
        true
    );
    assert_eq!(status["state_summary"]["detached_running_count"], 1);
    assert_eq!(status["state_summary"]["route_a_record_count"], 1);
}

#[test]
fn route_b_and_c_records_are_counted_for_snapshot_continue() {
    let records = vec![
        record(
            "req-b",
            "api-runtime",
            "route_b_api_runtime",
            "finished",
            100,
        ),
        record(
            "req-c",
            "server-runtime",
            "route_c_server_runtime",
            "cancel_requested",
            300,
        ),
    ];
    let status = status_payload_for(&[], &records, &[]);

    assert_eq!(status["current_state"], "detached_journal_recoverable");
    assert_eq!(status["can_continue_from_snapshot"], true);
    assert_eq!(
        status["restart_recovery"]["next_action"],
        "journal_replay_then_snapshot_continue"
    );
    assert_eq!(status["state_summary"]["route_b_record_count"], 1);
    assert_eq!(status["state_summary"]["route_c_record_count"], 1);
    assert_eq!(status["state_summary"]["detached_running_count"], 1);
    assert_eq!(status["state_summary"]["terminal_record_count"], 1);
    assert_eq!(status["latest_recoverable_task"]["task_id"], "req-c");
}

#[test]
fn sidecar_session_takes_priority_for_restart_attach_and_approval_recovery() {
    let sidecars = vec![sidecar("sidecar-1", "req-sidecar", now_ms())];
    let records = vec![record(
        "req-sidecar",
        "codex",
        "route_a_external_cli",
        "running",
        100,
    )];
    let status = status_payload_for(&[], &records, &sidecars);

    assert_eq!(status["status"], "sidecar_recoverable_continuity");
    assert_eq!(status["current_state"], "managed_sidecar_attachable");
    assert_eq!(status["managed_tty_reattach_supported"], true);
    assert_eq!(status["managed_tty_reattach_currently_available"], true);
    assert_eq!(status["can_resume_after_node_restart"], true);
    assert_eq!(status["can_approve_after_node_restart"], true);
    assert_eq!(status["post_restart_approval_capability_supported"], true);
    assert_eq!(status["post_restart_approval_currently_available"], true);
    assert_eq!(
        status["capability_summary"]["managed_pty_conpty_sidecar"]["currently_available"],
        true
    );
    assert_eq!(
        status["capability_summary"]["post_restart_tool_approval"]["recoverable_count"],
        1
    );
    assert_eq!(
        status["restart_recovery"]["mode"],
        "managed_pty_conpty_sidecar_attach"
    );
    assert_eq!(status["restart_recovery"]["restores_original_tty"], true);
    assert_eq!(
        status["restart_recovery"]["restores_tool_approval_waiter"],
        true
    );
    assert_eq!(
        status["latest_recoverable_task"]["recovery_kind"],
        "managed_pty_conpty_sidecar_attach"
    );
    assert_eq!(
        status["latest_recoverable_task"]["can_write_terminal"],
        true
    );
    assert_eq!(
        status["latest_recoverable_task"]["can_resize_terminal"],
        true
    );
    assert_eq!(status["latest_sidecar_session"]["session_id"], "sidecar-1");
    assert_eq!(status["state_summary"]["sidecar_stream_replay_count"], 1);
    assert_eq!(status["state_summary"]["sidecar_attachable_count"], 1);
    assert_eq!(
        status["resume_order"].as_array().unwrap().first().unwrap()["kind"],
        "managed_pty_conpty_sidecar_attach"
    );
}

#[test]
fn pipe_json_sidecar_session_is_followable_without_tty() {
    let sidecars = vec![pipe_sidecar("sidecar-pipe-1", "req-pipe", now_ms())];
    let records = vec![record(
        "req-pipe",
        "codex",
        "route_a_external_cli",
        "running",
        100,
    )];
    let status = status_payload_for(&[], &records, &sidecars);

    assert_eq!(status["status"], "sidecar_recoverable_continuity");
    assert_eq!(
        status["current_state"],
        "managed_pipe_json_sidecar_followable"
    );
    assert_eq!(
        status["restart_recovery"]["next_action"],
        "managed_pipe_json_sidecar_follow"
    );
    assert_eq!(
        status["restart_recovery"]["mode"],
        "managed_pipe_json_sidecar_follow"
    );
    assert_eq!(status["restart_recovery"]["restores_original_tty"], false);
    assert_eq!(
        status["restart_recovery"]["restores_json_pipe_output"],
        true
    );
    assert_eq!(status["managed_conpty_sidecar_active"], false);
    assert_eq!(status["managed_pipe_json_sidecar_active"], true);
    assert_eq!(status["managed_tty_reattach_currently_available"], false);
    assert_eq!(
        status["sidecar_protocol_mode"],
        "managed_pipe_json_output_replay_cancel"
    );
    assert_eq!(
        status["latest_recoverable_task"]["recovery_kind"],
        "managed_pipe_json_sidecar_follow"
    );
    assert_eq!(
        status["latest_recoverable_task"]["can_attach_sidecar"],
        false
    );
    assert_eq!(
        status["latest_recoverable_task"]["can_stream_live_output"],
        true
    );
    assert_eq!(
        status["latest_recoverable_task"]["can_write_terminal"],
        false
    );
    assert_eq!(status["latest_recoverable_task"]["can_cancel"], true);
    assert_eq!(
        status["capability_summary"]["managed_pipe_json_sidecar"]["currently_available"],
        true
    );
    assert_eq!(
        status["capability_summary"]["managed_pty_conpty_sidecar"]["currently_available"],
        false
    );
    assert_eq!(status["state_summary"]["sidecar_stream_replay_count"], 1);
    assert_eq!(status["state_summary"]["sidecar_attachable_count"], 0);
}

fn active_control(req_id: &str) -> ActiveCliPromptView {
    ActiveCliPromptView {
        req_id: req_id.to_string(),
        run_handle_id: req_id.to_string(),
        cli_name: "codex".to_string(),
        route: "route_a_external_cli".to_string(),
        cwd: Some("D:/demo".to_string()),
        runtime_permission: Some("project_write".to_string()),
        requires_cloud_control: false,
        started_at_ms: 1,
        last_heartbeat_ms: 2,
        control_lease_expires_at_ms: 47_000,
        os_pid: Some(4242),
        control_handle_live: true,
        pending_approvals: Vec::new(),
    }
}

fn codex_record(req_id: &str, status: &str, updated_at_ms: u128) -> TaskJournalRecord {
    let mut record = record(
        req_id,
        "codex",
        "route_a_external_cli",
        status,
        updated_at_ms,
    );
    record.codex_session_id = Some("codex-session-uuid".to_string());
    record.codex_session_scope_key = Some("scope-key".to_string());
    record.codex_session_updated_at_ms = Some(updated_at_ms);
    record
}

fn sidecar(session_id: &str, task_id: &str, last_seen_at_ms: u128) -> CliSidecarSessionRecord {
    let mut session = CliSidecarSessionRecord::managed_conpty(
        session_id,
        task_id,
        "codex",
        "route_a_external_cli",
        Some("D:/demo".to_string()),
        Some("npipe://elon/sidecar-1".to_string()),
        Some(100),
        Some(200),
        last_seen_at_ms,
    );
    session.last_seen_at_ms = last_seen_at_ms;
    session
}

fn pipe_sidecar(session_id: &str, task_id: &str, last_seen_at_ms: u128) -> CliSidecarSessionRecord {
    let mut session = CliSidecarSessionRecord::managed_pipe_json(
        session_id,
        task_id,
        "codex",
        "route_a_external_cli",
        Some("D:/demo".to_string()),
        Some("D:/state/output.jsonl".to_string()),
        Some(100),
        Some(200),
        last_seen_at_ms,
    );
    session.last_seen_at_ms = last_seen_at_ms;
    session
}

fn record(
    req_id: &str,
    cli_name: &str,
    route: &str,
    status: &str,
    updated_at_ms: u128,
) -> TaskJournalRecord {
    TaskJournalRecord {
        req_id: req_id.to_string(),
        cli_name: cli_name.to_string(),
        route: Some(route.to_string()),
        run_handle_id: Some(req_id.to_string()),
        cwd: Some("D:/demo".to_string()),
        runtime_permission: Some("project_write".to_string()),
        os_pid: None,
        process_started_at_ms: None,
        process_identity: None,
        codex_session_id: None,
        codex_session_scope_key: None,
        codex_session_updated_at_ms: None,
        status: status.to_string(),
        phase: "reasoning".to_string(),
        current_command: None,
        last_progress_ms: None,
        heartbeat_at_ms: None,
        timeout_policy: None,
        dispatch: None,
        started_at_ms: 1,
        updated_at_ms,
        cancel_requested_at_ms: None,
        cancel_intent: None,
    }
}
