use std::collections::BTreeMap;

use anyhow::{bail, Result};

use crate::compute_federation::{
    capacity_instrument::{
        ComputeCapacityInstrument, COMPUTE_CAPACITY_INSTRUMENT_REVISION,
        COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_CURRENCY,
        COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_UNIT,
    },
    offer::ComputeOffer,
};

pub(super) fn validate_exact(value: &str, label: &str, maximum: usize) -> Result<()> {
    if value.trim() != value
        || value.is_empty()
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}

pub(super) fn validate_expected_identity(
    instrument_id: &str,
    revision: i64,
    digest: &str,
) -> Result<()> {
    validate_exact(instrument_id, "capacity instrument ID", 200)?;
    validate_digest(digest, "capacity instrument digest")?;
    if revision != COMPUTE_CAPACITY_INSTRUMENT_REVISION {
        bail!("capacity instrument revision must be the frozen revision");
    }
    Ok(())
}

pub(super) fn validate_contract_for_offer(
    instrument: &ComputeCapacityInstrument,
    offer: &ComputeOffer,
) -> Result<()> {
    if instrument.sku_id != offer.sku.sku_id
        || instrument.sku_digest != offer.sku.sku_digest
        || instrument.availability_sla_tier != offer.sku.sla_tier
        || instrument.region_or_data_zone != offer.sku.region_or_data_zone
        || instrument.verification_tier != offer.sku.verification_tier
        || instrument.settlement_currency != COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_CURRENCY
        || instrument.settlement_unit != COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_UNIT
        || offer.price_terms.currency != COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_CURRENCY
    {
        bail!("capacity instrument does not match the Offer SKU and settlement contract");
    }
    if !offer
        .delivery_windows
        .iter()
        .any(|window| window == &instrument.delivery_window)
    {
        bail!("capacity instrument delivery window is not exact for the Offer");
    }
    let components = offer
        .price_terms
        .components
        .iter()
        .map(|component| (component.meter.as_str(), component))
        .collect::<BTreeMap<_, _>>();
    if components.len() != offer.price_terms.components.len()
        || components.len() != instrument.contract_units.len()
    {
        bail!("capacity instrument and Offer price meter sets differ");
    }
    for unit in &instrument.contract_units {
        let component = components
            .get(unit.meter.as_str())
            .ok_or_else(|| anyhow::anyhow!("capacity instrument meter is absent from Offer"))?;
        if unit.unit_size != component.unit_size {
            bail!("capacity instrument contract units do not match Offer price units");
        }
        let capacity = offer
            .capacity
            .iter()
            .find(|line| {
                line.bucket.delivery_window == instrument.delivery_window.binding
                    && line.bucket.meter == unit.meter
            })
            .ok_or_else(|| anyhow::anyhow!("Offer lacks an exact instrument capacity line"))?;
        if capacity.bucket.quantum_units != unit.unit_size
            || unit.quantity_units <= 0
            || unit.quantity_units % capacity.bucket.quantum_units != 0
            || unit.quantity_units > capacity.reservable_units
        {
            bail!("capacity instrument quantity is incompatible with Offer capacity");
        }
    }
    Ok(())
}
