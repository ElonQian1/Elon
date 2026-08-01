use super::*;

fn fixture(email: &str) -> (Store, std::path::PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "elon-node-run-lease-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("store should open");
    let user = store
        .create_user(email, "secret1", None, None)
        .expect("user should be created");
    (store, path, user.id)
}

fn start_run<'a>(
    call_id: &'a str,
    user_id: &'a str,
    usage_mode: &'a str,
) -> NodeComputeRunStart<'a> {
    NodeComputeRunStart {
        compute_call_id: call_id,
        consumer_user_id: user_id,
        provider_user_id: Some(user_id),
        node_id: "node-a",
        model_id: Some("qwen"),
        feature: "node_llm",
        usage_mode,
        route_reason: None,
    }
}

#[test]
fn terminal_usage_is_frozen_before_settlement_and_final_state_is_immutable() {
    let (store, path, user_id) = fixture("node-usage-received@example.com");
    store
        .start_node_compute_run(start_run(
            "node_llm:usage-received",
            &user_id,
            "server_node_llm",
        ))
        .unwrap();

    assert!(store
        .mark_server_node_llm_usage_received("node_llm:usage-received", Some(&user_id), 8, 5,)
        .unwrap());
    let pending = store
        .get_node_compute_run_by_compute_call_id("node_llm:usage-received")
        .unwrap()
        .unwrap();
    assert_eq!(pending.status, "usage_received");
    assert_eq!(pending.prompt_tokens, 8);
    assert_eq!(pending.completion_tokens, 5);
    assert_eq!(pending.settlement_status.as_deref(), Some("pending"));
    assert!(!store
        .heartbeat_started_server_node_llm_run("node_llm:usage-received")
        .unwrap());
    assert!(store.node_quality_scores().unwrap().is_empty());

    let settled = store
        .finish_node_compute_run(
            "node_llm:usage-received",
            NodeComputeRunFinish {
                provider_user_id: Some(&user_id),
                status: "settled",
                prompt_tokens: 8,
                completion_tokens: 5,
                billed_cost_rmb_fen: 3,
                provider_earned_fen: 2,
                settlement_status: Some("settled"),
                error_message: None,
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(settled.status, "settled");

    let late = store
        .finish_node_compute_run(
            "node_llm:usage-received",
            NodeComputeRunFinish {
                provider_user_id: None,
                status: "failed",
                prompt_tokens: 0,
                completion_tokens: 0,
                billed_cost_rmb_fen: 0,
                provider_earned_fen: 0,
                settlement_status: Some("released_error"),
                error_message: Some("late overwrite"),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(late.status, "settled");
    assert_eq!(late.prompt_tokens, 8);
    assert_eq!(late.settlement_status.as_deref(), Some("settled"));

    store
        .start_node_compute_run(start_run(
            "pc_agent_cli:verification-pending",
            &user_id,
            "pc_agent_cli",
        ))
        .unwrap();
    store
        .finish_node_compute_run(
            "pc_agent_cli:verification-pending",
            NodeComputeRunFinish {
                provider_user_id: None,
                status: "verification_pending",
                prompt_tokens: 0,
                completion_tokens: 0,
                billed_cost_rmb_fen: 0,
                provider_earned_fen: 0,
                settlement_status: Some("usage_verification_pending"),
                error_message: None,
            },
        )
        .unwrap();
    let verified = store
        .finish_node_compute_run(
            "pc_agent_cli:verification-pending",
            NodeComputeRunFinish {
                provider_user_id: Some(&user_id),
                status: "settled",
                prompt_tokens: 2,
                completion_tokens: 1,
                billed_cost_rmb_fen: 1,
                provider_earned_fen: 1,
                settlement_status: Some("settled"),
                error_message: None,
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(verified.status, "settled");

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn expired_execution_reconcile_is_scoped_atomic_and_idempotent() {
    let (store, path, user_id) = fixture("node-expired-run@example.com");
    for (call_id, usage_mode) in [
        ("node_llm:expired", "server_node_llm"),
        ("node_llm:fresh", "server_node_llm"),
        ("node_llm:usage-received", "server_node_llm"),
        ("pc_agent_cli:expired", "pc_agent_cli"),
    ] {
        store
            .start_node_compute_run(start_run(call_id, &user_id, usage_mode))
            .unwrap();
    }
    assert!(store
        .mark_server_node_llm_usage_received("node_llm:usage-received", Some(&user_id), 3, 2,)
        .unwrap());
    {
        let conn = store.conn.lock().unwrap();
        conn.execute(
            "UPDATE node_compute_runs SET updated_at='2000-01-01T00:00:00Z'
              WHERE compute_call_id IN (
                'node_llm:expired',
                'node_llm:usage-received',
                'pc_agent_cli:expired'
              )",
            [],
        )
        .unwrap();
    }

    let expired = store.mark_expired_started_server_node_llm_runs().unwrap();
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].compute_call_id, "node_llm:expired");
    let reconciled = store
        .get_node_compute_run_by_compute_call_id("node_llm:expired")
        .unwrap()
        .unwrap();
    assert_eq!(reconciled.status, "failed");
    assert_eq!(
        reconciled.settlement_status.as_deref(),
        Some("expired_released")
    );
    assert_eq!(
        reconciled.error_message.as_deref(),
        Some("node LLM execution lease expired before terminal event")
    );
    assert_eq!(run_status(&store, "node_llm:fresh"), "started");
    assert_eq!(
        run_status(&store, "node_llm:usage-received"),
        "usage_received"
    );
    assert_eq!(run_status(&store, "pc_agent_cli:expired"), "started");
    assert!(store
        .mark_expired_started_server_node_llm_runs()
        .unwrap()
        .is_empty());

    let late = store
        .finish_node_compute_run(
            "node_llm:expired",
            NodeComputeRunFinish {
                provider_user_id: Some(&user_id),
                status: "settled",
                prompt_tokens: 100,
                completion_tokens: 100,
                billed_cost_rmb_fen: 10,
                provider_earned_fen: 8,
                settlement_status: Some("settled"),
                error_message: None,
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(late.status, "failed");
    assert_eq!(late.settlement_status.as_deref(), Some("expired_released"));

    drop(store);
    let _ = std::fs::remove_file(path);
}

fn run_status(store: &Store, call_id: &str) -> String {
    store
        .get_node_compute_run_by_compute_call_id(call_id)
        .unwrap()
        .unwrap()
        .status
}
