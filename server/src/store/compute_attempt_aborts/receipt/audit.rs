use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaimBinding, ComputeCapacityClaimState},
    execution::{
        ComputeAttemptLease, ComputeJobVersionBinding, ATTEMPT_STATUS_TERMINAL,
        JOB_STATUS_CANCELED, JOB_STATUS_RUNNING, RESERVATION_STATUS_ACTIVE,
        RESERVATION_STATUS_RELEASED,
    },
};

use super::{
    super::super::{
        compute_attempt_activations::compute_attempt_activation_on,
        compute_attempt_leases::compute_attempt_lease_digest,
        compute_capacity_claim_rows::stored_claim_version_on,
        compute_capacity_ledger::ComputeCapacityLedgerWriteReceipt,
        compute_capacity_posting::balances_for_transaction_on,
        compute_job_registry::registered_job_version_on,
        compute_reservation_registry::registered_reservation_version_on,
    },
    super::{ComputeAttemptAbortReceipt, COMPUTE_ATTEMPT_ABORT_SCHEMA},
    StoredAttemptAbort,
};

#[derive(Serialize)]
struct CanonicalAbortEvent<'a> {
    schema: &'static str,
    abort_id: &'a str,
    lease_id: &'a str,
    provider_id: &'a str,
    consumer_account_id: &'a str,
    executor_abort_ref: &'a str,
    reason_code: &'a str,
    fencing_generation: i64,
    source_lease_revision: i64,
    source_lease_digest: &'a str,
    terminal_lease_revision: i64,
    terminal_lease_digest: &'a str,
    terminal_lease_json: &'a str,
    job_id: &'a str,
    source_job_revision: i64,
    source_job_digest: &'a str,
    terminal_job_revision: i64,
    terminal_job_digest: &'a str,
    reservation_id: &'a str,
    source_reservation_revision: i64,
    source_reservation_digest: &'a str,
    terminal_reservation_revision: i64,
    terminal_reservation_digest: &'a str,
    capacity_claim_id: &'a str,
    source_claim_revision: i64,
    source_claim_digest: &'a str,
    terminal_claim_revision: i64,
    terminal_claim_digest: &'a str,
    budget_reservation_id: &'a str,
    budget_refunded_fen: i64,
    budget_terminal_status: &'a str,
    capacity_transaction_id: &'a str,
    capacity_transaction_digest: &'a str,
    activation_request_digest: &'a str,
    request_digest: &'a str,
    idempotency_scope: &'a str,
    idempotency_key: &'a str,
    aborted_by_user_id: &'a str,
    aborted_at: &'a str,
}

