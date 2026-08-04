use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_OFFER_LIFECYCLE_SCHEMA: &str = "compute_federation.offer_lifecycle.v1";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DrainComputeOfferRequest {
    pub expected_offer_version: i64,
    pub expected_offer_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_drain: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TerminateComputeOfferRequest {
    pub expected_offer_version: i64,
    pub expected_offer_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_terminal: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeOfferLifecycleReceipt {
    pub schema: &'static str,
    pub event_id: String,
    pub offer_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub previous_status: String,
    pub target_status: String,
    pub previous_offer_version: i64,
    pub previous_offer_digest: String,
    pub target_offer_version: i64,
    pub target_offer_digest: String,
    pub reason: String,
    pub event_digest: String,
    pub changed_by_user_id: String,
    pub changed_at: String,
    pub replayed: bool,
    pub quote_candidate_effect: &'static str,
    pub reservation_effect: &'static str,
    pub attempt_effect: &'static str,
    pub funds_effect: &'static str,
}
