use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection};

use crate::store::compute_attempt_usage::{
    latest_compute_attempt_usage_declaration_on, ComputeAttemptUsageDeclarationReceipt,
};

use super::{support::StoredTerminalCandidate, ComputeAttemptTerminalCandidateReceipt};

pub(in crate::store) fn terminal_candidate_exists_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<bool> {
    let exists = conn.query_row(
        "SELECT EXISTS (
             SELECT 1
               FROM compute_attempt_terminal_candidates
              WHERE lease_id=?1
         )",
        params![lease_id],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(exists != 0)
}

pub(super) fn terminal_candidate_receipt_on(
    conn: &Connection,
    stored: StoredTerminalCandidate,
    replayed: bool,
) -> Result<ComputeAttemptTerminalCandidateReceipt> {
    let receipt = stored.into_receipt(replayed)?;
    let usage = latest_compute_attempt_usage_declaration_on(conn, &receipt.lease_id)?
        .ok_or_else(|| anyhow!("Attempt 终态候选绑定的最终用量快照不存在"))?;
    ensure_final_usage_head(&receipt, &usage)?;
    Ok(receipt)
}

fn ensure_final_usage_head(
    candidate: &ComputeAttemptTerminalCandidateReceipt,
    usage: &ComputeAttemptUsageDeclarationReceipt,
) -> Result<()> {
    if candidate.final_usage_snapshot_id != usage.snapshot_id
        || candidate.final_usage_sequence_no != usage.sequence_no
        || candidate.final_cumulative_usage_digest != usage.cumulative_usage_digest
        || candidate.lease_id != usage.lease_id
        || candidate.provider_id != usage.provider_id
        || candidate.consumer_account_id != usage.consumer_account_id
        || candidate.source_lease_revision != usage.source_lease_revision
        || candidate.source_lease_digest != usage.source_lease_digest
        || candidate.fencing_generation != usage.fencing_generation
        || candidate.job_id != usage.job_id
        || candidate.job_revision != usage.job_revision
        || candidate.job_digest != usage.job_digest
        || candidate.reservation_id != usage.reservation_id
        || candidate.reservation_revision != usage.reservation_revision
        || candidate.reservation_digest != usage.reservation_digest
        || candidate.capacity_claim_id != usage.capacity_claim_id
        || candidate.capacity_claim_revision != usage.capacity_claim_revision
        || candidate.capacity_claim_digest != usage.capacity_claim_digest
    {
        bail!("Attempt 终态候选绑定的最终用量已不是当前声明流头");
    }
    Ok(())
}
