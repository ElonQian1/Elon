use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_BATCH_SCHEMA: &str =
    "compute_federation.platform_reference_price_curve_batch.v1";
pub(crate) const COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ENTRY_SCHEMA: &str =
    "compute_federation.platform_reference_price_curve_entry.v1";
pub(crate) const COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_METHODOLOGY: &str =
    "platform_reference_fallback_v1";
pub(crate) const COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CONFIRMATION: &str =
    "confirm_platform_reference_price_curve_batch";
pub(crate) const COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CURRENCY: &str = "CNY";
pub(crate) const COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ROUNDING_MODE: &str = "half_even";
pub(crate) const COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SOURCE_KIND: &str = "fallback_curve";
pub(crate) const COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SOURCE_ID_PREFIX: &str =
    "platform_reference_curve:";
pub(crate) const COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SAMPLE_COUNT: i64 = 0;

/// Canonical platform batch. Serde support does not grant review or application authority.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePlatformReferencePriceCurveBatchEnvelope {
    pub schema: String,
    pub batch_id: String,
    pub batch_digest: String,
    pub batch_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub batch: ComputePlatformReferencePriceCurveBatch,
}

/// Administrator-declared source material with no market-sample or Snapshot authority.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePlatformReferencePriceCurveBatch {
    pub submitted_by_admin_user_id: String,
    pub curve_id: String,
    pub curve_version: i64,
    pub methodology_kind: String,
    pub valid_from: String,
    pub valid_until: String,
    pub quote_ttl_seconds: i64,
    pub rounding_mode: String,
    pub entries: Vec<ComputePlatformReferencePriceCurveEntryIntent>,
    pub entry_set_digest: String,
    pub idempotency_key: String,
    pub confirmation: String,
    pub submission_note: String,
    pub submitted_at: String,
}

/// Exact Offer-bound fallback intent. It is not an index, mark, trade, Job, or Reservation.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePlatformReferencePriceCurveEntryIntent {
    pub entry_key: String,
    pub provider_id: String,
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
    pub sku_id: String,
    pub sku_digest: String,
    pub delivery_window_id: String,
    pub delivery_window_digest: String,
    pub pricing_mode: String,
    pub currency: String,
    pub offer_curve_id: Option<String>,
    pub offer_curve_version: Option<i64>,
    pub instrument_id: Option<String>,
    pub components: Vec<ComputePlatformReferencePriceCurveComponent>,
    pub fee_rules: Vec<ComputePlatformReferencePriceCurveFeeRule>,
    pub consumer_max_amount_micros: i64,
    pub provider_max_amount_micros: i64,
}

/// Source-specific copy of a v171-compatible integer price component.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePlatformReferencePriceCurveComponent {
    pub meter: String,
    pub unit_size: i64,
    pub consumer_unit_price_micros: i64,
    pub provider_unit_price_micros: i64,
    pub max_units: i64,
}

/// Shape is explicit for canonical denial; V1 validation requires this array to be empty.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePlatformReferencePriceCurveFeeRule {
    pub fee_kind: String,
    pub charged_to: String,
    pub fixed_amount_micros: i64,
    pub rate_basis_points: i64,
    pub maximum_amount_micros: Option<i64>,
}

/// Immutable per-entry projection prepared only after the containing batch identity is known.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePlatformReferencePriceCurveEntryEnvelope {
    pub schema: String,
    pub batch_id: String,
    pub batch_digest: String,
    pub entry_id: String,
    pub entry_digest: String,
    pub ordinal: i64,
    pub entry: ComputePlatformReferencePriceCurveEntryIntent,
}
