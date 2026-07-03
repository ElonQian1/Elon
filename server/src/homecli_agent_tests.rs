use super::*;

struct TestApprovalAgent {
    manager: Arc<AgentManager>,
    cmd_rx: mpsc::UnboundedReceiver<ServerToAgent>,
    pending_rx: mpsc::UnboundedReceiver<AgentToServer>,
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
    let ping_acks: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let (pending_tx, pending_rx) = mpsc::unbounded_channel();
    pending.lock().await.insert("req".to_string(), pending_tx);
    manager.agents.write().await.insert(
        "agent".to_string(),
        AgentEntry {
            session_id: "session".to_string(),
            agent_id: "agent".to_string(),
            version: "test".to_string(),
            device_name: None,
            hardware: None,
            storage: None,
            dev_runtime: None,
            allowed_clis: Vec::new(),
            allowed_cwds: Vec::new(),
            connected_at: 0,
            cmd_tx,
            pending,
            approval_acks: approval_acks.clone(),
            ping_acks: ping_acks.clone(),
            session_shutdown,
        },
    );
    TestApprovalAgent {
        manager,
        cmd_rx,
        pending_rx,
        approval_acks,
        ping_acks,
        session_shutdown_rx,
    }
}

#[test]
fn cli_prompt_cancel_handle_sends_cancel_for_req_id() {
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
    let handle = CliPromptCancelHandle {
        req_id: "req-123".to_string(),
        cmd_tx,
    };

    assert!(handle.cancel());
    match cmd_rx.try_recv() {
        Ok(ServerToAgent::Cancel { task_id }) => assert_eq!(task_id, "req-123"),
        other => panic!("expected cancel message, got {other:?}"),
    }
}

#[tokio::test]
async fn cli_prompt_dispatch_probes_agent_before_sending_prompt() {
    let TestApprovalAgent {
        manager,
        mut cmd_rx,
        ping_acks,
        ..
    } = registered_approval_agent().await;

    let manager_for_dispatch = manager.clone();
    let dispatch_task = tokio::spawn(async move {
        manager_for_dispatch
            .dispatch_cli_prompt_with_context_control(
                "agent",
                "codex".to_string(),
                Vec::new(),
                None,
                None,
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
            ..
        }) => {
            assert_eq!(cli, "codex");
            assert_eq!(prompt, "hello");
            assert!(project_context.is_none());
        }
        other => panic!("expected CLI prompt after probe ACK, got {other:?}"),
    }

    let dispatch = tokio::time::timeout(Duration::from_secs(1), dispatch_task)
        .await
        .expect("dispatch should finish after probe ACK")
        .expect("dispatch join should succeed")
        .expect("dispatch should succeed");
    assert!(!dispatch.req_id.is_empty());
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
