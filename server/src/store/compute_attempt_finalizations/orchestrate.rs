use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::Transaction;

use crate::compute_federation::{
    capacity::{
        ComputeCapacityClaimBinding, ComputeCapacityClaimState, ComputeCapacityOfferBinding,
    },
    execution::{
        ComputeJobVersionBinding, JOB_STATUS_RUNNING, JOB_STATUS_VERIFICATION_PENDING,
        RESERVATION_STATUS_ACTIVE, RESERVATION_STATUS_CONSUMED,
    },
};

use super::{
    super::{
        compute_attempt_execution_receipts::compute_attempt_execution_receipt_on,
        compute_attempt_leases::{
            current_lease_state_on, finalize_verified_attempt_lease_on,
            FinalizeVerifiedAttemptLease,
        },
        compute_attempt_terminals::compute_attempt_terminal_candidate_on,
        compute_capacity_claim_rows::stored_claim_on,
        compute_job_registry::{current_registered_job_on, register_compute_job_on},
        compute_reservation_registry::{
            current_registered_reservation_on, register_compute_reservation_on,
        },
        new_id,
    },
    capacity::{capacity_effect, finalize_attempt_capacity_on, FinalizeAttemptCapacityInput},
    support::finalization_event_digest,
    ComputeAttemptFinalizationReceipt, ComputeAttemptRevisionBinding,
    FinalizeComputeAttemptRequest, COMPUTE_ATTEMPT_FINALIZATION_SCHEMA,
};

