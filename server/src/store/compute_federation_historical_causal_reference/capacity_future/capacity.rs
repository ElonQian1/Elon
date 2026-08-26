use anyhow::{bail, ensure, Result};

use crate::{
    compute_federation::capacity_commitment::ComputeCapacityCommitmentQuantity,
    store::{
        compute_capacity_commitments::validate_contract_multiple,
        compute_capacity_instruments::HistoricalCapacityInstrumentSettlementSource,
        compute_delivery_allocations::HistoricalDeliveryAllocationSettlementSource,
    },
};

pub(super) fn validate_capacity_future_historical_owners(
    instrument: &HistoricalCapacityInstrumentSettlementSource,
    allocation: &HistoricalDeliveryAllocationSettlementSource,
) -> Result<()> {
    let authority = allocation.authority();
    let transfer = authority.transfer();
    let commitment = transfer.commitment();
    let parent = authority.parent_claim();
    let parent_result = authority.parent_result_claim();
    let child = authority.child_claim();

    ensure!(
        commitment.instrument_id == instrument.instrument().instrument_id,
        "capacity-future Commitment references a different CapacityInstrument"
    );
    ensure!(
        allocation.grant() == transfer.grant(),
        "capacity-future DeliveryAllocation Grant drifted from sealed transfer authority"
    );
    ensure!(
        commitment.claim.claim_id == parent.claim_id
            && commitment.claim.claim_revision == parent.revision
            && commitment.claim.claim_digest == parent.claim_digest,
        "capacity-future Commitment Claim root drifted from historical v228 authority"
    );
    ensure!(
        parent.lines == parent_result.lines && parent.lines == child.lines,
        "capacity-future whole-only parent release and child hold are not conserved"
    );

    let quantities = parent
        .lines
        .iter()
        .map(|line| ComputeCapacityCommitmentQuantity {
            meter: line.bucket.meter.clone(),
            quantity_units: line.quantity_units,
        })
        .collect::<Vec<_>>();
    validate_contract_multiple(&quantities, &instrument.instrument().contract_units)?;

    if parent.lines.len() != instrument.instrument().contract_units.len() {
        bail!("capacity-future Claim and CapacityInstrument contract unit counts differ");
    }
    for (index, (line, unit)) in parent
        .lines
        .iter()
        .zip(&instrument.instrument().contract_units)
        .enumerate()
    {
        if line.line_no != index as i64 + 1
            || line.bucket.meter != unit.meter
            || line.bucket.quantum_units != unit.unit_size
        {
            bail!(
                "capacity-future Claim meter order or unit granularity differs from CapacityInstrument"
            );
        }
    }
    Ok(())
}
