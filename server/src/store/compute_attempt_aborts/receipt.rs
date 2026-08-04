use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection};

use crate::compute_federation::capacity::ComputeCapacityClaimBinding;

use super::{
    super::{
        billing_reservations::BillingReservationOutcome,
        compute_attempt_activations::{
            compute_attempt_activation_on, ComputeAttemptActivationReceipt,
        },
        compute_attempt_leases::ComputeAttemptLeaseStateReceipt,
        compute_capacity_claim_return::ReturnAttemptCapacityClaimReceipt,
        compute_job_registry::ComputeJobRegistrationReceipt,
        compute_reservation_registry::ComputeReservationRegistrationReceipt,
    },
    validation::NormalizedAttemptAbort,
    ComputeAttemptAbortReceipt, COMPUTE_ATTEMPT_ABORT_SCHEMA,
};

mod audit;

use audit::{abort_event_digest, audit_and_convert};

pub(super) struct AttemptAbortPersistence<'a> {
    pub abort_id: String,
    pub request: &'a NormalizedAttemptAbort,
    pub activation: &'a ComputeAttemptActivationReceipt,
    pub source_lease: &'a ComputeAttemptLeaseStateReceipt,
    pub terminal_lease: &'a ComputeAttemptLeaseStateReceipt,
    pub source_job: &'a ComputeJobRegistrationReceipt,
    pub terminal_job: &'a ComputeJobRegistrationReceipt,
    pub source_reservation: &'a ComputeReservationRegistrationReceipt,
    pub terminal_reservation: &'a ComputeReservationRegistrationReceipt,
    pub source_claim: ComputeCapacityClaimBinding,
    pub returned_capacity: &'a ReturnAttemptCapacityClaimReceipt,
    pub billing: &'a BillingReservationOutcome,
    pub aborted_at: &'a str,
}

pub(super) struct StoredAttemptAbort {
    abort_id: String,
    lease_id: String,
    provider_id: String,
    consumer_account_id: String,
    executor_abort_ref: String,
    reason_code: String,
    fencing_generation: i64,
    source_lease_revision: i64,
    source_lease_digest: String,
    terminal_lease_revision: i64,
    terminal_lease_digest: String,
    terminal_lease_json: String,
    job_id: String,
    source_job_revision: i64,
    source_job_digest: String,
    terminal_job_revision: i64,
    terminal_job_digest: String,
    reservation_id: String,
    source_reservation_revision: i64,
    source_reservation_digest: String,
    terminal_reservation_revision: i64,
    terminal_reservation_digest: String,
    capacity_claim_id: String,
    source_claim_revision: i64,
    source_claim_digest: String,
    terminal_claim_revision: i64,
    terminal_claim_digest: String,
    budget_reservation_id: String,
    budget_refunded_fen: i64,
    budget_terminal_status: String,
    capacity_transaction_id: String,
    capacity_transaction_digest: String,
    activation_request_digest: String,
    request_digest: String,
    event_digest: String,
    idempotency_scope: String,
    idempotency_key: String,
    aborted_by_user_id: String,
    aborted_at: String,
}

