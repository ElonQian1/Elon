use super::*;
use crate::store::{NodeComputeRunFinish, NodeComputeRunStart, UpdateNodeComputeSharingPolicy};

fn fixture() -> (Store, String, String, String) {
    let path = std::env::temp_dir().join(format!(
        "elon-node-sharing-health-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = store
        .create_user("health-owner@example.com", "secret1", None, None)
        .unwrap();
    let consumer = store
        .create_user("health-consumer@example.com", "secret1", None, None)
        .unwrap();
    let node_id = "health-node".to_string();
    store
        .create_node_credential(
            &node_id,
            "secret-hash",
            &owner.id,
            Some("health node"),
            None,
            Some("health-install"),
        )
        .unwrap();
    store
        .update_node_compute_sharing_policy(
            &owner.id,
            &node_id,
            UpdateNodeComputeSharingPolicy {
                enabled: true,
                allowed_model_ids: vec!["qwen".into()],
                max_concurrent_runs: 4,
                daily_token_limit: 10_000,
            },
        )
        .unwrap();
    (store, owner.id, consumer.id, node_id)
}

fn start<'a>(
    call_id: &'a str,
    consumer: &'a str,
    owner: &'a str,
    node_id: &'a str,
) -> NodeComputeRunStart<'a> {
    NodeComputeRunStart {
        compute_call_id: call_id,
        consumer_user_id: consumer,
        provider_user_id: Some(owner),
        node_id,
        model_id: Some("qwen"),
        feature: "node_llm",
        usage_mode: "server_node_llm",
        route_reason: Some("runtime_health_test"),
    }
}

#[test]
fn owner_health_reports_overrun_failure_and_expired_run_without_counting_self_use() {
    let (store, owner, consumer, node_id) = fixture();
    let initial = store
        .node_compute_sharing_runtime_health(&node_id, &owner)
        .unwrap();
    assert_eq!(initial.status, "healthy");

    store
        .claim_shared_node_compute_run_with_budget(
            start("node_llm:health-overrun", &consumer, &owner, &node_id),
            10,
        )
        .unwrap();
    store
        .finish_node_compute_run(
            "node_llm:health-overrun",
            NodeComputeRunFinish {
                provider_user_id: Some(&owner),
                status: "settled",
                prompt_tokens: 8,
                completion_tokens: 7,
                billed_cost_rmb_fen: 1,
                provider_earned_fen: 1,
                settlement_status: Some("settled"),
                error_message: None,
            },
        )
        .unwrap();
    store
        .claim_shared_node_compute_run_with_budget(
            start("node_llm:health-failed", &consumer, &owner, &node_id),
            20,
        )
        .unwrap();
    store
        .finish_node_compute_run(
            "node_llm:health-failed",
            NodeComputeRunFinish {
                provider_user_id: Some(&owner),
                status: "failed",
                prompt_tokens: 0,
                completion_tokens: 0,
                billed_cost_rmb_fen: 0,
                provider_earned_fen: 0,
                settlement_status: Some("released_error"),
                error_message: Some("fixture failure"),
            },
        )
        .unwrap();
    store
        .claim_shared_node_compute_run_with_budget(
            start("node_llm:health-expired", &consumer, &owner, &node_id),
            30,
        )
        .unwrap();
    store
        .start_node_compute_run(start("node_llm:health-self", &owner, &owner, &node_id))
        .unwrap();
    {
        let conn = store.conn.lock().unwrap();
        conn.execute(
            "UPDATE node_compute_runs SET updated_at='2000-01-01T00:00:00Z'
              WHERE compute_call_id IN ('node_llm:health-expired', 'node_llm:health-self')",
            [],
        )
        .unwrap();
    }

    let health = store
        .node_compute_sharing_runtime_health(&node_id, &owner)
        .unwrap();
    assert_eq!(health.status, "critical");
    assert_eq!(health.completed_runs_24h, 2);
    assert_eq!(health.failed_runs_24h, 1);
    assert_eq!(health.budget_overrun_runs_24h, 1);
    assert_eq!(health.budget_overrun_tokens_24h, 5);
    assert_eq!(health.expired_active_runs, 1);
    assert_eq!(
        health.attention_codes,
        vec![
            "token_budget_overrun",
            "expired_active_run",
            "recent_execution_failure"
        ]
    );
}

#[test]
fn runtime_health_is_owner_only() {
    let (store, _owner, consumer, node_id) = fixture();
    let error = store
        .node_compute_sharing_runtime_health(&node_id, &consumer)
        .unwrap_err();
    assert!(error.to_string().contains("不属于当前用户"));
}

#[test]
fn recent_failure_without_critical_signal_is_warning() {
    let (store, owner, consumer, node_id) = fixture();
    store
        .claim_shared_node_compute_run_with_budget(
            start("node_llm:health-warning", &consumer, &owner, &node_id),
            20,
        )
        .unwrap();
    store
        .finish_node_compute_run(
            "node_llm:health-warning",
            NodeComputeRunFinish {
                provider_user_id: Some(&owner),
                status: "failed",
                prompt_tokens: 0,
                completion_tokens: 0,
                billed_cost_rmb_fen: 0,
                provider_earned_fen: 0,
                settlement_status: Some("released_error"),
                error_message: Some("fixture failure"),
            },
        )
        .unwrap();

    let health = store
        .node_compute_sharing_runtime_health(&node_id, &owner)
        .unwrap();
    assert_eq!(health.status, "warning");
    assert_eq!(health.attention_codes, vec!["recent_execution_failure"]);
}

#[test]
fn usage_received_is_not_reported_as_terminal_or_expired_execution() {
    let (store, owner, consumer, node_id) = fixture();
    store
        .claim_shared_node_compute_run_with_budget(
            start(
                "node_llm:health-usage-received",
                &consumer,
                &owner,
                &node_id,
            ),
            20,
        )
        .unwrap();
    assert!(store
        .mark_server_node_llm_usage_received("node_llm:health-usage-received", Some(&owner), 4, 3,)
        .unwrap());
    {
        let conn = store.conn.lock().unwrap();
        conn.execute(
            "UPDATE node_compute_runs SET updated_at='2000-01-01T00:00:00Z'
              WHERE compute_call_id='node_llm:health-usage-received'",
            [],
        )
        .unwrap();
    }

    let health = store
        .node_compute_sharing_runtime_health(&node_id, &owner)
        .unwrap();
    assert_eq!(health.status, "healthy");
    assert_eq!(health.completed_runs_24h, 0);
    assert_eq!(health.expired_active_runs, 0);
    assert!(health.attention_codes.is_empty());
}
