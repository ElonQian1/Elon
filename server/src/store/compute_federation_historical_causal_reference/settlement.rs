use anyhow::{anyhow, bail, Result};
use rusqlite::Connection;

use crate::{
    compute_federation::{
        execution::ComputeJobVersionBinding,
        federation_historical_causal_reference::{
            build_settlement_source_carrier, SettlementSourceLineageV1,
        },
    },
    store::{
        compute_attempt_execution_receipts::compute_attempt_execution_receipt_by_id_on,
        compute_attempt_finalizations::compute_attempt_historical_finalization_on,
        compute_attempt_settlements::compute_attempt_settlement_by_receipt_id_on,
        compute_job_registry::registered_historical_job_version_on,
        compute_offer_registry::registered_historical_offer_version_on,
        compute_price_snapshot_registry::registered_historical_price_snapshot_on,
        compute_provider_registry::registered_provider_version_on,
        compute_reservation_registry::registered_historical_reservation_version_on,
    },
};

use super::{
    execution::resolve_execution_source_lineage_on,
    source_refs::{
        execution_receipt_ref, finalization_ref, job_ref, price_snapshot_ref, provider_ref,
        reservation_ref, settlement_ref, validate_settlement_source_links,
        SettlementSourceLinkFacts,
    },
    FederationHistoricalLineageAccessScope, ValidatedFederationHistoricalLineage,
};

