use super::*;

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-node-runs-test-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("store should open"), path)
}

#[test]
fn start_is_idempotent_and_finish_records_settlement() {
    let (store, path) = temp_store();
    let consumer = store
        .create_user("node-run-consumer@example.com", "secret1", None, None)
        .unwrap();
    let initial_provider = store
        .create_user(
            "node-run-initial-provider@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let provider = store
        .create_user("node-run-provider@example.com", "secret1", None, None)
        .unwrap();

    let first = store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:req-1",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&initial_provider.id),
            node_id: "node-a",
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .unwrap();
    let second = store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:req-1",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&initial_provider.id),
            node_id: "node-a",
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(first.status, "started");
    let fetched = store
        .get_node_compute_run_by_compute_call_id("pc_agent_cli:req-1")
        .unwrap()
        .unwrap();
    assert_eq!(fetched.id, first.id);

    let finished = store
        .finish_node_compute_run(
            "pc_agent_cli:req-1",
            NodeComputeRunFinish {
                provider_user_id: Some(&provider.id),
                status: "settled",
                prompt_tokens: 10,
                completion_tokens: 20,
                billed_cost_rmb_fen: 30,
                provider_earned_fen: 24,
                settlement_status: Some("billed"),
                error_message: None,
            },
        )
        .unwrap()
        .unwrap();

    assert_eq!(finished.status, "settled");
    assert_eq!(finished.prompt_tokens, 10);
    assert_eq!(finished.completion_tokens, 20);
    assert_eq!(finished.billed_cost_rmb_fen, 30);
    assert_eq!(finished.provider_earned_fen, 24);
    assert_eq!(
        finished.provider_user_id.as_deref(),
        Some(provider.id.as_str())
    );
    assert!(finished.finished_at.is_some());

    let scores = store.node_quality_scores().unwrap();
    let score = scores.get("node-a").unwrap();
    assert_eq!(score.total_runs, 1);
    assert_eq!(score.successful_runs, 1);
    assert_eq!(score.success_rate_x1000, 1000);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn startup_interrupts_started_pc_agent_runs() {
    let (store, path) = temp_store();
    let consumer = store
        .create_user("node-run-restart@example.com", "secret1", None, None)
        .unwrap();
    let provider = store
        .create_user(
            "node-run-restart-provider@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();

    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:restart-1",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&provider.id),
            node_id: "node-a",
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .unwrap();
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "node_llm:still-running",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&provider.id),
            node_id: "node-a",
            model_id: Some("gpt"),
            feature: "node_llm",
            usage_mode: "node_llm",
            route_reason: None,
        })
        .unwrap();

    assert_eq!(store.mark_interrupted_started_pc_agent_runs().unwrap(), 1);
    let pc_run = store
        .get_node_compute_run_by_compute_call_id("pc_agent_cli:restart-1")
        .unwrap()
        .unwrap();
    assert_eq!(pc_run.status, "failed");
    assert!(pc_run.finished_at.is_some());
    assert_eq!(
        pc_run.error_message.as_deref(),
        Some("server restarted before PC CLI terminal event")
    );
    let other_run = store
        .get_node_compute_run_by_compute_call_id("node_llm:still-running")
        .unwrap()
        .unwrap();
    assert_eq!(other_run.status, "started");

    drop(store);
    let _ = std::fs::remove_file(path);
}
