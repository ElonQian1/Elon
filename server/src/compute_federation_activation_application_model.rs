use serde::Serialize;

pub(crate) const COMPUTE_ACTIVATION_APPLICATION_SCHEMA: &str =
    "compute_federation.activation_application.v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationApplicationReceipt {
    pub schema: &'static str,
    pub application_id: String,
    pub plan_id: String,
    pub request_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub plan_digest: String,
    pub target_provider_policy_revision: i64,
    pub target_provider_digest: String,
    pub pool_lifecycle_event_id: String,
    pub application_digest: String,
    pub applied_by_user_id: String,
    pub applied_at: String,
    pub replayed: bool,
    pub activation_effect: &'static str,
    pub offer_effect: &'static str,
}
