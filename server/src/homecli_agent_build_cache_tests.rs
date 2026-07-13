use super::*;

struct BuildCacheTestAgent {
    manager: Arc<AgentManager>,
    cmd_rx: mpsc::UnboundedReceiver<ServerToAgent>,
}

async fn registered_build_cache_agent() -> BuildCacheTestAgent {
    let manager = Arc::new(AgentManager::new());
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (session_shutdown, _session_shutdown_rx) = watch::channel(false);
    manager.agents.write().await.insert(
        "agent".to_string(),
        AgentEntry {
            session_id: "build-cache-session".to_string(),
            agent_id: "agent".to_string(),
            version: "build-cache-test".to_string(),
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
            pending: Arc::new(Mutex::new(HashMap::new())),
            cli_pending_ids: Arc::new(Mutex::new(HashSet::new())),
            approval_acks: Arc::new(Mutex::new(HashMap::new())),
            ping_acks: Arc::new(Mutex::new(HashMap::new())),
            session_shutdown,
        },
    );
    BuildCacheTestAgent { manager, cmd_rx }
}

#[tokio::test]
async fn project_prompt_rejects_agent_without_build_cache_capability() {
    let BuildCacheTestAgent { manager, .. } = registered_build_cache_agent().await;
    manager
        .agents
        .write()
        .await
        .get_mut("agent")
        .expect("registered agent")
        .capabilities
        .clear();

    let error = match manager
        .dispatch_cli_prompt_with_context_control(
            "agent",
            "codex".to_string(),
            Vec::new(),
            Some("D:/project".to_string()),
            Some(project_context()),
            "build".to_string(),
        )
        .await
    {
        Ok(_) => panic!("legacy agent must be rejected"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("版本过旧"));
}

#[tokio::test]
async fn project_exec_rejects_agent_without_build_cache_capability() {
    let BuildCacheTestAgent {
        manager,
        mut cmd_rx,
        ..
    } = registered_build_cache_agent().await;
    manager
        .agents
        .write()
        .await
        .get_mut("agent")
        .expect("registered agent")
        .capabilities
        .clear();

    let error = match manager
        .dispatch_with_project_context(
            "agent",
            "cargo".to_string(),
            vec!["check".to_string()],
            "D:/project".to_string(),
            Vec::new(),
            Some(project_context()),
        )
        .await
    {
        Ok(_) => panic!("legacy agent must be rejected"),
        Err(error) => error,
    };

    let message = error.to_string();
    assert!(message.contains("版本过旧"));
    assert!(message.contains(homecli_proto::CAP_PROJECT_BUILD_CACHE_V1));
    assert!(matches!(
        cmd_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn project_exec_allows_agent_with_build_cache_capability() {
    let BuildCacheTestAgent {
        manager,
        mut cmd_rx,
        ..
    } = registered_build_cache_agent().await;

    let expected_context = project_context();
    let (task_id, _rx) = manager
        .dispatch_with_project_context(
            "agent",
            "cargo".to_string(),
            vec!["check".to_string()],
            "D:/project".to_string(),
            Vec::new(),
            Some(expected_context.clone()),
        )
        .await
        .expect("capable agent must accept project exec");

    match cmd_rx.try_recv() {
        Ok(ServerToAgent::Exec {
            task_id: sent_task_id,
            project_context,
            ..
        }) => {
            assert_eq!(sent_task_id, task_id);
            let actual = project_context.expect("project context must be retained");
            assert_eq!(actual.project_id, expected_context.project_id);
            assert_eq!(actual.conversation_id, expected_context.conversation_id);
            assert_eq!(
                actual.runtime_permission,
                expected_context.runtime_permission
            );
        }
        other => panic!("expected project exec, got {other:?}"),
    }
}

#[tokio::test]
async fn non_project_exec_allows_agent_without_build_cache_capability() {
    let BuildCacheTestAgent {
        manager,
        mut cmd_rx,
        ..
    } = registered_build_cache_agent().await;
    manager
        .agents
        .write()
        .await
        .get_mut("agent")
        .expect("registered agent")
        .capabilities
        .clear();

    let (task_id, _rx) = manager
        .dispatch(
            "agent",
            "powershell".to_string(),
            vec!["-NoProfile".to_string()],
            "D:/scratch".to_string(),
            Vec::new(),
        )
        .await
        .expect("non-project task must remain compatible with a legacy agent");

    match cmd_rx.try_recv() {
        Ok(ServerToAgent::Exec {
            task_id: sent_task_id,
            project_context,
            ..
        }) => {
            assert_eq!(sent_task_id, task_id);
            assert!(project_context.is_none());
        }
        other => panic!("expected non-project exec, got {other:?}"),
    }
}

#[tokio::test]
async fn project_workspace_mutations_reject_agent_without_build_cache_capability() {
    let BuildCacheTestAgent { manager, .. } = registered_build_cache_agent().await;
    manager
        .agents
        .write()
        .await
        .get_mut("agent")
        .expect("registered agent")
        .capabilities
        .clear();

    let provision_error = manager
        .dispatch_project_workspace_provision(
            "agent",
            "project".into(),
            "user".into(),
            "Project".into(),
            "blank".into(),
            None,
            None,
        )
        .await
        .expect_err("legacy agent must not provision a workspace");
    let storage_error = manager
        .dispatch_project_storage_repo_prepare(
            "agent",
            "project".into(),
            "user".into(),
            "Project".into(),
            None,
            None,
            false,
        )
        .await
        .expect_err("legacy agent must not prepare project storage");
    let cleanup_error = manager
        .dispatch_project_workspace_cleanup(
            "agent",
            "project".into(),
            "D:/managed/project/repo".into(),
        )
        .await
        .expect_err("legacy agent must not cleanup a workspace");

    for error in [provision_error, storage_error, cleanup_error] {
        assert!(error.to_string().contains("版本过旧"));
    }
}

#[test]
fn legacy_register_without_capabilities_defaults_to_empty() {
    let register: AgentToServer = serde_json::from_str(
        r#"{
            "type": "register",
            "agent_id": "legacy-agent",
            "version": "0.3.68",
            "proto_version": 4
        }"#,
    )
    .expect("legacy register frame must remain decodable");

    match register {
        AgentToServer::Register {
            proto_version,
            capabilities,
            ..
        } => {
            assert_eq!(proto_version, 4);
            assert!(capabilities.is_empty());
        }
        other => panic!("expected register frame, got {other:?}"),
    }
}

fn project_context() -> homecli_proto::CliProjectContext {
    homecli_proto::CliProjectContext {
        project_id: "project-1".to_string(),
        conversation_id: "conversation-1".to_string(),
        runtime_permission: Some("project_write".to_string()),
    }
}
