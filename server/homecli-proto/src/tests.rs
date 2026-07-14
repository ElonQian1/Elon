use super::{
    AgentToServer, CliCompletionEnvelope, CliCompletionProducerIdentity, CliProjectContext,
    CliWorkspaceStatus, ServerToAgent, PROTO_VERSION,
};

#[test]
fn old_project_document_snapshot_without_metadata_still_decodes() {
    let json = r##"{
        "type": "project_documents_read",
        "req_id": "req-docs",
        "snapshot": {
            "workspace_path": "C:/repo",
            "documents": [{
                "path": "AGENTS.md",
                "title": "Rules",
                "content": "# Rules",
                "truncated": false,
                "byte_len": 7
            }]
        }
    }"##;

    let decoded: AgentToServer = serde_json::from_str(json).expect("decode old document snapshot");
    match decoded {
        AgentToServer::ProjectDocumentsRead { snapshot, .. } => {
            assert_eq!(snapshot.documents.len(), 1);
            assert!(snapshot.documents[0].metadata.role.is_empty());
        }
        other => panic!("expected project document snapshot, got {other:?}"),
    }
}

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

fn completion_fixture() -> CliCompletionEnvelope {
    CliCompletionEnvelope {
        event_id: "completion-node-a-req-1".to_string(),
        req_id: "req-1".to_string(),
        cli: "codex".to_string(),
        origin: "local_offline".to_string(),
        producer_identity: Some(CliCompletionProducerIdentity {
            owner_user_id: "owner-a".to_string(),
            agent_id: "node-a".to_string(),
            install_id: "install-a".to_string(),
        }),
        project_context: Some(CliProjectContext {
            project_id: "project-a".to_string(),
            conversation_id: "conversation-a".to_string(),
            runtime_permission: Some("project_write".to_string()),
        }),
        channel_id: Some("channel-ai".to_string()),
        prompt: Some("离线继续检查这个项目".to_string()),
        final_output: "任务已完成。".to_string(),
        exit_ok: true,
        error: None,
        session_id: Some("codex-session-a".to_string()),
        prompt_tokens: Some(120),
        cached_input_tokens: Some(20),
        completion_tokens: Some(30),
        reasoning_tokens: Some(10),
        total_tokens: Some(150),
        model: Some("gpt-5.4".to_string()),
        workspace_status: Some(CliWorkspaceStatus {
            base_workspace_path: Some("D:\\repo".to_string()),
            active_workspace_path: "D:\\repo-worktree".to_string(),
            isolated: true,
            branch: Some("ai/session/project-a/conversation-a".to_string()),
            prepare_status: "prepared".to_string(),
            merge_status: Some("merged".to_string()),
            merge_message: Some("fast-forwarded main".to_string()),
        }),
        created_at_ms: 1_783_920_000_000,
    }
}

#[test]
fn durable_completion_protocol_version_is_advertised() {
    assert!(PROTO_VERSION >= 7);
}

#[test]
fn cli_completion_replay_round_trips_all_terminal_fields() {
    let message = AgentToServer::CliCompletionReplay {
        completion: completion_fixture(),
    };
    let json = serde_json::to_string(&message).expect("encode completion replay");
    let decoded: AgentToServer = serde_json::from_str(&json).expect("decode completion replay");

    match decoded {
        AgentToServer::CliCompletionReplay { completion } => {
            assert_eq!(completion.event_id, "completion-node-a-req-1");
            assert_eq!(completion.req_id, "req-1");
            assert_eq!(completion.cli, "codex");
            assert_eq!(completion.origin, "local_offline");
            assert_eq!(completion.channel_id.as_deref(), Some("channel-ai"));
            assert_eq!(completion.prompt.as_deref(), Some("离线继续检查这个项目"));
            assert_eq!(completion.final_output, "任务已完成。");
            assert!(completion.exit_ok);
            assert_eq!(completion.prompt_tokens, Some(120));
            assert_eq!(completion.cached_input_tokens, Some(20));
            assert_eq!(completion.completion_tokens, Some(30));
            assert_eq!(completion.reasoning_tokens, Some(10));
            assert_eq!(completion.total_tokens, Some(150));
            assert_eq!(completion.model.as_deref(), Some("gpt-5.4"));
            assert_eq!(
                completion
                    .project_context
                    .as_ref()
                    .map(|context| context.conversation_id.as_str()),
                Some("conversation-a")
            );
            assert_eq!(
                completion
                    .workspace_status
                    .as_ref()
                    .and_then(|status| status.merge_status.as_deref()),
                Some("merged")
            );
        }
        other => panic!("expected completion replay, got {other:?}"),
    }
}

#[test]
fn completion_replay_is_not_routed_through_transient_req_waiters() {
    let message = AgentToServer::CliCompletionReplay {
        completion: completion_fixture(),
    };

    assert_eq!(message.req_id(), None);
    assert_eq!(message.task_id(), None);
    assert!(message.is_final_req_msg());
}

#[test]
fn completion_prompt_and_channel_default_to_none_for_cloud_replays() {
    let json = r#"{
        "type": "cli_completion_replay",
        "completion": {
            "event_id": "completion-old-shape",
            "req_id": "req-old-shape",
            "cli": "codex",
            "origin": "cloud_dispatch",
            "final_output": "done",
            "exit_ok": true,
            "created_at_ms": 1783920000000
        }
    }"#;

    let decoded: AgentToServer = serde_json::from_str(json).expect("decode prompt-less replay");
    match decoded {
        AgentToServer::CliCompletionReplay { completion } => {
            assert_eq!(completion.prompt, None);
            assert_eq!(completion.channel_id, None);
            assert!(completion.project_context.is_none());
        }
        other => panic!("expected completion replay, got {other:?}"),
    }
}

#[test]
fn completion_ack_optional_status_fields_default_safely() {
    let json = r#"{
        "type": "cli_completion_ack",
        "event_id": "completion-node-a-req-1",
        "req_id": "req-1",
        "accepted": true
    }"#;

    let decoded: ServerToAgent = serde_json::from_str(json).expect("decode completion ACK");
    match decoded {
        ServerToAgent::CliCompletionAck {
            event_id,
            req_id,
            accepted,
            deduplicated,
            retryable,
            error,
        } => {
            assert_eq!(event_id, "completion-node-a-req-1");
            assert_eq!(req_id, "req-1");
            assert!(accepted);
            assert!(!deduplicated);
            assert!(!retryable);
            assert_eq!(error, None);
        }
        other => panic!("expected completion ACK, got {other:?}"),
    }
}

#[test]
fn durable_completion_can_rebuild_legacy_cli_done() {
    let completion = completion_fixture();
    match completion.to_cli_done() {
        AgentToServer::CliDone {
            req_id,
            exit_ok,
            prompt_tokens,
            completion_tokens,
            session_id,
            workspace_status,
            ..
        } => {
            assert_eq!(req_id, "req-1");
            assert!(exit_ok);
            assert_eq!(prompt_tokens, Some(120));
            assert_eq!(completion_tokens, Some(30));
            assert_eq!(session_id.as_deref(), Some("codex-session-a"));
            assert_eq!(
                workspace_status
                    .as_ref()
                    .and_then(|status| status.merge_status.as_deref()),
                Some("merged")
            );
        }
        other => panic!("expected rebuilt cli_done, got {other:?}"),
    }
}
