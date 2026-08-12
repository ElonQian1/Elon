use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, TransactionBehavior};

use crate::{
    compute_federation::{
        capacity_instrument::{
            validate_compute_capacity_instrument_offer_adoption_receipt,
            ComputeCapacityInstrumentOfferAdoptionReceipt,
            COMPUTE_CAPACITY_INSTRUMENT_CANONICALIZATION,
            COMPUTE_CAPACITY_INSTRUMENT_DIGEST_ALGORITHM,
            COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_CONFIRMATION,
            COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_RECEIPT_SCHEMA,
        },
        market::PRICING_MODE_CAPACITY_FUTURE,
        offer::{ComputeOffer, OFFER_STATUS_ACTIVE},
    },
    store::{new_id, Store},
};

use super::{
    canonical::canonical_adoption,
    read::{adoption_by_idempotency_on, adoption_by_offer_on, currentness_on},
    types::{
        AdoptComputeCapacityInstrumentOffer, ComputeCapacityInstrumentAdoptionAuthority,
        ComputeCapacityInstrumentOfferAdoptionWriteReceipt, StoredAdoption,
    },
    validation::{
        validate_contract_for_offer, validate_digest, validate_exact, validate_expected_identity,
    },
    write::now,
};
use crate::store::{
    compute_offer_publications::audited_compute_offer_publication_on,
    compute_offer_registry::current_registered_offer_on,
};

impl Store {
    pub(crate) fn adopt_compute_capacity_instrument_offer(
        &self,
        input: AdoptComputeCapacityInstrumentOffer,
    ) -> Result<ComputeCapacityInstrumentOfferAdoptionWriteReceipt> {
        validate_adoption_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) =
            adoption_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            validate_adoption_replay(&input, &existing)?;
            let instrument = exact_instrument_root(
                &tx,
                &input.instrument_id,
                input.expected_instrument_revision,
                &input.expected_instrument_digest,
            )?;
            tx.commit()?;
            return Ok(ComputeCapacityInstrumentOfferAdoptionWriteReceipt {
                instrument,
                adoption: existing.adoption,
                replayed: true,
            });
        }
        if adoption_by_offer_on(&tx, &input.offer_id)?.is_some() {
            bail!("Offer already has a different immutable capacity-instrument adoption");
        }
        let (instrument, _) = exact_active_instrument(
            &tx,
            &input.instrument_id,
            input.expected_instrument_revision,
            &input.expected_instrument_digest,
        )?;
        if input.adopted_by_admin_user_id == instrument.registered_by_admin_user_id {
            bail!("capacity-instrument adoption requires an actor distinct from registrar");
        }
        let offer = current_registered_offer_on(&tx, &input.offer_id)?
            .ok_or_else(|| anyhow!("capacity-instrument adoption Offer does not exist"))?;
        if offer.offer.offer_version != input.expected_offer_version
            || offer.offer.offer_digest != input.expected_offer_digest
            || offer.offer.status != OFFER_STATUS_ACTIVE
            || offer.offer.price_terms.pricing_mode != PRICING_MODE_CAPACITY_FUTURE
            || offer.offer.price_terms.instrument_id.as_deref()
                != Some(input.instrument_id.as_str())
        {
            bail!("only the current active exact capacity_future Offer can be adopted");
        }
        validate_contract_for_offer(&instrument, &offer.offer)?;
        let publication = require_exact_publication(&tx, &offer.offer)?;
        if publication.publication_id != input.expected_publication_id
            || publication.publication_digest != input.expected_publication_digest
        {
            bail!("capacity-instrument adoption publication identity is stale");
        }
        let occurred_at = now();
        let mut adoption = ComputeCapacityInstrumentOfferAdoptionReceipt {
            schema: COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_RECEIPT_SCHEMA.to_string(),
            adoption_receipt_id: new_id("compute_capacity_instrument_offer_adoption"),
            adoption_receipt_digest: String::new(),
            canonicalization: COMPUTE_CAPACITY_INSTRUMENT_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_CAPACITY_INSTRUMENT_DIGEST_ALGORITHM.to_string(),
            instrument_id: instrument.instrument_id.clone(),
            instrument_revision: instrument.instrument_revision,
            instrument_digest: instrument.instrument_digest.clone(),
            offer_id: offer.offer.offer_id,
            offer_version: offer.offer.offer_version,
            offer_digest: offer.offer.offer_digest,
            publication_id: publication.publication_id,
            publication_digest: publication.publication_digest,
            adopted_by_admin_user_id: input.adopted_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            adopted_at: occurred_at.clone(),
            recorded_at: occurred_at,
        };
        adoption.adoption_receipt_digest = canonical_adoption(&adoption)?.1;
        validate_compute_capacity_instrument_offer_adoption_receipt(&adoption)?;
        let (adoption_json, digest) = canonical_adoption(&adoption)?;
        if digest != adoption.adoption_receipt_digest {
            bail!("capacity-instrument adoption digest drifted before persistence");
        }
        insert_adoption(&tx, &adoption, &adoption_json)?;
        let stored = adoption_by_offer_on(&tx, &adoption.offer_id)?
            .ok_or_else(|| anyhow!("capacity-instrument adoption disappeared after insert"))?;
        if stored.adoption != adoption || stored.adoption_json != adoption_json {
            bail!("capacity-instrument adoption write failed exact readback");
        }
        tx.commit()?;
        Ok(ComputeCapacityInstrumentOfferAdoptionWriteReceipt {
            instrument,
            adoption,
            replayed: false,
        })
    }
}

