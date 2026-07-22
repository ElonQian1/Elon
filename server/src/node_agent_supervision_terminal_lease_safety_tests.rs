use super::*;

#[test]
fn terminal_parent_cannot_release_while_resume_descendant_is_active() {
    let active = Path::new("C:/workspace/conversation-worktrees/elon-self/root");
    let task = record("resume-child", active);
    let contract = descendant("root-task");
    assert!(candidate_blocks_release(&task, Some(&contract), "root-task", active).unwrap());
    assert!(candidate_blocks_release(
        &record("same-root-other-worktree", Path::new("C:/other")),
        Some(&contract),
        "root-task",
        active,
    )
    .unwrap());
}

#[test]
fn unknown_or_wrong_root_workspace_occupancy_fails_closed() {
    let active = Path::new("C:/workspace/conversation-worktrees/elon-self/root");
    let task = record("unknown", active);
    assert!(candidate_blocks_release(&task, None, "root-task", active).is_err());
    assert!(candidate_blocks_release(
        &task,
        Some(&descendant("foreign-root")),
        "root-task",
        active,
    )
    .is_err());
}

#[test]
fn unrelated_durable_root_skips_expensive_contract_lookup() {
    let foreign = serde_json::json!({ "root_task_id": "foreign-root" });
    assert!(!candidate_requires_contract_lookup(
        Some(&foreign),
        false,
        "root-task",
    ));
    assert!(candidate_requires_contract_lookup(
        Some(&foreign),
        true,
        "root-task",
    ));
    assert!(candidate_requires_contract_lookup(None, false, "root-task",));
}

#[test]
fn terminal_sidecar_metadata_does_not_outlive_execution_ownership() {
    assert!(!sidecar_metadata_blocks_release(
        "terminal-parent",
        "terminal-parent",
        Some("done"),
        true,
        true,
    ));
    assert!(!sidecar_metadata_blocks_release(
        "terminal-parent",
        "terminal-sibling",
        Some("failed"),
        true,
        true,
    ));
    assert!(sidecar_metadata_blocks_release(
        "terminal-parent",
        "running-descendant",
        Some("running"),
        true,
        true,
    ));
    assert!(sidecar_metadata_blocks_release(
        "terminal-parent",
        "unknown-live-task",
        None,
        true,
        true,
    ));
}

fn descendant(root: &str) -> SupervisionContract {
    SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: "resume_original".to_string(),
        parent_task_id: Some("old-parent".to_string()),
        root_task_id: Some(root.to_string()),
        acceptance_criteria: vec![],
        improvement_policy: "after_task_only".to_string(),
    }
}

fn record(task_id: &str, workspace: &Path) -> LocalTaskRecord {
    LocalTaskRecord {
        task_id: task_id.to_string(),
        owner_user_id: "owner".to_string(),
        agent_id: "agent".to_string(),
        install_id: "install".to_string(),
        project_id: "elon-self".to_string(),
        channel_id: None,
        conversation_id: "root".to_string(),
        workspace_path: workspace.to_string_lossy().to_string(),
        prompt: "prompt".to_string(),
        cli: "codex".to_string(),
        runtime_permission: "full_access".to_string(),
        execution_origin: "local_offline".to_string(),
        billing_source: "own_codex".to_string(),
        status: "running".to_string(),
        error: None,
        final_reply: None,
        model: None,
        codex_session_id: None,
        input_tokens: None,
        cached_input_tokens: None,
        output_tokens: None,
        reasoning_tokens: None,
        total_tokens: None,
        workspace_status: None,
        sync_state: "local_only".to_string(),
        completion_event_id: None,
        started_at_ms: 1,
        finished_at_ms: None,
        server_ack_at_ms: None,
    }
}
