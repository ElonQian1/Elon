use serde::{Deserialize, Serialize};

use super::{
    market::{ComputeDeliveryWindow, ComputePriceTerms, ComputeSku},
    workload::{ComputeModelRef, ComputeRuntimeRef},
};

pub(crate) const COMPUTE_OFFER_SCHEMA: &str = "compute_federation.offer.v1";

pub(crate) const OFFER_STATUS_DRAFT: &str = "draft";
pub(crate) const OFFER_STATUS_ACTIVE: &str = "active";
pub(crate) const OFFER_STATUS_DRAINING: &str = "draining";
pub(crate) const OFFER_STATUS_EXPIRED: &str = "expired";
pub(crate) const OFFER_STATUS_REVOKED: &str = "revoked";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeOfferCapacity {
    pub meter: String,
    pub total_units: i64,
    pub reservable_units: i64,
    pub committed_units: i64,
    pub max_concurrent_attempts: i64,
    pub max_attempt_runtime_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeOfferAuthorization {
    pub public: bool,
    pub allowed_account_ids: Vec<String>,
    pub allowed_project_ids: Vec<String>,
    pub allowed_data_classes: Vec<String>,
    pub policy_revision: i64,
}

/// Hardware facts are not collapsed into one self-reported profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeOfferResourceProfile {
    pub declared_profile_digest: String,
    pub observed_profile_digest: Option<String>,
    pub verified_profile_digest: Option<String>,
    pub accelerator_kind: String,
    pub accelerator_count: i64,
    pub vram_bytes: i64,
    pub ram_bytes: i64,
}

/// An offer is immutable once referenced. Changes create a higher version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeOffer {
    pub schema: String,
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
    pub provider_id: String,
    pub provider_kind: String,
    pub status: String,
    pub sku: ComputeSku,
    pub model: Option<ComputeModelRef>,
    pub runtime: ComputeRuntimeRef,
    pub resource_profile: ComputeOfferResourceProfile,
    pub capacity: Vec<ComputeOfferCapacity>,
    pub authorization: ComputeOfferAuthorization,
    pub delivery_windows: Vec<ComputeDeliveryWindow>,
    pub price_terms: ComputePriceTerms,
    pub valid_from: String,
    pub valid_until: String,
    pub created_at: String,
}
