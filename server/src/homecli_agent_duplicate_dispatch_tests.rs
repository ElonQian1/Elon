use super::*;

struct ReconnectAgent {
    manager: Arc<AgentManager>,
    cmd_rx: mpsc::UnboundedReceiver<ServerToAgent>,
    stale_rx: mpsc::UnboundedReceiver<AgentToServer>,
    pending: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
    cli_pending_ids: Arc<Mutex<HashSet<String>>>,
    ping_acks: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
}

async fn reconnect_agent() -> ReconnectAgent {
    let manager = Arc::new(AgentManager::new());
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (session_shutdown, _) = watch::channel(false);
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let cli_pending_ids = Arc::new(Mutex::new(HashSet::new()));
    let approval_acks = Arc::new(Mutex::new(HashMap::new()));
    let ping_acks = Arc::new(Mutex::new(HashMap::new()));
    let (stale_tx, stale_rx) = mpsc::unbounded_channel();
    pending.lock().await.insert("req".to_string(), stale_tx);
    cli_pending_ids.lock().await.insert("req".to_string());
    manager.agents.write().await.insert(
        "agent".to_string(),
        AgentEntry {
            session_id: "reconnected-session".to_string(),
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
            connected_at: 2,
            cmd_tx,
            pending: pending.clone(),
            cli_pending_ids: cli_pending_ids.clone(),
            approval_acks,
            ping_acks: ping_acks.clone(),
            session_shutdown,
        },
    );
    ReconnectAgent {
        manager,
        cmd_rx,
        stale_rx,
        pending,
        cli_pending_ids,
        ping_acks,
    }
}

fn completion() -> homecli_proto::CliCompletionEnvelope {
    homecli_proto::CliCompletionEnvelope {
        event_id: "event-req".to_string(),
        req_id: "req".to_string(),
        cli: "codex".to_string(),
        origin: "cloud_dispatch".to_string(),
        producer_identity: Some(homecli_proto::CliCompletionProducerIdentity {
            owner_user_id: "owner-test".to_string(),
            agent_id: "agent".to_string(),
            install_id: "install-test".to_string(),
        }),
        project_context: None,
        channel_id: None,
        prompt: None,
        final_output: "original result".to_string(),
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

#[tokio::test]
async fn accepted_loss_reconnect_redispatch_waits_for_original_durable_completion() {
    let ReconnectAgent {
        manager,
        mut cmd_rx,
        mut stale_rx,
        pending,
        cli_pending_ids,
        ping_acks,
    } = reconnect_agent().await;

    // The first CliPromptAccepted disappeared with the socket. Keep its caller
    // recoverable until the exact req_id is redispatched on the new session.
    manager
        .recover_session_pending_until(
            "agent",
            &pending,
            &cli_pending_ids,
            "accepted frame lost",
            std::time::Instant::now() + Duration::from_secs(30),
        )
        .await;
    assert_eq!(manager.recovering_cli_count().await, 1);

    let manager_for_dispatch = manager.clone();
    let dispatch_task = tokio::spawn(async move {
        manager_for_dispatch
            .dispatch_cli_prompt_with_context_control_id_and_credential_binding(
                "req".to_string(),
                "agent",
                "codex".to_string(),
                Vec::new(),
                None,
                None,
                None,
                false,
                None,
                "continue original work".to_string(),
            )
            .await
    });
    let nonce = match cmd_rx.recv().await {
        Some(ServerToAgent::Ping { nonce }) => nonce.expect("probe ping has nonce"),
        other => panic!("expected reconnect probe ping, got {other:?}"),
    };
    ping_acks
        .lock()
        .await
        .remove(&nonce)
        .expect("probe ping waiter")
        .send(())
        .expect("probe receiver remains live");
    assert!(matches!(
        cmd_rx.recv().await,
        Some(ServerToAgent::CliPrompt { req_id, .. }) if req_id == "req"
    ));
    let mut dispatch = dispatch_task
        .await
        .expect("reconnect dispatch join")
        .expect("reconnect dispatch succeeds");
    assert_eq!(manager.recovering_cli_count().await, 0);

    // An already-running duplicate is an idempotent acceptance, never a false
    // terminal. The fresh receiver remains registered for durable completion.
    pending
        .lock()
        .await
        .get("req")
        .cloned()
        .expect("fresh active receiver")
        .send(AgentToServer::CliPromptAccepted {
            req_id: "req".to_string(),
            cli: Some("codex".to_string()),
            cwd: None,
            runtime_permission: None,
        })
        .expect("duplicate acceptance reaches fresh receiver");
    assert!(matches!(
        dispatch.rx.recv().await,
        Some(AgentToServer::CliPromptAccepted { req_id, .. }) if req_id == "req"
    ));

    assert!(
        manager
            .deliver_accepted_cli_replay("agent", &completion())
            .await
    );
    assert!(matches!(
        dispatch.rx.recv().await,
        Some(AgentToServer::CliChunk { req_id, text })
            if req_id == "req" && text == "original result"
    ));
    assert!(matches!(
        dispatch.rx.recv().await,
        Some(AgentToServer::CliDone {
            req_id,
            exit_ok: true,
            error: None,
            ..
        }) if req_id == "req"
    ));
    assert!(stale_rx.try_recv().is_err());
}

#[tokio::test]
async fn completed_cloud_dispatch_does_not_emit_a_late_deadline_cancel() {
    let ReconnectAgent {
        manager,
        mut cmd_rx,
        pending,
        cli_pending_ids,
        ping_acks,
        ..
    } = reconnect_agent().await;
    let deadline = (chrono::Utc::now() + chrono::Duration::milliseconds(400))
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
    let manager_for_dispatch = manager.clone();
    let dispatch_task = tokio::spawn(async move {
        manager_for_dispatch
            .dispatch_cli_prompt_with_context_control_id_and_credential_binding(
                "req".to_string(),
                "agent",
                "codex".to_string(),
                Vec::new(),
                None,
                None,
                None,
                true,
                Some(deadline),
                "finish before deadline".to_string(),
            )
            .await
    });
    let nonce = match cmd_rx.recv().await {
        Some(ServerToAgent::Ping { nonce }) => nonce.expect("probe ping has nonce"),
        other => panic!("expected deadline probe ping, got {other:?}"),
    };
    ping_acks
        .lock()
        .await
        .remove(&nonce)
        .expect("probe ping waiter")
        .send(())
        .expect("probe receiver remains live");
    assert!(matches!(
        cmd_rx.recv().await,
        Some(ServerToAgent::CliPrompt { req_id, .. }) if req_id == "req"
    ));
    let mut dispatch = dispatch_task
        .await
        .expect("deadline dispatch join")
        .expect("deadline dispatch succeeds");

    assert!(
        route_req_message_to_pending(&pending, &cli_pending_ids, completion().to_cli_done()).await
    );
    assert!(matches!(
        dispatch.rx.recv().await,
        Some(AgentToServer::CliDone {
            req_id,
            exit_ok: true,
            ..
        }) if req_id == "req"
    ));
    assert!(!pending.lock().await.contains_key("req"));
    assert!(!cli_pending_ids.lock().await.contains("req"));
    assert!(
        tokio::time::timeout(Duration::from_millis(700), cmd_rx.recv())
            .await
            .is_err(),
        "completed request must not receive a stale deadline Cancel"
    );
}
