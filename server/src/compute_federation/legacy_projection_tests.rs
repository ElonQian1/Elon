use crate::store::NodeComputeRun;

use super::{
    is_legacy_llm_v1_compatible, project_legacy_llm_v1_list, project_legacy_llm_v1_lists,
    LEGACY_COMPATIBILITY_PARTIAL, LEGACY_LLM_V1_PROJECTION_LIST_SCHEMA,
    LEGACY_LLM_V1_PROJECTION_SCHEMA, LEGACY_METERING_PROVIDER_REPORTED,
};

fn run(
    id: &str,
    feature: &str,
    usage_mode: &str,
    provider_user_id: Option<&str>,
) -> NodeComputeRun {
    NodeComputeRun {
        id: id.to_string(),
        compute_call_id: format!("node_llm:{id}"),
        consumer_user_id: "consumer-a".to_string(),
        provider_user_id: provider_user_id.map(str::to_string),
        node_id: format!("node-{id}"),
        model_id: Some("model-a".to_string()),
        feature: feature.to_string(),
        usage_mode: usage_mode.to_string(),
        billing_source: "legacy_node".to_string(),
        resource_owner_user_id: provider_user_id.map(str::to_string),
        lease_id: None,
        offline_policy: "deny_offline".to_string(),
        replay_deadline: None,
        max_cost_rmb_fen: 0,
        allowance_id: None,
        status: "settled".to_string(),
        started_at: "2026-08-12T01:00:00Z".to_string(),
        finished_at: Some("2026-08-12T01:00:01Z".to_string()),
        duration_ms: Some(1_000),
        prompt_tokens: 12,
        completion_tokens: 34,
        reserved_token_budget: 128,
        billed_cost_rmb_fen: 5,
        provider_earned_fen: 4,
        settlement_status: Some("settled".to_string()),
        route_reason: Some("legacy_node_test".to_string()),
        error_message: None,
        created_at: "2026-08-12T01:00:00Z".to_string(),
        updated_at: "2026-08-12T01:00:01Z".to_string(),
    }
}

#[test]
fn compatibility_requires_exact_llm_feature_and_usage_mode() {
    let compatible = run(
        "compatible",
        "node_llm",
        "server_node_llm",
        Some("provider-a"),
    );
    assert!(is_legacy_llm_v1_compatible(&compatible));

    for incompatible in [
        run("pc-cli", "pc_node_exec", "pc_agent_cli", Some("provider-a")),
        run(
            "wrong-feature",
            "other",
            "server_node_llm",
            Some("provider-a"),
        ),
        run("wrong-mode", "node_llm", "other", Some("provider-a")),
        run(
            "padded-feature",
            " node_llm ",
            "server_node_llm",
            Some("provider-a"),
        ),
        run(
            "padded-mode",
            "node_llm",
            " server_node_llm ",
            Some("provider-a"),
        ),
    ] {
        assert!(!is_legacy_llm_v1_compatible(&incompatible));
    }
}

#[test]
fn batch_projection_filters_incompatible_runs_and_preserves_source_order() {
    let runs = vec![
        run("first", "node_llm", "server_node_llm", Some("provider-a")),
        run("pc-cli", "pc_node_exec", "pc_agent_cli", None),
        run("second", "node_llm", "server_node_llm", None),
    ];

    let projected = project_legacy_llm_v1_list(&runs);
    let ids = projected
        .iter()
        .map(|projection| projection.source_run_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["first", "second"]);
    assert_eq!(projected[1].provider_account_id, None);
}

#[test]
fn projection_remains_partial_unverified_and_names_every_missing_contract() {
    let projected =
        project_legacy_llm_v1_list(&[run("compatible", "node_llm", "server_node_llm", None)]);
    let projection = &projected[0];

    assert_eq!(projection.schema, LEGACY_LLM_V1_PROJECTION_SCHEMA);
    assert_eq!(projection.compatibility_level, LEGACY_COMPATIBILITY_PARTIAL);
    assert_eq!(projection.metering_trust, LEGACY_METERING_PROVIDER_REPORTED);
    assert_eq!(projection.provider_account_id, None);
    assert_eq!(
        projection.missing_contracts,
        [
            "compute_provider_id",
            "compute_provider_version",
            "compute_provider_digest",
            "compute_offer_version",
            "compute_offer_digest",
            "compute_price_snapshot",
            "compute_job_id",
            "compute_job_version",
            "compute_job_digest",
            "compute_reservation",
            "compute_attempt_lease",
            "attempt_fencing_generation",
            "runner_and_plugin_digests",
            "model_and_tokenizer_digests",
            "input_and_output_digests",
            "observed_usage",
            "verified_usage",
            "compute_execution_receipt",
            "compute_settlement_receipt",
        ]
        .map(str::to_string)
    );
}

#[test]
fn list_envelope_projects_each_side_independently_and_keeps_empty_arrays() {
    let consuming = vec![
        run(
            "consumer-compatible",
            "node_llm",
            "server_node_llm",
            Some("provider-a"),
        ),
        run("consumer-pc", "pc_node_exec", "pc_agent_cli", None),
    ];
    let projection = project_legacy_llm_v1_lists(&consuming, &[]);

    assert_eq!(projection.schema, LEGACY_LLM_V1_PROJECTION_LIST_SCHEMA);
    assert_eq!(projection.consuming.len(), 1);
    assert_eq!(projection.consuming[0].source_run_id, "consumer-compatible");
    assert!(projection.providing.is_empty());
}