pub(super) fn audit_and_convert(
    conn: &Connection,
    stored: &StoredAttemptAbort,
    replayed: bool,
) -> Result<ComputeAttemptAbortReceipt> {
    let terminal_lease: ComputeAttemptLease = serde_json::from_str(&stored.terminal_lease_json)?;
    if compute_attempt_lease_digest(&terminal_lease)? != stored.terminal_lease_digest
        || terminal_lease.lease_id != stored.lease_id
        || terminal_lease.provider_id != stored.provider_id
        || terminal_lease.job_id != stored.job_id
        || terminal_lease.reservation_id != stored.reservation_id
        || terminal_lease.fencing_generation != stored.fencing_generation
        || terminal_lease.status != ATTEMPT_STATUS_TERMINAL
        || terminal_lease.last_heartbeat_at.is_some()
        || terminal_lease.terminal_reason_code.as_deref() != Some(stored.reason_code.as_str())
        || stored.terminal_lease_revision != stored.source_lease_revision + 1
        || abort_event_digest(stored)? != stored.event_digest
    {
        bail!("Attempt 中止回执的 Lease 快照或事件摘要审计失败");
    }
    let activation = compute_attempt_activation_on(conn, &stored.lease_id)?;
    if activation.request_digest != stored.activation_request_digest
        || stored.source_lease_revision != 1
        || stored.source_lease_digest != activation.lease_digest
        || activation.running_job.job_revision != stored.source_job_revision
        || activation.running_job.job_digest != stored.source_job_digest
        || activation.active_reservation_revision != stored.source_reservation_revision
        || activation.active_reservation_digest != stored.source_reservation_digest
        || activation.active_claim.claim_revision != stored.source_claim_revision
        || activation.active_claim.claim_digest != stored.source_claim_digest
        || activation.budget_reservation_id != stored.budget_reservation_id
        || activation.budget_reserved_fen != stored.budget_refunded_fen
    {
        bail!("Attempt 中止回执与原始激活回执不一致");
    }
    audit_versions(conn, stored)?;
    audit_billing_and_capacity(conn, stored, &activation.capacity_ledger.transaction_id)?;
    let balances = balances_for_transaction_on(conn, &stored.capacity_transaction_id)?;
    Ok(ComputeAttemptAbortReceipt {
        schema: COMPUTE_ATTEMPT_ABORT_SCHEMA,
        abort_id: stored.abort_id.clone(),
        terminal_lease,
        source_lease_revision: stored.source_lease_revision,
        source_lease_digest: stored.source_lease_digest.clone(),
        terminal_lease_revision: stored.terminal_lease_revision,
        terminal_lease_digest: stored.terminal_lease_digest.clone(),
        source_job: ComputeJobVersionBinding {
            job_id: stored.job_id.clone(),
            job_revision: stored.source_job_revision,
            job_digest: stored.source_job_digest.clone(),
        },
        terminal_job: ComputeJobVersionBinding {
            job_id: stored.job_id.clone(),
            job_revision: stored.terminal_job_revision,
            job_digest: stored.terminal_job_digest.clone(),
        },
        source_reservation_revision: stored.source_reservation_revision,
        source_reservation_digest: stored.source_reservation_digest.clone(),
        terminal_reservation_revision: stored.terminal_reservation_revision,
        terminal_reservation_digest: stored.terminal_reservation_digest.clone(),
        source_claim: ComputeCapacityClaimBinding {
            claim_id: stored.capacity_claim_id.clone(),
            claim_revision: stored.source_claim_revision,
            claim_digest: stored.source_claim_digest.clone(),
        },
        returned_claim: ComputeCapacityClaimBinding {
            claim_id: stored.capacity_claim_id.clone(),
            claim_revision: stored.terminal_claim_revision,
            claim_digest: stored.terminal_claim_digest.clone(),
        },
        budget_reservation_id: stored.budget_reservation_id.clone(),
        budget_refunded_fen: stored.budget_refunded_fen,
        budget_terminal_status: stored.budget_terminal_status.clone(),
        capacity_ledger: ComputeCapacityLedgerWriteReceipt {
            transaction_id: stored.capacity_transaction_id.clone(),
            transaction_digest: stored.capacity_transaction_digest.clone(),
            ledger_sequence: capacity_ledger_sequence_on(conn, &stored.capacity_transaction_id)?,
            event_kind: "attempt_returned".to_string(),
            request_digest: stored.request_digest.clone(),
            replayed,
            current_balances: balances,
        },
        activation_request_digest: stored.activation_request_digest.clone(),
        executor_abort_ref: stored.executor_abort_ref.clone(),
        reason_code: stored.reason_code.clone(),
        request_digest: stored.request_digest.clone(),
        event_digest: stored.event_digest.clone(),
        aborted_by_user_id: stored.aborted_by_user_id.clone(),
        aborted_at: stored.aborted_at.clone(),
        execution_effect: "external_abort_assertion_only",
        capacity_effect: "returned_to_available",
        reservation_effect: "released",
        money_effect: "preauthorization_refunded",
        replayed,
    })
}

