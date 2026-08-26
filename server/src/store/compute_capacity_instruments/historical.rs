use anyhow::{anyhow, bail, Result};
use rusqlite::Connection;

use crate::compute_federation::capacity_instrument::{
    ComputeCapacityInstrument, ComputeCapacityInstrumentActivationReceipt,
    ComputeCapacityInstrumentOfferAdoptionReceipt,
};

use super::{
    super::compute_offer_publications::audited_historical_compute_offer_publication_on,
    read::{
        activation_by_instrument_on, historical_adoption_by_exact_offer_on, instrument_by_id_on,
    },
};

/// Exact historical owners needed by the capacity-future retained settlement bridge.
/// Retirement is deliberately not a read gate: a later retirement cannot erase an earlier
/// activation and exact Offer adoption.
pub(in crate::store) struct HistoricalCapacityInstrumentSettlementSource {
    instrument: ComputeCapacityInstrument,
    activation: ComputeCapacityInstrumentActivationReceipt,
    adoption: ComputeCapacityInstrumentOfferAdoptionReceipt,
}

impl HistoricalCapacityInstrumentSettlementSource {
    pub(in crate::store) fn instrument(&self) -> &ComputeCapacityInstrument {
        &self.instrument
    }

    pub(in crate::store) fn activation(&self) -> &ComputeCapacityInstrumentActivationReceipt {
        &self.activation
    }

    pub(in crate::store) fn adoption(&self) -> &ComputeCapacityInstrumentOfferAdoptionReceipt {
        &self.adoption
    }
}

pub(in crate::store) fn audited_historical_capacity_instrument_settlement_source_on(
    conn: &Connection,
    instrument_id: &str,
    offer_id: &str,
    offer_version: i64,
    offer_digest: &str,
) -> Result<Option<HistoricalCapacityInstrumentSettlementSource>> {
    let Some(root) = instrument_by_id_on(conn, instrument_id)? else {
        return Ok(None);
    };
    let activation = activation_by_instrument_on(conn, instrument_id)?
        .ok_or_else(|| anyhow!("historical CapacityInstrument lacks activation owner"))?;
    let adoption =
        historical_adoption_by_exact_offer_on(conn, offer_id, offer_version, offer_digest)?
            .ok_or_else(|| anyhow!("historical CapacityInstrument lacks exact Offer adoption"))?;
    let publication = audited_historical_compute_offer_publication_on(conn, offer_id)?
        .ok_or_else(|| anyhow!("historical CapacityInstrument adoption lacks publication"))?;

    if activation.activation.instrument_id != root.instrument.instrument_id
        || activation.activation.instrument_revision != root.instrument.instrument_revision
        || activation.activation.instrument_digest != root.instrument.instrument_digest
        || adoption.adoption.instrument_id != root.instrument.instrument_id
        || adoption.adoption.instrument_revision != root.instrument.instrument_revision
        || adoption.adoption.instrument_digest != root.instrument.instrument_digest
        || adoption.adoption.offer_id != offer_id
        || adoption.adoption.offer_version != offer_version
        || adoption.adoption.offer_digest != offer_digest
        || adoption.adoption.publication_id != publication.publication_id
        || adoption.adoption.publication_digest != publication.publication_digest
        || publication.offer_id != offer_id
        || publication.active_offer_version != offer_version
        || publication.active_offer_digest != offer_digest
    {
        bail!("historical CapacityInstrument activation/adoption/publication chain drifted");
    }

    Ok(Some(HistoricalCapacityInstrumentSettlementSource {
        instrument: root.instrument,
        activation: activation.activation,
        adoption: adoption.adoption,
    }))
}
