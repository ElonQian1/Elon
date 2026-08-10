use super::*;

struct TestApprovalAgent {
    manager: Arc<AgentManager>,
    cmd_rx: mpsc::UnboundedReceiver<ServerToAgent>,
    pending_rx: mpsc::UnboundedReceiver<AgentToServer>,
    cli_pending_ids: Arc<Mutex<HashSet<String>>>,
    approval_acks: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
    ping_acks: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
    session_shutdown_rx: watch::Receiver<bool>,
}

async fn registered_approval_agent() -> TestApprovalAgent {
    let manager = Arc::new(AgentManager::new());
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (session_shutdown, session_shutdown_rx) = watch::channel(false);
    let pending: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let approval_acks: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let cli_pending_ids = Arc::new(Mutex::new(HashSet::new()));
    let ping_acks: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let (pending_tx, pending_rx) = mpsc::unbounded_channel();
    pending.lock().await.insert("req".to_string(), pending_tx);
    manager.agents.write().await.insert(
        "agent".to_string(),
        AgentEntry {
            process_session: AgentProcessSessionKey::new("agent", "session"),
            agent_id: "agent".to_string(),
            version: "test".to_string(),
            proto_version: homecli_proto::PROTO_VERSION,
            capabilities: vec![homecli_proto::CAP_PROJECT_BUILD_CACHE_V1.to_string()],
            device_name: None,
            hardware: None,
            storage: None,
            dev_runtime: None,
            lifecycle: None,
            allowed_clis: Vec::new(),
            allowed_cwds: Vec::new(),
            connected_at: 0,
            cmd_tx,
            pending,
            cli_pending_ids: cli_pending_ids.clone(),
            approval_acks: approval_acks.clone(),
            ping_acks: ping_acks.clone(),
            session_shutdown,
        },
    );
    TestApprovalAgent {
        manager,
        cmd_rx,
        pending_rx,
        cli_pending_ids,
        approval_acks,
        ping_acks,
        session_shutdown_rx,
    }
}

fn completion(req_id: &str, final_output: &str) -> homecli_proto::CliCompletionEnvelope {
    homecli_proto::CliCompletionEnvelope {
        event_id: format!("event-{req_id}"),
        req_id: req_id.to_string(),
        cli: "codex".to_string(),
        origin: "cloud_dispatch".to_string(),
        producer_identity: Some(homecli_proto::CliCompletionProducerIdentity {
            owner_user_id: "owner-test".to_string(),
            agent_id: "agent-test".to_string(),
            install_id: "install-test".to_string(),
        }),
        project_context: None,
        channel_id: None,
        prompt: None,
        final_output: final_output.to_string(),
        exit_ok: true,
        error: None,
        session_id: Some("session-result".to_string()),
        prompt_tokens: Some(10),
        cached_input_tokens: Some(2),
        completion_tokens: Some(5),
        reasoning_tokens: Some(1),
        total_tokens: Some(15),
        model: Some("gpt-test".to_string()),
        workspace_status: None,
        created_at_ms: 1,
    }
}

#[test]
fn cli_prompt_cancel_handle_sends_cancel_for_req_id() {
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
    let handle = CliPromptCancelHandle {
        req_id: "req-123".to_string(),
        cmd_tx,
        process_session: AgentProcessSessionKey::new("agent", "session"),
    };

    assert!(handle.cancel());
    match cmd_rx.try_recv() {
        Ok(ServerToAgent::Cancel { task_id, audit }) => {
            assert_eq!(task_id, "req-123");
            assert_eq!(audit.source.as_deref(), Some("cancel_handle"));
        }
        other => panic!("expected cancel message, got {other:?}"),
    }
}

