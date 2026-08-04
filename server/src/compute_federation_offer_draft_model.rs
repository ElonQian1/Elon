use serde::{Deserialize, Serialize};

use crate::compute_federation::{
    market::{ComputeFeeRule, ComputePriceComponent},
    offer::ComputeOfferExecutionLimits,
    workload::{ComputeModelRef, ComputeRuntimeRef},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateMyComputeOfferDraftRequest {
    pub idempotency_key: String,
    pub sku: ComputeOfferDraftSkuInput,
    pub model: Option<ComputeModelRef>,
    pub runtime: ComputeRuntimeRef,
    pub resource_profile: ComputeOfferDraftResourceProfileInput,
    pub capacity: Vec<ComputeOfferDraftCapacityInput>,
    pub execution_limits: ComputeOfferExecutionLimits,
    pub authorization: ComputeOfferDraftAuthorizationInput,
    pub price_terms: ComputeOfferDraftPriceTermsInput,
    pub valid_from: String,
    pub valid_until: String,
    pub confirm_create: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeOfferDraftSkuInput {
    pub sku_id: String,
    pub task_kind: String,
    pub context_or_shape_bucket: String,
    pub verification_tier: String,
    pub sla_tier: String,
    pub delivery_window_class: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeOfferDraftResourceProfileInput {
    pub accelerator_kind: String,
    pub accelerator_count: i64,
    pub vram_bytes: i64,
    pub ram_bytes: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeOfferDraftCapacityInput {
    pub bucket_id: String,
    pub total_units: i64,
    pub reservable_units: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeOfferDraftAuthorizationInput {
    pub public: bool,
    pub allowed_account_ids: Vec<String>,
    pub allowed_project_ids: Vec<String>,
    pub allowed_data_classes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeOfferDraftPriceTermsInput {
    pub pricing_mode: String,
    pub currency: String,
    pub curve_id: Option<String>,
    pub curve_version: Option<i64>,
    pub instrument_id: Option<String>,
    pub components: Vec<ComputePriceComponent>,
    pub fee_rules: Vec<ComputeFeeRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeMyComputeOfferDraftRequest {
    pub expected_offer_version: i64,
    pub expected_offer_digest: String,
    pub confirm_revoke: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviseMyComputeOfferDraftRequest {
    pub expected_offer_version: i64,
    pub expected_offer_digest: String,
    pub sku: ComputeOfferDraftSkuInput,
    pub model: Option<ComputeModelRef>,
    pub runtime: ComputeRuntimeRef,
    pub resource_profile: ComputeOfferDraftResourceProfileInput,
    pub capacity: Vec<ComputeOfferDraftCapacityInput>,
    pub execution_limits: ComputeOfferExecutionLimits,
    pub authorization: ComputeOfferDraftAuthorizationInput,
    pub price_terms: ComputeOfferDraftPriceTermsInput,
    pub valid_from: String,
    pub valid_until: String,
    pub confirm_revise: bool,
}
