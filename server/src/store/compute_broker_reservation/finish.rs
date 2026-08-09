use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Transaction};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaimBinding, ComputeCapacityOfferBinding},
    execution::{
        ComputeJobVersionBinding, JOB_STATUS_CANCELED, JOB_STATUS_FAILED, JOB_STATUS_RESERVED,
        RESERVATION_STATUS_ACTIVE, RESERVATION_STATUS_EXPIRED, RESERVATION_STATUS_RELEASED,
    },
};

use super::super::{
    billing_reservations::release_billing_call_reservation_on,
    compute_capacity_claim_transitions::{
        finish_compute_capacity_reservation_claim_on, ComputeCapacityClaimTerminalAction,
        FinishComputeCapacityClaim,
    },
    compute_job_registry::{current_registered_job_on, register_compute_job_on},
    compute_reservation_registry::{
        current_registered_reservation_on, register_compute_reservation_on,
    },
};
use super::{
    finish_receipt::persist_broker_finish_receipt_on,
    finish_validation::{billing_terminal_status, NormalizedBrokerFinishRequest},
    receipt::broker_reserve_binding_on,
    validation::broker_compute_call_id,
    ComputeBrokerFinishAction, ComputeBrokerFinishReceipt,
};

pub(super) fn finish_new_broker_contract_on(
    tx: &Transaction<'_>,
    request: &NormalizedBrokerFinishRequest,
) -> Result<ComputeBrokerFinishReceipt> {
    let source_reservation = current_registered_reservation_on(tx, &request.reservation_id)?
        .ok_or_else(|| anyhow!("Broker 终态绑定的 Reservation 不存在"))?;
    if source_reservation.revision != request.expected_reservation_revision
        || source_reservation.reservation_digest != request.expected_reservation_digest
        || source_reservation.reservation.status != RESERVATION_STATUS_ACTIVE
    {
        bail!("Broker 终态只能基于当前 active Reservation 精确版本");
    }
    let source_job = current_registered_job_on(tx, &source_reservation.reservation.job.job_id)?
        .ok_or_else(|| anyhow!("Broker 终态绑定的当前 Job 不存在"))?;
    if source_job.job.consumer_account_id != request.consumer_account_id
        || source_job.job.status != JOB_STATUS_RESERVED
        || source_job.revision != source_reservation.reservation.job.job_revision
        || source_job.job_digest != source_reservation.reservation.job.job_digest
    {
        bail!("Broker 终态只处理消费者自己的 reserved Job，运行中任务必须走 Attempt 路径");
    }
    let unresolved_start: bool = tx.query_row(
        "SELECT EXISTS(
            SELECT 1
              FROM compute_attempt_dispatch_commands c
              LEFT JOIN compute_attempt_dispatch_acks a ON a.command_id=c.command_id
             WHERE c.reservation_id=?1
               AND (a.command_id IS NULL OR a.outcome='accepted')
        )",
        params![request.reservation_id],
        |row| row.get(0),
    )?;
    if unresolved_start {
        bail!("Broker 终态必须等待 Attempt Gateway 的明确拒绝或未来 no-start/cancel 证明");
    }
    let reserve =
        broker_reserve_binding_on(tx, &request.reservation_id, &request.consumer_account_id)?;
    if reserve.reservation_revision != source_reservation.revision
        || reserve.reservation_digest != source_reservation.reservation_digest
        || reserve.reserved_job.job_revision != source_job.revision
        || reserve.reserved_job.job_digest != source_job.job_digest
        || reserve.capacity_claim != source_reservation.reservation.capacity_claim
        || reserve.budget_reservation_id
            != source_reservation.reservation.consumer_authorization_ref
    {
        bail!("Broker 终态与原子预留回执绑定不一致");
    }

    let billing = release_billing_call_reservation_on(
        tx,
        &request.consumer_account_id,
        &broker_compute_call_id(&request.reservation_id),
        &reserve.budget_reservation_id,
        billing_terminal_status(request.action),
    )?;
    if billing.reserved_fen != reserve.budget_reserved_fen {
        bail!("Broker 终态退款金额与原子预留回执不一致");
    }
    let (claim_action, job_status, reservation_status) = terminal_states(request.action);
    let claim = finish_compute_capacity_reservation_claim_on(
        tx,
        FinishComputeCapacityClaim {
            claim_id: reserve.capacity_claim.claim_id.clone(),
            expected_revision: reserve.capacity_claim.claim_revision,
            action: claim_action,
            idempotency_scope: format!("compute_broker_finish:{}", request.consumer_account_id),
            idempotency_key: request.idempotency_key.clone(),
            occurred_at: request.occurred_at.clone(),
        },
        ComputeCapacityOfferBinding {
            offer_id: source_reservation.reservation.offer.offer_id.clone(),
            offer_version: source_reservation.reservation.offer.offer_version,
            offer_digest: source_reservation.reservation.offer.offer_digest.clone(),
        },
        &source_job.job.job_id,
        &request.reservation_id,
    )?;

    let mut terminal_job = source_job.job.clone();
    terminal_job.status = job_status.to_string();
    terminal_job.updated_at = claim.recorded_at.clone();
    let terminal_job_receipt = register_compute_job_on(tx, &terminal_job, source_job.revision)?;

    let mut terminal_reservation = source_reservation.reservation.clone();
    terminal_reservation.status = reservation_status.to_string();
    terminal_reservation.updated_at = claim.recorded_at.clone();
    terminal_reservation.released_at = Some(claim.recorded_at.clone());
    terminal_reservation.job = ComputeJobVersionBinding {
        job_id: terminal_job_receipt.job.job_id.clone(),
        job_revision: terminal_job_receipt.revision,
        job_digest: terminal_job_receipt.job_digest.clone(),
    };
    terminal_reservation.capacity_claim = ComputeCapacityClaimBinding {
        claim_id: claim.claim_id.clone(),
        claim_revision: claim.revision,
        claim_digest: claim.claim_digest.clone(),
    };
    let terminal_reservation_receipt =
        register_compute_reservation_on(tx, &terminal_reservation, source_reservation.revision)?;
    persist_broker_finish_receipt_on(
        tx,
        request,
        &billing,
        &source_job,
        &terminal_job_receipt,
        &reserve.capacity_claim,
        &claim,
        &source_reservation,
        &terminal_reservation_receipt,
    )
}

fn terminal_states(
    action: ComputeBrokerFinishAction,
) -> (
    ComputeCapacityClaimTerminalAction,
    &'static str,
    &'static str,
) {
    match action {
        ComputeBrokerFinishAction::Release => (
            ComputeCapacityClaimTerminalAction::Release,
            JOB_STATUS_CANCELED,
            RESERVATION_STATUS_RELEASED,
        ),
        ComputeBrokerFinishAction::Expire => (
            ComputeCapacityClaimTerminalAction::Expire,
            JOB_STATUS_FAILED,
            RESERVATION_STATUS_EXPIRED,
        ),
    }
}
