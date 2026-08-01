use super::*;
use crate::store::NodeComputeRunFinish;

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon-node-compute-sharing-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

fn setup() -> (Store, String, String, String) {
    let store = temp_store();
    let owner = store
        .create_user("sharing-owner@example.com", "secret1", None, None)
        .unwrap();
    let consumer = store
        .create_user("sharing-consumer@example.com", "secret1", None, None)
        .unwrap();
    let node_id = "node-shared".to_string();
    store
        .create_node_credential(
            &node_id,
            "secret-hash",
            &owner.id,
            Some("shared node"),
            None,
            Some("install-shared"),
        )
        .unwrap();
    (store, owner.id, consumer.id, node_id)
}

fn start<'a>(
    call_id: &'a str,
    consumer: &'a str,
    owner: &'a str,
    node_id: &'a str,
    model_id: &'a str,
) -> NodeComputeRunStart<'a> {
    NodeComputeRunStart {
        compute_call_id: call_id,
        consumer_user_id: consumer,
        provider_user_id: Some(owner),
        node_id,
        model_id: Some(model_id),
        feature: "node_llm",
        usage_mode: "server_node_llm",
        route_reason: Some("shared_policy_test"),
    }
}

#[test]
fn sharing_is_disabled_until_owner_explicitly_selects_models() {
    let (store, owner, consumer, node_id) = setup();
    let status = store
        .node_compute_sharing_status(&node_id, &owner, Some("qwen"))
        .unwrap();
    assert!(!status.available);
    assert_eq!(status.availability, "sharing_disabled");
    assert!(store
        .claim_shared_node_compute_run(start(
            "node_llm:disabled",
            &consumer,
            &owner,
            &node_id,
            "qwen",
        ))
        .is_err());

    let error = store
        .update_node_compute_sharing_policy(
            &owner,
            &node_id,
            UpdateNodeComputeSharingPolicy {
                enabled: true,
                allowed_model_ids: vec![],
                max_concurrent_runs: 1,
                daily_token_limit: 0,
            },
        )
        .unwrap_err();
    assert!(error.to_string().contains("至少选择一个"));
}

#[test]
fn admission_is_model_scoped_idempotent_and_concurrency_bounded() {
    let (store, owner, consumer, node_id) = setup();
    store
        .update_node_compute_sharing_policy(
            &owner,
            &node_id,
            UpdateNodeComputeSharingPolicy {
                enabled: true,
                allowed_model_ids: vec!["qwen".into()],
                max_concurrent_runs: 1,
                daily_token_limit: 10_000,
            },
        )
        .unwrap();

    let first = store
        .claim_shared_node_compute_run(start("node_llm:first", &consumer, &owner, &node_id, "qwen"))
        .unwrap();
    let replay = store
        .claim_shared_node_compute_run(start("node_llm:first", &consumer, &owner, &node_id, "qwen"))
        .unwrap();
    assert_eq!(first.id, replay.id);

    let blocked = store
        .claim_shared_node_compute_run(start(
            "node_llm:second",
            &consumer,
            &owner,
            &node_id,
            "qwen",
        ))
        .unwrap_err();
    assert!(blocked.to_string().contains("concurrency_limit_reached"));
    let model_blocked = store
        .claim_shared_node_compute_run(start(
            "node_llm:other-model",
            &consumer,
            &owner,
            &node_id,
            "llama",
        ))
        .unwrap_err();
    assert!(model_blocked.to_string().contains("model_not_allowed"));
}

#[test]
fn only_node_owner_can_change_supply_policy() {
    let (store, _owner, consumer, node_id) = setup();
    let error = store
        .update_node_compute_sharing_policy(
            &consumer,
            &node_id,
            UpdateNodeComputeSharingPolicy {
                enabled: false,
                allowed_model_ids: vec![],
                max_concurrent_runs: 1,
                daily_token_limit: 0,
            },
        )
        .unwrap_err();
    assert!(error.to_string().contains("不属于当前用户"));
}

#[test]
fn completed_shared_usage_closes_the_daily_budget() {
    let (store, owner, consumer, node_id) = setup();
    store
        .update_node_compute_sharing_policy(
            &owner,
            &node_id,
            UpdateNodeComputeSharingPolicy {
                enabled: true,
                allowed_model_ids: vec!["qwen".into()],
                max_concurrent_runs: 1,
                daily_token_limit: 10,
            },
        )
        .unwrap();
    store
        .claim_shared_node_compute_run(start(
            "node_llm:daily-first",
            &consumer,
            &owner,
            &node_id,
            "qwen",
        ))
        .unwrap();
    store
        .finish_node_compute_run(
            "node_llm:daily-first",
            NodeComputeRunFinish {
                provider_user_id: Some(&owner),
                status: "settled",
                prompt_tokens: 4,
                completion_tokens: 6,
                billed_cost_rmb_fen: 1,
                provider_earned_fen: 1,
                settlement_status: Some("settled"),
                error_message: None,
            },
        )
        .unwrap();

    let error = store
        .claim_shared_node_compute_run(start(
            "node_llm:daily-second",
            &consumer,
            &owner,
            &node_id,
            "qwen",
        ))
        .unwrap_err();
    assert!(error.to_string().contains("daily_token_limit_reached"));
}

#[test]
fn disabling_supply_keeps_exact_replay_but_rejects_new_work() {
    let (store, owner, consumer, node_id) = setup();
    store
        .update_node_compute_sharing_policy(
            &owner,
            &node_id,
            UpdateNodeComputeSharingPolicy {
                enabled: true,
                allowed_model_ids: vec!["qwen".into()],
                max_concurrent_runs: 2,
                daily_token_limit: 0,
            },
        )
        .unwrap();
    let first = store
        .claim_shared_node_compute_run(start(
            "node_llm:disable-replay",
            &consumer,
            &owner,
            &node_id,
            "qwen",
        ))
        .unwrap();
    store
        .update_node_compute_sharing_policy(
            &owner,
            &node_id,
            UpdateNodeComputeSharingPolicy {
                enabled: false,
                allowed_model_ids: vec!["qwen".into()],
                max_concurrent_runs: 2,
                daily_token_limit: 0,
            },
        )
        .unwrap();

    let replay = store
        .claim_shared_node_compute_run(start(
            "node_llm:disable-replay",
            &consumer,
            &owner,
            &node_id,
            "qwen",
        ))
        .unwrap();
    assert_eq!(first.id, replay.id);
    assert!(store
        .claim_shared_node_compute_run(start(
            "node_llm:disable-new",
            &consumer,
            &owner,
            &node_id,
            "qwen",
        ))
        .is_err());

    let rebound = store
        .claim_shared_node_compute_run(start(
            "node_llm:disable-replay",
            &consumer,
            &owner,
            &node_id,
            "llama",
        ))
        .unwrap_err();
    assert!(rebound.to_string().contains("不能绑定到不同"));
}
