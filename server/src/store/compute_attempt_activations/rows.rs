use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    capacity::ComputeCapacityClaimBinding,
    execution::{ComputeAttemptLease, ComputeJobVersionBinding},
};

use super::{
    super::{
        compute_broker_reservation::BrokerReserveBinding,
        compute_capacity_claim_activation::ActivateReservationCapacityClaimReceipt,
        compute_capacity_claim_rows::stored_claim_version_on,
        compute_capacity_ledger::ComputeCapacityLedgerWriteReceipt,
        compute_capacity_posting::balances_for_transaction_on,
        compute_job_registry::{registered_job_version_on, ComputeJobRegistrationReceipt},
        compute_reservation_registry::{
            registered_reservation_version_on, ComputeReservationRegistrationReceipt,
        },
    },
    ComputeAttemptActivationReceipt, NormalizedAttemptActivation,
    ATTEMPT_ACTIVATION_EXECUTION_EFFECT, ATTEMPT_ACTIVATION_MONEY_EFFECT,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn persist_attempt_activation_on(
    conn: &Connection,
    request: &NormalizedAttemptActivation,
    idempotency_scope: &str,
    lease: &ComputeAttemptLease,
    lease_digest: &str,
    source_job: &ComputeJobRegistrationReceipt,
    running_job: &ComputeJobRegistrationReceipt,
    source_reservation: &ComputeReservationRegistrationReceipt,
    active_reservation: &ComputeReservationRegistrationReceipt,
    source_claim: &ComputeCapacityClaimBinding,
    active_capacity: &ActivateReservationCapacityClaimReceipt,
    broker: &BrokerReserveBinding,
    activated_at: &str,
) -> Result<ComputeAttemptActivationReceipt> {
    conn.execute(
        "INSERT INTO compute_attempt_activations (
            lease_id, reservation_id, job_id, provider_id, consumer_account_id,
            executor_id, attempt_no, fencing_generation, executor_acceptance_ref,
            budget_reservation_id, budget_reserved_fen,
            source_job_revision, source_job_digest,
            running_job_revision, running_job_digest,
            source_reservation_revision, source_reservation_digest,
            active_reservation_revision, active_reservation_digest,
            source_claim_revision, capacity_claim_id, source_claim_digest,
            active_claim_revision, active_claim_digest,
            capacity_transaction_id, capacity_transaction_digest,
            request_digest, lease_digest, lease_json,
            idempotency_scope, idempotency_key,
            activated_by_user_id, activated_at, created_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
            ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21,
            ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31,
            ?32, ?33, ?33
         )",
        params![
            lease.lease_id,
            lease.reservation_id,
            lease.job_id,
            lease.provider_id,
            source_job.job.consumer_account_id,
            lease.executor_id,
            lease.attempt_no,
            lease.fencing_generation,
            request.executor_acceptance_ref,
            broker.budget_reservation_id,
            broker.budget_reserved_fen,
            source_job.revision,
            source_job.job_digest,
            running_job.revision,
            running_job.job_digest,
            source_reservation.revision,
            source_reservation.reservation_digest,
            active_reservation.revision,
            active_reservation.reservation_digest,
            source_claim.claim_revision,
            source_claim.claim_id,
            source_claim.claim_digest,
            active_capacity.claim.claim_revision,
            active_capacity.claim.claim_digest,
            active_capacity.ledger.transaction_id,
            active_capacity.ledger.transaction_digest,
            request.request_digest,
            lease_digest,
            serde_json::to_string(lease)?,
            idempotency_scope,
            request.idempotency_key,
            request.activated_by_user_id,
            activated_at,
        ],
    )?;
    attempt_activation_on(conn, "", &lease.lease_id, None)?
        .ok_or_else(|| anyhow!("Attempt 激活回执写入后不可见"))
}