pub(super) fn finalize_attempt_on(
    tx: &Transaction<'_>,
    request: &FinalizeComputeAttemptRequest,
    request_digest: &str,
    idempotency_scope: &str,
) -> Result<ComputeAttemptFinalizationReceipt> {
    let execution = compute_attempt_execution_receipt_on(tx, &request.lease_id)?;
    if execution.receipt.receipt_id != request.expected_execution_receipt_id
        || execution.receipt.receipt_digest != request.expected_execution_receipt_digest
    {
        bail!("Attempt 可信终态必须绑定精确 v193 Execution Receipt");
    }
    let candidate = compute_attempt_terminal_candidate_on(tx, &request.lease_id)?
        .ok_or_else(|| anyhow!("Attempt 可信终态引用的 Provider 候选不存在"))?;
    if execution.receipt.execution_status != candidate.outcome
        || execution.receipt.finished_at != candidate.declared_at
        || execution.receipt.attempt_lease_id != candidate.lease_id
        || execution.receipt.fencing_generation != candidate.fencing_generation
    {
        bail!("Execution Receipt 与 Provider 终态候选不一致");
    }

    let source_lease = current_lease_state_on(tx, &request.lease_id)?
        .ok_or_else(|| anyhow!("Attempt Lease 当前状态不存在"))?;
    if source_lease.lease_revision != request.expected_lease_revision
        || source_lease.lease_digest != request.expected_lease_digest
        || source_lease.lease_revision != candidate.source_lease_revision
        || source_lease.lease_digest != candidate.source_lease_digest
        || source_lease.lease.fencing_generation != request.expected_fencing_generation
        || source_lease.lease.fencing_generation != execution.receipt.fencing_generation
        || source_lease.lease.status != crate::compute_federation::execution::ATTEMPT_STATUS_RUNNING
        || source_lease.lease.last_heartbeat_at.is_none()
    {
        bail!("可信终态拒绝候选生成后的 Lease 续租、代次漂移或非 running 状态");
    }

    let source_job = current_registered_job_on(tx, &execution.receipt.job_id)?
        .ok_or_else(|| anyhow!("Attempt 可信终态引用的 Job 不存在"))?;
    if source_job.revision != request.expected_job_revision
        || source_job.job_digest != request.expected_job_digest
        || source_job.revision != candidate.job_revision
        || source_job.job_digest != candidate.job_digest
        || source_job.job.status != JOB_STATUS_RUNNING
    {
        bail!("Attempt 可信终态只能推进候选绑定的当前 running Job 精确版本");
    }

    let source_reservation =
        current_registered_reservation_on(tx, &execution.receipt.reservation_id)?
            .ok_or_else(|| anyhow!("Attempt 可信终态引用的 Reservation 不存在"))?;
    if source_reservation.revision != request.expected_reservation_revision
        || source_reservation.reservation_digest != request.expected_reservation_digest
        || source_reservation.revision != candidate.reservation_revision
        || source_reservation.reservation_digest != candidate.reservation_digest
        || source_reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || source_reservation.reservation.job.job_revision != source_job.revision
        || source_reservation.reservation.job.job_digest != source_job.job_digest
    {
        bail!("Attempt 可信终态只能消费候选绑定的当前 active Reservation 精确版本");
    }

    let source_claim = stored_claim_on(tx, &candidate.capacity_claim_id)?
        .ok_or_else(|| anyhow!("Attempt 可信终态引用的 Capacity Claim 不存在"))?;
    if source_claim.revision != request.expected_claim_revision
        || source_claim.claim_digest != request.expected_claim_digest
        || source_claim.revision != candidate.capacity_claim_revision
        || source_claim.claim_digest != candidate.capacity_claim_digest
        || source_claim.state != ComputeCapacityClaimState::Active
        || source_reservation.reservation.capacity_claim.claim_id != source_claim.claim_id
        || source_reservation.reservation.capacity_claim.claim_revision != source_claim.revision
        || source_reservation.reservation.capacity_claim.claim_digest != source_claim.claim_digest
    {
        bail!("Attempt 可信终态只能收口候选绑定的当前 active Claim 精确版本");
    }

    ensure_effective_time(
        &execution.receipt.finished_at,
        &source_lease.updated_at,
        &source_job.job.updated_at,
        &source_reservation.reservation.updated_at,
        &source_claim.updated_at,
        &source_job.job.workload.deadline_at,
        &source_reservation.reservation.expires_at,
    )?;
    let effective_at = execution.receipt.finished_at.clone();
    let finalized_at = Utc::now().to_rfc3339();
    let source_claim_binding = ComputeCapacityClaimBinding {
        claim_id: source_claim.claim_id.clone(),
        claim_revision: source_claim.revision,
        claim_digest: source_claim.claim_digest.clone(),
    };
    let activation = super::super::compute_attempt_activations::compute_attempt_activation_on(
        tx,
        &request.lease_id,
    )?;
    let capacity = finalize_attempt_capacity_on(
        tx,
        FinalizeAttemptCapacityInput {
            claim_id: &source_claim.claim_id,
            expected_revision: source_claim.revision,
            expected_digest: &source_claim.claim_digest,
            offer: ComputeCapacityOfferBinding {
                offer_id: source_reservation.reservation.offer.offer_id.clone(),
                offer_version: source_reservation.reservation.offer.offer_version,
                offer_digest: source_reservation.reservation.offer.offer_digest.clone(),
            },
            job_id: &source_job.job.job_id,
            reservation_id: &source_reservation.reservation.reservation_id,
            attempt_lease_id: &request.lease_id,
            fencing_generation: request.expected_fencing_generation,
            activation_request_digest: &activation.request_digest,
            execution_receipt_id: &execution.receipt.receipt_id,
            compensable_usage: &execution.receipt.usage.compensable_usage,
            finalization_request_digest: request_digest,
            idempotency_scope,
            idempotency_key: &request.idempotency_key,
            effective_at: &effective_at,
        },
    )?;
    let capacity_effect = capacity_effect(&capacity).to_string();

    let mut pending_job = source_job.job.clone();
    pending_job.status = JOB_STATUS_VERIFICATION_PENDING.to_string();
    pending_job.updated_at = effective_at.clone();
    let pending_job = register_compute_job_on(tx, &pending_job, source_job.revision)?;

    let mut consumed_reservation = source_reservation.reservation.clone();
    consumed_reservation.status = RESERVATION_STATUS_CONSUMED.to_string();
    consumed_reservation.updated_at = effective_at.clone();
    consumed_reservation.consumed_at = Some(effective_at.clone());
    consumed_reservation.job = ComputeJobVersionBinding {
        job_id: pending_job.job.job_id.clone(),
        job_revision: pending_job.revision,
        job_digest: pending_job.job_digest.clone(),
    };
    consumed_reservation.capacity_claim = capacity.terminal_claim.clone();
    let consumed_reservation =
        register_compute_reservation_on(tx, &consumed_reservation, source_reservation.revision)?;

    let terminal_lease = finalize_verified_attempt_lease_on(
        tx,
        FinalizeVerifiedAttemptLease {
            lease_id: &request.lease_id,
            expected_revision: source_lease.lease_revision,
            expected_digest: &source_lease.lease_digest,
            expected_fencing_generation: request.expected_fencing_generation,
            reason_code: &candidate.reason_code,
            actor_user_id: &request.finalized_by_user_id,
            finalized_at: &effective_at,
        },
    )?;

    let mut receipt = ComputeAttemptFinalizationReceipt {
        schema: COMPUTE_ATTEMPT_FINALIZATION_SCHEMA.to_string(),
        finalization_id: new_id("compute_attempt_finalization"),
        execution_receipt_id: execution.receipt.receipt_id,
        execution_receipt_digest: execution.receipt.receipt_digest,
        lease_id: request.lease_id.clone(),
        provider_id: candidate.provider_id,
        consumer_account_id: candidate.consumer_account_id,
        outcome: candidate.outcome,
        reason_code: candidate.reason_code,
        source_lease: ComputeAttemptRevisionBinding {
            revision: source_lease.lease_revision,
            digest: source_lease.lease_digest,
        },
        terminal_lease: ComputeAttemptRevisionBinding {
            revision: terminal_lease.lease_revision,
            digest: terminal_lease.lease_digest,
        },
        source_job: ComputeJobVersionBinding {
            job_id: source_job.job.job_id,
            job_revision: source_job.revision,
            job_digest: source_job.job_digest,
        },
        terminal_job: ComputeJobVersionBinding {
            job_id: pending_job.job.job_id,
            job_revision: pending_job.revision,
            job_digest: pending_job.job_digest,
        },
        source_reservation: ComputeAttemptRevisionBinding {
            revision: source_reservation.revision,
            digest: source_reservation.reservation_digest,
        },
        terminal_reservation: ComputeAttemptRevisionBinding {
            revision: consumed_reservation.revision,
            digest: consumed_reservation.reservation_digest,
        },
        source_claim: source_claim_binding,
        terminal_claim: capacity.terminal_claim,
        compensable_usage: capacity.compensable_usage,
        capacity_consumed: capacity.capacity_consumed,
        capacity_returned: capacity.capacity_returned,
        capacity_transactions: capacity.transactions,
        request_digest: request_digest.to_string(),
        event_digest: String::new(),
        finalized_by_user_id: request.finalized_by_user_id.clone(),
        effective_at,
        finalized_at,
        execution_effect: "trusted_terminal_applied".to_string(),
        lease_effect: "terminal".to_string(),
        job_effect: "verification_pending".to_string(),
        capacity_effect,
        reservation_effect: "consumed".to_string(),
        money_effect: "preauthorization_unchanged".to_string(),
        settlement_effect: "pending".to_string(),
        replayed: false,
    };
    receipt.event_digest = finalization_event_digest(&receipt)?;
    Ok(receipt)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn ensure_effective_time(
    effective_at: &str,
    lease_updated_at: &str,
    job_updated_at: &str,
    reservation_updated_at: &str,
    claim_updated_at: &str,
    deadline_at: &str,
    reservation_expires_at: &str,
) -> Result<()> {
    let effective = parse_utc("Execution Receipt finished_at", effective_at)?;
    for (label, value) in [
        ("Lease 更新时间", lease_updated_at),
        ("Job 更新时间", job_updated_at),
        ("Reservation 更新时间", reservation_updated_at),
        ("Capacity Claim 更新时间", claim_updated_at),
    ] {
        if effective <= parse_utc(label, value)? {
            bail!("Execution Receipt 终态时间必须晚于全部当前状态版本");
        }
    }
    if effective >= parse_utc("Job 截止时间", deadline_at)?
        || effective > parse_utc("Reservation 到期时间", reservation_expires_at)?
    {
        bail!("Execution Receipt 终态时间越过 Job 或 Reservation 边界");
    }
    Ok(())
}

fn parse_utc(label: &str, value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("{label} 不是 RFC3339"))?
        .with_timezone(&Utc))
}
