use serde::Serialize;

use crate::compute_federation::provider::ComputeProvider;

pub(crate) const RECOVERY_PLAN_SCHEMA: &str = "compute_federation.activation_recovery_plan.v1";
pub(crate) const RECOVERY_REVIEW_SCHEMA: &str = "compute_federation.activation_recovery_review.v1";
pub(crate) const RECOVERY_APPLICATION_SCHEMA: &str =
    "compute_federation.activation_recovery_application.v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationRecoveryPlan {
    pub schema: &'static str,
    pub recovery_plan_id: String,
    pub quarantine_id: String,
    pub application_id: String,
    pub request_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub expected_quarantine_digest: String,
    pub expected_provider_policy_revision: i64,
    pub expected_provider_digest: String,
    pub expected_capacity_epoch: i64,
    pub expected_pool_revision: i64,
    pub expected_pool_digest: String,
    pub target_provider_policy_revision: i64,
    pub target_provider_digest: String,
    pub target_provider: ComputeProvider,
    pub routing_digest: String,
    pub remediation_summary: String,
    pub evidence_refs: Vec<String>,
    pub evidence_refs_digest: String,
    pub status: String,
    pub plan_digest: String,
    pub prepared_by_user_id: String,
    pub prepared_at: String,
    pub applied_at: Option<String>,
    pub superseded_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationRecoveryPlanReceipt {
    pub plan: ComputeActivationRecoveryPlan,
    pub replayed: bool,
    pub provider_effect: &'static str,
    pub pool_effect: &'static str,
    pub offer_effect: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationRecoveryReviewReceipt {
    pub schema: &'static str,
    pub recovery_review_id: String,
    pub recovery_plan_id: String,
    pub request_id: String,
    pub plan_digest: String,
    pub prepared_by_user_id: String,
    pub reviewed_by_user_id: String,
    pub review_note: Option<String>,
    pub request_digest: String,
    pub review_digest: String,
    pub reviewed_at: String,
    pub replayed: bool,
    pub recovery_effect: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationRecoveryApplicationReceipt {
    pub schema: &'static str,
    pub recovery_application_id: String,
    pub recovery_plan_id: String,
    pub recovery_review_id: String,
    pub quarantine_id: String,
    pub request_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub plan_digest: String,
    pub review_digest: String,
    pub recovered_provider_policy_revision: i64,
    pub recovered_provider_digest: String,
    pub capacity_epoch: i64,
    pub pool_lifecycle_event_id: String,
    pub application_digest: String,
    pub applied_by_user_id: String,
    pub applied_at: String,
    pub replayed: bool,
    pub provider_effect: &'static str,
    pub pool_effect: &'static str,
    pub offer_effect: &'static str,
    pub node_effect: &'static str,
    pub money_effect: &'static str,
}
