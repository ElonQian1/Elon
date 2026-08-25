use anyhow::{ensure, Context, Result};

use super::super::receipts::{BALANCE_STATE_PENDING, COMPUTE_SETTLEMENT_RECEIPT_SCHEMA};

use super::{
    source_inputs::ComputeCapacityFutureSettlementLineageSources,
    source_support::{positive_u64, settlement_source_carrier, CanonicalSourceLineages},
    types::COMPUTE_CAPACITY_FUTURE_SETTLEMENT_CURRENCY,
};

pub(super) fn validate_settlement_source_equations(
    sources: &ComputeCapacityFutureSettlementLineageSources<'_>,
    canonical_sources: &CanonicalSourceLineages<'_>,
) -> Result<()> {
    let execution = canonical_sources.execution;
    let settlement = canonical_sources.settlement;
    let audit = &sources.attempt_settlement;
    let receipt = audit.settlement;
    let exercise = sources
        .delivery_allocation_exercise
        .exercise
        .as_ref()
        .context("capacity-future bridge requires exercise evidence")?;
    ensure!(
        settlement.execution_receipt == execution.execution_receipt
            && settlement.execution_lineage_digest == sources.execution_source.lineage_digest()
            && settlement.price_snapshot == execution.price_snapshot
            && settlement.provider == execution.provider
            && settlement.source_job.job_id == execution.job.job_id
            && execution.job.job_revision.checked_add(1)
                == Some(settlement.source_job.job_revision)
            && settlement.terminal_job.job_id == execution.job.job_id
            && settlement.source_job.job_revision.checked_add(1)
                == Some(settlement.terminal_job.job_revision)
            && settlement.terminal_reservation.reservation_id
                == execution.reservation.reservation_id
            && execution.reservation.reservation_revision.checked_add(1)
                == Some(settlement.terminal_reservation.reservation_revision),
        "capacity-future settlement source does not close execution lineage"
    );
    ensure!(
        audit.settlement_event_digest == settlement.attempt_settlement.settlement_event_digest
            && audit.lease_id == execution.attempt_lease_source.lease_id
            && audit.finalization_id == settlement.finalization.finalization_id
            && audit.finalization_event_digest == settlement.finalization.finalization_event_digest
            && audit.budget_reservation_id == exercise.budget_reservation_id
            && audit.budget_reserved_fen == exercise.reserved_amount_fen
            && positive_u64(audit.provider_policy_revision, "v195 Provider revision")?
                == settlement.provider.policy_revision
            && audit.provider_digest == settlement.provider.provider_digest
            && audit.source_job.job_id == settlement.source_job.job_id
            && positive_u64(audit.source_job.job_revision, "v195 source Job revision")?
                == settlement.source_job.job_revision
            && audit.source_job.job_digest == settlement.source_job.job_digest
            && audit.terminal_job.job_id == settlement.terminal_job.job_id
            && positive_u64(
                audit.terminal_job.job_revision,
                "v195 terminal Job revision"
            )? == settlement.terminal_job.job_revision
            && audit.terminal_job.job_digest == settlement.terminal_job.job_digest,
        "capacity-future v195 audit view differs from settlement source"
    );
    ensure!(
        receipt.schema == COMPUTE_SETTLEMENT_RECEIPT_SCHEMA
            && receipt.settlement_receipt_id == settlement.attempt_settlement.settlement_receipt_id
            && receipt.settlement_receipt_digest
                == settlement.attempt_settlement.settlement_receipt_digest
            && receipt.execution_receipt_id == execution.execution_receipt.execution_receipt_id
            && receipt.execution_receipt_digest
                == execution.execution_receipt.execution_receipt_digest
            && receipt.reservation_id == settlement.terminal_reservation.reservation_id
            && receipt.price_snapshot_id == execution.price_snapshot.price_snapshot_id
            && receipt.price_snapshot_digest == execution.price_snapshot.price_snapshot_digest
            && receipt.consumer_account_id == sources.delivery_allocation_grant.consumer_account_id
            && receipt.currency == COMPUTE_CAPACITY_FUTURE_SETTLEMENT_CURRENCY
            && receipt.balance_state == BALANCE_STATE_PENDING
            && receipt.correction_of_receipt_id.is_none()
            && receipt.available_at.is_none(),
        "capacity-future v195 receipt differs from settlement source"
    );
    if let Some(release) = canonical_sources.release {
        ensure!(
            release.attempt_settlement == settlement.attempt_settlement
                && release.settlement_lineage_digest
                    == settlement_source_carrier(sources).lineage_digest(),
            "capacity-future release source does not close settlement lineage"
        );
    }
    Ok(())
}
