use anyhow::{bail, Result};
use rusqlite::TransactionBehavior;
use serde::Serialize;

use crate::compute_federation::{
    capacity::ComputeCapacityClaimBinding,
    execution::{ComputeAttemptLease, ComputeJobVersionBinding},
};

use super::{compute_capacity_ledger::ComputeCapacityLedgerWriteReceipt, Store};

mod orchestrate;
mod receipt;
mod validation;

use orchestrate::abort_staging_attempt_on;
use receipt::{attempt_abort_by_lease_on, replay_attempt_abort_on};
use validation::normalize_abort_request;

pub(crate) const COMPUTE_ATTEMPT_ABORT_SCHEMA: &str = "compute_federation.attempt_abort.v1";

#[derive(Debug, Clone)]
pub(crate) struct AbortComputeAttemptRequest {
    pub lease_id: String,
    pub provider_id: String,
    pub expected_lease_revision: i64,
    pub expected_lease_digest: String,
    pub expected_fencing_generation: i64,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub expected_reservation_revision: i64,
    pub expected_reservation_digest: String,
    pub expected_claim_revision: i64,
    pub expected_claim_digest: String,
    pub executor_abort_ref: String,
    pub reason_code: String,
    pub idempotency_key: String,
    pub aborted_by_user_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptAbortReceipt {
    pub schema: &'static str,
    pub abort_id: String,
    pub terminal_lease: ComputeAttemptLease,
    pub source_lease_revision: i64,
    pub source_lease_digest: String,
    pub terminal_lease_revision: i64,
    pub terminal_lease_digest: String,
    pub source_job: ComputeJobVersionBinding,
    pub terminal_job: ComputeJobVersionBinding,
    pub source_reservation_revision: i64,
    pub source_reservation_digest: String,
    pub terminal_reservation_revision: i64,
    pub terminal_reservation_digest: String,
    pub source_claim: ComputeCapacityClaimBinding,
    pub returned_claim: ComputeCapacityClaimBinding,
    pub budget_reservation_id: String,
    pub budget_refunded_fen: i64,
    pub budget_terminal_status: String,
    pub capacity_ledger: ComputeCapacityLedgerWriteReceipt,
    pub activation_request_digest: String,
    pub executor_abort_ref: String,
    pub reason_code: String,
    pub request_digest: String,
    pub event_digest: String,
    pub aborted_by_user_id: String,
    pub aborted_at: String,
    pub execution_effect: &'static str,
    pub capacity_effect: &'static str,
    pub reservation_effect: &'static str,
    pub money_effect: &'static str,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn abort_compute_attempt(
        &self,
        request: &AbortComputeAttemptRequest,
    ) -> Result<ComputeAttemptAbortReceipt> {
        let request = normalize_abort_request(request)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay_attempt_abort_on(&tx, &request)? {
            tx.commit()?;
            return Ok(receipt);
        }
        let receipt = abort_staging_attempt_on(&tx, &request)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_attempt_abort(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptAbortReceipt> {
        if lease_id.is_empty() || lease_id.trim() != lease_id {
            bail!("Attempt Lease ID 无效");
        }
        attempt_abort_by_lease_on(&*self.conn()?, lease_id)?
            .ok_or_else(|| anyhow::anyhow!("Attempt 中止回执不存在"))
    }
}