pub(super) fn attempt_activation_on(
    conn: &Connection,
    idempotency_scope: &str,
    key: &str,
    expected_request_digest: Option<&str>,
) -> Result<Option<ComputeAttemptActivationReceipt>> {
    let stored = if idempotency_scope.is_empty() {
        stored_activation_on(conn, "WHERE a.lease_id=?1", params![key])?
    } else {
        stored_activation_on(
            conn,
            "WHERE a.idempotency_scope=?1 AND a.idempotency_key=?2",
            params![idempotency_scope, key],
        )?
    };
    let Some(stored) = stored else {
        return Ok(None);
    };
    if expected_request_digest.is_some_and(|digest| digest != stored.request_digest) {
        bail!("相同 Attempt 激活幂等键不能用于不同请求");
    }
    audit_stored_activation_on(conn, &stored)?;
    Ok(Some(
        stored.into_receipt(conn, expected_request_digest.is_some())?,
    ))
}

struct StoredAttemptActivation {
    lease_id: String,
    reservation_id: String,
    job_id: String,
    provider_id: String,
    consumer_account_id: String,
    executor_id: String,
    attempt_no: i64,
    fencing_generation: i64,
    executor_acceptance_ref: String,
    budget_reservation_id: String,
    budget_reserved_fen: i64,
    source_job_revision: i64,
    source_job_digest: String,
    running_job_revision: i64,
    running_job_digest: String,
    source_reservation_revision: i64,
    source_reservation_digest: String,
    active_reservation_revision: i64,
    active_reservation_digest: String,
    capacity_claim_id: String,
    source_claim_revision: i64,
    source_claim_digest: String,
    active_claim_revision: i64,
    active_claim_digest: String,
    capacity_transaction_id: String,
    capacity_transaction_digest: String,
    stored_capacity_transaction_digest: String,
    ledger_sequence: i64,
    event_kind: String,
    ledger_request_digest: String,
    ledger_claim_id: String,
    ledger_claim_effect: String,
    ledger_claim_effect_key: String,
    ledger_job_id: String,
    ledger_reservation_id: String,
    ledger_attempt_lease_id: String,
    ledger_fencing_generation: i64,
    ledger_subject_kind: String,
    ledger_subject_id: String,
    request_digest: String,
    lease_digest: String,
    lease_json: String,
    idempotency_scope: String,
    idempotency_key: String,
    activated_by_user_id: String,
    activated_at: String,
}

impl StoredAttemptActivation {
    fn lease(&self) -> Result<ComputeAttemptLease> {
        serde_json::from_str(&self.lease_json).map_err(Into::into)
    }

    fn into_receipt(
        self,
        conn: &Connection,
        replayed: bool,
    ) -> Result<ComputeAttemptActivationReceipt> {
        let lease = self.lease()?;
        let balances = balances_for_transaction_on(conn, &self.capacity_transaction_id)?;
        Ok(ComputeAttemptActivationReceipt {
            lease,
            lease_digest: self.lease_digest,
            request_digest: self.request_digest.clone(),
            executor_acceptance_ref: self.executor_acceptance_ref,
            source_job: ComputeJobVersionBinding {
                job_id: self.job_id.clone(),
                job_revision: self.source_job_revision,
                job_digest: self.source_job_digest,
            },
            running_job: ComputeJobVersionBinding {
                job_id: self.job_id,
                job_revision: self.running_job_revision,
                job_digest: self.running_job_digest,
            },
            source_reservation_revision: self.source_reservation_revision,
            source_reservation_digest: self.source_reservation_digest,
            active_reservation_revision: self.active_reservation_revision,
            active_reservation_digest: self.active_reservation_digest,
            source_claim: ComputeCapacityClaimBinding {
                claim_id: self.capacity_claim_id.clone(),
                claim_revision: self.source_claim_revision,
                claim_digest: self.source_claim_digest,
            },
            active_claim: ComputeCapacityClaimBinding {
                claim_id: self.capacity_claim_id,
                claim_revision: self.active_claim_revision,
                claim_digest: self.active_claim_digest,
            },
            budget_reservation_id: self.budget_reservation_id,
            budget_reserved_fen: self.budget_reserved_fen,
            capacity_ledger: ComputeCapacityLedgerWriteReceipt {
                transaction_id: self.capacity_transaction_id,
                transaction_digest: self.capacity_transaction_digest,
                ledger_sequence: self.ledger_sequence,
                event_kind: self.event_kind,
                request_digest: self.request_digest,
                replayed,
                current_balances: balances,
            },
            activated_by_user_id: self.activated_by_user_id,
            activated_at: self.activated_at,
            execution_effect: ATTEMPT_ACTIVATION_EXECUTION_EFFECT,
            money_effect: ATTEMPT_ACTIVATION_MONEY_EFFECT,
            replayed,
        })
    }
}

