use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_OFFER_PUBLICATION_SCHEMA: &str = "compute_federation.offer_publication.v1";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublishComputeOfferDraftRequest {
    pub expected_offer_version: i64,
    pub expected_offer_digest: String,
    pub idempotency_key: String,
    pub confirm_publish: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeOfferPublicationReceipt {
    pub schema: &'static str,
    pub publication_id: String,
    pub offer_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub source_offer_version: i64,
    pub source_offer_digest: String,
    pub active_offer_version: i64,
    pub active_offer_digest: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub publication_digest: String,
    pub approved_by_user_id: String,
    pub published_at: String,
    pub replayed: bool,
    pub offer_effect: &'static str,
    pub price_snapshot_effect: &'static str,
    pub capacity_effect: &'static str,
    pub funds_effect: &'static str,
}
