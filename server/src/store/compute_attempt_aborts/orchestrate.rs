use anyhow::{anyhow, bail, Result};
use rusqlite::Transaction;

use crate::compute_federation::{
    capacity::{
        ComputeCapacityClaimBinding, ComputeCapacityClaimState, ComputeCapacityOfferBinding,
    },
    execution::{
        ComputeJobVersionBinding, ATTEMPT_STATUS_STAGING, JOB_STATUS_CANCELED, JOB_STATUS_RUNNING,
        RESERVATION_STATUS_ACTIVE, RESERVATION_STATUS_RELEASED,
    },
};

use super::{
    super::{
        billing_reservations::release_billing_call_reservation_on,
        compute_attempt_activations::compute_attempt_activation_on,
        compute_attempt_leases::{
            compute_attempt_lease_state_on, terminate_staging_attempt_lease_on,
            TerminateStagingAttemptLease,
        },
        compute_broker_reservation::{broker_compute_call_id, broker_reserve_binding_on},
        compute_capacity_claim_return::{
            return_attempt_capacity_claim_on, ReturnAttemptCapacityClaim,
        },
        compute_capacity_claim_rows::stored_claim_on,
        compute_job_registry::{current_registered_job_on, register_compute_job_on},
        compute_provider_registry::current_registered_provider_on,
        compute_reservation_registry::{
            current_registered_reservation_on, register_compute_reservation_on,
        },
        new_id,
    },
    receipt::{persist_attempt_abort_on, AttemptAbortPersistence},
    validation::{abort_timestamp, NormalizedAttemptAbort},
    ComputeAttemptAbortReceipt,
};

