use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaim, ComputeCapacityClaimBinding, ComputeCapacityClaimState},
    execution::{
        ComputeJobVersionBinding, JOB_STATUS_QUOTED, JOB_STATUS_RESERVED, RESERVATION_STATUS_ACTIVE,
    },
};

use super::super::{
    billing_reservations::BillingReservationOutcome,
    compute_capacity_claim_rows::stored_claim_version_on,
    compute_job_registry::{registered_job_version_on, ComputeJobRegistrationReceipt},
    compute_reservation_registry::{
        registered_reservation_version_on, ComputeReservationRegistrationReceipt,
    },
    now,
};
use super::{
    validation::{broker_compute_call_id, NormalizedBrokerReserveRequest},
    ComputeBrokerReservationReceipt, BROKER_BILLING_FEATURE, BROKER_BILLING_USAGE_MODE,
    BROKER_BUDGET_ADAPTER,
};

struct StoredBrokerReserveReceipt {
    reservation_id: String,
    consumer_account_id: String,
    idempotency_key: String,
    request_digest: String,
    budget_adapter: String,
    budget_reservation_id: String,
    budget_reserved_fen: i64,
    capacity_claim_id: String,
    capacity_claim_revision: i64,
    capacity_claim_digest: String,
    job_id: String,
    source_job_revision: i64,
    source_job_digest: String,
    reserved_job_revision: i64,
    reserved_job_digest: String,
    reservation_revision: i64,
    reservation_digest: String,
}

pub(crate) struct BrokerReserveBinding {
    pub budget_reservation_id: String,
    pub budget_reserved_fen: i64,
    pub capacity_claim: ComputeCapacityClaimBinding,
    pub reserved_job: ComputeJobVersionBinding,
    pub reservation_revision: i64,
    pub reservation_digest: String,
}

