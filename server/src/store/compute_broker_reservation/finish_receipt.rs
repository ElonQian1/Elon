use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaimBinding, ComputeCapacityClaimState},
    execution::{
        ComputeJobVersionBinding, JOB_STATUS_CANCELED, JOB_STATUS_FAILED, JOB_STATUS_RESERVED,
        RESERVATION_STATUS_ACTIVE, RESERVATION_STATUS_EXPIRED, RESERVATION_STATUS_RELEASED,
    },
};

use super::super::{
    billing_reservations::BillingReservationOutcome,
    compute_capacity_claim_rows::stored_claim_version_on,
    compute_capacity_claim_transitions::FinishComputeCapacityClaimReceipt,
    compute_job_registry::{registered_job_version_on, ComputeJobRegistrationReceipt},
    compute_reservation_registry::{
        registered_reservation_version_on, ComputeReservationRegistrationReceipt,
    },
};
use super::{
    finish_validation::{action_value, billing_terminal_status, NormalizedBrokerFinishRequest},
    validation::broker_compute_call_id,
    ComputeBrokerFinishAction, ComputeBrokerFinishReceipt,
};

struct StoredBrokerFinishReceipt {
    reservation_id: String,
    consumer_account_id: String,
    idempotency_key: String,
    request_digest: String,
    terminal_action: String,
    budget_reservation_id: String,
    budget_terminal_status: String,
    budget_refunded_fen: i64,
    job_id: String,
    source_job_revision: i64,
    source_job_digest: String,
    terminal_job_revision: i64,
    terminal_job_digest: String,
    source_claim_id: String,
    source_claim_revision: i64,
    source_claim_digest: String,
    terminal_claim_revision: i64,
    terminal_claim_digest: String,
    source_reservation_revision: i64,
    source_reservation_digest: String,
    terminal_reservation_revision: i64,
    terminal_reservation_digest: String,
    occurred_at: String,
    recorded_at: String,
}

pub(super) fn replay_broker_finish_on(
    conn: &Connection,
    request: &NormalizedBrokerFinishRequest,
) -> Result<Option<ComputeBrokerFinishReceipt>> {
    let rows = matching_finish_receipts_on(conn, request)?;
    let Some(stored) = rows.first() else {
        return Ok(None);
    };
    if rows.len() != 1
        || stored.reservation_id != request.reservation_id
        || stored.consumer_account_id != request.consumer_account_id
        || stored.idempotency_key != request.idempotency_key
        || stored.request_digest != request.request_digest
        || stored.terminal_action != action_value(request.action)
        || stored.occurred_at != request.occurred_at
    {
        bail!("Broker 终态 Reservation ID 或消费者幂等键不能重放为不同请求");
    }
    audit_finish_receipt_on(conn, request, stored)?;
    Ok(Some(to_receipt(stored, request.action, true)))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn persist_broker_finish_receipt_on(
    conn: &Connection,
    request: &NormalizedBrokerFinishRequest,
    billing: &BillingReservationOutcome,
    source_job: &ComputeJobRegistrationReceipt,
    terminal_job: &ComputeJobRegistrationReceipt,
    source_claim: &ComputeCapacityClaimBinding,
    terminal_claim: &FinishComputeCapacityClaimReceipt,
    source_reservation: &ComputeReservationRegistrationReceipt,
    terminal_reservation: &ComputeReservationRegistrationReceipt,
) -> Result<ComputeBrokerFinishReceipt> {
    conn.execute(
        "INSERT INTO compute_broker_finish_receipts (
            reservation_id, consumer_account_id, idempotency_key,
            request_digest, terminal_action, budget_reservation_id,
            budget_terminal_status, budget_refunded_fen, job_id,
            source_job_revision, source_job_digest, terminal_job_revision,
            terminal_job_digest, source_claim_id, source_claim_revision,
            source_claim_digest, terminal_claim_revision,
            terminal_claim_digest, source_reservation_revision,
            source_reservation_digest, terminal_reservation_revision,
            terminal_reservation_digest, occurred_at, recorded_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
            ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
            ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
         )",
        params![
            request.reservation_id,
            request.consumer_account_id,
            request.idempotency_key,
            request.request_digest,
            action_value(request.action),
            billing.reservation_id,
            billing.status,
            billing.reserved_fen,
            source_job.job.job_id,
            source_job.revision,
            source_job.job_digest,
            terminal_job.revision,
            terminal_job.job_digest,
            source_claim.claim_id,
            source_claim.claim_revision,
            source_claim.claim_digest,
            terminal_claim.revision,
            terminal_claim.claim_digest,
            source_reservation.revision,
            source_reservation.reservation_digest,
            terminal_reservation.revision,
            terminal_reservation.reservation_digest,
            request.occurred_at,
            terminal_claim.recorded_at,
        ],
    )?;
    Ok(ComputeBrokerFinishReceipt {
        reservation_id: request.reservation_id.clone(),
        consumer_account_id: request.consumer_account_id.clone(),
        action: request.action,
        budget_reservation_id: billing.reservation_id.clone(),
        budget_refunded_fen: billing.reserved_fen,
        capacity_claim: ComputeCapacityClaimBinding {
            claim_id: terminal_claim.claim_id.clone(),
            claim_revision: terminal_claim.revision,
            claim_digest: terminal_claim.claim_digest.clone(),
        },
        terminal_job: ComputeJobVersionBinding {
            job_id: terminal_job.job.job_id.clone(),
            job_revision: terminal_job.revision,
            job_digest: terminal_job.job_digest.clone(),
        },
        reservation_revision: terminal_reservation.revision,
        reservation_digest: terminal_reservation.reservation_digest.clone(),
        status: terminal_reservation.reservation.status.clone(),
        recorded_at: terminal_claim.recorded_at.clone(),
        replayed: false,
    })
}