pub(super) fn resolve_settlement_source_lineage_on(
    conn: &Connection,
    settlement_receipt_id: &str,
    settlement_receipt_digest: &str,
    settlement_event_digest: &str,
) -> Result<ValidatedFederationHistoricalLineage> {
    validate_root_triple(
        settlement_receipt_id,
        settlement_receipt_digest,
        settlement_event_digest,
    )?;
    let settlement = compute_attempt_settlement_by_receipt_id_on(conn, settlement_receipt_id)?
        .ok_or_else(|| anyhow!("Settlement Receipt exact root does not exist"))?;
    if settlement.settlement.settlement_receipt_id != settlement_receipt_id
        || settlement.settlement.settlement_receipt_digest != settlement_receipt_digest
        || settlement.event_digest != settlement_event_digest
    {
        bail!("Settlement Receipt exact root triple does not match retained v195 owner data");
    }

    let rebuilt_execution = resolve_execution_source_lineage_on(
        conn,
        &settlement.settlement.execution_receipt_id,
        &settlement.settlement.execution_receipt_digest,
    )?;
    let execution = compute_attempt_execution_receipt_by_id_on(
        conn,
        &settlement.settlement.execution_receipt_id,
    )?
    .ok_or_else(|| anyhow!("Settlement source Execution Receipt does not exist"))?;
    let finalization = compute_attempt_historical_finalization_on(conn, &settlement.lease_id)?;
    let source_job = registered_historical_job_version_on(
        conn,
        &settlement.source_job.job_id,
        settlement.source_job.job_revision,
    )?
    .ok_or_else(|| anyhow!("Settlement source verification-pending Job does not exist"))?;
    let terminal_job = registered_historical_job_version_on(
        conn,
        &settlement.terminal_job.job_id,
        settlement.terminal_job.job_revision,
    )?
    .ok_or_else(|| anyhow!("Settlement source settled Job does not exist"))?;
    let terminal_reservation = registered_historical_reservation_version_on(
        conn,
        &settlement.settlement.reservation_id,
        finalization.terminal_reservation.revision,
    )?
    .ok_or_else(|| anyhow!("Settlement source terminal Reservation does not exist"))?;
    let snapshot =
        registered_historical_price_snapshot_on(conn, &settlement.settlement.price_snapshot_id)?
            .ok_or_else(|| anyhow!("Settlement source Price Snapshot does not exist"))?;
    let offer =
        registered_historical_offer_version_on(conn, &snapshot.offer_id, snapshot.offer_version)?
            .ok_or_else(|| anyhow!("Settlement source historical Offer does not exist"))?;
    let provider = registered_provider_version_on(
        conn,
        &offer.offer.provider_id,
        offer.provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("Settlement source historical Provider does not exist"))?;
    let access_scope = FederationHistoricalLineageAccessScope::from_historical_job_and_provider(
        &source_job.job.consumer_account_id,
        source_job.job.project_id.as_deref(),
        &provider.provider.owner_account_id,
    )?;
    access_scope.ensure_job_matches(
        &terminal_job.job.consumer_account_id,
        terminal_job.job.project_id.as_deref(),
    )?;
    access_scope.ensure_same_as(rebuilt_execution.access_scope())?;

    if finalization.finalization_id != settlement.finalization_id
        || finalization.event_digest != settlement.finalization_event_digest
        || finalization.execution_receipt_id != execution.receipt.receipt_id
        || finalization.execution_receipt_digest != execution.receipt.receipt_digest
        || finalization.lease_id != settlement.lease_id
        || execution.receipt.attempt_lease_id != settlement.lease_id
        || source_job.job_digest != settlement.source_job.job_digest
        || terminal_job.job_digest != settlement.terminal_job.job_digest
        || terminal_reservation.reservation_digest != finalization.terminal_reservation.digest
        || snapshot.snapshot_digest != settlement.settlement.price_snapshot_digest
        || offer.offer.provider_id != execution.receipt.provider_id
        || offer.provider_policy_revision != settlement.provider_policy_revision
        || offer.provider_digest != settlement.provider_digest
        || provider.provider.policy_revision != settlement.provider_policy_revision
        || provider.provider_digest != settlement.provider_digest
    {
        bail!("Settlement source retained owner bodies do not form one exact historical chain");
    }

    let attempt_settlement = settlement_ref(
        &settlement.settlement.settlement_receipt_id,
        &settlement.settlement.settlement_receipt_digest,
        &settlement.event_digest,
    );
    let execution_receipt = execution_receipt_ref(
        &execution.receipt.receipt_id,
        &execution.receipt.receipt_digest,
    );
    let finalization_lineage =
        finalization_ref(&finalization.finalization_id, &finalization.event_digest);
    let price_snapshot = price_snapshot_ref(&snapshot.snapshot_id, &snapshot.snapshot_digest);
    let provider_lineage = provider_ref(
        &offer.offer.provider_id,
        offer.provider_policy_revision,
        &offer.provider_digest,
    )?;
    let source_job_lineage = job_ref(&settlement.source_job)?;
    let terminal_job_lineage = job_ref(&settlement.terminal_job)?;
    let terminal_reservation_lineage = reservation_ref(
        &terminal_reservation.reservation.reservation_id,
        terminal_reservation.revision,
        &terminal_reservation.reservation_digest,
    )?;
    let lineage = SettlementSourceLineageV1 {
        attempt_settlement,
        execution_receipt,
        execution_lineage_digest: rebuilt_execution.lineage_digest().to_string(),
        finalization: finalization_lineage,
        price_snapshot,
        provider: provider_lineage,
        source_job: source_job_lineage,
        terminal_job: terminal_job_lineage,
        terminal_reservation: terminal_reservation_lineage,
    };
    let facts = SettlementSourceLinkFacts {
        audited_attempt_settlement: settlement_ref(
            &settlement.settlement.settlement_receipt_id,
            &settlement.settlement.settlement_receipt_digest,
            &settlement.event_digest,
        ),
        rebuilt_execution_receipt: execution_receipt_ref(
            &execution.receipt.receipt_id,
            &execution.receipt.receipt_digest,
        ),
        rebuilt_execution_lineage_digest: rebuilt_execution.lineage_digest().to_string(),
        settlement_execution_receipt: execution_receipt_ref(
            &settlement.settlement.execution_receipt_id,
            &settlement.settlement.execution_receipt_digest,
        ),
        audited_finalization: finalization_ref(
            &finalization.finalization_id,
            &finalization.event_digest,
        ),
        finalization_execution_receipt: execution_receipt_ref(
            &finalization.execution_receipt_id,
            &finalization.execution_receipt_digest,
        ),
        finalization_provider_id: finalization.provider_id.clone(),
        finalization_source_job: job_ref(&finalization.source_job)?,
        finalization_terminal_job: job_ref(&finalization.terminal_job)?,
        finalization_terminal_reservation: reservation_ref(
            &terminal_reservation.reservation.reservation_id,
            finalization.terminal_reservation.revision,
            &finalization.terminal_reservation.digest,
        )?,
        settlement_price_snapshot: price_snapshot_ref(
            &settlement.settlement.price_snapshot_id,
            &settlement.settlement.price_snapshot_digest,
        ),
        audited_provider: provider_ref(
            &provider.provider.provider_id,
            provider.provider.policy_revision,
            &provider.provider_digest,
        )?,
        settlement_provider: provider_ref(
            &offer.offer.provider_id,
            settlement.provider_policy_revision,
            &settlement.provider_digest,
        )?,
        execution_provider_id: execution.receipt.provider_id.clone(),
        settlement_source_job: job_ref(&settlement.source_job)?,
        settlement_terminal_job: job_ref(&settlement.terminal_job)?,
        settlement_reservation_id: settlement.settlement.reservation_id.clone(),
        execution_reservation_id: execution.receipt.reservation_id.clone(),
        settlement_lease_id: settlement.lease_id.clone(),
        execution_lease_id: execution.receipt.attempt_lease_id.clone(),
        finalization_lease_id: finalization.lease_id.clone(),
        source_job_status: source_job.job.status,
        terminal_job_status: terminal_job.job.status,
        settlement_balance_state: settlement.settlement.balance_state.clone(),
        lineage,
    };
    validate_settlement_source_links(&facts)?;
    ValidatedFederationHistoricalLineage::from_carrier(
        build_settlement_source_carrier(facts.lineage)?,
        access_scope,
    )
}

fn validate_root_triple(
    settlement_receipt_id: &str,
    settlement_receipt_digest: &str,
    settlement_event_digest: &str,
) -> Result<()> {
    if settlement_receipt_id.trim().is_empty()
        || settlement_receipt_id != settlement_receipt_id.trim()
        || settlement_receipt_id.len() > 240
        || settlement_receipt_id.chars().any(char::is_control)
    {
        bail!("Settlement Receipt ID is invalid");
    }
    for (label, digest) in [
        ("Settlement Receipt digest", settlement_receipt_digest),
        ("Settlement event digest", settlement_event_digest),
    ] {
        if digest.len() != 64
            || digest != digest.to_ascii_lowercase()
            || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            bail!("{label} must be a lowercase SHA-256 digest");
        }
    }
    Ok(())
}
