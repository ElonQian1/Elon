use serde::{Deserialize, Serialize};

use super::super::market::ComputeDeliveryWindow;

pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_SCHEMA: &str =
    "compute_federation.capacity_instrument.v1";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_RECEIPT_SCHEMA: &str =
    "compute_federation.capacity_instrument_activation_receipt.v1";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_RECEIPT_SCHEMA: &str =
    "compute_federation.capacity_instrument_retirement_receipt.v1";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_RECEIPT_SCHEMA: &str =
    "compute_federation.capacity_instrument_offer_adoption_receipt.v1";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_CURRENTNESS_SCHEMA: &str =
    "compute_federation.capacity_instrument_currentness.v1";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_REVISION: i64 = 1;
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_CURRENCY: &str = "CNY";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_UNIT: &str = "platform_balance_cny_micros";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_STATUS_REGISTERED: &str = "registered";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_STATUS_ACTIVE: &str = "active";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_STATUS_RETIRED: &str = "retired";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_REGISTRATION_CONFIRMATION: &str =
    "confirm_compute_capacity_instrument_registration";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_CONFIRMATION: &str =
    "confirm_compute_capacity_instrument_activation";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_CONFIRMATION: &str =
    "confirm_compute_capacity_instrument_retirement";
pub(crate) const COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_CONFIRMATION: &str =
    "confirm_compute_capacity_instrument_offer_adoption";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityInstrumentContractUnit {
    pub meter: String,
    pub unit_size: i64,
    pub quantity_units: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityInstrument {
    pub schema: String,
    pub instrument_id: String,
    pub instrument_revision: i64,
    pub instrument_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub sku_id: String,
    pub sku_digest: String,
    pub delivery_window: ComputeDeliveryWindow,
    pub contract_units: Vec<ComputeCapacityInstrumentContractUnit>,
    pub availability_sla_tier: String,
    pub region_or_data_zone: String,
    pub verification_tier: String,
    pub settlement_currency: String,
    pub settlement_unit: String,
    pub registered_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub registered_at: String,
    pub recorded_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityInstrumentActivationReceipt {
    pub schema: String,
    pub activation_receipt_id: String,
    pub activation_receipt_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub instrument_id: String,
    pub instrument_revision: i64,
    pub instrument_digest: String,
    pub activated_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub activated_at: String,
    pub recorded_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityInstrumentRetirementReceipt {
    pub schema: String,
    pub retirement_receipt_id: String,
    pub retirement_receipt_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub instrument_id: String,
    pub instrument_revision: i64,
    pub instrument_digest: String,
    pub retired_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub retired_at: String,
    pub recorded_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityInstrumentOfferAdoptionReceipt {
    pub schema: String,
    pub adoption_receipt_id: String,
    pub adoption_receipt_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub instrument_id: String,
    pub instrument_revision: i64,
    pub instrument_digest: String,
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
    pub publication_id: String,
    pub publication_digest: String,
    pub adopted_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub adopted_at: String,
    pub recorded_at: String,
}