fn matching_finish_receipts_on(
    conn: &Connection,
    request: &NormalizedBrokerFinishRequest,
) -> Result<Vec<StoredBrokerFinishReceipt>> {
    let mut statement = conn.prepare(
        "SELECT reservation_id, consumer_account_id, idempotency_key,
                request_digest, terminal_action, budget_reservation_id,
                budget_terminal_status, budget_refunded_fen, job_id,
                source_job_revision, source_job_digest, terminal_job_revision,
                terminal_job_digest, source_claim_id, source_claim_revision,
                source_claim_digest, terminal_claim_revision,
                terminal_claim_digest, source_reservation_revision,
                source_reservation_digest, terminal_reservation_revision,
                terminal_reservation_digest, occurred_at, recorded_at
           FROM compute_broker_finish_receipts
          WHERE reservation_id=?1
             OR (consumer_account_id=?2 AND idempotency_key=?3)
          ORDER BY reservation_id LIMIT 2",
    )?;
    statement
        .query_map(
            params![
                request.reservation_id,
                request.consumer_account_id,
                request.idempotency_key
            ],
            |row| {
                Ok(StoredBrokerFinishReceipt {
                    reservation_id: row.get(0)?,
                    consumer_account_id: row.get(1)?,
                    idempotency_key: row.get(2)?,
                    request_digest: row.get(3)?,
                    terminal_action: row.get(4)?,
                    budget_reservation_id: row.get(5)?,
                    budget_terminal_status: row.get(6)?,
                    budget_refunded_fen: row.get(7)?,
                    job_id: row.get(8)?,
                    source_job_revision: row.get(9)?,
                    source_job_digest: row.get(10)?,
                    terminal_job_revision: row.get(11)?,
                    terminal_job_digest: row.get(12)?,
                    source_claim_id: row.get(13)?,
                    source_claim_revision: row.get(14)?,
                    source_claim_digest: row.get(15)?,
                    terminal_claim_revision: row.get(16)?,
                    terminal_claim_digest: row.get(17)?,
                    source_reservation_revision: row.get(18)?,
                    source_reservation_digest: row.get(19)?,
                    terminal_reservation_revision: row.get(20)?,
                    terminal_reservation_digest: row.get(21)?,
                    occurred_at: row.get(22)?,
                    recorded_at: row.get(23)?,
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn audit_finish_receipt_on(
    conn: &Connection,
    request: &NormalizedBrokerFinishRequest,
    stored: &StoredBrokerFinishReceipt,
) -> Result<()> {
    let source_job = registered_job_version_on(conn, &stored.job_id, stored.source_job_revision)?
        .ok_or_else(|| anyhow!("Broker 终态回执的 source Job 历史版本缺失"))?;
    let terminal_job =
        registered_job_version_on(conn, &stored.job_id, stored.terminal_job_revision)?
            .ok_or_else(|| anyhow!("Broker 终态回执的 terminal Job 历史版本缺失"))?;
    let source_claim =
        stored_claim_version_on(conn, &stored.source_claim_id, stored.source_claim_revision)?
            .ok_or_else(|| anyhow!("Broker 终态回执的 source Claim 历史版本缺失"))?;
    let terminal_claim = stored_claim_version_on(
        conn,
        &stored.source_claim_id,
        stored.terminal_claim_revision,
    )?
    .ok_or_else(|| anyhow!("Broker 终态回执的 terminal Claim 历史版本缺失"))?;
    let source_reservation = registered_reservation_version_on(
        conn,
        &stored.reservation_id,
        stored.source_reservation_revision,
    )?
    .ok_or_else(|| anyhow!("Broker 终态回执的 source Reservation 历史版本缺失"))?;
    let terminal_reservation = registered_reservation_version_on(
        conn,
        &stored.reservation_id,
        stored.terminal_reservation_revision,
    )?
    .ok_or_else(|| anyhow!("Broker 终态回执的 terminal Reservation 历史版本缺失"))?;
    let billing = billing_finish_on(conn, &stored.budget_reservation_id)?
        .ok_or_else(|| anyhow!("Broker 终态回执的余额退款记录缺失"))?;
    let (job_status, reservation_status, claim_state) = expected_terminal_states(request.action);
    if source_job.job.status != JOB_STATUS_RESERVED
        || source_job.job_digest != stored.source_job_digest
        || terminal_job.job.status != job_status
        || terminal_job.job_digest != stored.terminal_job_digest
        || source_claim.state != ComputeCapacityClaimState::Held
        || source_claim.claim_digest != stored.source_claim_digest
        || terminal_claim.state != claim_state
        || terminal_claim.claim_digest != stored.terminal_claim_digest
        || terminal_claim.updated_at != stored.recorded_at
        || source_reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || source_reservation.reservation_digest != stored.source_reservation_digest
        || source_reservation.revision != request.expected_reservation_revision
        || source_reservation.reservation_digest != request.expected_reservation_digest
        || terminal_reservation.reservation.status != reservation_status
        || terminal_reservation.reservation_digest != stored.terminal_reservation_digest
        || terminal_reservation.reservation.updated_at != stored.recorded_at
        || terminal_reservation.reservation.job.job_revision != stored.terminal_job_revision
        || terminal_reservation.reservation.job.job_digest != stored.terminal_job_digest
        || terminal_reservation
            .reservation
            .capacity_claim
            .claim_revision
            != stored.terminal_claim_revision
        || terminal_reservation.reservation.capacity_claim.claim_digest
            != stored.terminal_claim_digest
        || billing.user_id != stored.consumer_account_id
        || billing.compute_call_id != broker_compute_call_id(&stored.reservation_id)
        || billing.status != stored.budget_terminal_status
        || billing.status != billing_terminal_status(request.action)
        || billing.refunded_fen != stored.budget_refunded_fen
    {
        bail!("Broker 原子终态回执的历史绑定审计失败");
    }
    Ok(())
}

fn expected_terminal_states(
    action: ComputeBrokerFinishAction,
) -> (&'static str, &'static str, ComputeCapacityClaimState) {
    match action {
        ComputeBrokerFinishAction::Release => (
            JOB_STATUS_CANCELED,
            RESERVATION_STATUS_RELEASED,
            ComputeCapacityClaimState::Released,
        ),
        ComputeBrokerFinishAction::Expire => (
            JOB_STATUS_FAILED,
            RESERVATION_STATUS_EXPIRED,
            ComputeCapacityClaimState::Expired,
        ),
    }
}

struct BillingFinish {
    user_id: String,
    compute_call_id: String,
    status: String,
    refunded_fen: i64,
}

fn billing_finish_on(conn: &Connection, reservation_id: &str) -> Result<Option<BillingFinish>> {
    conn.query_row(
        "SELECT user_id, compute_call_id, status, refunded_fen
           FROM billing_reservations WHERE id=?1",
        params![reservation_id],
        |row| {
            Ok(BillingFinish {
                user_id: row.get(0)?,
                compute_call_id: row.get(1)?,
                status: row.get(2)?,
                refunded_fen: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn to_receipt(
    stored: &StoredBrokerFinishReceipt,
    action: ComputeBrokerFinishAction,
    replayed: bool,
) -> ComputeBrokerFinishReceipt {
    let (_, reservation_status, _) = expected_terminal_states(action);
    ComputeBrokerFinishReceipt {
        reservation_id: stored.reservation_id.clone(),
        consumer_account_id: stored.consumer_account_id.clone(),
        action,
        budget_reservation_id: stored.budget_reservation_id.clone(),
        budget_refunded_fen: stored.budget_refunded_fen,
        capacity_claim: ComputeCapacityClaimBinding {
            claim_id: stored.source_claim_id.clone(),
            claim_revision: stored.terminal_claim_revision,
            claim_digest: stored.terminal_claim_digest.clone(),
        },
        terminal_job: ComputeJobVersionBinding {
            job_id: stored.job_id.clone(),
            job_revision: stored.terminal_job_revision,
            job_digest: stored.terminal_job_digest.clone(),
        },
        reservation_revision: stored.terminal_reservation_revision,
        reservation_digest: stored.terminal_reservation_digest.clone(),
        status: reservation_status.to_string(),
        recorded_at: stored.recorded_at.clone(),
        replayed,
    }
}
