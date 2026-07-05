use super::{AgentToServer, ServerToAgent};

#[test]
fn old_tool_approval_decision_without_dispatch_id_still_decodes() {
    let json = r#"{
        "type": "tool_approval_decision",
        "req_id": "req",
        "approval_id": "tap_1_1",
        "decision": "approve"
    }"#;

    let msg: ServerToAgent = serde_json::from_str(json).expect("decode old decision message");
    match msg {
        ServerToAgent::ToolApprovalDecision {
            req_id,
            approval_id,
            dispatch_id,
            decision,
        } => {
            assert_eq!(req_id, "req");
            assert_eq!(approval_id, "tap_1_1");
            assert_eq!(dispatch_id, "");
            assert_eq!(decision, "approve");
        }
        other => panic!("expected tool approval decision, got {other:?}"),
    }
}

#[test]
fn tool_approval_ack_is_not_routed_as_req_stream_message() {
    let msg = AgentToServer::ToolApprovalDecisionAck {
        req_id: "req".to_string(),
        approval_id: "tap_1_1".to_string(),
        dispatch_id: "dispatch".to_string(),
        accepted: true,
    };

    assert_eq!(msg.req_id(), None);
    assert_eq!(msg.task_id(), None);
}

#[test]
fn cli_prompt_accepted_keeps_req_stream_open() {
    let msg = AgentToServer::CliPromptAccepted {
        req_id: "req".to_string(),
        cli: Some("codex".to_string()),
        cwd: Some("D:\\work".to_string()),
        runtime_permission: Some("danger_full_access".to_string()),
    };

    assert_eq!(msg.req_id(), Some("req"));
    assert_eq!(msg.task_id(), None);
    assert!(!msg.is_final_req_msg());
}

#[test]
fn cli_task_journal_snapshot_is_routed_as_final_req_message() {
    let msg = AgentToServer::CliTaskJournalSnapshot {
        req_id: "req".to_string(),
        task_id: "pc-task".to_string(),
        ok: true,
        snapshot: Some(serde_json::json!({"resume": {"status": "detached"}})),
        error: None,
    };

    assert_eq!(msg.req_id(), Some("req"));
    assert_eq!(msg.task_id(), None);
    assert!(msg.is_final_req_msg());
}

#[test]
fn old_cli_done_without_session_id_still_decodes() {
    let json = r#"{
        "type": "cli_done",
        "req_id": "req",
        "exit_ok": true
    }"#;

    let msg: AgentToServer = serde_json::from_str(json).expect("decode old cli_done");
    match msg {
        AgentToServer::CliDone { session_id, .. } => {
            assert_eq!(session_id, None);
        }
        other => panic!("expected cli_done, got {other:?}"),
    }
}

#[test]
fn cli_done_decodes_session_id() {
    let json = r#"{
        "type": "cli_done",
        "req_id": "req",
        "exit_ok": true,
        "session_id": "019f2125-41f3-7e01-b676-ef7a0e5ee392"
    }"#;

    let msg: AgentToServer = serde_json::from_str(json).expect("decode cli_done");
    match msg {
        AgentToServer::CliDone { session_id, .. } => {
            assert_eq!(
                session_id.as_deref(),
                Some("019f2125-41f3-7e01-b676-ef7a0e5ee392")
            );
        }
        other => panic!("expected cli_done, got {other:?}"),
    }
}