pub(super) fn persist_attempt_abort_on(
    conn: &Connection,
    input: AttemptAbortPersistence<'_>,
) -> Result<ComputeAttemptAbortReceipt> {
    let mut stored = StoredAttemptAbort {
        abort_id: input.abort_id,
        lease_id: input.request.lease_id.clone(),
        provider_id: input.request.provider_id.clone(),
        consumer_account_id: input.source_job.job.consumer_account_id.clone(),
        executor_abort_ref: input.request.executor_abort_ref.clone(),
        reason_code: input.request.reason_code.clone(),
        fencing_generation: input.request.expected_fencing_generation,
        source_lease_revision: input.source_lease.lease_revision,
        source_lease_digest: input.source_lease.lease_digest.clone(),
        terminal_lease_revision: input.terminal_lease.lease_revision,
        terminal_lease_digest: input.terminal_lease.lease_digest.clone(),
        terminal_lease_json: serde_json::to_string(&input.terminal_lease.lease)?,
        job_id: input.source_job.job.job_id.clone(),
        source_job_revision: input.source_job.revision,
        source_job_digest: input.source_job.job_digest.clone(),
        terminal_job_revision: input.terminal_job.revision,
        terminal_job_digest: input.terminal_job.job_digest.clone(),
        reservation_id: input.source_reservation.reservation.reservation_id.clone(),
        source_reservation_revision: input.source_reservation.revision,
        source_reservation_digest: input.source_reservation.reservation_digest.clone(),
        terminal_reservation_revision: input.terminal_reservation.revision,
        terminal_reservation_digest: input.terminal_reservation.reservation_digest.clone(),
        capacity_claim_id: input.source_claim.claim_id.clone(),
        source_claim_revision: input.source_claim.claim_revision,
        source_claim_digest: input.source_claim.claim_digest.clone(),
        terminal_claim_revision: input.returned_capacity.claim.claim_revision,
        terminal_claim_digest: input.returned_capacity.claim.claim_digest.clone(),
        budget_reservation_id: input.billing.reservation_id.clone(),
        budget_refunded_fen: input.billing.reserved_fen,
        budget_terminal_status: input.billing.status.clone(),
        capacity_transaction_id: input.returned_capacity.ledger.transaction_id.clone(),
        capacity_transaction_digest: input.returned_capacity.ledger.transaction_digest.clone(),
        activation_request_digest: input.activation.request_digest.clone(),
        request_digest: input.request.request_digest.clone(),
        event_digest: String::new(),
        idempotency_scope: input.request.idempotency_scope.clone(),
        idempotency_key: input.request.idempotency_key.clone(),
        aborted_by_user_id: input.request.aborted_by_user_id.clone(),
        aborted_at: input.aborted_at.to_string(),
    };
    stored.event_digest = abort_event_digest(&stored)?;
    insert_abort_on(conn, &stored)?;
    attempt_abort_by_lease_on(conn, &stored.lease_id)?
        .ok_or_else(|| anyhow!("Attempt 中止回执写入后不可见"))
}

pub(super) fn replay_attempt_abort_on(
    conn: &Connection,
    request: &NormalizedAttemptAbort,
) -> Result<Option<ComputeAttemptAbortReceipt>> {
    let rows = stored_aborts_on(
        conn,
        "WHERE lease_id=?1 OR (idempotency_scope=?2 AND idempotency_key=?3)",
        params![
            request.lease_id,
            request.idempotency_scope,
            request.idempotency_key
        ],
    )?;
    let Some(stored) = rows.first() else {
        return Ok(None);
    };
    if rows.len() != 1
        || stored.lease_id != request.lease_id
        || stored.provider_id != request.provider_id
        || stored.source_lease_revision != request.expected_lease_revision
        || stored.source_lease_digest != request.expected_lease_digest
        || stored.fencing_generation != request.expected_fencing_generation
        || stored.source_job_revision != request.expected_job_revision
        || stored.source_job_digest != request.expected_job_digest
        || stored.source_reservation_revision != request.expected_reservation_revision
        || stored.source_reservation_digest != request.expected_reservation_digest
        || stored.source_claim_revision != request.expected_claim_revision
        || stored.source_claim_digest != request.expected_claim_digest
        || stored.executor_abort_ref != request.executor_abort_ref
        || stored.reason_code != request.reason_code
        || stored.idempotency_scope != request.idempotency_scope
        || stored.idempotency_key != request.idempotency_key
        || stored.aborted_by_user_id != request.aborted_by_user_id
        || stored.request_digest != request.request_digest
    {
        bail!("Attempt 中止 Lease 或幂等键不能重放为不同请求");
    }
    audit_and_convert(conn, stored, true).map(Some)
}

pub(super) fn attempt_abort_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<ComputeAttemptAbortReceipt>> {
    let rows = stored_aborts_on(conn, "WHERE lease_id=?1", params![lease_id.trim()])?;
    let Some(stored) = rows.first() else {
        return Ok(None);
    };
    if rows.len() != 1 {
        bail!("Attempt Lease 存在多个中止回执");
    }
    audit_and_convert(conn, stored, false).map(Some)
}

