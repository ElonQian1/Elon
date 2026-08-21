use anyhow::{bail, Result};

use super::super::super::{ComputeAttemptFinalizationReceipt, FinalizeComputeAttemptRequest};

pub(super) fn ensure_request_bindings(
    request: &FinalizeComputeAttemptRequest,
    receipt: &ComputeAttemptFinalizationReceipt,
) -> Result<()> {
    if request.lease_id != receipt.lease_id
        || request.expected_execution_receipt_id != receipt.execution_receipt_id
        || request.expected_execution_receipt_digest != receipt.execution_receipt_digest
        || request.expected_lease_revision != receipt.source_lease.revision
        || request.expected_lease_digest != receipt.source_lease.digest
        || request.expected_job_revision != receipt.source_job.job_revision
        || request.expected_job_digest != receipt.source_job.job_digest
        || request.expected_reservation_revision != receipt.source_reservation.revision
        || request.expected_reservation_digest != receipt.source_reservation.digest
        || request.expected_claim_revision != receipt.source_claim.claim_revision
        || request.expected_claim_digest != receipt.source_claim.claim_digest
        || request.finalized_by_user_id != receipt.finalized_by_user_id
        || request.expected_fencing_generation <= 0
        || receipt.terminal_lease.revision != receipt.source_lease.revision + 1
        || receipt.terminal_job.job_revision != receipt.source_job.job_revision + 1
        || receipt.terminal_reservation.revision != receipt.source_reservation.revision + 1
        || receipt.terminal_claim.claim_revision != receipt.source_claim.claim_revision + 1
    {
        bail!("Attempt 可信终态请求与源/目标版本绑定不一致");
    }
    Ok(())
}