fn audit_versions(conn: &Connection, stored: &StoredAttemptAbort) -> Result<()> {
    let source_job = registered_job_version_on(conn, &stored.job_id, stored.source_job_revision)?
        .ok_or_else(|| anyhow!("Attempt 中止引用的 source Job 历史版本不存在"))?;
    let terminal_job =
        registered_job_version_on(conn, &stored.job_id, stored.terminal_job_revision)?
            .ok_or_else(|| anyhow!("Attempt 中止引用的 terminal Job 历史版本不存在"))?;
    if source_job.job_digest != stored.source_job_digest
        || source_job.job.status != JOB_STATUS_RUNNING
        || terminal_job.job_digest != stored.terminal_job_digest
        || terminal_job.job.status != JOB_STATUS_CANCELED
        || terminal_job.job.updated_at != stored.aborted_at
    {
        bail!("Attempt 中止引用的 Job 历史版本审计失败");
    }
    let source_reservation = registered_reservation_version_on(
        conn,
        &stored.reservation_id,
        stored.source_reservation_revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 中止引用的 source Reservation 历史版本不存在"))?;
    let terminal_reservation = registered_reservation_version_on(
        conn,
        &stored.reservation_id,
        stored.terminal_reservation_revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 中止引用的 terminal Reservation 历史版本不存在"))?;
    if source_reservation.reservation_digest != stored.source_reservation_digest
        || source_reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || terminal_reservation.reservation_digest != stored.terminal_reservation_digest
        || terminal_reservation.reservation.status != RESERVATION_STATUS_RELEASED
        || terminal_reservation.reservation.released_at.as_deref()
            != Some(stored.aborted_at.as_str())
    {
        bail!("Attempt 中止引用的 Reservation 历史版本审计失败");
    }
    let source_claim = stored_claim_version_on(
        conn,
        &stored.capacity_claim_id,
        stored.source_claim_revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 中止引用的 source Claim 历史版本不存在"))?;
    let terminal_claim = stored_claim_version_on(
        conn,
        &stored.capacity_claim_id,
        stored.terminal_claim_revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 中止引用的 terminal Claim 历史版本不存在"))?;
    if source_claim.claim_digest != stored.source_claim_digest
        || source_claim.state != ComputeCapacityClaimState::Active
        || terminal_claim.claim_digest != stored.terminal_claim_digest
        || terminal_claim.state != ComputeCapacityClaimState::Released
        || terminal_claim.terminal_at.as_deref() != Some(stored.aborted_at.as_str())
    {
        bail!("Attempt 中止引用的 Capacity Claim 历史版本审计失败");
    }
    Ok(())
}

fn audit_billing_and_capacity(
    conn: &Connection,
    stored: &StoredAttemptAbort,
    activation_transaction_id: &str,
) -> Result<()> {
    let billing = conn
        .query_row(
            "SELECT user_id, reserved_fen, refunded_fen, status
               FROM billing_reservations WHERE id=?1",
            params![stored.budget_reservation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("Attempt 中止引用的预算预授权不存在"))?;
    if billing.0 != stored.consumer_account_id
        || billing.1 != stored.budget_refunded_fen
        || billing.2 != stored.budget_refunded_fen
        || billing.3 != stored.budget_terminal_status
        || stored.budget_terminal_status != "released_no_usage"
    {
        bail!("Attempt 中止预算退款审计失败");
    }
    let capacity = conn
        .query_row(
            "SELECT transaction_digest, event_kind, claim_id, claim_effect,
                    claim_effect_key, job_id, reservation_id, attempt_lease_id,
                    fencing_generation, idempotency_scope, idempotency_key,
                    request_digest, subject_kind, subject_id, recorded_at,
                    causal_transaction_id
               FROM compute_capacity_ledger_transactions WHERE transaction_id=?1",
            params![stored.capacity_transaction_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, String>(14)?,
                    row.get::<_, String>(15)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("Attempt 中止引用的容量账本事务不存在"))?;
    if capacity.0 != stored.capacity_transaction_digest
        || capacity.1 != "attempt_returned"
        || capacity.2 != stored.capacity_claim_id
        || capacity.3 != "released"
        || capacity.4 != stored.idempotency_key
        || capacity.5 != stored.job_id
        || capacity.6 != stored.reservation_id
        || capacity.7 != stored.lease_id
        || capacity.8 != stored.fencing_generation
        || capacity.9 != stored.idempotency_scope
        || capacity.10 != stored.idempotency_key
        || capacity.11 != stored.request_digest
        || capacity.12 != "compute_attempt_lease"
        || capacity.13 != stored.lease_id
        || capacity.14 != stored.aborted_at
        || capacity.15 != activation_transaction_id
    {
        bail!("Attempt 中止容量归还事务审计失败");
    }
    Ok(())
}

fn capacity_ledger_sequence_on(conn: &Connection, transaction_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT ledger_sequence FROM compute_capacity_ledger_transactions WHERE transaction_id=?1",
        params![transaction_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub(super) fn abort_event_digest(stored: &StoredAttemptAbort) -> Result<String> {
    let canonical = CanonicalAbortEvent {
        schema: COMPUTE_ATTEMPT_ABORT_SCHEMA,
        abort_id: &stored.abort_id,
        lease_id: &stored.lease_id,
        provider_id: &stored.provider_id,
        consumer_account_id: &stored.consumer_account_id,
        executor_abort_ref: &stored.executor_abort_ref,
        reason_code: &stored.reason_code,
        fencing_generation: stored.fencing_generation,
        source_lease_revision: stored.source_lease_revision,
        source_lease_digest: &stored.source_lease_digest,
        terminal_lease_revision: stored.terminal_lease_revision,
        terminal_lease_digest: &stored.terminal_lease_digest,
        terminal_lease_json: &stored.terminal_lease_json,
        job_id: &stored.job_id,
        source_job_revision: stored.source_job_revision,
        source_job_digest: &stored.source_job_digest,
        terminal_job_revision: stored.terminal_job_revision,
        terminal_job_digest: &stored.terminal_job_digest,
        reservation_id: &stored.reservation_id,
        source_reservation_revision: stored.source_reservation_revision,
        source_reservation_digest: &stored.source_reservation_digest,
        terminal_reservation_revision: stored.terminal_reservation_revision,
        terminal_reservation_digest: &stored.terminal_reservation_digest,
        capacity_claim_id: &stored.capacity_claim_id,
        source_claim_revision: stored.source_claim_revision,
        source_claim_digest: &stored.source_claim_digest,
        terminal_claim_revision: stored.terminal_claim_revision,
        terminal_claim_digest: &stored.terminal_claim_digest,
        budget_reservation_id: &stored.budget_reservation_id,
        budget_refunded_fen: stored.budget_refunded_fen,
        budget_terminal_status: &stored.budget_terminal_status,
        capacity_transaction_id: &stored.capacity_transaction_id,
        capacity_transaction_digest: &stored.capacity_transaction_digest,
        activation_request_digest: &stored.activation_request_digest,
        request_digest: &stored.request_digest,
        idempotency_scope: &stored.idempotency_scope,
        idempotency_key: &stored.idempotency_key,
        aborted_by_user_id: &stored.aborted_by_user_id,
        aborted_at: &stored.aborted_at,
    };
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
}
