use serde::Serialize;

use crate::store::NodeComputeRun;

use super::{provider::PROVIDER_KIND_USER_NODE, workload::TASK_KIND_LLM_CHAT};

pub(crate) const LEGACY_LLM_V1_PROJECTION_SCHEMA: &str =
    "compute_federation.legacy_llm_v1_projection.v1";
pub(crate) const LEGACY_COMPATIBILITY_PARTIAL: &str = "partial";
pub(crate) const LEGACY_METERING_PROVIDER_REPORTED: &str = "provider_reported_unverified";

/// A read-only view of facts that the old node LLM record can actually prove.
/// It intentionally does not fabricate an offer, price snapshot or verified receipt.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LegacyLlmV1CompatibilityProjection {
    pub schema: String,
    pub compatibility_level: String,
    pub provider_kind: String,
    pub task_kind: String,
    pub source_run_id: String,
    pub source_compute_call_id: String,
    pub consumer_account_id: String,
    pub provider_account_id: Option<String>,
    pub node_id: String,
    pub model_id: Option<String>,
    pub feature: String,
    pub run_status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub reserved_token_budget: i64,
    pub provider_reported_prompt_tokens: i64,
    pub provider_reported_completion_tokens: i64,
    pub legacy_billed_cost_rmb_fen: i64,
    pub legacy_provider_earned_rmb_fen: i64,
    pub legacy_settlement_status: Option<String>,
    pub metering_trust: String,
    pub missing_contracts: Vec<String>,
}

pub(crate) fn project_legacy_llm_v1(run: &NodeComputeRun) -> LegacyLlmV1CompatibilityProjection {
    LegacyLlmV1CompatibilityProjection {
        schema: LEGACY_LLM_V1_PROJECTION_SCHEMA.to_string(),
        compatibility_level: LEGACY_COMPATIBILITY_PARTIAL.to_string(),
        provider_kind: PROVIDER_KIND_USER_NODE.to_string(),
        task_kind: TASK_KIND_LLM_CHAT.to_string(),
        source_run_id: run.id.clone(),
        source_compute_call_id: run.compute_call_id.clone(),
        consumer_account_id: run.consumer_user_id.clone(),
        provider_account_id: run.provider_user_id.clone(),
        node_id: run.node_id.clone(),
        model_id: run.model_id.clone(),
        feature: run.feature.clone(),
        run_status: run.status.clone(),
        started_at: run.started_at.clone(),
        finished_at: run.finished_at.clone(),
        reserved_token_budget: run.reserved_token_budget,
        provider_reported_prompt_tokens: run.prompt_tokens,
        provider_reported_completion_tokens: run.completion_tokens,
        legacy_billed_cost_rmb_fen: run.billed_cost_rmb_fen,
        legacy_provider_earned_rmb_fen: run.provider_earned_fen,
        legacy_settlement_status: run.settlement_status.clone(),
        metering_trust: LEGACY_METERING_PROVIDER_REPORTED.to_string(),
        missing_contracts: vec![
            "compute_offer_version".to_string(),
            "compute_offer_digest".to_string(),
            "compute_price_snapshot".to_string(),
            "attempt_fencing_generation".to_string(),
            "runner_and_plugin_digests".to_string(),
            "model_and_tokenizer_digests".to_string(),
            "input_and_output_digests".to_string(),
            "observed_usage".to_string(),
            "verified_usage".to_string(),
        ],
    }
}