#[test]
fn legacy_cancel_json_defaults_missing_audit_fields() {
    let message: ServerToAgent =
        serde_json::from_str(r#"{"type":"cancel","task_id":"legacy-task"}"#)
            .expect("legacy cancel must remain compatible");
    match message {
        ServerToAgent::Cancel { task_id, audit } => {
            assert_eq!(task_id, "legacy-task");
            assert_eq!(audit, homecli_proto::CancelRequestAudit::default());
        }
        other => panic!("expected cancel message, got {other:?}"),
    }
}

#[tokio::test]
async fn manager_sends_best_effort_cancel_to_exact_online_agent() {
    let TestApprovalAgent {
        manager,
        mut cmd_rx,
        ..
    } = registered_approval_agent().await;

    assert!(
        manager
            .cancel_cli_prompt_on_agent("agent", "revoked-request")
            .await
    );
    match cmd_rx.try_recv() {
        Ok(ServerToAgent::Cancel { task_id, audit }) => {
            assert_eq!(task_id, "revoked-request");
            assert_eq!(audit.reason.as_deref(), Some("authorization_revoked"));
        }
        other => panic!("expected manager cancel message, got {other:?}"),
    }
    assert!(
        !manager
            .cancel_cli_prompt_on_agent("offline-agent", "revoked-request")
            .await
    );
}

#[tokio::test]
async fn non_project_prompt_allows_legacy_agent_after_dispatch_probe() {
    let TestApprovalAgent {
        manager,
        mut cmd_rx,
        ping_acks,
        ..
    } = registered_approval_agent().await;
    manager
        .agents
        .write()
        .await
        .get_mut("agent")
        .expect("registered agent")
        .capabilities
        .clear();

    let expected_deadline = (chrono::Utc::now() + chrono::Duration::minutes(10)).to_rfc3339();
    let deadline_for_dispatch = expected_deadline.clone();
    let manager_for_dispatch = manager.clone();
    let dispatch_task = tokio::spawn(async move {
        manager_for_dispatch
            .dispatch_cli_prompt_with_context_control_id_and_credential_binding(
                "req-cloud-deadline".to_string(),
                "agent",
                "codex".to_string(),
                Vec::new(),
                None,
                None,
                None,
                true,
                Some(deadline_for_dispatch),
                "hello".to_string(),
            )
            .await
    });

    let nonce = match cmd_rx.recv().await {
        Some(ServerToAgent::Ping { nonce }) => nonce.expect("probe ping has nonce"),
        other => panic!("expected probe ping before CLI prompt, got {other:?}"),
    };
    let ping_ack = ping_acks
        .lock()
        .await
        .remove(&nonce)
        .expect("probe ping waiter should be registered");
    ping_ack.send(()).expect("probe waiter should be live");

    match cmd_rx.recv().await {
        Some(ServerToAgent::CliPrompt {
            cli,
            prompt,
            project_context,
            cloud_control_deadline,
            cloud_control_issued_at,
            cloud_control_ttl_ms,
            ..
        }) => {
            assert_eq!(cli, "codex");
            assert_eq!(prompt, "hello");
            assert!(project_context.is_none());
            assert_eq!(
                cloud_control_deadline.as_deref(),
                Some(expected_deadline.as_str())
            );
            assert!(cloud_control_issued_at.is_some());
            assert!(cloud_control_ttl_ms.is_some_and(|ttl_ms| ttl_ms > 0 && ttl_ms <= 600_000));
        }
        other => panic!("expected CLI prompt after probe ACK, got {other:?}"),
    }

    let dispatch = tokio::time::timeout(Duration::from_secs(1), dispatch_task)
        .await
        .expect("dispatch should finish after probe ACK")
        .expect("dispatch join should succeed")
        .expect("dispatch should succeed");
    assert_eq!(dispatch.req_id, "req-cloud-deadline");
}

#[tokio::test]
async fn cloud_dispatch_sends_best_effort_cancel_at_frozen_server_deadline() {
    let TestApprovalAgent {
        manager,
        mut cmd_rx,
        ping_acks,
        ..
    } = registered_approval_agent().await;
    let deadline = (chrono::Utc::now() + chrono::Duration::seconds(1)).to_rfc3339();
    let manager_for_dispatch = manager.clone();
    let dispatch_task = tokio::spawn(async move {
        manager_for_dispatch
            .dispatch_cli_prompt_with_context_control_id_and_credential_binding(
                "req-server-deadline".to_string(),
                "agent",
                "codex".to_string(),
                Vec::new(),
                None,
                None,
                None,
                true,
                Some(deadline),
                "hello".to_string(),
            )
            .await
    });

    let nonce = match cmd_rx.recv().await {
        Some(ServerToAgent::Ping { nonce }) => nonce.expect("probe ping has nonce"),
        other => panic!("expected probe ping before CLI prompt, got {other:?}"),
    };
    ping_acks
        .lock()
        .await
        .remove(&nonce)
        .expect("probe ping waiter should be registered")
        .send(())
        .expect("probe waiter should be live");
    assert!(matches!(
        cmd_rx.recv().await,
        Some(ServerToAgent::CliPrompt { .. })
    ));
    dispatch_task
        .await
        .expect("dispatch join should succeed")
        .expect("dispatch should succeed");

    match tokio::time::timeout(Duration::from_secs(3), cmd_rx.recv())
        .await
        .expect("server deadline cancel should arrive")
    {
        Some(ServerToAgent::Cancel { task_id, audit }) => {
            assert_eq!(task_id, "req-server-deadline");
            assert_eq!(audit.source.as_deref(), Some("cloud_control_deadline"));
        }
        other => panic!("expected server deadline cancel, got {other:?}"),
    }
}

#[tokio::test]
async fn legacy_protocol_is_rejected_before_cli_dispatch() {
    let TestApprovalAgent {
        manager,
        mut cmd_rx,
        ..
    } = registered_approval_agent().await;
    manager
        .agents
        .write()
        .await
        .get_mut("agent")
        .expect("agent should be registered")
        .proto_version = DURABLE_CLI_COMPLETION_PROTO_VERSION - 1;

    let error = manager
        .dispatch_cli_prompt_with_context_control(
            "agent",
            "codex".to_string(),
            Vec::new(),
            None,
            None,
            "hello".to_string(),
        )
        .await
        .err()
        .expect("legacy protocol must not receive CLI work");

    assert!(error
        .to_string()
        .contains("durable completion protocol v5+"));
    assert!(
        cmd_rx.try_recv().is_err(),
        "legacy protocol must be rejected before ping or prompt dispatch"
    );
}

#[tokio::test]
async fn protocol_v6_is_rejected_before_dispatching_monotonic_cloud_ttl() {
    let TestApprovalAgent { manager, .. } = registered_approval_agent().await;
    manager
        .agents
        .write()
        .await
        .get_mut("agent")
        .unwrap()
        .proto_version = CLOUD_CONTROL_DEADLINE_PROTO_VERSION - 1;

    let error = manager
        .dispatch_cli_prompt_with_context_control_id_and_credential_binding(
            "req-v6".to_string(),
            "agent",
            "codex".to_string(),
            Vec::new(),
            None,
            None,
            None,
            true,
            Some("2030-01-01T00:10:00Z".to_string()),
            "hello".to_string(),
        )
        .await
        .err()
        .expect("protocol v6 must not receive a monotonic cloud TTL");

    assert!(error.to_string().contains("protocol v7+"));
}

#[test]
fn server_dispatch_window_freezes_remaining_ttl_at_issue_time() {
    let issued_at = chrono::DateTime::parse_from_rfc3339("2030-01-01T00:00:07Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let window =
        freeze_cloud_control_dispatch_window_at("2030-01-01T00:00:10Z", issued_at).unwrap();

    assert_eq!(window.ttl_ms, 3_000);
    assert_eq!(window.issued_at, "2030-01-01T00:00:07.000000000Z");
}

#[tokio::test]
async fn controlled_dispatch_without_absolute_deadline_is_rejected_before_ping() {
    let TestApprovalAgent {
        manager,
        mut cmd_rx,
        ..
    } = registered_approval_agent().await;

    let error = manager
        .dispatch_cli_prompt_with_context_control(
            "agent",
            "codex".to_string(),
            Vec::new(),
            None,
            None,
            "hello".to_string(),
        )
        .await
        .err()
        .expect("controlled dispatch without deadline must fail");

    assert!(error
        .to_string()
        .contains("absolute authorization deadline"));
    assert!(cmd_rx.try_recv().is_err());
}

#[test]
fn heartbeat_closes_only_after_consecutive_missed_acks() {
    assert!(!heartbeat::heartbeat_should_close_session(1));
    assert!(!heartbeat::heartbeat_should_close_session(
        heartbeat::AGENT_HEARTBEAT_MAX_MISSED_ACKS - 1
    ));
    assert!(heartbeat::heartbeat_should_close_session(
        heartbeat::AGENT_HEARTBEAT_MAX_MISSED_ACKS
    ));
}

#[tokio::test]
async fn protocol_ping_control_uses_ws_text_and_waits_for_ack() {
    let (control_tx, mut control_rx) = mpsc::unbounded_channel();
    let ping_acks: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let ping_acks_for_task = ping_acks.clone();
    let ping_task = tokio::spawn(async move {
        heartbeat::send_protocol_ping_control(
            "agent",
            &control_tx,
            &ping_acks_for_task,
            Duration::from_secs(1),
        )
        .await
    });

    let nonce = match control_rx.recv().await {
        Some(Message::Text(text)) => match serde_json::from_str::<ServerToAgent>(&text)
            .expect("ping text should deserialize")
        {
            ServerToAgent::Ping { nonce } => nonce.expect("control ping should carry nonce"),
            other => panic!("expected protocol ping, got {other:?}"),
        },
        other => panic!("expected control text ping, got {other:?}"),
    };
    let ack = ping_acks
        .lock()
        .await
        .remove(&nonce)
        .expect("ping waiter should be registered");
    ack.send(()).expect("ping receiver should be live");

    ping_task
        .await
        .expect("ping task should join")
        .expect("ping should complete after ACK");
}

#[tokio::test]
async fn tool_approval_decision_waits_for_matching_ack() {
    let TestApprovalAgent {
        manager,
        mut cmd_rx,
        approval_acks,
        ..
    } = registered_approval_agent().await;

    let manager_for_send = manager.clone();
    let send_task = tokio::spawn(async move {
        manager_for_send
            .send_tool_approval_decision("req", "tap_1_1", "approve")
            .await
    });
    let dispatch_id = match cmd_rx.recv().await {
        Some(ServerToAgent::ToolApprovalDecision {
            req_id,
            approval_id,
            dispatch_id,
            decision,
        }) => {
            assert_eq!(req_id, "req");
            assert_eq!(approval_id, "tap_1_1");
            assert_eq!(decision, "approve");
            dispatch_id
        }
        other => panic!("expected tool approval decision, got {other:?}"),
    };
    let ack_key = tool_approval_ack_key("req", "tap_1_1", &dispatch_id);
    let ack_tx = approval_acks
        .lock()
        .await
        .remove(&ack_key)
        .expect("ack waiter should be registered");
    ack_tx.send(true).expect("ack receiver should be live");

    assert!(send_task.await.unwrap().unwrap());
}

#[tokio::test]
async fn stale_tool_approval_ack_does_not_complete_new_dispatch() {
    let TestApprovalAgent {
        manager,
        mut cmd_rx,
        approval_acks,
        ..
    } = registered_approval_agent().await;

    let manager_for_first = manager.clone();
    let first_send = tokio::spawn(async move {
        manager_for_first
            .send_tool_approval_decision("req", "tap_1_1", "approve")
            .await
    });
    let stale_dispatch_id = match cmd_rx.recv().await {
        Some(ServerToAgent::ToolApprovalDecision { dispatch_id, .. }) => dispatch_id,
        other => panic!("expected first tool approval decision, got {other:?}"),
    };
    let stale_key = tool_approval_ack_key("req", "tap_1_1", &stale_dispatch_id);
    approval_acks
        .lock()
        .await
        .remove(&stale_key)
        .expect("first ack waiter should be registered");

    let manager_for_retry = manager.clone();
    let retry_send = tokio::spawn(async move {
        manager_for_retry
            .send_tool_approval_decision("req", "tap_1_1", "approve")
            .await
    });
    let retry_dispatch_id = match cmd_rx.recv().await {
        Some(ServerToAgent::ToolApprovalDecision { dispatch_id, .. }) => dispatch_id,
        other => panic!("expected retry tool approval decision, got {other:?}"),
    };
    assert_ne!(stale_dispatch_id, retry_dispatch_id);

    let stale_key = tool_approval_ack_key("req", "tap_1_1", &stale_dispatch_id);
    assert!(
        approval_acks.lock().await.remove(&stale_key).is_none(),
        "stale ACK key must not match retry waiter"
    );
    let retry_key = tool_approval_ack_key("req", "tap_1_1", &retry_dispatch_id);
    let retry_ack = approval_acks
        .lock()
        .await
        .remove(&retry_key)
        .expect("retry ack waiter should be registered");
    retry_ack
        .send(true)
        .expect("retry ack receiver should be live");

    assert!(retry_send.await.unwrap().unwrap());
    assert!(first_send.await.unwrap().is_err());
}

#[tokio::test]
async fn close_agent_session_fails_pending_work_and_signals_shutdown() {
    let TestApprovalAgent {
        manager,
        mut pending_rx,
        approval_acks,
        mut session_shutdown_rx,
        ..
    } = registered_approval_agent().await;
    let (ack_tx, ack_rx) = oneshot::channel();
    approval_acks.lock().await.insert("ack".to_string(), ack_tx);

    assert!(
        manager
            .close_agent_session("agent", "test forced close")
            .await
    );
    tokio::time::timeout(Duration::from_secs(1), session_shutdown_rx.changed())
        .await
        .expect("session shutdown signal should arrive")
        .expect("shutdown sender should still be alive");
    assert!(*session_shutdown_rx.borrow());

    match pending_rx.recv().await {
        Some(AgentToServer::CliDone { exit_ok, error, .. }) => {
            assert!(!exit_ok);
            assert_eq!(error.as_deref(), Some("test forced close"));
        }
        other => panic!("expected failed CliDone, got {other:?}"),
    }
    assert!(!ack_rx.await.expect("approval ack should be rejected"));
}

#[tokio::test]
async fn disconnect_recovers_only_cli_pending_and_fails_other_requests() {
    let manager = AgentManager::new();
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let cli_pending_ids = Arc::new(Mutex::new(HashSet::new()));
    let (cli_tx, mut cli_rx) = mpsc::unbounded_channel();
    let (http_tx, mut http_rx) = mpsc::unbounded_channel();
    {
        let mut pending = pending.lock().await;
        let mut cli_ids = cli_pending_ids.lock().await;
        pending.insert("cli-req".to_string(), cli_tx);
        pending.insert("http-req".to_string(), http_tx);
        cli_ids.insert("cli-req".to_string());
    }

    manager
        .recover_session_pending_until(
            "node-a",
            &pending,
            &cli_pending_ids,
            "network lost",
            std::time::Instant::now() + Duration::from_secs(30),
        )
        .await;

    assert_eq!(manager.recovering_cli_count().await, 1);
    assert!(
        cli_rx.try_recv().is_err(),
        "CLI must stay pending during grace"
    );
    match http_rx.recv().await {
        Some(AgentToServer::CliDone {
            req_id,
            exit_ok,
            error,
            ..
        }) => {
            assert_eq!(req_id, "http-req");
            assert!(!exit_ok);
            assert_eq!(error.as_deref(), Some("network lost"));
        }
        other => panic!("expected fail-fast terminal frame, got {other:?}"),
    }
}

#[tokio::test]
async fn accepted_replay_wakes_recovering_cli_receiver_with_output_and_terminal() {
    let manager = AgentManager::new();
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let cli_pending_ids = Arc::new(Mutex::new(HashSet::new()));
    let (tx, mut rx) = mpsc::unbounded_channel();
    {
        let mut pending = pending.lock().await;
        let mut cli_ids = cli_pending_ids.lock().await;
        pending.insert("cli-req".to_string(), tx);
        cli_ids.insert("cli-req".to_string());
    }
    manager
        .recover_session_pending_until(
            "node-a",
            &pending,
            &cli_pending_ids,
            "network lost",
            std::time::Instant::now() + Duration::from_secs(30),
        )
        .await;

    assert!(
        manager
            .deliver_accepted_cli_replay("node-a", &completion("cli-req", "final answer"))
            .await
    );
    assert_eq!(manager.recovering_cli_count().await, 0);
    match rx.recv().await {
        Some(AgentToServer::CliChunk { req_id, text }) => {
            assert_eq!(req_id, "cli-req");
            assert_eq!(text, "final answer");
        }
        other => panic!("expected replayed final output, got {other:?}"),
    }
    match rx.recv().await {
        Some(AgentToServer::CliDone {
            req_id,
            exit_ok,
            session_id,
            ..
        }) => {
            assert_eq!(req_id, "cli-req");
            assert!(exit_ok);
            assert_eq!(session_id.as_deref(), Some("session-result"));
        }
        other => panic!("expected replayed CliDone, got {other:?}"),
    }
}

#[tokio::test]
async fn replay_from_different_node_cannot_steal_recovering_receiver() {
    let manager = AgentManager::new();
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let cli_pending_ids = Arc::new(Mutex::new(HashSet::new()));
    let (tx, mut rx) = mpsc::unbounded_channel();
    pending.lock().await.insert("cli-req".to_string(), tx);
    cli_pending_ids.lock().await.insert("cli-req".to_string());
    manager
        .recover_session_pending_until(
            "node-a",
            &pending,
            &cli_pending_ids,
            "network lost",
            std::time::Instant::now() + Duration::from_secs(30),
        )
        .await;

    assert!(
        !manager
            .deliver_accepted_cli_replay("node-b", &completion("cli-req", "wrong node"))
            .await
    );
    assert_eq!(manager.recovering_cli_count().await, 1);
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn recovery_deadline_eventually_releases_waiting_runner() {
    let manager = AgentManager::new();
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let cli_pending_ids = Arc::new(Mutex::new(HashSet::new()));
    let (tx, mut rx) = mpsc::unbounded_channel();
    pending.lock().await.insert("cli-req".to_string(), tx);
    cli_pending_ids.lock().await.insert("cli-req".to_string());
    let deadline = std::time::Instant::now();
    manager
        .recover_session_pending_until(
            "node-a",
            &pending,
            &cli_pending_ids,
            "network lost",
            deadline,
        )
        .await;

    assert_eq!(manager.expire_recovering_cli_at(deadline).await, 1);
    match rx.recv().await {
        Some(AgentToServer::CliDone { exit_ok, error, .. }) => {
            assert!(!exit_ok);
            assert!(error.unwrap_or_default().contains("短线恢复等待超时"));
        }
        other => panic!("expected recovery timeout CliDone, got {other:?}"),
    }
}

#[tokio::test]
async fn accepted_replay_can_overtake_legacy_done_on_active_session() {
    let TestApprovalAgent {
        manager,
        mut pending_rx,
        cli_pending_ids,
        ..
    } = registered_approval_agent().await;
    cli_pending_ids.lock().await.insert("req".to_string());

    assert!(
        manager
            .deliver_accepted_cli_replay("agent", &completion("req", "durable answer"))
            .await
    );
    assert!(matches!(
        pending_rx.recv().await,
        Some(AgentToServer::CliChunk { .. })
    ));
    assert!(matches!(
        pending_rx.recv().await,
        Some(AgentToServer::CliDone { exit_ok: true, .. })
    ));
}

#[tokio::test]
async fn reconnect_active_receiver_wins_over_stale_recovery_sender() {
    let TestApprovalAgent {
        manager,
        mut pending_rx,
        cli_pending_ids,
        ..
    } = registered_approval_agent().await;
    cli_pending_ids.lock().await.insert("req".to_string());

    let stale_pending = Arc::new(Mutex::new(HashMap::new()));
    let stale_cli_ids = Arc::new(Mutex::new(HashSet::new()));
    let (stale_tx, mut stale_rx) = mpsc::unbounded_channel();
    stale_pending
        .lock()
        .await
        .insert("req".to_string(), stale_tx);
    stale_cli_ids.lock().await.insert("req".to_string());
    manager
        .recover_session_pending_until(
            "agent",
            &stale_pending,
            &stale_cli_ids,
            "previous connection lost",
            std::time::Instant::now() + Duration::from_secs(30),
        )
        .await;
    assert_eq!(manager.recovering_cli_count().await, 1);

    assert!(
        manager
            .deliver_accepted_cli_replay("agent", &completion("req", "current answer"))
            .await
    );
    assert_eq!(manager.recovering_cli_count().await, 0);
    assert!(matches!(
        pending_rx.recv().await,
        Some(AgentToServer::CliChunk { text, .. }) if text == "current answer"
    ));
    assert!(matches!(
        pending_rx.recv().await,
        Some(AgentToServer::CliDone { exit_ok: true, .. })
    ));
    assert!(stale_rx.try_recv().is_err());
}
