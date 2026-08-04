use anyhow::{anyhow, bail, Result};
use rusqlite::TransactionBehavior;
use serde::{Deserialize, Serialize};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaimBinding, ComputeCapacityClaimState},
    execution::{
        ComputeJobVersionBinding, ComputeReservedCapacity, ATTEMPT_STATUS_RUNNING,
        JOB_STATUS_RUNNING, RESERVATION_STATUS_ACTIVE,
    },
    receipts::ComputeMeterReading,
};

use super::{
    compute_attempt_execution_receipts::{
        compute_attempt_execution_receipt_on, ComputeAttemptExecutionReceiptEnvelope,
    },
    compute_attempt_leases::current_lease_state_on,
    compute_attempt_terminals::{
        compute_attempt_terminal_candidate_on, ComputeAttemptTerminalCandidateReceipt,
    },
    compute_capacity_claim_rows::stored_claim_on,
    compute_job_registry::current_registered_job_on,
    compute_reservation_registry::current_registered_reservation_on,
    Store,
};

mod capacity;
mod orchestrate;
mod pending_queue;
mod support;

use pending_queue::list_pending_finalization_lease_ids_on;
use support::{
    finalization_by_idempotency_on, finalization_by_lease_on, finalization_request_digest,
    normalize_finalization_request, persist_finalization_on,
};

