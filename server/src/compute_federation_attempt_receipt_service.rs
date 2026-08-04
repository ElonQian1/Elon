use anyhow::{bail, Result};
use serde::Deserialize;

use crate::{
    compute_federation_attempt_service::get_for_participant,
    store::{
        ComputeAttemptExecutionReceiptEnvelope, ComputePendingExecutionReceiptCandidate,
        IssueComputeAttemptExecutionReceiptRequest, Store,
    },
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IssueComputeAttemptExecutionReceiptBody {
    pub expected_verification_decision_id: String,
    pub expected_verification_event_digest: String,
    pub idempotency_key: String,
    pub confirm_execution_receipt_only: bool,
}

pub(crate) fn issue_for_platform_admin(
    store: &Store,
    admin_user_id: &str,
    lease_id: &str,
    request: IssueComputeAttemptExecutionReceiptBody,
) -> Result<ComputeAttemptExecutionReceiptEnvelope> {
    if !request.confirm_execution_receipt_only {
        bail!("签发 Execution Receipt 前必须确认本操作不推进状态、容量或结算");
    }
    store.issue_compute_attempt_execution_receipt(&IssueComputeAttemptExecutionReceiptRequest {
        lease_id: lease_id.to_string(),
        expected_verification_decision_id: request.expected_verification_decision_id,
        expected_verification_event_digest: request.expected_verification_event_digest,
        idempotency_key: request.idempotency_key,
        issued_by_user_id: admin_user_id.to_string(),
    })
}

pub(crate) fn list_pending_for_platform_admin(
    store: &Store,
    limit: usize,
) -> Result<Vec<ComputePendingExecutionReceiptCandidate>> {
    store.list_pending_compute_attempt_execution_receipts(limit)
}

pub(crate) fn get_for_attempt_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptExecutionReceiptEnvelope> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_attempt_execution_receipt(lease_id)
}
