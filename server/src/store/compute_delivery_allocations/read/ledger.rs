use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::{
    capacity::ComputeCapacityClaim, capacity_commitment::ComputeCapacityCommitment,
    delivery_allocation::ComputeDeliveryAllocationLedgerEvidence,
};

pub(super) fn audit_parent_release_on(
    conn: &Connection,
    evidence: &ComputeDeliveryAllocationLedgerEvidence,
    claim: &ComputeCapacityClaim,
    commitment: &ComputeCapacityCommitment,
) -> Result<()> {
    let found = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_ledger_transactions WHERE
                transaction_id=?1 AND transaction_digest=?2 AND ledger_sequence=?3
                AND event_kind='reservation_released' AND causal_transaction_id=?4
                AND claim_id=?5 AND claim_effect='released' AND pool_id=?6
                AND capacity_epoch=?7 AND delivery_window_id=?8 AND offer_id=?9
                AND offer_version=?10 AND offer_digest=?11 AND job_id IS NULL
                AND reservation_id IS NULL AND subject_kind='compute_capacity_commitment'
                AND subject_id=?12",
            params![
                evidence.transaction_id,
                evidence.transaction_digest,
                evidence.ledger_sequence,
                evidence.causal_transaction_id,
                claim.claim_id,
                commitment.pool.pool_id,
                commitment.pool.capacity_epoch,
                commitment.delivery_window.binding.window_id,
                commitment.offer.offer_id,
                commitment.offer.offer_version,
                commitment.offer.offer_digest,
                commitment.commitment_id,
            ],
            |_| Ok(()),
        )
        .optional()?;
    if found.is_none() {
        bail!("DeliveryAllocation parent release ledger header/causal binding 不一致");
    }
    audit_ledger_legs_on(conn, evidence, claim, LedgerTransfer::Release)
}

pub(super) fn audit_child_hold_on(
    conn: &Connection,
    evidence: &ComputeDeliveryAllocationLedgerEvidence,
    claim: &ComputeCapacityClaim,
    commitment: &ComputeCapacityCommitment,
    job_id: &str,
    reservation_id: &str,
    parent_release_transaction_id: &str,
) -> Result<()> {
    let found = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_ledger_transactions WHERE
                transaction_id=?1 AND transaction_digest=?2 AND ledger_sequence=?3
                AND event_kind='reservation_held' AND causal_transaction_id=?4
                AND claim_id=?5 AND claim_effect='held' AND pool_id=?6
                AND capacity_epoch=?7 AND delivery_window_id=?8 AND offer_id=?9
                AND offer_version=?10 AND offer_digest=?11 AND job_id=?12
                AND reservation_id=?13 AND subject_kind='compute_reservation'
                AND subject_id=?13",
            params![
                evidence.transaction_id,
                evidence.transaction_digest,
                evidence.ledger_sequence,
                parent_release_transaction_id,
                claim.claim_id,
                commitment.pool.pool_id,
                commitment.pool.capacity_epoch,
                commitment.delivery_window.binding.window_id,
                commitment.offer.offer_id,
                commitment.offer.offer_version,
                commitment.offer.offer_digest,
                job_id,
                reservation_id,
            ],
            |_| Ok(()),
        )
        .optional()?;
    if found.is_none() || evidence.causal_transaction_id != parent_release_transaction_id {
        bail!("DeliveryAllocation child hold ledger header/causal binding 不一致");
    }
    audit_ledger_legs_on(conn, evidence, claim, LedgerTransfer::Hold)
}

fn audit_ledger_legs_on(
    conn: &Connection,
    evidence: &ComputeDeliveryAllocationLedgerEvidence,
    claim: &ComputeCapacityClaim,
    transfer: LedgerTransfer,
) -> Result<()> {
    let leg_count = conn.query_row(
        "SELECT COUNT(*) FROM compute_capacity_ledger_legs WHERE transaction_id=?1",
        params![evidence.transaction_id],
        |row| row.get::<_, i64>(0),
    )?;
    if leg_count != i64::try_from(claim.lines.len())?.saturating_mul(2) {
        bail!("DeliveryAllocation ledger legs 数量不等于 2*N");
    }
    let (from_account, to_account) = match transfer {
        LedgerTransfer::Release => ("held", "available"),
        LedgerTransfer::Hold => ("available", "held"),
    };
    for line in &claim.lines {
        let exact = conn
            .query_row(
                "SELECT 1
                   WHERE EXISTS (
                       SELECT 1 FROM compute_capacity_ledger_legs
                        WHERE transaction_id=?1 AND line_no=?2 AND leg_role='from'
                          AND bucket_id=?3 AND meter=?4 AND account=?5 AND delta_units=?6)
                     AND EXISTS (
                       SELECT 1 FROM compute_capacity_ledger_legs
                        WHERE transaction_id=?1 AND line_no=?2 AND leg_role='to'
                          AND bucket_id=?3 AND meter=?4 AND account=?7 AND delta_units=?8)",
                params![
                    evidence.transaction_id,
                    line.line_no,
                    line.bucket.bucket_id,
                    line.bucket.meter,
                    from_account,
                    -line.quantity_units,
                    to_account,
                    line.quantity_units,
                ],
                |_| Ok(()),
            )
            .optional()?;
        if exact.is_none() {
            bail!("DeliveryAllocation ledger leg 与 whole-only Claim line 不一致");
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum LedgerTransfer {
    Release,
    Hold,
}