fn audit_stored_activation_on(conn: &Connection, stored: &StoredAttemptActivation) -> Result<()> {
    let lease = stored.lease()?;
    let recomputed_lease_digest = hex::encode(Sha256::digest(stored.lease_json.as_bytes()));
    if recomputed_lease_digest != stored.lease_digest
        || lease.lease_id != stored.lease_id
        || lease.reservation_id != stored.reservation_id
        || lease.job_id != stored.job_id
        || lease.provider_id != stored.provider_id
        || lease.executor_id != stored.executor_id
        || lease.attempt_no != stored.attempt_no
        || lease.fencing_generation != stored.fencing_generation
        || lease.issued_at != stored.activated_at
        || stored.idempotency_scope.trim().is_empty()
        || stored.idempotency_key.trim().is_empty()
    {
        bail!("Attempt 激活回执与 Lease 不可变快照不一致");
    }
    if stored.capacity_transaction_digest != stored.stored_capacity_transaction_digest
        || stored.event_kind != "attempt_activated"
        || stored.ledger_request_digest != stored.request_digest
        || stored.ledger_claim_id != stored.capacity_claim_id
        || stored.ledger_claim_effect != "active"
        || stored.ledger_claim_effect_key != stored.lease_id
        || stored.ledger_job_id != stored.job_id
        || stored.ledger_reservation_id != stored.reservation_id
        || stored.ledger_attempt_lease_id != stored.lease_id
        || stored.ledger_fencing_generation != stored.fencing_generation
        || stored.ledger_subject_kind != "compute_attempt_lease"
        || stored.ledger_subject_id != stored.lease_id
    {
        bail!("Attempt 激活回执引用了错误的容量账本事件");
    }
    audit_job_versions(conn, stored)?;
    audit_reservation_versions(conn, stored)?;
    audit_claim_versions(conn, stored)?;
    Ok(())
}

fn audit_job_versions(conn: &Connection, stored: &StoredAttemptActivation) -> Result<()> {
    for (revision, digest) in [
        (
            stored.source_job_revision,
            stored.source_job_digest.as_str(),
        ),
        (
            stored.running_job_revision,
            stored.running_job_digest.as_str(),
        ),
    ] {
        let job = registered_job_version_on(conn, &stored.job_id, revision)?
            .ok_or_else(|| anyhow!("Attempt 激活绑定的 Job 历史版本不存在"))?;
        if job.job_digest != digest || job.job.consumer_account_id != stored.consumer_account_id {
            bail!("Attempt 激活绑定的 Job 历史版本审计失败");
        }
    }
    Ok(())
}

fn audit_reservation_versions(conn: &Connection, stored: &StoredAttemptActivation) -> Result<()> {
    for (revision, digest) in [
        (
            stored.source_reservation_revision,
            stored.source_reservation_digest.as_str(),
        ),
        (
            stored.active_reservation_revision,
            stored.active_reservation_digest.as_str(),
        ),
    ] {
        let reservation =
            registered_reservation_version_on(conn, &stored.reservation_id, revision)?
                .ok_or_else(|| anyhow!("Attempt 激活绑定的 Reservation 历史版本不存在"))?;
        if reservation.reservation_digest != digest {
            bail!("Attempt 激活绑定的 Reservation 历史版本审计失败");
        }
    }
    Ok(())
}