pub(crate) fn broker_reserve_binding_on(
    conn: &Connection,
    reservation_id: &str,
    consumer_account_id: &str,
) -> Result<BrokerReserveBinding> {
    let stored = conn
        .query_row(
            "SELECT budget_reservation_id, budget_reserved_fen,
                    capacity_claim_id, capacity_claim_revision,
                    capacity_claim_digest, job_id, reserved_job_revision,
                    reserved_job_digest, reservation_revision,
                    reservation_digest, budget_adapter
               FROM compute_broker_reserve_receipts
              WHERE reservation_id=?1 AND consumer_account_id=?2",
            params![reservation_id.trim(), consumer_account_id.trim()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("Reservation 缺少平台余额 Broker 原子预留回执"))?;
    if stored.10 != BROKER_BUDGET_ADAPTER {
        bail!("Reservation 的 Broker 预算适配器不受当前终态入口支持");
    }
    Ok(BrokerReserveBinding {
        budget_reservation_id: stored.0,
        budget_reserved_fen: stored.1,
        capacity_claim: ComputeCapacityClaimBinding {
            claim_id: stored.2,
            claim_revision: stored.3,
            claim_digest: stored.4,
        },
        reserved_job: ComputeJobVersionBinding {
            job_id: stored.5,
            job_revision: stored.6,
            job_digest: stored.7,
        },
        reservation_revision: stored.8,
        reservation_digest: stored.9,
    })
}

pub(super) fn replay_broker_reserve_on(
    conn: &Connection,
    request: &NormalizedBrokerReserveRequest,
) -> Result<Option<ComputeBrokerReservationReceipt>> {
    let rows = matching_receipts_on(conn, request)?;
    let Some(stored) = rows.first() else {
        return Ok(None);
    };
    if rows.len() != 1
        || stored.reservation_id != request.reservation_id
        || stored.consumer_account_id != request.consumer_account_id
        || stored.idempotency_key != request.idempotency_key
        || stored.request_digest != request.request_digest
        || stored.budget_adapter != BROKER_BUDGET_ADAPTER
    {
        bail!("Broker Reservation ID 或消费者幂等键不能重放为不同请求");
    }
    audit_stored_receipt_on(conn, request, stored)?;
    Ok(Some(to_receipt(stored, true)))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn persist_broker_reserve_receipt_on(
    conn: &Connection,
    request: &NormalizedBrokerReserveRequest,
    budget: &BillingReservationOutcome,
    claim: &ComputeCapacityClaim,
    source_job: &ComputeJobRegistrationReceipt,
    reserved_job: &ComputeJobRegistrationReceipt,
    reservation: &ComputeReservationRegistrationReceipt,
) -> Result<ComputeBrokerReservationReceipt> {
    conn.execute(
        "INSERT INTO compute_broker_reserve_receipts (
            reservation_id, consumer_account_id, idempotency_key,
            request_digest, budget_adapter, budget_reservation_id,
            budget_reserved_fen, capacity_claim_id,
            capacity_claim_revision, capacity_claim_digest, job_id,
            source_job_revision, source_job_digest, reserved_job_revision,
            reserved_job_digest, reservation_revision,
            reservation_digest, created_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
            ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18
         )",
        params![
            request.reservation_id,
            request.consumer_account_id,
            request.idempotency_key,
            request.request_digest,
            BROKER_BUDGET_ADAPTER,
            budget.reservation_id,
            budget.reserved_fen,
            claim.claim_id,
            claim.revision,
            claim.claim_digest,
            source_job.job.job_id,
            source_job.revision,
            source_job.job_digest,
            reserved_job.revision,
            reserved_job.job_digest,
            reservation.revision,
            reservation.reservation_digest,
            now(),
        ],
    )?;
    Ok(ComputeBrokerReservationReceipt {
        reservation_id: reservation.reservation.reservation_id.clone(),
        consumer_account_id: request.consumer_account_id.clone(),
        budget_adapter: BROKER_BUDGET_ADAPTER.to_string(),
        budget_reservation_id: budget.reservation_id.clone(),
        budget_reserved_fen: budget.reserved_fen,
        capacity_claim: ComputeCapacityClaimBinding {
            claim_id: claim.claim_id.clone(),
            claim_revision: claim.revision,
            claim_digest: claim.claim_digest.clone(),
        },
        reserved_job: ComputeJobVersionBinding {
            job_id: reserved_job.job.job_id.clone(),
            job_revision: reserved_job.revision,
            job_digest: reserved_job.job_digest.clone(),
        },
        reservation_revision: reservation.revision,
        reservation_digest: reservation.reservation_digest.clone(),
        status: reservation.reservation.status.clone(),
        replayed: false,
    })
}

fn matching_receipts_on(
    conn: &Connection,
    request: &NormalizedBrokerReserveRequest,
) -> Result<Vec<StoredBrokerReserveReceipt>> {
    let mut statement = conn.prepare(
        "SELECT reservation_id, consumer_account_id, idempotency_key,
                request_digest, budget_adapter, budget_reservation_id,
                budget_reserved_fen, capacity_claim_id,
                capacity_claim_revision, capacity_claim_digest, job_id,
                source_job_revision, source_job_digest, reserved_job_revision,
                reserved_job_digest, reservation_revision, reservation_digest
           FROM compute_broker_reserve_receipts
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
                Ok(StoredBrokerReserveReceipt {
                    reservation_id: row.get(0)?,
                    consumer_account_id: row.get(1)?,
                    idempotency_key: row.get(2)?,
                    request_digest: row.get(3)?,
                    budget_adapter: row.get(4)?,
                    budget_reservation_id: row.get(5)?,
                    budget_reserved_fen: row.get(6)?,
                    capacity_claim_id: row.get(7)?,
                    capacity_claim_revision: row.get(8)?,
                    capacity_claim_digest: row.get(9)?,
                    job_id: row.get(10)?,
                    source_job_revision: row.get(11)?,
                    source_job_digest: row.get(12)?,
                    reserved_job_revision: row.get(13)?,
                    reserved_job_digest: row.get(14)?,
                    reservation_revision: row.get(15)?,
                    reservation_digest: row.get(16)?,
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn audit_stored_receipt_on(
    conn: &Connection,
    request: &NormalizedBrokerReserveRequest,
    stored: &StoredBrokerReserveReceipt,
) -> Result<()> {
    let source_job = registered_job_version_on(conn, &stored.job_id, stored.source_job_revision)?
        .ok_or_else(|| anyhow!("Broker 回执绑定的 quoted Job 历史版本缺失"))?;
    let reserved_job =
        registered_job_version_on(conn, &stored.job_id, stored.reserved_job_revision)?
            .ok_or_else(|| anyhow!("Broker 回执绑定的 reserved Job 历史版本缺失"))?;
    let claim = stored_claim_version_on(
        conn,
        &stored.capacity_claim_id,
        stored.capacity_claim_revision,
    )?
    .ok_or_else(|| anyhow!("Broker 回执绑定的 Capacity Claim 历史版本缺失"))?;
    let reservation = registered_reservation_version_on(
        conn,
        &stored.reservation_id,
        stored.reservation_revision,
    )?
    .ok_or_else(|| anyhow!("Broker 回执绑定的 Reservation 历史版本缺失"))?;
    let billing = billing_contract_on(conn, &stored.budget_reservation_id)?
        .ok_or_else(|| anyhow!("Broker 回执绑定的余额预授权不存在"))?;
    if source_job.job.status != JOB_STATUS_QUOTED
        || source_job.job_digest != stored.source_job_digest
        || source_job.revision != request.expected_job_revision
        || source_job.job_digest != request.expected_job_digest
        || reserved_job.job.status != JOB_STATUS_RESERVED
        || reserved_job.job_digest != stored.reserved_job_digest
        || claim.state != ComputeCapacityClaimState::Held
        || claim.claim_digest != stored.capacity_claim_digest
        || reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || reservation.reservation_digest != stored.reservation_digest
        || reservation.reservation.consumer_authorization_ref != stored.budget_reservation_id
        || reservation.reservation.capacity_claim.claim_id != stored.capacity_claim_id
        || reservation.reservation.capacity_claim.claim_revision != stored.capacity_claim_revision
        || reservation.reservation.capacity_claim.claim_digest != stored.capacity_claim_digest
        || reservation.reservation.job.job_revision != stored.reserved_job_revision
        || reservation.reservation.job.job_digest != stored.reserved_job_digest
        || billing.user_id != stored.consumer_account_id
        || billing.compute_call_id != broker_compute_call_id(&stored.reservation_id)
        || billing.feature != BROKER_BILLING_FEATURE
        || billing.usage_mode != BROKER_BILLING_USAGE_MODE
        || billing.model.as_deref()
            != source_job
                .job
                .workload
                .model
                .as_ref()
                .map(|value| value.model_id.as_str())
        || billing.reserved_fen != stored.budget_reserved_fen
    {
        bail!("Broker 原子预留回执的历史绑定审计失败");
    }
    Ok(())
}

struct BillingContract {
    user_id: String,
    compute_call_id: String,
    feature: String,
    usage_mode: String,
    model: Option<String>,
    reserved_fen: i64,
}

fn billing_contract_on(conn: &Connection, reservation_id: &str) -> Result<Option<BillingContract>> {
    conn.query_row(
        "SELECT user_id, compute_call_id, feature, usage_mode,
                model, reserved_fen
           FROM billing_reservations WHERE id=?1",
        params![reservation_id],
        |row| {
            Ok(BillingContract {
                user_id: row.get(0)?,
                compute_call_id: row.get(1)?,
                feature: row.get(2)?,
                usage_mode: row.get(3)?,
                model: row.get(4)?,
                reserved_fen: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn to_receipt(
    stored: &StoredBrokerReserveReceipt,
    replayed: bool,
) -> ComputeBrokerReservationReceipt {
    ComputeBrokerReservationReceipt {
        reservation_id: stored.reservation_id.clone(),
        consumer_account_id: stored.consumer_account_id.clone(),
        budget_adapter: stored.budget_adapter.clone(),
        budget_reservation_id: stored.budget_reservation_id.clone(),
        budget_reserved_fen: stored.budget_reserved_fen,
        capacity_claim: ComputeCapacityClaimBinding {
            claim_id: stored.capacity_claim_id.clone(),
            claim_revision: stored.capacity_claim_revision,
            claim_digest: stored.capacity_claim_digest.clone(),
        },
        reserved_job: ComputeJobVersionBinding {
            job_id: stored.job_id.clone(),
            job_revision: stored.reserved_job_revision,
            job_digest: stored.reserved_job_digest.clone(),
        },
        reservation_revision: stored.reservation_revision,
        reservation_digest: stored.reservation_digest.clone(),
        status: RESERVATION_STATUS_ACTIVE.to_string(),
        replayed,
    }
}
