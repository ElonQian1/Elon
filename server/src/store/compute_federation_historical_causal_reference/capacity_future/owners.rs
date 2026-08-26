use anyhow::{anyhow, bail, ensure, Result};
use rusqlite::Connection;

use crate::{
    compute_federation::{
        capacity_future_settlement_lineage::{
            ComputeCapacityFutureSettlementLineageSources,
            ComputeCapacityFutureSettlementStageSources,
            UntrustedCapacityFutureAttemptSettlementAuditView,
        },
        federation_historical_causal_reference::{
            federation_historical_causal_reference_from_json, FederationHistoricalLineageV1,
            UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
        },
        market::{ComputePriceSnapshot, PRICING_MODE_CAPACITY_FUTURE},
    },
    store::{
        compute_attempt_execution_receipts::{
            compute_attempt_historical_execution_receipt_by_lease_on,
            ComputeAttemptExecutionReceiptEnvelope,
        },
        compute_attempt_settlement_releases::compute_attempt_historical_settlement_release_by_lease_on,
        compute_attempt_settlements::{
            compute_attempt_historical_settlement_by_lease_on, ComputeAttemptSettlementReceipt,
        },
        compute_capacity_instruments::{
            audited_historical_capacity_instrument_settlement_source_on,
            HistoricalCapacityInstrumentSettlementSource,
        },
        compute_delivery_allocations::{
            audited_historical_delivery_allocation_settlement_source_on,
            HistoricalDeliveryAllocationSettlementSource,
        },
        compute_price_snapshot_registry::registered_historical_price_snapshot_on,
    },
};

use super::super::{
    execution, release, settlement, verification, FederationHistoricalLineageAccessScope,
    ValidatedFederationHistoricalLineage,
};

pub(super) struct ResolvedCapacityFutureSettlementOwners {
    instrument: HistoricalCapacityInstrumentSettlementSource,
    allocation: HistoricalDeliveryAllocationSettlementSource,
    price_snapshot: ComputePriceSnapshot,
    execution: ComputeAttemptExecutionReceiptEnvelope,
    attempt_settlement: ComputeAttemptSettlementReceipt,
    execution_source: UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    execution_verification_source: UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    settlement_source: UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    settlement_release_source: Option<UntrustedFederationHistoricalCausalReferenceEnvelopeV1>,
    access_scope: FederationHistoricalLineageAccessScope,
}

impl ResolvedCapacityFutureSettlementOwners {
    pub(super) fn instrument_source(&self) -> &HistoricalCapacityInstrumentSettlementSource {
        &self.instrument
    }

    pub(super) fn allocation_source(&self) -> &HistoricalDeliveryAllocationSettlementSource {
        &self.allocation
    }

    pub(super) fn sources(&self) -> ComputeCapacityFutureSettlementLineageSources<'_> {
        let commitment = self.allocation.authority().transfer().commitment();
        let settlement_stage = match &self.settlement_release_source {
            Some(settlement_release_source) => {
                ComputeCapacityFutureSettlementStageSources::AvailableReleaseSource {
                    settlement_source: &self.settlement_source,
                    settlement_release_source,
                }
            }
            None => ComputeCapacityFutureSettlementStageSources::PendingSettlementSource {
                settlement_source: &self.settlement_source,
            },
        };
        ComputeCapacityFutureSettlementLineageSources {
            instrument: self.instrument.instrument(),
            instrument_activation: self.instrument.activation(),
            instrument_offer_adoption: self.instrument.adoption(),
            commitment,
            delivery_allocation_grant: self.allocation.grant(),
            delivery_allocation_exercise: self.allocation.terminal(),
            price_snapshot: &self.price_snapshot,
            execution_receipt: &self.execution.receipt,
            attempt_settlement: UntrustedCapacityFutureAttemptSettlementAuditView {
                settlement: &self.attempt_settlement.settlement,
                settlement_event_digest: &self.attempt_settlement.event_digest,
                lease_id: &self.attempt_settlement.lease_id,
                finalization_id: &self.attempt_settlement.finalization_id,
                finalization_event_digest: &self.attempt_settlement.finalization_event_digest,
                budget_reservation_id: &self.attempt_settlement.budget_reservation_id,
                budget_reserved_fen: self.attempt_settlement.budget_reserved_fen,
                provider_policy_revision: self.attempt_settlement.provider_policy_revision,
                provider_digest: &self.attempt_settlement.provider_digest,
                source_job: &self.attempt_settlement.source_job,
                terminal_job: &self.attempt_settlement.terminal_job,
            },
            execution_source: &self.execution_source,
            execution_verification_source: &self.execution_verification_source,
            settlement_stage,
        }
    }

    pub(super) fn into_access_scope(self) -> FederationHistoricalLineageAccessScope {
        self.access_scope
    }
}

