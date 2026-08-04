use serde::Serialize;

use crate::compute_federation::provider::ComputeProvider;

pub(crate) const COMPUTE_ACTIVATION_PLAN_SCHEMA: &str = "compute_federation.activation_plan.v1";
pub(crate) const ACTIVATION_PLAN_STATUS_PREPARED: &str = "prepared";

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
