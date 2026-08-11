use super::*;

use crate::compute_federation::legacy::LEGACY_LLM_V1_PROJECTION_LIST_SCHEMA;

#[test]
fn empty_usage_keeps_both_raw_lists_and_advertises_the_compatibility_schema() {
    let value = node_usage_response(Vec::new(), Vec::new());

    assert_eq!(value["consuming"], serde_json::json!([]));
    assert_eq!(value["providing"], serde_json::json!([]));
    assert_eq!(
        value["federation_compatibility"]["schema"],
        LEGACY_LLM_V1_PROJECTION_LIST_SCHEMA
    );
    assert_eq!(
        value["federation_compatibility"]["consuming"],
        serde_json::json!([])
    );
    assert_eq!(
        value["federation_compatibility"]["providing"],
        serde_json::json!([])
    );
}

#[test]
fn raw_pc_cli_usage_remains_visible_but_is_not_projected_as_legacy_llm() {
    let pc_run = run("pc-1", "pc_agent_cli", "pc_agent");
    let expected_raw = serde_json::to_value([&pc_run]).unwrap();

    let value = node_usage_response(vec![pc_run], Vec::new());

    assert_eq!(value["consuming"], expected_raw);
    assert_eq!(
        value["federation_compatibility"]["consuming"],
        serde_json::json!([])
    );
}

#[test]
fn compatibility_projection_preserves_live_llm_order_without_changing_raw_fields() {
    let consuming = vec![
        run("consume-2", "server_node_llm", "node_llm"),
        run("consume-pc", "pc_agent_cli", "pc_agent"),
        run("consume-1", "server_node_llm", "node_llm"),
    ];
    let providing = vec![
        run("provide-3", "server_node_llm", "node_llm"),
        run("provide-2", "server_node_llm", "node_llm"),
        run("provide-pc", "pc_agent_cli", "pc_agent"),
    ];
    let expected_consuming = serde_json::to_value(&consuming).unwrap();
    let expected_providing = serde_json::to_value(&providing).unwrap();

    let value = node_usage_response(consuming, providing);

    assert_eq!(value["consuming"], expected_consuming);
    assert_eq!(value["providing"], expected_providing);
    assert_eq!(
        source_run_ids(&value["federation_compatibility"]["consuming"]),
        vec!["consume-2", "consume-1"]
    );
    assert_eq!(
        source_run_ids(&value["federation_compatibility"]["providing"]),
        vec!["provide-3", "provide-2"]
    );
}

fn source_run_ids(value: &serde_json::Value) -> Vec<&str> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|projection| projection["source_run_id"].as_str().unwrap())
        .collect()
}

fn run(id: &str, usage_mode: &str, feature: &str) -> NodeComputeRun {
    NodeComputeRun {
        id: id.to_string(),
        compute_call_id: format!("{usage_mode}:{id}"),
        consumer_user_id: format!("consumer-{id}"),
        provider_user_id: Some(format!("provider-{id}")),
        node_id: format!("node-{id}"),
        model_id: Some("model-v1".to_string()),
        feature: feature.to_string(),
        usage_mode: usage_mode.to_string(),
        billing_source: "platform".to_string(),
        resource_owner_user_id: None,
        lease_id: None,
        offline_policy: "deny".to_string(),
        replay_deadline: None,
        max_cost_rmb_fen: 101,
        allowance_id: None,
        status: "settled".to_string(),
        started_at: "2026-08-12T00:00:00Z".to_string(),
        finished_at: Some("2026-08-12T00:00:01Z".to_string()),
        duration_ms: Some(1_000),
        prompt_tokens: 11,
        completion_tokens: 7,
        reserved_token_budget: 32,
        billed_cost_rmb_fen: 9,
        provider_earned_fen: 7,
        settlement_status: Some("settled".to_string()),
        route_reason: Some("usage response test".to_string()),
        error_message: None,
        created_at: "2026-08-12T00:00:00Z".to_string(),
        updated_at: "2026-08-12T00:00:01Z".to_string(),
    }
}
