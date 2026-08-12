use serde::Serialize;

use crate::compute_federation::{
    capacity_instrument::{
        ComputeCapacityInstrument, ComputeCapacityInstrumentActivationReceipt,
        ComputeCapacityInstrumentContractUnit, ComputeCapacityInstrumentOfferAdoptionReceipt,
        ComputeCapacityInstrumentRetirementReceipt,
    },
    market::ComputeDeliveryWindow,
};

#[derive(Clone, Debug)]
pub(crate) struct RegisterComputeCapacityInstrument {
    pub instrument_id: String,
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
}

#[derive(Clone, Debug)]
pub(crate) struct ActivateComputeCapacityInstrument {
    pub instrument_id: String,
    pub expected_instrument_revision: i64,
    pub expected_instrument_digest: String,
    pub activated_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug)]
pub(crate) struct RetireComputeCapacityInstrument {
    pub instrument_id: String,
    pub expected_instrument_revision: i64,
    pub expected_instrument_digest: String,
    pub retired_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug)]
pub(crate) struct AdoptComputeCapacityInstrumentOffer {
    pub instrument_id: String,
    pub expected_instrument_revision: i64,
    pub expected_instrument_digest: String,
    pub offer_id: String,
    pub expected_offer_version: i64,
    pub expected_offer_digest: String,
    pub expected_publication_id: String,
    pub expected_publication_digest: String,
    pub adopted_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ComputeCapacityInstrumentRegistrationWriteReceipt {
    pub instrument: ComputeCapacityInstrument,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ComputeCapacityInstrumentActivationWriteReceipt {
    pub instrument: ComputeCapacityInstrument,
    pub activation: ComputeCapacityInstrumentActivationReceipt,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ComputeCapacityInstrumentRetirementWriteReceipt {
    pub instrument: ComputeCapacityInstrument,
    pub retirement: ComputeCapacityInstrumentRetirementReceipt,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ComputeCapacityInstrumentOfferAdoptionWriteReceipt {
    pub instrument: ComputeCapacityInstrument,
    pub adoption: ComputeCapacityInstrumentOfferAdoptionReceipt,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ComputeCapacityInstrumentCurrentnessReceipt {
    pub schema: &'static str,
    pub instrument: ComputeCapacityInstrument,
    pub current_status: String,
    pub activation: Option<ComputeCapacityInstrumentActivationReceipt>,
    pub retirement: Option<ComputeCapacityInstrumentRetirementReceipt>,
}

pub(super) struct StoredInstrument {
    pub instrument: ComputeCapacityInstrument,
    pub instrument_json: String,
}

pub(super) struct StoredActivation {
    pub activation: ComputeCapacityInstrumentActivationReceipt,
    pub activation_json: String,
}

pub(super) struct StoredRetirement {
    pub retirement: ComputeCapacityInstrumentRetirementReceipt,
    pub retirement_json: String,
}

pub(super) struct StoredAdoption {
    pub adoption: ComputeCapacityInstrumentOfferAdoptionReceipt,
    pub adoption_json: String,
}

/// Non-serializable authority passed only between Store transaction kernels.
pub(in crate::store) struct ComputeCapacityInstrumentAdoptionAuthority {
    pub(in crate::store) instrument: ComputeCapacityInstrument,
    pub(in crate::store) activation: ComputeCapacityInstrumentActivationReceipt,
    pub(in crate::store) adoption: ComputeCapacityInstrumentOfferAdoptionReceipt,
}