/// Current authority gate used inside the caller's existing transaction.
pub(in crate::store) fn require_current_capacity_instrument_adoption_on(
    conn: &Connection,
    offer: &ComputeOffer,
    instrument_id: Option<&str>,
) -> Result<Option<ComputeCapacityInstrumentAdoptionAuthority>> {
    if offer.price_terms.pricing_mode != PRICING_MODE_CAPACITY_FUTURE {
        return Ok(None);
    }
    let current = current_registered_offer_on(conn, &offer.offer_id)?
        .ok_or_else(|| anyhow!("capacity_future current Offer does not exist"))?;
    if current.offer.offer_version != offer.offer_version
        || current.offer.offer_digest != offer.offer_digest
        || current.offer != *offer
    {
        bail!("capacity_future consumer does not bind the current exact Offer");
    }
    require_capacity_instrument_adoption_for_historical_offer_on(conn, offer, instrument_id)
}

/// Preserves exercise of an immutable formerly-active Offer after its current projection drains.
pub(in crate::store) fn require_capacity_instrument_adoption_for_historical_offer_on(
    conn: &Connection,
    offer: &ComputeOffer,
    instrument_id: Option<&str>,
) -> Result<Option<ComputeCapacityInstrumentAdoptionAuthority>> {
    if offer.price_terms.pricing_mode != PRICING_MODE_CAPACITY_FUTURE {
        return Ok(None);
    }
    let instrument_id = instrument_id
        .ok_or_else(|| anyhow!("capacity_future consumer lacks a capacity instrument ID"))?;
    if offer.price_terms.instrument_id.as_deref() != Some(instrument_id) {
        bail!("capacity_future consumer instrument ID differs from Offer");
    }
    let currentness = currentness_on(conn, instrument_id)?
        .ok_or_else(|| anyhow!("capacity_future capacity instrument does not exist"))?;
    if currentness.current_status
        != crate::compute_federation::capacity_instrument::COMPUTE_CAPACITY_INSTRUMENT_STATUS_ACTIVE
        || currentness.retirement.is_some()
    {
        bail!("capacity_future capacity instrument is not current active");
    }
    validate_contract_for_offer(&currentness.instrument, offer)?;
    let activation = currentness
        .activation
        .ok_or_else(|| anyhow!("capacity_future active instrument lacks activation"))?;
    let adoption = adoption_by_offer_on(conn, &offer.offer_id)?
        .ok_or_else(|| anyhow!("capacity_future Offer lacks exact instrument adoption"))?
        .adoption;
    let publication = require_exact_publication(conn, offer)?;
    if adoption.instrument_id != currentness.instrument.instrument_id
        || adoption.instrument_revision != currentness.instrument.instrument_revision
        || adoption.instrument_digest != currentness.instrument.instrument_digest
        || adoption.offer_id != offer.offer_id
        || adoption.offer_version != offer.offer_version
        || adoption.offer_digest != offer.offer_digest
        || adoption.publication_id != publication.publication_id
        || adoption.publication_digest != publication.publication_digest
    {
        bail!("capacity_future adoption no longer matches current exact authorities");
    }
    Ok(Some(ComputeCapacityInstrumentAdoptionAuthority {
        instrument: currentness.instrument,
        activation,
        adoption,
    }))
}

fn exact_active_instrument(
    conn: &Connection,
    instrument_id: &str,
    revision: i64,
    digest: &str,
) -> Result<(
    crate::compute_federation::capacity_instrument::ComputeCapacityInstrument,
    crate::compute_federation::capacity_instrument::ComputeCapacityInstrumentActivationReceipt,
)> {
    let current = currentness_on(conn, instrument_id)?
        .ok_or_else(|| anyhow!("capacity instrument does not exist"))?;
    if current.instrument.instrument_revision != revision
        || current.instrument.instrument_digest != digest
        || current.current_status
            != crate::compute_federation::capacity_instrument::COMPUTE_CAPACITY_INSTRUMENT_STATUS_ACTIVE
        || current.retirement.is_some()
    {
        bail!("capacity instrument is not current active at the expected revision and digest");
    }
    let activation = current
        .activation
        .ok_or_else(|| anyhow!("active capacity instrument lacks activation"))?;
    Ok((current.instrument, activation))
}

