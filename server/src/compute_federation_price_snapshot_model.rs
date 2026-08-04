use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublishMyComputePriceSnapshotRequest {
    pub expected_offer_version: i64,
    pub expected_offer_digest: String,
    pub delivery_window_id: String,
    pub consumer_max_amount_micros: i64,
    pub provider_max_amount_micros: i64,
    pub ttl_seconds: i64,
    pub rounding_mode: String,
    pub idempotency_key: String,
    pub confirm_publish: bool,
}
