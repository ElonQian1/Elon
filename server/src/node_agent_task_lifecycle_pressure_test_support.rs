use crate::node_agent_task_journal::TaskJournalRecord;

pub(crate) fn journal_record_for_workspace(
    req_id: &str,
    cwd: &std::path::Path,
    updated_at_ms: u128,
) -> TaskJournalRecord {
    TaskJournalRecord {
        req_id: req_id.to_string(),
        cli_name: "server-runtime".to_string(),
        route: Some("route_c_server_runtime".to_string()),
        run_handle_id: Some(req_id.to_string()),
        cwd: Some(cwd.to_string_lossy().to_string()),
        runtime_permission: Some("project_write".to_string()),
        os_pid: None,
        process_started_at_ms: None,
        process_identity: None,
        codex_session_id: None,
        codex_session_scope_key: None,
        codex_session_updated_at_ms: None,
        status: "canceled".to_string(),
        phase: "reasoning".to_string(),
        current_command: None,
        last_progress_ms: None,
        heartbeat_at_ms: None,
        timeout_policy: None,
        dispatch: None,
        started_at_ms: updated_at_ms.saturating_sub(5),
        updated_at_ms,
        cancel_requested_at_ms: Some(updated_at_ms.saturating_sub(1)),
        cancel_intent: None,
    }
}