pub(super) fn resolve_capacity_future_settlement_owners_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<ResolvedCapacityFutureSettlementOwners>> {
    let Some(attempt_settlement) =
        compute_attempt_historical_settlement_by_lease_on(conn, lease_id)?
    else {
        return Ok(None);
    };
    let price_snapshot = registered_historical_price_snapshot_on(
        conn,
        &attempt_settlement.settlement.price_snapshot_id,
    )?
    .ok_or_else(|| anyhow!("capacity-future settlement lacks historical Price Snapshot"))?;
    if price_snapshot.pricing_mode != PRICING_MODE_CAPACITY_FUTURE {
        return Ok(None);
    }

    let execution_receipt =
        compute_attempt_historical_execution_receipt_by_lease_on(conn, lease_id)?
            .ok_or_else(|| anyhow!("capacity-future settlement lacks historical v193 owner"))?;
    let execution_validated = execution::resolve_execution_source_lineage_on(
        conn,
        &execution_receipt.receipt.receipt_id,
        &execution_receipt.receipt.receipt_digest,
    )?;
    let verification_validated =
        verification::resolve_execution_verification_source_lineage_on(conn, &execution_receipt)?;
    let settlement_validated = settlement::resolve_settlement_source_lineage_on(
        conn,
        &attempt_settlement.settlement.settlement_receipt_id,
        &attempt_settlement.settlement.settlement_receipt_digest,
        &attempt_settlement.event_digest,
    )?;

    let (execution_source, execution_scope) = into_carrier_and_scope(execution_validated)?;
    let (execution_verification_source, verification_scope) =
        into_carrier_and_scope(verification_validated)?;
    let (settlement_source, settlement_scope) = into_carrier_and_scope(settlement_validated)?;
    settlement_scope.ensure_same_as(&execution_scope)?;
    settlement_scope.ensure_same_as(&verification_scope)?;

    let settlement_release_source =
        match compute_attempt_historical_settlement_release_by_lease_on(conn, lease_id)? {
            Some(release_receipt) => {
                let validated =
                    release::resolve_settlement_release_source_lineage_on(conn, &release_receipt)?;
                let (carrier, release_scope) = into_carrier_and_scope(validated)?;
                settlement_scope.ensure_same_as(&release_scope)?;
                Some(carrier)
            }
            None => None,
        };

    let execution_lineage = match execution_source.lineage() {
        FederationHistoricalLineageV1::ExecutionSource(lineage) => lineage,
        _ => bail!("capacity-future execution owner did not resolve as execution_source_v1"),
    };
    let Some(allocation) = audited_historical_delivery_allocation_settlement_source_on(
        conn,
        &execution_lineage.reservation.reservation_id,
        &execution_lineage.capacity_claim.claim_id,
    )?
    else {
        return Ok(None);
    };
    let commitment = allocation.authority().transfer().commitment();
    let instrument = audited_historical_capacity_instrument_settlement_source_on(
        conn,
        &commitment.instrument_id,
        &commitment.offer.offer_id,
        commitment.offer.offer_version,
        &commitment.offer.offer_digest,
    )?
    .ok_or_else(|| anyhow!("capacity-future Commitment lacks historical CapacityInstrument"))?;

    ensure!(
        attempt_settlement.lease_id == lease_id
            && execution_receipt.receipt.attempt_lease_id == lease_id,
        "capacity-future Lease root drifted across v193 and v195 owners"
    );
    Ok(Some(ResolvedCapacityFutureSettlementOwners {
        instrument,
        allocation,
        price_snapshot,
        execution: execution_receipt,
        attempt_settlement,
        execution_source,
        execution_verification_source,
        settlement_source,
        settlement_release_source,
        access_scope: settlement_scope,
    }))
}

fn into_carrier_and_scope(
    validated: ValidatedFederationHistoricalLineage,
) -> Result<(
    UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    FederationHistoricalLineageAccessScope,
)> {
    let expected_digest = validated.lineage_digest().to_string();
    let carrier = federation_historical_causal_reference_from_json(validated.canonical_json())?;
    let (sealed_digest, scope) = validated.into_lineage_digest_and_access_scope();
    ensure!(
        carrier.lineage_digest() == expected_digest && sealed_digest == expected_digest,
        "retained F0 carrier digest drifted while sealing capacity-future owners"
    );
    Ok((carrier, scope))
}
