use super::{task_attach_state, task_resume_contract, task_resume_contract_with_journal_approvals};
use crate::{
    node_agent_active_task::ActiveCliPromptView,
    node_agent_task_approval_snapshot::TaskApprovalJournalTracker,
    node_agent_task_journal::TaskJournalRecord, node_agent_tool_approval::PendingToolApprovalView,
};
use serde_json::json;

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
        updated_at_ms: 2,
        cancel_requested_at_ms: None,
        cancel_intent: None,
    }
}

fn codex_record(status: &str) -> TaskJournalRecord {
    let mut record = record(status);
    record.codex_session_id = Some("session-uuid".to_string());
    record.codex_session_scope_key = Some("scope-a".to_string());
    record.codex_session_updated_at_ms = Some(9);
    record
}

fn active_handle() -> ActiveCliPromptView {
    ActiveCliPromptView {
        req_id: "task-1".to_string(),
        run_handle_id: "task-1".to_string(),
        cli_name: "server-runtime".to_string(),
        route: "route_c_server_runtime".to_string(),
        cwd: Some("D:/demo".to_string()),
        runtime_permission: Some("project_write".to_string()),
        requires_cloud_control: false,
        started_at_ms: 1,
        last_heartbeat_ms: 2,
        control_lease_expires_at_ms: 47_000,
        os_pid: None,
        control_handle_live: true,
        pending_approvals: vec![PendingToolApprovalView {
            approval_id: "tap_1_1".to_string(),
            registered_at_ms: 3,
            expires_at_ms: 30_003,
        }],
    }
}

#[test]
fn live_contract_is_honest_about_stream_replay() {
    let running = record("running");
    let attach = task_attach_state(Some(&running), Some(active_handle()));
    let resume = task_resume_contract(&attach);

    assert_eq!(resume.status, "live");
    assert!(resume.can_reconnect);
    assert!(resume.can_cancel);
    assert!(!resume.can_stream_live_output);
    assert!(resume.can_replay_journal_events);
    assert!(resume.can_approve_tools);
    assert!(!resume.can_resume_codex_session);
    assert!(resume.codex_session.is_none());
    assert_eq!(resume.active_approval_ids, vec!["tap_1_1"]);
    assert_eq!(resume.tool_approval_recovery.status, "active_waiter");
    assert!(resume.tool_approval_recovery.can_approve_now);
    assert_eq!(resume.tool_approval_recovery.journal_pending_count, 0);
    assert_eq!(
        resume.tool_approval_recovery.active_approval_ids,
        vec!["tap_1_1"]
    );
    assert_eq!(
        resume
            .run_handle
            .as_ref()
            .map(|handle| handle.route.as_str()),
        Some("route_c_server_runtime")
    );
    assert_eq!(resume.next_action, "wait_or_cancel");
    assert_eq!(resume.strategy.kind, "control_handle_reconnect");
    assert_eq!(resume.tty_reattach.status, "not_supported");
    assert!(!resume.tty_reattach.supported);
    assert_eq!(resume.tty_reattach.mode, "no_original_cli_tty_reattach");
}

#[test]
fn detached_contract_requires_snapshot_continue() {
    let running = record("running");
    let attach = task_attach_state(Some(&running), None);
    let resume = task_resume_contract(&attach);

    assert_eq!(attach.status, "detached");
    assert!(!resume.can_reconnect);
    assert!(!resume.can_cancel);
    assert!(!resume.can_approve_tools);
    assert!(resume.active_approval_ids.is_empty());
    assert_eq!(resume.tool_approval_recovery.status, "lost_after_restart");
    assert!(!resume.tool_approval_recovery.can_approve_now);
    assert_eq!(resume.tool_approval_recovery.journal_pending_count, 0);
    assert_eq!(
        resume.tool_approval_recovery.pending_after_restart_action,
        "continue_from_snapshot"
    );
    assert!(resume
        .tool_approval_recovery
        .reason
        .contains("历史审批卡必须失效"));
    assert_eq!(resume.next_action, "continue_from_snapshot");
    assert_eq!(resume.strategy.kind, "snapshot_continue");
    assert!(resume.strategy.requires_new_task);
    assert_eq!(
        resume.tty_reattach.fallback,
        "journal_replay_snapshot_continue_and_codex_session_resume"
    );
}

