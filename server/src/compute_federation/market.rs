use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_SKU_SCHEMA: &str = "compute_federation.sku.v1";
pub(crate) const COMPUTE_PRICE_TERMS_SCHEMA: &str = "compute_federation.price_terms.v1";
pub(crate) const COMPUTE_PRICE_SNAPSHOT_SCHEMA: &str = "compute_federation.price_snapshot.v1";

pub(crate) const PRICING_MODE_SPOT: &str = "spot";
pub(crate) const PRICING_MODE_INDEX_LOCKED: &str = "index_locked";
pub(crate) const PRICING_MODE_CAPACITY_FORWARD: &str = "capacity_forward";
pub(crate) const PRICING_MODE_CAPACITY_FUTURE: &str = "capacity_future";

pub(crate) const PRICE_SOURCE_TRADE: &str = "trade";
pub(crate) const PRICE_SOURCE_INDEX: &str = "index";
pub(crate) const PRICE_SOURCE_MARK: &str = "mark";
pub(crate) const PRICE_SOURCE_FALLBACK_CURVE: &str = "fallback_curve";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeDeliveryWindowBinding {
    pub window_id: String,
    pub window_digest: String,
}

/// Stable half-open UTC interval: `[starts_at_utc, ends_at_utc)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeDeliveryWindow {
    pub binding: ComputeDeliveryWindowBinding,
    pub starts_at_utc: String,
    pub ends_at_utc: String,
}

/// Standardized market identity. The digest is calculated outside this model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeSku {
    pub schema: String,
    pub sku_id: String,
    pub task_kind: String,
    pub model_family: Option<String>,
    pub model_digest: Option<String>,
    pub tokenizer_digest: Option<String>,
    pub runtime_family: String,
    pub precision: String,
    pub context_or_shape_bucket: String,
    pub verification_tier: String,
    pub sla_tier: String,
    pub region_or_data_zone: String,
    pub delivery_window_class: String,
    pub metering_units: Vec<String>,
    pub sku_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputePriceComponent {
    pub meter: String,
    pub unit_size: i64,
    pub consumer_unit_price_micros: i64,
    pub provider_unit_price_micros: i64,
    pub max_units: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeFeeRule {
    pub fee_kind: String,
    pub charged_to: String,
    pub fixed_amount_micros: i64,
    pub rate_basis_points: i64,
    pub maximum_amount_micros: Option<i64>,
}

/// Non-binding commercial terms published with an offer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputePriceTerms {
    pub schema: String,
    pub pricing_mode: String,
    pub currency: String,
    pub curve_id: Option<String>,
    pub curve_version: Option<i64>,
    pub instrument_id: Option<String>,
    pub components: Vec<ComputePriceComponent>,
    pub fee_rules: Vec<ComputeFeeRule>,
    pub valid_until: String,
}

/// Provenance for a future curve, index, mark or matched trade.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputePriceSource {
    pub source_kind: String,
    pub source_id: String,
    pub source_version: i64,
    pub observation_window_start: String,
    pub observation_window_end: String,
    pub sample_count: i64,
    pub source_digest: String,
}

/// Immutable binding created before a reservation is activated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputePriceSnapshot {
    pub schema: String,
    pub snapshot_id: String,
    pub snapshot_digest: String,
    pub quote_id: String,
    pub pricing_mode: String,
    pub sku: ComputeSku,
    pub provider_id: String,
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
    pub delivery_window: ComputeDeliveryWindow,
    pub currency: String,
    pub components: Vec<ComputePriceComponent>,
    pub fee_rules: Vec<ComputeFeeRule>,
    pub consumer_max_amount_micros: i64,
    pub provider_max_amount_micros: i64,
    pub price_source: ComputePriceSource,
    pub trade_id: Option<String>,
    pub instrument_id: Option<String>,
    pub rounding_mode: String,
    pub quoted_at: String,
    pub expires_at: String,
}
