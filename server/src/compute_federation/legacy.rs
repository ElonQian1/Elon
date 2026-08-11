use serde::Serialize;

use crate::store::NodeComputeRun;

use super::{provider::PROVIDER_KIND_USER_NODE, workload::TASK_KIND_LLM_CHAT};

pub(crate) const LEGACY_LLM_V1_PROJECTION_SCHEMA: &str =
    "compute_federation.legacy_llm_v1_projection.v1";
pub(crate) const LEGACY_LLM_V1_PROJECTION_LIST_SCHEMA: &str =
    "compute_federation.legacy_llm_v1_projection_list.v1";
pub(crate) const LEGACY_COMPATIBILITY_PARTIAL: &str = "partial";
pub(crate) const LEGACY_METERING_PROVIDER_REPORTED: &str = "provider_reported_unverified";

const LEGACY_LLM_V1_MISSING_CONTRACTS: &[&str] = &[
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
];

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

/// Additive compatibility envelope for the two existing `/api/me/node-usage` views.
///
/// The source arrays remain authoritative legacy records. These projections only expose the
/// subset that can be labelled as an LLM chat without inventing federation identities.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LegacyLlmV1CompatibilityProjectionList {
    pub schema: String,
    pub consuming: Vec<LegacyLlmV1CompatibilityProjection>,
    pub providing: Vec<LegacyLlmV1CompatibilityProjection>,
}

pub(crate) fn is_legacy_llm_v1_compatible(run: &NodeComputeRun) -> bool {
    run.feature == "node_llm" && run.usage_mode == "server_node_llm"
}

pub(crate) fn project_legacy_llm_v1_list(
    runs: &[NodeComputeRun],
) -> Vec<LegacyLlmV1CompatibilityProjection> {
    runs.iter()
        .filter(|run| is_legacy_llm_v1_compatible(run))
        .map(project_legacy_llm_v1)
        .collect()
}

pub(crate) fn project_legacy_llm_v1_lists(
    consuming: &[NodeComputeRun],
    providing: &[NodeComputeRun],
) -> LegacyLlmV1CompatibilityProjectionList {
    LegacyLlmV1CompatibilityProjectionList {
        schema: LEGACY_LLM_V1_PROJECTION_LIST_SCHEMA.to_string(),
        consuming: project_legacy_llm_v1_list(consuming),
        providing: project_legacy_llm_v1_list(providing),
    }
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
        missing_contracts: LEGACY_LLM_V1_MISSING_CONTRACTS
            .iter()
            .map(|contract| (*contract).to_string())
            .collect(),
    }
}

#[cfg(test)]
#[path = "legacy_projection_tests.rs"]
mod tests;
