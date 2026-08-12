use crate::{
    compute_federation::{
        capacity_instrument::{
            ComputeCapacityInstrument, ComputeCapacityInstrumentContractUnit,
            COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_CONFIRMATION,
            COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_CONFIRMATION,
            COMPUTE_CAPACITY_INSTRUMENT_REGISTRATION_CONFIRMATION,
            COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_CONFIRMATION,
            COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_CURRENCY,
            COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_UNIT,
        },
        market::{ComputePriceSnapshot, PRICE_SOURCE_FALLBACK_CURVE},
        offer::ComputeOffer,
    },
    compute_federation_offer_publication_model::ComputeOfferPublicationReceipt,
    store::{
        compute_price_snapshot_digest, ActivateComputeCapacityInstrument,
        AdoptComputeCapacityInstrumentOffer, RegisterComputeCapacityInstrument,
        RetireComputeCapacityInstrument, Store,
    },
};

use super::Fixture;

impl Fixture {
    pub(crate) fn instrument_registration_input(&self) -> RegisterComputeCapacityInstrument {
        RegisterComputeCapacityInstrument {
            instrument_id: self.capacity_instrument.instrument_id.clone(),
            sku_id: self.capacity_instrument.sku_id.clone(),
            sku_digest: self.capacity_instrument.sku_digest.clone(),
            delivery_window: self.capacity_instrument.delivery_window.clone(),
            contract_units: self.capacity_instrument.contract_units.clone(),
            availability_sla_tier: self.capacity_instrument.availability_sla_tier.clone(),
            region_or_data_zone: self.capacity_instrument.region_or_data_zone.clone(),
            verification_tier: self.capacity_instrument.verification_tier.clone(),
            settlement_currency: self.capacity_instrument.settlement_currency.clone(),
            settlement_unit: self.capacity_instrument.settlement_unit.clone(),
            registered_by_admin_user_id: self.capacity_instrument_registrar_id.clone(),
            confirmation: COMPUTE_CAPACITY_INSTRUMENT_REGISTRATION_CONFIRMATION.into(),
            idempotency_scope: scope("register", &self.capacity_instrument.instrument_id),
            idempotency_key: "register".into(),
        }
    }

    pub(crate) fn instrument_activation_input(&self) -> ActivateComputeCapacityInstrument {
        ActivateComputeCapacityInstrument {
            instrument_id: self.capacity_instrument.instrument_id.clone(),
            expected_instrument_revision: self.capacity_instrument.instrument_revision,
            expected_instrument_digest: self.capacity_instrument.instrument_digest.clone(),
            activated_by_admin_user_id: self.admin_id.clone(),
            confirmation: COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_CONFIRMATION.into(),
            idempotency_scope: scope("activate", &self.capacity_instrument.instrument_id),
            idempotency_key: "activate".into(),
        }
    }

    pub(crate) fn instrument_adoption_input(&self) -> AdoptComputeCapacityInstrumentOffer {
        AdoptComputeCapacityInstrumentOffer {
            instrument_id: self.capacity_instrument.instrument_id.clone(),
            expected_instrument_revision: self.capacity_instrument.instrument_revision,
            expected_instrument_digest: self.capacity_instrument.instrument_digest.clone(),
            offer_id: self.offer.offer_id.clone(),
            expected_offer_version: self.offer.offer_version,
            expected_offer_digest: self.offer.offer_digest.clone(),
            expected_publication_id: self.publication.publication_id.clone(),
            expected_publication_digest: self.publication.publication_digest.clone(),
            adopted_by_admin_user_id: self.admin_id.clone(),
            confirmation: COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_CONFIRMATION.into(),
            idempotency_scope: scope("adopt", &self.capacity_instrument.instrument_id),
            idempotency_key: "adopt".into(),
        }
    }

    pub(crate) fn fresh_price_snapshot(&self, suffix: &str) -> ComputePriceSnapshot {
        let mut snapshot = self
            .store
            .compute_price_snapshot(&self.binding.snapshot_id)
            .unwrap()
            .snapshot;
        snapshot.snapshot_id = format!("capacity-instrument-snapshot-{suffix}");
        snapshot.quote_id = format!("capacity-instrument-quote-{suffix}");
        snapshot.snapshot_digest.clear();
        snapshot.price_source.source_kind = PRICE_SOURCE_FALLBACK_CURVE.into();
        snapshot.price_source.source_id = format!("offer_fallback_curve:{}", self.offer.offer_id);
        snapshot.price_source.source_version = self.offer.offer_version;
        snapshot.price_source.sample_count = 0;
        snapshot.price_source.source_digest = self.offer.offer_digest.clone();
        snapshot.snapshot_digest = compute_price_snapshot_digest(&snapshot).unwrap();
        snapshot
    }

    pub(crate) fn retire_instrument(&self, suffix: &str) {
        self.store
            .retire_compute_capacity_instrument(RetireComputeCapacityInstrument {
                instrument_id: self.capacity_instrument.instrument_id.clone(),
                expected_instrument_revision: self.capacity_instrument.instrument_revision,
                expected_instrument_digest: self.capacity_instrument.instrument_digest.clone(),
                retired_by_admin_user_id: self.admin_id.clone(),
                reason: format!("capacity-instrument test retirement: {suffix}"),
                confirmation: COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_CONFIRMATION.into(),
                idempotency_scope: scope("retire", &self.capacity_instrument.instrument_id),
                idempotency_key: suffix.into(),
            })
            .unwrap();
    }
}