#[test]
fn detached_contract_exposes_journal_pending_approval_ids_without_claiming_approval() {
    let mut tracker = TaskApprovalJournalTracker::default();
    tracker.observe_event(
        1,
        &json!({
            "type": "tool_approval_required",
            "approval_id": "tap_restart_1",
            "tool": "write_file"
        }),
    );
    let approvals = tracker.finish();
    let running = record("running");
    let attach = task_attach_state(Some(&running), None);
    let resume = task_resume_contract_with_journal_approvals(&attach, &approvals);

    assert_eq!(resume.status, "detached");
    assert!(!resume.can_approve_tools);
    assert!(resume.active_approval_ids.is_empty());
    assert_eq!(resume.tool_approval_recovery.status, "lost_after_restart");
    assert!(!resume.tool_approval_recovery.can_approve_now);
    assert_eq!(
        resume.tool_approval_recovery.journal_pending_approval_ids,
        vec!["tap_restart_1"]
    );
    assert_eq!(resume.tool_approval_recovery.journal_pending_count, 1);
    assert_eq!(
        resume.tool_approval_recovery.pending_after_restart_action,
        "continue_from_snapshot"
    );
}

#[test]
fn cancel_requested_without_live_handle_cannot_be_canceled_again() {
    let canceling = record("cancel_requested");
    let attach = task_attach_state(Some(&canceling), None);
    let resume = task_resume_contract(&attach);

    assert_eq!(attach.status, "detached");
    assert!(!resume.can_reconnect);
    assert!(!resume.can_cancel);
    assert!(!resume.can_approve_tools);
    assert_eq!(resume.tool_approval_recovery.status, "lost_after_restart");
    assert_eq!(resume.next_action, "continue_from_snapshot");
    assert!(resume
        .limitations
        .iter()
        .any(|item| item.contains("节点重启后不能重新绑定原进程控制句柄")));
}

#[test]
fn live_codex_task_exposes_control_and_session_continuity() {
    let running = codex_record("running");
    let attach = task_attach_state(Some(&running), Some(active_handle()));
    let resume = task_resume_contract(&attach);

    assert_eq!(resume.status, "live");
    assert!(resume.can_reconnect);
    assert!(resume.can_cancel);
    assert!(resume.can_resume_codex_session);
    assert_eq!(resume.next_action, "wait_or_cancel");
    assert_eq!(
        resume
            .codex_session
            .as_ref()
            .map(|session| session.id.as_str()),
        Some("session-uuid")
    );
}

#[test]
fn codex_session_is_exposed_for_snapshot_continue() {
    let running = codex_record("running");
    let resume = task_resume_contract(&task_attach_state(Some(&running), None));

    assert_eq!(resume.status, "detached");
    assert!(resume.can_resume_codex_session);
    assert_eq!(
        resume
            .codex_session
            .as_ref()
            .map(|session| session.id.as_str()),
        Some("session-uuid")
    );
    assert_eq!(
        resume
            .codex_session
            .as_ref()
            .map(|session| session.scope_key.as_str()),
        Some("scope-a")
    );
}

#[test]
fn terminal_and_missing_contracts_do_not_claim_reconnect() {
    let finished = record("finished");
    let terminal = task_resume_contract(&task_attach_state(Some(&finished), None));
    let missing = task_resume_contract(&task_attach_state(None, None));

    assert_eq!(terminal.status, "terminal");
    assert_eq!(terminal.next_action, "continue_from_snapshot");
    assert_eq!(
        terminal.tool_approval_recovery.status,
        "closed_by_terminal_task"
    );
    assert_eq!(missing.status, "missing");
    assert_eq!(missing.strategy.kind, "cloud_snapshot_only");
    assert!(!missing.strategy.uses_local_journal);
    assert_eq!(missing.tty_reattach.status, "not_supported");
    assert_eq!(missing.tool_approval_recovery.status, "unavailable");
}
