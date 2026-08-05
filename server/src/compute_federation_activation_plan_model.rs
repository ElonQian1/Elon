use serde::Serialize;

use crate::compute_federation::provider::ComputeProvider;

pub(crate) const COMPUTE_ACTIVATION_PLAN_SCHEMA: &str = "compute_federation.activation_plan.v1";
pub(crate) const ACTIVATION_PLAN_STATUS_PREPARED: &str = "prepared";
pub(crate) const ACTIVATION_PLAN_STATUS_APPLIED: &str = "applied";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationPlan {
    pub schema: &'static str,
    pub plan_id: String,
    pub request_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub expected_request_digest: String,
    pub expected_provider_policy_revision: i64,
    pub expected_provider_digest: String,
    pub expected_capacity_epoch: i64,
    pub expected_pool_revision: i64,
    pub expected_pool_digest: String,
    pub target_provider_policy_revision: i64,
    pub target_provider_digest: String,
    pub target_provider: ComputeProvider,
    pub endpoint_digest: String,
    pub status: String,
    pub plan_digest: String,
    pub prepared_by_user_id: String,
    pub prepared_at: String,
    pub applied_at: Option<String>,
    pub superseded_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationPlanReceipt {
    pub plan: ComputeActivationPlan,
    pub replayed: bool,
    pub activation_effect: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationPlanPreflightReport {
    pub schema: &'static str,
    pub plan_id: String,
    pub request_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub plan_status: String,
    pub checked_at: String,
    pub plan_status_prepared: bool,
    pub request_approved: bool,
    pub request_digest_matches: bool,
    pub request_binding_matches: bool,
    pub provider_version_matches: bool,
    pub provider_status_registering: bool,
    pub target_provider_identity_matches: bool,
    pub target_provider_revision_matches: bool,
    pub target_provider_contract_ready: bool,
    pub pool_provider_matches: bool,
    pub pool_version_matches: bool,
    pub pool_status_registering: bool,
    pub ledger_audit_healthy: bool,
    pub ledger_audit_digest_matches: bool,
    pub plan_review_present: bool,
    pub plan_review_digest_matches: bool,
    pub plan_review_separation_valid: bool,
    pub ready_for_apply: bool,
    pub blockers: Vec<String>,
    pub activation_effect: &'static str,
}