fn insert_abort_on(conn: &Connection, stored: &StoredAttemptAbort) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_attempt_aborts (
            abort_id, lease_id, provider_id, consumer_account_id,
            executor_abort_ref, reason_code, fencing_generation,
            source_lease_revision, source_lease_digest,
            terminal_lease_revision, terminal_lease_digest, terminal_lease_json,
            job_id, source_job_revision, source_job_digest,
            terminal_job_revision, terminal_job_digest,
            reservation_id, source_reservation_revision, source_reservation_digest,
            terminal_reservation_revision, terminal_reservation_digest,
            capacity_claim_id, source_claim_revision, source_claim_digest,
            terminal_claim_revision, terminal_claim_digest,
            budget_reservation_id, budget_refunded_fen, budget_terminal_status,
            capacity_transaction_id, capacity_transaction_digest,
            activation_request_digest, request_digest, event_digest,
            idempotency_scope, idempotency_key, aborted_by_user_id,
            aborted_at, created_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
            ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
            ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?39
         )",
        params![
            stored.abort_id,
            stored.lease_id,
            stored.provider_id,
            stored.consumer_account_id,
            stored.executor_abort_ref,
            stored.reason_code,
            stored.fencing_generation,
            stored.source_lease_revision,
            stored.source_lease_digest,
            stored.terminal_lease_revision,
            stored.terminal_lease_digest,
            stored.terminal_lease_json,
            stored.job_id,
            stored.source_job_revision,
            stored.source_job_digest,
            stored.terminal_job_revision,
            stored.terminal_job_digest,
            stored.reservation_id,
            stored.source_reservation_revision,
            stored.source_reservation_digest,
            stored.terminal_reservation_revision,
            stored.terminal_reservation_digest,
            stored.capacity_claim_id,
            stored.source_claim_revision,
            stored.source_claim_digest,
            stored.terminal_claim_revision,
            stored.terminal_claim_digest,
            stored.budget_reservation_id,
            stored.budget_refunded_fen,
            stored.budget_terminal_status,
            stored.capacity_transaction_id,
            stored.capacity_transaction_digest,
            stored.activation_request_digest,
            stored.request_digest,
            stored.event_digest,
            stored.idempotency_scope,
            stored.idempotency_key,
            stored.aborted_by_user_id,
            stored.aborted_at,
        ],
    )?;
    Ok(())
}

fn stored_aborts_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Vec<StoredAttemptAbort>> {
    let mut statement = conn.prepare(&format!(
        "SELECT abort_id, lease_id, provider_id, consumer_account_id,
                executor_abort_ref, reason_code, fencing_generation,
                source_lease_revision, source_lease_digest,
                terminal_lease_revision, terminal_lease_digest, terminal_lease_json,
                job_id, source_job_revision, source_job_digest,
                terminal_job_revision, terminal_job_digest,
                reservation_id, source_reservation_revision, source_reservation_digest,
                terminal_reservation_revision, terminal_reservation_digest,
                capacity_claim_id, source_claim_revision, source_claim_digest,
                terminal_claim_revision, terminal_claim_digest,
                budget_reservation_id, budget_refunded_fen, budget_terminal_status,
                capacity_transaction_id, capacity_transaction_digest,
                activation_request_digest, request_digest, event_digest,
                idempotency_scope, idempotency_key, aborted_by_user_id, aborted_at
           FROM compute_attempt_aborts {filter} ORDER BY abort_id LIMIT 2"
    ))?;
    let rows = statement
        .query_map(parameters, |row| {
            Ok(StoredAttemptAbort {
                abort_id: row.get(0)?,
                lease_id: row.get(1)?,
                provider_id: row.get(2)?,
                consumer_account_id: row.get(3)?,
                executor_abort_ref: row.get(4)?,
                reason_code: row.get(5)?,
                fencing_generation: row.get(6)?,
                source_lease_revision: row.get(7)?,
                source_lease_digest: row.get(8)?,
                terminal_lease_revision: row.get(9)?,
                terminal_lease_digest: row.get(10)?,
                terminal_lease_json: row.get(11)?,
                job_id: row.get(12)?,
                source_job_revision: row.get(13)?,
                source_job_digest: row.get(14)?,
                terminal_job_revision: row.get(15)?,
                terminal_job_digest: row.get(16)?,
                reservation_id: row.get(17)?,
                source_reservation_revision: row.get(18)?,
                source_reservation_digest: row.get(19)?,
                terminal_reservation_revision: row.get(20)?,
                terminal_reservation_digest: row.get(21)?,
                capacity_claim_id: row.get(22)?,
                source_claim_revision: row.get(23)?,
                source_claim_digest: row.get(24)?,
                terminal_claim_revision: row.get(25)?,
                terminal_claim_digest: row.get(26)?,
                budget_reservation_id: row.get(27)?,
                budget_refunded_fen: row.get(28)?,
                budget_terminal_status: row.get(29)?,
                capacity_transaction_id: row.get(30)?,
                capacity_transaction_digest: row.get(31)?,
                activation_request_digest: row.get(32)?,
                request_digest: row.get(33)?,
                event_digest: row.get(34)?,
                idempotency_scope: row.get(35)?,
                idempotency_key: row.get(36)?,
                aborted_by_user_id: row.get(37)?,
                aborted_at: row.get(38)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