pub(super) fn establish_authority(
    store: &Store,
    offer: &ComputeOffer,
    publication: &ComputeOfferPublicationReceipt,
    registrar_id: &str,
    authority_actor_id: &str,
) -> ComputeCapacityInstrument {
    let mut contract_units = vec![
        ComputeCapacityInstrumentContractUnit {
            meter: "tokens".into(),
            unit_size: 10,
            quantity_units: 20,
        },
        ComputeCapacityInstrumentContractUnit {
            meter: "concurrency".into(),
            unit_size: 1,
            quantity_units: 1,
        },
    ];
    contract_units.sort_by(|left, right| left.meter.cmp(&right.meter));
    let instrument_id = offer.price_terms.instrument_id.clone().unwrap();
    let instrument = register(
        store,
        offer,
        registrar_id,
        instrument_id.clone(),
        contract_units,
    );
    assert_activation_role_separation(store, registrar_id, &instrument);
    activate(store, authority_actor_id, &instrument);
    assert_adoption_role_separation(store, offer, publication, registrar_id, &instrument);
    adopt(store, offer, publication, authority_actor_id, &instrument);
    instrument
}

fn register(
    store: &Store,
    offer: &ComputeOffer,
    registrar_id: &str,
    instrument_id: String,
    contract_units: Vec<ComputeCapacityInstrumentContractUnit>,
) -> ComputeCapacityInstrument {
    store
        .register_compute_capacity_instrument(RegisterComputeCapacityInstrument {
            instrument_id: instrument_id.clone(),
            sku_id: offer.sku.sku_id.clone(),
            sku_digest: offer.sku.sku_digest.clone(),
            delivery_window: offer.delivery_windows.first().unwrap().clone(),
            contract_units,
            availability_sla_tier: offer.sku.sla_tier.clone(),
            region_or_data_zone: offer.sku.region_or_data_zone.clone(),
            verification_tier: offer.sku.verification_tier.clone(),
            settlement_currency: COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_CURRENCY.into(),
            settlement_unit: COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_UNIT.into(),
            registered_by_admin_user_id: registrar_id.into(),
            confirmation: COMPUTE_CAPACITY_INSTRUMENT_REGISTRATION_CONFIRMATION.into(),
            idempotency_scope: scope("register", &instrument_id),
            idempotency_key: "register".into(),
        })
        .unwrap()
        .instrument
}

fn assert_activation_role_separation(
    store: &Store,
    registrar_id: &str,
    instrument: &ComputeCapacityInstrument,
) {
    let error = store
        .activate_compute_capacity_instrument(activation_input(
            registrar_id,
            instrument,
            "activate-denied",
        ))
        .unwrap_err();
    assert!(error.to_string().contains("actor distinct from registrar"));
}

fn assert_adoption_role_separation(
    store: &Store,
    offer: &ComputeOffer,
    publication: &ComputeOfferPublicationReceipt,
    registrar_id: &str,
    instrument: &ComputeCapacityInstrument,
) {
    let error = store
        .adopt_compute_capacity_instrument_offer(adoption_input(
            offer,
            publication,
            registrar_id,
            instrument,
            "adopt-denied",
        ))
        .unwrap_err();
    assert!(error.to_string().contains("actor distinct from registrar"));
}

fn activate(store: &Store, actor_id: &str, instrument: &ComputeCapacityInstrument) {
    store
        .activate_compute_capacity_instrument(activation_input(actor_id, instrument, "activate"))
        .unwrap();
}

fn adopt(
    store: &Store,
    offer: &ComputeOffer,
    publication: &ComputeOfferPublicationReceipt,
    actor_id: &str,
    instrument: &ComputeCapacityInstrument,
) {
    store
        .adopt_compute_capacity_instrument_offer(adoption_input(
            offer,
            publication,
            actor_id,
            instrument,
            "adopt",
        ))
        .unwrap();
}

fn activation_input(
    actor_id: &str,
    instrument: &ComputeCapacityInstrument,
    suffix: &str,
) -> ActivateComputeCapacityInstrument {
    ActivateComputeCapacityInstrument {
        instrument_id: instrument.instrument_id.clone(),
        expected_instrument_revision: instrument.instrument_revision,
        expected_instrument_digest: instrument.instrument_digest.clone(),
        activated_by_admin_user_id: actor_id.into(),
        confirmation: COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_CONFIRMATION.into(),
        idempotency_scope: scope(suffix, &instrument.instrument_id),
        idempotency_key: suffix.into(),
    }
}

fn adoption_input(
    offer: &ComputeOffer,
    publication: &ComputeOfferPublicationReceipt,
    actor_id: &str,
    instrument: &ComputeCapacityInstrument,
    suffix: &str,
) -> AdoptComputeCapacityInstrumentOffer {
    AdoptComputeCapacityInstrumentOffer {
        instrument_id: instrument.instrument_id.clone(),
        expected_instrument_revision: instrument.instrument_revision,
        expected_instrument_digest: instrument.instrument_digest.clone(),
        offer_id: offer.offer_id.clone(),
        expected_offer_version: offer.offer_version,
        expected_offer_digest: offer.offer_digest.clone(),
        expected_publication_id: publication.publication_id.clone(),
        expected_publication_digest: publication.publication_digest.clone(),
        adopted_by_admin_user_id: actor_id.into(),
        confirmation: COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_CONFIRMATION.into(),
        idempotency_scope: scope(suffix, &instrument.instrument_id),
        idempotency_key: suffix.into(),
    }
}

fn scope(operation: &str, instrument_id: &str) -> String {
    format!("capacity-instrument-test:{operation}:{instrument_id}")
}
