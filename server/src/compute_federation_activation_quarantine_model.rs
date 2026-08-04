use serde::Serialize;

pub(crate) const COMPUTE_ACTIVATION_QUARANTINE_SCHEMA: &str =
    "compute_federation.activation_quarantine.v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationQuarantineReceipt {
    pub schema: &'static str,
    pub quarantine_id: String,
    pub application_id: String,
    pub request_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub application_digest: String,
    pub previous_provider_policy_revision: i64,
    pub previous_provider_digest: String,
    pub quarantined_provider_policy_revision: i64,
    pub quarantined_provider_digest: String,
    pub capacity_epoch: i64,
    pub pool_lifecycle_event_id: String,
    pub reason: String,
    pub quarantine_digest: String,
    pub quarantined_by_user_id: String,
    pub quarantined_at: String,
    pub replayed: bool,
    pub provider_effect: &'static str,
    pub pool_effect: &'static str,
    pub offer_effect: &'static str,
}