pub(super) fn abort_staging_attempt_on(
    tx: &Transaction<'_>,
    request: &NormalizedAttemptAbort,
) -> Result<ComputeAttemptAbortReceipt> {
    let activation = compute_attempt_activation_on(tx, &request.lease_id)?;
    if activation.lease.provider_id != request.provider_id
        || activation.lease.fencing_generation != request.expected_fencing_generation
    {
        bail!("Attempt 中止请求与原始激活合同不一致");
    }
    let provider = current_registered_provider_on(tx, &request.provider_id)?
        .ok_or_else(|| anyhow!("Attempt 中止引用的 Provider 不存在"))?;
    if provider.provider.owner_account_id != request.aborted_by_user_id {
        bail!("只有当前 Provider 所有者可以登记 staging Attempt 中止");
    }

    let source_lease = compute_attempt_lease_state_on(tx, &request.lease_id)?;
    if source_lease.lease_revision != request.expected_lease_revision
        || source_lease.lease_digest != request.expected_lease_digest
        || source_lease.lease_revision != 1
        || source_lease.lease_digest != activation.lease_digest
        || source_lease.lease != activation.lease
        || source_lease.lease.status != ATTEMPT_STATUS_STAGING
        || source_lease.lease.last_heartbeat_at.is_some()
    {
        bail!("只有当前精确版本且从未记录心跳的 staging Lease 可以无用量中止");
    }

    let source_job = current_registered_job_on(tx, &activation.lease.job_id)?
        .ok_or_else(|| anyhow!("Attempt 中止引用的 Job 不存在"))?;
    if source_job.revision != request.expected_job_revision
        || source_job.job_digest != request.expected_job_digest
        || source_job.job.status != JOB_STATUS_RUNNING
        || source_job.revision != activation.running_job.job_revision
        || source_job.job_digest != activation.running_job.job_digest
    {
        bail!("Attempt 中止只能基于激活回执绑定的当前 running Job 精确版本");
    }

    let source_reservation =
        current_registered_reservation_on(tx, &activation.lease.reservation_id)?
            .ok_or_else(|| anyhow!("Attempt 中止引用的 Reservation 不存在"))?;
    if source_reservation.revision != request.expected_reservation_revision
        || source_reservation.reservation_digest != request.expected_reservation_digest
        || source_reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || source_reservation.revision != activation.active_reservation_revision
        || source_reservation.reservation_digest != activation.active_reservation_digest
        || source_reservation.reservation.job.job_revision != source_job.revision
        || source_reservation.reservation.job.job_digest != source_job.job_digest
    {
        bail!("Attempt 中止只能基于激活回执绑定的当前 active Reservation 精确版本");
    }

    let source_claim = stored_claim_on(tx, &activation.active_claim.claim_id)?
        .ok_or_else(|| anyhow!("Attempt 中止引用的 Capacity Claim 不存在"))?;
    if source_claim.revision != request.expected_claim_revision
        || source_claim.claim_digest != request.expected_claim_digest
        || source_claim.state != ComputeCapacityClaimState::Active
        || source_claim.revision != activation.active_claim.claim_revision
        || source_claim.claim_digest != activation.active_claim.claim_digest
        || source_reservation.reservation.capacity_claim != activation.active_claim
    {
        bail!("Attempt 中止只能归还激活回执绑定的当前 active Capacity Claim");
    }

    let consumer_account_id = source_job.job.consumer_account_id.as_str();
    let broker =
        broker_reserve_binding_on(tx, &activation.lease.reservation_id, consumer_account_id)?;
    if broker.budget_reservation_id != activation.budget_reservation_id
        || broker.budget_reserved_fen != activation.budget_reserved_fen
        || broker.reserved_job != activation.source_job
        || broker.reservation_revision != activation.source_reservation_revision
        || broker.reservation_digest != activation.source_reservation_digest
    {
        bail!("Attempt 中止与原始 Broker 预留回执不一致");
    }

    let aborted_at = abort_timestamp(
        &source_job.job.updated_at,
        &source_reservation.reservation.updated_at,
        &source_lease.updated_at,
        &source_job.job.workload.deadline_at,
    )?;
    let billing = release_billing_call_reservation_on(
        tx,
        consumer_account_id,
        &broker_compute_call_id(&activation.lease.reservation_id),
        &activation.budget_reservation_id,
        "released_no_usage",
    )?;
    if billing.reserved_fen != activation.budget_reserved_fen {
        bail!("Attempt 中止退款金额与原始预授权不一致");
    }

    let returned_capacity = return_attempt_capacity_claim_on(
        tx,
        ReturnAttemptCapacityClaim {
            claim_id: source_claim.claim_id.clone(),
            expected_revision: source_claim.revision,
            expected_digest: source_claim.claim_digest.clone(),
            offer: ComputeCapacityOfferBinding {
                offer_id: source_reservation.reservation.offer.offer_id.clone(),
                offer_version: source_reservation.reservation.offer.offer_version,
                offer_digest: source_reservation.reservation.offer.offer_digest.clone(),
            },
            job_id: source_job.job.job_id.clone(),
            reservation_id: activation.lease.reservation_id.clone(),
            attempt_lease_id: activation.lease.lease_id.clone(),
            fencing_generation: activation.lease.fencing_generation,
            activation_request_digest: activation.request_digest.clone(),
            abort_request_digest: request.request_digest.clone(),
            idempotency_scope: request.idempotency_scope.clone(),
            idempotency_key: request.idempotency_key.clone(),
            returned_at: aborted_at.clone(),
        },
    )?;

    let mut terminal_job = source_job.job.clone();
    terminal_job.status = JOB_STATUS_CANCELED.to_string();
    terminal_job.updated_at = aborted_at.clone();
    let terminal_job = register_compute_job_on(tx, &terminal_job, source_job.revision)?;

    let mut terminal_reservation = source_reservation.reservation.clone();
    terminal_reservation.status = RESERVATION_STATUS_RELEASED.to_string();
    terminal_reservation.updated_at = aborted_at.clone();
    terminal_reservation.released_at = Some(aborted_at.clone());
    terminal_reservation.job = ComputeJobVersionBinding {
        job_id: terminal_job.job.job_id.clone(),
        job_revision: terminal_job.revision,
        job_digest: terminal_job.job_digest.clone(),
    };
    terminal_reservation.capacity_claim = ComputeCapacityClaimBinding {
        claim_id: returned_capacity.claim.claim_id.clone(),
        claim_revision: returned_capacity.claim.claim_revision,
        claim_digest: returned_capacity.claim.claim_digest.clone(),
    };
    let terminal_reservation =
        register_compute_reservation_on(tx, &terminal_reservation, source_reservation.revision)?;
    let terminal_lease = terminate_staging_attempt_lease_on(
        tx,
        TerminateStagingAttemptLease {
            lease_id: &request.lease_id,
            expected_revision: source_lease.lease_revision,
            expected_digest: &source_lease.lease_digest,
            expected_fencing_generation: request.expected_fencing_generation,
            reason_code: &request.reason_code,
            actor_user_id: &request.aborted_by_user_id,
            terminated_at: &aborted_at,
        },
    )?;

    persist_attempt_abort_on(
        tx,
        AttemptAbortPersistence {
            abort_id: new_id("compute_attempt_abort"),
            request,
            activation: &activation,
            source_lease: &source_lease,
            terminal_lease: &terminal_lease,
            source_job: &source_job,
            terminal_job: &terminal_job,
            source_reservation: &source_reservation,
            terminal_reservation: &terminal_reservation,
            source_claim: ComputeCapacityClaimBinding {
                claim_id: source_claim.claim_id,
                claim_revision: source_claim.revision,
                claim_digest: source_claim.claim_digest,
            },
            returned_capacity: &returned_capacity,
            billing: &billing,
            aborted_at: &aborted_at,
        },
    )
}
