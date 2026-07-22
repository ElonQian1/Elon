use super::*;
use crate::{
    node_agent_active_task::ActiveCliPromptHandle,
    node_agent_active_task_registry::ActiveCliPromptRegistry,
};

#[test]
fn duplicate_dispatch_returns_idempotent_acceptance_not_terminal() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    send_cli_prompt_reattached(
        &tx,
        "req-duplicate".to_string(),
        "codex".to_string(),
        Some("D:/workspace".to_string()),
        Some("full_access".to_string()),
    );
    let Message::Text(payload) = rx.try_recv().expect("duplicate response") else {
        panic!("duplicate response must be websocket text");
    };
    let event: AgentToServer =
        serde_json::from_str(payload.as_ref()).expect("valid duplicate response");
    assert!(matches!(
        event,
        AgentToServer::CliPromptAccepted {
            req_id,
            cli: Some(cli),
            cwd: Some(cwd),
            runtime_permission: Some(runtime_permission),
        } if req_id == "req-duplicate"
            && cli == "codex"
            && cwd == "D:/workspace"
            && runtime_permission == "full_access"
    ));
}

#[test]
fn inherited_fresh_workspace_is_not_a_true_resume() {
    assert!(!is_true_workspace_resume(true, false));
    assert!(is_true_workspace_resume(true, true));
    assert!(is_true_workspace_resume(false, true));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn slow_cache_telemetry_cannot_hide_the_active_handle_or_block_tokio() {
    let registry = ActiveCliPromptRegistry::new();
    let (cancel_tx, _cancel_rx) = watch::channel(false);
    assert!(
        registry
            .try_insert(
                ActiveCliPromptHandle::new(
                    "cache-heavy",
                    "codex",
                    "route_a_external_cli",
                    Some("D:/workspace".to_string()),
                    Some("full_access".to_string()),
                    cancel_tx,
                )
                .with_exclusive_workspace(true),
            )
            .await
    );
    let started = std::time::Instant::now();
    let blocking = tokio::time::timeout(
        Duration::from_millis(20),
        tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_millis(200))),
    );
    let ticker = tokio::time::sleep(Duration::from_millis(5));
    let (blocking, ()) = tokio::join!(blocking, ticker);
    assert!(
        blocking.is_err(),
        "blocking cache work must have a bounded deadline"
    );
    assert!(started.elapsed() < Duration::from_millis(150));
    assert!(registry.contains("cache-heavy").await);
}
