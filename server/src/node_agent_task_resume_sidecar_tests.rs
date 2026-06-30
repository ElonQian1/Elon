use crate::{
    node_agent_cli_sidecar::{now_ms, CliSidecarSessionRecord},
    node_agent_task_approval_snapshot::TaskApprovalJournalTracker,
    node_agent_task_journal::TaskJournalRecord,
    node_agent_task_resume::{
        task_attach_state_with_sidecar, task_resume_contract_with_journal_approvals,
    },
};
use serde_json::json;

#[test]
fn sidecar_contract_can_reattach_and_recover_pending_approval() {
    let mut tracker = TaskApprovalJournalTracker::default();
    tracker.observe_event(
        1,
        &json!({
            "type": "tool_approval_required",
            "approval_id": "tap_restart_1",
            "tool": "apply_patch"
        }),
    );
    let approvals = tracker.finish();
    let running = record("running");
    let attach = task_attach_state_with_sidecar(Some(&running), None, Some(sidecar("sidecar-1")));
    let resume = task_resume_contract_with_journal_approvals(&attach, &approvals);
    let resume_json = serde_json::to_value(resume).expect("resume should serialize");

    assert_eq!(resume_json["status"], "sidecar_recoverable");
    assert_eq!(resume_json["can_reconnect"], true);
    assert_eq!(resume_json["can_cancel"], true);
    assert_eq!(resume_json["can_stream_live_output"], true);
    assert_eq!(resume_json["can_approve_tools"], true);
    assert_eq!(resume_json["next_action"], "attach_sidecar");
    assert_eq!(
        resume_json["strategy"]["kind"],
        "managed_pty_conpty_sidecar_attach"
    );
    assert_eq!(resume_json["tty_reattach"]["supported"], true);
    assert_eq!(
        resume_json["tty_reattach"]["mode"],
        "managed_pty_conpty_sidecar_reattach"
    );
    assert_eq!(
        resume_json["tool_approval_recovery"]["status"],
        "sidecar_waiter_recoverable"
    );
    assert_eq!(
        resume_json["tool_approval_recovery"]["pending_after_restart_action"],
        "approve_or_deny_sidecar_waiter"
    );
    assert_eq!(
        resume_json["tool_approval_recovery"]["journal_pending_approval_ids"],
        json!(["tap_restart_1"])
    );
    assert_eq!(resume_json["sidecar_session"]["session_id"], "sidecar-1");
}

fn record(status: &str) -> TaskJournalRecord {
    TaskJournalRecord {
        req_id: "task-1".to_string(),
        cli_name: "codex".to_string(),
        route: Some("route_a_external_cli".to_string()),
        run_handle_id: Some("task-1".to_string()),
        cwd: Some("D:/demo".to_string()),
        runtime_permission: Some("project_write".to_string()),
        os_pid: Some(4242),
        process_started_at_ms: Some(1),
        codex_session_id: None,
        codex_session_scope_key: None,
        codex_session_updated_at_ms: None,
        status: status.to_string(),
        started_at_ms: 1,
        updated_at_ms: 2,
        cancel_requested_at_ms: None,
    }
}

fn sidecar(session_id: &str) -> CliSidecarSessionRecord {
    CliSidecarSessionRecord::managed_conpty(
        session_id,
        "task-1",
        "codex",
        "route_a_external_cli",
        Some("D:/demo".to_string()),
        Some("npipe://elon/sidecar-1".to_string()),
        Some(100),
        Some(200),
        now_ms(),
    )
}
