use anyhow::{bail, Result};
use serde::Deserialize;

use crate::{
    compute_federation_attempt_service::get_for_participant,
    store::{ComputeAttemptFinalizationReceipt, FinalizeComputeAttemptRequest, Store},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FinalizeComputeAttemptBody {
    pub expected_execution_receipt_id: String,
    pub expected_execution_receipt_digest: String,
    pub expected_lease_revision: i64,
    pub expected_lease_digest: String,
    pub expected_fencing_generation: i64,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub expected_reservation_revision: i64,
    pub expected_reservation_digest: String,
    pub expected_claim_revision: i64,
    pub expected_claim_digest: String,
    pub idempotency_key: String,
    pub confirm_trusted_terminal_and_capacity: bool,
}

pub(crate) fn finalize_for_platform_admin(
    store: &Store,
    admin_user_id: &str,
    lease_id: &str,
    body: FinalizeComputeAttemptBody,
) -> Result<ComputeAttemptFinalizationReceipt> {
    if !body.confirm_trusted_terminal_and_capacity {
        bail!("应用可信终态前必须确认本操作会推进状态和容量但不会结算资金");
    }
    store.finalize_compute_attempt(&FinalizeComputeAttemptRequest {
        lease_id: lease_id.to_string(),
        expected_execution_receipt_id: body.expected_execution_receipt_id,
        expected_execution_receipt_digest: body.expected_execution_receipt_digest,
        expected_lease_revision: body.expected_lease_revision,
        expected_lease_digest: body.expected_lease_digest,
        expected_fencing_generation: body.expected_fencing_generation,
        expected_job_revision: body.expected_job_revision,
        expected_job_digest: body.expected_job_digest,
        expected_reservation_revision: body.expected_reservation_revision,
        expected_reservation_digest: body.expected_reservation_digest,
        expected_claim_revision: body.expected_claim_revision,
        expected_claim_digest: body.expected_claim_digest,
        idempotency_key: body.idempotency_key,
        finalized_by_user_id: admin_user_id.to_string(),
    })
}

pub(crate) fn get_for_attempt_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptFinalizationReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_attempt_finalization(lease_id)
}