pub(crate) const COMPUTE_ATTEMPT_FINALIZATION_SCHEMA: &str =
    "compute_federation.attempt_finalization.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FinalizeComputeAttemptRequest {
    pub lease_id: String,
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
    pub finalized_by_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeAttemptRevisionBinding {
    pub revision: i64,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeAttemptCapacityTransactionRef {
    pub transaction_id: String,
    pub transaction_digest: String,
    pub ledger_sequence: i64,
    pub event_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeAttemptFinalizationReceipt {
    pub schema: String,
    pub finalization_id: String,
    pub execution_receipt_id: String,
    pub execution_receipt_digest: String,
    pub lease_id: String,
    pub provider_id: String,
    pub consumer_account_id: String,
    pub outcome: String,
    pub reason_code: String,
    pub source_lease: ComputeAttemptRevisionBinding,
    pub terminal_lease: ComputeAttemptRevisionBinding,
    pub source_job: ComputeJobVersionBinding,
    pub terminal_job: ComputeJobVersionBinding,
    pub source_reservation: ComputeAttemptRevisionBinding,
    pub terminal_reservation: ComputeAttemptRevisionBinding,
    pub source_claim: ComputeCapacityClaimBinding,
    pub terminal_claim: ComputeCapacityClaimBinding,
    pub compensable_usage: Vec<ComputeMeterReading>,
    pub capacity_consumed: Vec<ComputeReservedCapacity>,
    pub capacity_returned: Vec<ComputeReservedCapacity>,
    pub capacity_transactions: Vec<ComputeAttemptCapacityTransactionRef>,
    pub request_digest: String,
    pub event_digest: String,
    pub finalized_by_user_id: String,
    pub effective_at: String,
    pub finalized_at: String,
    pub execution_effect: String,
    pub lease_effect: String,
    pub job_effect: String,
    pub capacity_effect: String,
    pub reservation_effect: String,
    pub money_effect: String,
    pub settlement_effect: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputePendingAttemptFinalizationCandidate {
    pub execution_receipt: ComputeAttemptExecutionReceiptEnvelope,
    pub terminal_candidate: ComputeAttemptTerminalCandidateReceipt,
    pub expected_lease: ComputeAttemptRevisionBinding,
    pub expected_fencing_generation: i64,
    pub expected_job: ComputeJobVersionBinding,
    pub expected_reservation: ComputeAttemptRevisionBinding,
    pub expected_claim: ComputeCapacityClaimBinding,
    pub compensable_usage: Vec<ComputeReservedCapacity>,
    pub lease_effect: &'static str,
    pub job_effect: &'static str,
    pub reservation_effect: &'static str,
    pub capacity_effect: &'static str,
    pub money_effect: &'static str,
    pub settlement_effect: &'static str,
}

impl Store {
    pub(crate) fn finalize_compute_attempt(
        &self,
        input: &FinalizeComputeAttemptRequest,
    ) -> Result<ComputeAttemptFinalizationReceipt> {
        let input = normalize_finalization_request(input)?;
        let request_digest = finalization_request_digest(&input)?;
        let idempotency_scope = format!(
            "compute_attempt_finalization:{}",
            input.finalized_by_user_id
        );
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            finalization_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同 Attempt 终态幂等键不能用于不同请求");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }
        if let Some(stored) = finalization_by_lease_on(&tx, &input.lease_id)? {
            if stored.request_digest != request_digest {
                bail!("同一 Attempt Lease 已绑定另一份可信终态回执");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let receipt =
            orchestrate::finalize_attempt_on(&tx, &input, &request_digest, &idempotency_scope)?;
        persist_finalization_on(&tx, &input, &receipt, &idempotency_scope)?;
        let stored =
            finalization_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
                .ok_or_else(|| anyhow::anyhow!("Attempt 可信终态回执写入后不可见"))?;
        let receipt = stored.into_receipt(&tx, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_attempt_finalization(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptFinalizationReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        compute_attempt_finalization_on(&*conn, lease_id)
    }

    pub(crate) fn list_pending_compute_attempt_finalizations(
        &self,
        limit: usize,
    ) -> Result<Vec<ComputePendingAttemptFinalizationCandidate>> {
        let conn = self.conn()?;
        list_pending_finalization_lease_ids_on(&conn, limit.clamp(1, 100))?
            .into_iter()
            .map(|lease_id| pending_finalization_candidate_on(&conn, &lease_id))
            .collect()
    }
}

fn pending_finalization_candidate_on(
    conn: &rusqlite::Connection,
    lease_id: &str,
) -> Result<ComputePendingAttemptFinalizationCandidate> {
    let execution = compute_attempt_execution_receipt_on(conn, lease_id)?;
    let candidate = compute_attempt_terminal_candidate_on(conn, lease_id)?
        .ok_or_else(|| anyhow!("待收口队列引用的 Provider 候选不存在"))?;
    if execution.receipt.execution_status != candidate.outcome
        || execution.receipt.finished_at != candidate.declared_at
        || execution.receipt.attempt_lease_id != candidate.lease_id
        || execution.receipt.fencing_generation != candidate.fencing_generation
    {
        bail!("待收口 Execution Receipt 与 Provider 终态候选不一致");
    }

    let lease = current_lease_state_on(conn, lease_id)?
        .ok_or_else(|| anyhow!("待收口 Attempt Lease 当前状态不存在"))?;
    if lease.lease_revision != candidate.source_lease_revision
        || lease.lease_digest != candidate.source_lease_digest
        || lease.lease.fencing_generation != candidate.fencing_generation
        || lease.lease.status != ATTEMPT_STATUS_RUNNING
        || lease.lease.last_heartbeat_at.is_none()
    {
        bail!("待收口 Attempt Lease 已续租、代次漂移或不再处于 running 状态");
    }

    let job = current_registered_job_on(conn, &execution.receipt.job_id)?
        .ok_or_else(|| anyhow!("待收口队列引用的 Job 不存在"))?;
    if job.revision != candidate.job_revision
        || job.job_digest != candidate.job_digest
        || job.job.status != JOB_STATUS_RUNNING
    {
        bail!("待收口 Job 已漂移或不再处于 running 状态");
    }

    let reservation = current_registered_reservation_on(conn, &execution.receipt.reservation_id)?
        .ok_or_else(|| anyhow!("待收口队列引用的 Reservation 不存在"))?;
    if reservation.revision != candidate.reservation_revision
        || reservation.reservation_digest != candidate.reservation_digest
        || reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || reservation.reservation.job.job_revision != job.revision
        || reservation.reservation.job.job_digest != job.job_digest
    {
        bail!("待收口 Reservation 已漂移或不再处于 active 状态");
    }

    let claim = stored_claim_on(conn, &candidate.capacity_claim_id)?
        .ok_or_else(|| anyhow!("待收口队列引用的 Capacity Claim 不存在"))?;
    if claim.revision != candidate.capacity_claim_revision
        || claim.claim_digest != candidate.capacity_claim_digest
        || claim.state != ComputeCapacityClaimState::Active
        || reservation.reservation.capacity_claim.claim_id != claim.claim_id
        || reservation.reservation.capacity_claim.claim_revision != claim.revision
        || reservation.reservation.capacity_claim.claim_digest != claim.claim_digest
    {
        bail!("待收口 Capacity Claim 已漂移或不再处于 active 状态");
    }

    orchestrate::ensure_effective_time(
        &execution.receipt.finished_at,
        &lease.updated_at,
        &job.job.updated_at,
        &reservation.reservation.updated_at,
        &claim.updated_at,
        &job.job.workload.deadline_at,
        &reservation.reservation.expires_at,
    )?;

    Ok(ComputePendingAttemptFinalizationCandidate {
        compensable_usage: execution.receipt.usage.compensable_usage.clone(),
        execution_receipt: execution,
        terminal_candidate: candidate,
        expected_lease: ComputeAttemptRevisionBinding {
            revision: lease.lease_revision,
            digest: lease.lease_digest,
        },
        expected_fencing_generation: lease.lease.fencing_generation,
        expected_job: ComputeJobVersionBinding {
            job_id: job.job.job_id,
            job_revision: job.revision,
            job_digest: job.job_digest,
        },
        expected_reservation: ComputeAttemptRevisionBinding {
            revision: reservation.revision,
            digest: reservation.reservation_digest,
        },
        expected_claim: ComputeCapacityClaimBinding {
            claim_id: claim.claim_id,
            claim_revision: claim.revision,
            claim_digest: claim.claim_digest,
        },
        lease_effect: "terminal",
        job_effect: "verification_pending",
        reservation_effect: "consumed",
        capacity_effect: "consume_compensable_usage_and_return_remainder",
        money_effect: "preauthorization_unchanged",
        settlement_effect: "pending",
    })
}

pub(super) fn compute_attempt_finalization_on(
    conn: &rusqlite::Connection,
    lease_id: &str,
) -> Result<ComputeAttemptFinalizationReceipt> {
    support::validate_exact("Attempt Lease ID", lease_id, 200)?;
    let stored = finalization_by_lease_on(conn, lease_id)?
        .ok_or_else(|| anyhow::anyhow!("Attempt 尚无可信终态回执"))?;
    stored.into_receipt(conn, false)
}