fn exact_instrument_root(
    conn: &Connection,
    instrument_id: &str,
    revision: i64,
    digest: &str,
) -> Result<crate::compute_federation::capacity_instrument::ComputeCapacityInstrument> {
    let current = currentness_on(conn, instrument_id)?
        .ok_or_else(|| anyhow!("capacity instrument does not exist"))?;
    if current.instrument.instrument_revision != revision
        || current.instrument.instrument_digest != digest
    {
        bail!("capacity instrument expected revision or digest is stale");
    }
    Ok(current.instrument)
}

fn require_exact_publication(
    conn: &Connection,
    offer: &ComputeOffer,
) -> Result<crate::compute_federation_offer_publication_model::ComputeOfferPublicationReceipt> {
    let publication = audited_compute_offer_publication_on(conn, &offer.offer_id)?
        .ok_or_else(|| anyhow!("capacity-instrument Offer lacks publication authority"))?;
    if publication.offer_id != offer.offer_id
        || publication.provider_id != offer.provider_id
        || publication.pool_id != offer.capacity_pool.pool_id
        || publication.active_offer_version != offer.offer_version
        || publication.active_offer_digest != offer.offer_digest
        || publication.source_offer_version <= 0
        || publication.source_offer_digest.len() != 64
        || publication.provider_policy_revision <= 0
        || publication.provider_digest.len() != 64
        || publication.approved_by_user_id.is_empty()
        || publication.published_at.is_empty()
    {
        bail!("capacity-instrument publication does not bind the exact Offer authority");
    }
    Ok(publication)
}

fn validate_adoption_input(input: &AdoptComputeCapacityInstrumentOffer) -> Result<()> {
    validate_expected_identity(
        &input.instrument_id,
        input.expected_instrument_revision,
        &input.expected_instrument_digest,
    )?;
    for (value, label, maximum) in [
        (&input.offer_id, "Offer ID", 200),
        (&input.expected_publication_id, "publication ID", 200),
        (&input.adopted_by_admin_user_id, "adoption actor", 200),
        (&input.idempotency_scope, "idempotency scope", 200),
        (&input.idempotency_key, "idempotency key", 200),
    ] {
        validate_exact(value, label, maximum)?;
    }
    validate_digest(&input.expected_offer_digest, "Offer digest")?;
    validate_digest(&input.expected_publication_digest, "publication digest")?;
    if !(1..=9_007_199_254_740_991).contains(&input.expected_offer_version)
        || input.confirmation != COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_CONFIRMATION
    {
        bail!("capacity-instrument adoption version or confirmation is invalid");
    }
    Ok(())
}

fn validate_adoption_replay(
    input: &AdoptComputeCapacityInstrumentOffer,
    stored: &StoredAdoption,
) -> Result<()> {
    let value = &stored.adoption;
    if value.instrument_id != input.instrument_id
        || value.instrument_revision != input.expected_instrument_revision
        || value.instrument_digest != input.expected_instrument_digest
        || value.offer_id != input.offer_id
        || value.offer_version != input.expected_offer_version
        || value.offer_digest != input.expected_offer_digest
        || value.publication_id != input.expected_publication_id
        || value.publication_digest != input.expected_publication_digest
        || value.adopted_by_admin_user_id != input.adopted_by_admin_user_id
        || value.confirmation != input.confirmation
    {
        bail!("capacity-instrument adoption idempotency key binds different input");
    }
    Ok(())
}

fn insert_adoption(
    conn: &Connection,
    value: &ComputeCapacityInstrumentOfferAdoptionReceipt,
    json: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_capacity_instrument_offer_adoptions (
            adoption_receipt_id,adoption_schema,adoption_receipt_digest,adoption_receipt_json,
            canonicalization,digest_algorithm,instrument_id,instrument_revision,instrument_digest,
            offer_id,offer_version,offer_digest,publication_id,publication_digest,
            adopted_by_admin_user_id,confirmation,idempotency_scope,idempotency_key,
            adopted_at,recorded_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
        params![
            value.adoption_receipt_id,
            value.schema,
            value.adoption_receipt_digest,
            json,
            value.canonicalization,
            value.digest_algorithm,
            value.instrument_id,
            value.instrument_revision,
            value.instrument_digest,
            value.offer_id,
            value.offer_version,
            value.offer_digest,
            value.publication_id,
            value.publication_digest,
            value.adopted_by_admin_user_id,
            value.confirmation,
            value.idempotency_scope,
            value.idempotency_key,
            value.adopted_at,
            value.recorded_at,
        ],
    )?;
    Ok(())
}