fn audit_claim_versions(conn: &Connection, stored: &StoredAttemptActivation) -> Result<()> {
    for (revision, digest) in [
        (
            stored.source_claim_revision,
            stored.source_claim_digest.as_str(),
        ),
        (
            stored.active_claim_revision,
            stored.active_claim_digest.as_str(),
        ),
    ] {
        let claim = stored_claim_version_on(conn, &stored.capacity_claim_id, revision)?
            .ok_or_else(|| anyhow!("Attempt 激活绑定的 Capacity Claim 历史版本不存在"))?;
        if claim.claim_digest != digest {
            bail!("Attempt 激活绑定的 Capacity Claim 历史版本审计失败");
        }
    }
    Ok(())
}

fn stored_activation_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredAttemptActivation>> {
    conn.query_row(
        &format!(
            "SELECT a.lease_id, a.reservation_id, a.job_id, a.provider_id,
                    a.consumer_account_id, a.executor_id, a.attempt_no,
                    a.fencing_generation, a.executor_acceptance_ref,
                    a.budget_reservation_id, a.budget_reserved_fen,
                    a.source_job_revision, a.source_job_digest,
                    a.running_job_revision, a.running_job_digest,
                    a.source_reservation_revision, a.source_reservation_digest,
                    a.active_reservation_revision, a.active_reservation_digest,
                    a.capacity_claim_id, a.source_claim_revision, a.source_claim_digest,
                    a.active_claim_revision, a.active_claim_digest,
                    a.capacity_transaction_id, a.capacity_transaction_digest,
                    t.transaction_digest, t.ledger_sequence, t.event_kind,
                    t.request_digest, t.claim_id, t.claim_effect, t.claim_effect_key,
                    t.job_id, t.reservation_id, t.attempt_lease_id,
                    t.fencing_generation, t.subject_kind, t.subject_id,
                    a.request_digest,
                    a.lease_digest, a.lease_json, a.idempotency_scope,
                    a.idempotency_key, a.activated_by_user_id, a.activated_at
               FROM compute_attempt_activations a
               JOIN compute_capacity_ledger_transactions t
                 ON t.transaction_id=a.capacity_transaction_id {filter}"
        ),
        parameters,
        |row| {
            Ok(StoredAttemptActivation {
                lease_id: row.get(0)?,
                reservation_id: row.get(1)?,
                job_id: row.get(2)?,
                provider_id: row.get(3)?,
                consumer_account_id: row.get(4)?,
                executor_id: row.get(5)?,
                attempt_no: row.get(6)?,
                fencing_generation: row.get(7)?,
                executor_acceptance_ref: row.get(8)?,
                budget_reservation_id: row.get(9)?,
                budget_reserved_fen: row.get(10)?,
                source_job_revision: row.get(11)?,
                source_job_digest: row.get(12)?,
                running_job_revision: row.get(13)?,
                running_job_digest: row.get(14)?,
                source_reservation_revision: row.get(15)?,
                source_reservation_digest: row.get(16)?,
                active_reservation_revision: row.get(17)?,
                active_reservation_digest: row.get(18)?,
                capacity_claim_id: row.get(19)?,
                source_claim_revision: row.get(20)?,
                source_claim_digest: row.get(21)?,
                active_claim_revision: row.get(22)?,
                active_claim_digest: row.get(23)?,
                capacity_transaction_id: row.get(24)?,
                capacity_transaction_digest: row.get(25)?,
                stored_capacity_transaction_digest: row.get(26)?,
                ledger_sequence: row.get(27)?,
                event_kind: row.get(28)?,
                ledger_request_digest: row.get(29)?,
                ledger_claim_id: row.get(30)?,
                ledger_claim_effect: row.get(31)?,
                ledger_claim_effect_key: row.get(32)?,
                ledger_job_id: row.get(33)?,
                ledger_reservation_id: row.get(34)?,
                ledger_attempt_lease_id: row.get(35)?,
                ledger_fencing_generation: row.get(36)?,
                ledger_subject_kind: row.get(37)?,
                ledger_subject_id: row.get(38)?,
                request_digest: row.get(39)?,
                lease_digest: row.get(40)?,
                lease_json: row.get(41)?,
                idempotency_scope: row.get(42)?,
                idempotency_key: row.get(43)?,
                activated_by_user_id: row.get(44)?,
                activated_at: row.get(45)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}
