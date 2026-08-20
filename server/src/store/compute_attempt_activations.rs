use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::{
    capacity::{ComputeCapacityClaimBinding, ComputeCapacityOfferBinding},
    execution::{
        ComputeAttemptLease, ComputeJobVersionBinding, ATTEMPT_STATUS_STAGING,
        COMPUTE_ATTEMPT_LEASE_SCHEMA, JOB_STATUS_RUNNING,
    },
};

use super::{
    compute_attempt_leases::{
        compute_attempt_lease_digest, initialize_compute_attempt_lease_state_on,
    },
    compute_broker_reservation::broker_reserve_binding_on,
    compute_capacity_claim_activation::{
        activate_reservation_capacity_claim_on, ActivateReservationCapacityClaim,
    },
    compute_capacity_ledger::ComputeCapacityLedgerWriteReceipt,
    compute_job_registry::{current_registered_job_on, register_compute_job_on},
    compute_reservation_registry::{
        current_registered_reservation_on, register_compute_reservation_on,
    },
    Store,
};

mod candidates;
mod preflight;
mod rows;
mod validation;

use candidates::list_activation_candidates_on;
use preflight::{
    activation_timestamp, ensure_broker_binding, ensure_budget_reserved, ensure_job_matches,
    ensure_lease_window, ensure_provider_and_offer_live, ensure_reservation_matches,
};
use rows::{attempt_activation_on, persist_attempt_activation_on};
use validation::normalize_activation;

pub(crate) const ATTEMPT_ACTIVATION_EXECUTION_EFFECT: &str = "none";
pub(crate) const ATTEMPT_ACTIVATION_MONEY_EFFECT: &str = "preauthorization_unchanged";

#[derive(Debug, Clone)]
pub(crate) struct ActivateComputeAttemptRequest {
    pub lease_id: String,
    pub reservation_id: String,
    pub provider_id: String,
    pub executor_id: String,
    pub shard_id: Option<String>,
    pub attempt_no: i64,
    pub fencing_generation: i64,
    pub executor_acceptance_ref: String,
    pub lease_credential_ref: String,
    pub lease_credential_hint: String,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub expected_reservation_revision: i64,
    pub expected_reservation_digest: String,
    pub expected_claim_revision: i64,
    pub expected_claim_digest: String,
    pub expires_at: String,
    pub hard_deadline_at: String,
    pub idempotency_key: String,
    pub activated_by_user_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptActivationReceipt {
    pub lease: ComputeAttemptLease,
    pub lease_digest: String,
    pub request_digest: String,
    pub executor_acceptance_ref: String,
    pub source_job: ComputeJobVersionBinding,
    pub running_job: ComputeJobVersionBinding,
    pub source_reservation_revision: i64,
    pub source_reservation_digest: String,
    pub active_reservation_revision: i64,
    pub active_reservation_digest: String,
    pub source_claim: ComputeCapacityClaimBinding,
    pub active_claim: ComputeCapacityClaimBinding,
    pub budget_reservation_id: String,
    pub budget_reserved_fen: i64,
    pub capacity_ledger: ComputeCapacityLedgerWriteReceipt,
    pub activated_by_user_id: String,
    pub activated_at: String,
    pub execution_effect: &'static str,
    pub money_effect: &'static str,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn list_compute_attempt_activation_candidates(
        &self,
        provider_id: &str,
        limit: usize,
    ) -> Result<Vec<super::compute_reservation_registry::ComputeReservationRegistrationReceipt>>
    {
        if provider_id.is_empty() || provider_id.trim() != provider_id {
            bail!("算力 Provider ID 无效");
        }
        list_activation_candidates_on(&*self.conn()?, provider_id, limit.clamp(1, 100))
    }

    pub(crate) fn activate_compute_attempt(
        &self,
        request: &ActivateComputeAttemptRequest,
    ) -> Result<ComputeAttemptActivationReceipt> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let receipt = activate_compute_attempt_on(&tx, request)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_attempt_activation(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptActivationReceipt> {
        if lease_id.is_empty() || lease_id.trim() != lease_id {
            bail!("Attempt Lease ID 无效");
        }
        compute_attempt_activation_on(&*self.conn()?, lease_id)
    }
}

/// The unique v185 mutation kernel. Callers that must atomically bind external evidence may run
/// it inside their own `BEGIN IMMEDIATE`; this function never opens or commits a transaction.
pub(super) fn activate_compute_attempt_on(
    conn: &Connection,
    request: &ActivateComputeAttemptRequest,
) -> Result<ComputeAttemptActivationReceipt> {
    activate_compute_attempt_with_time_on(conn, request, None)
}

pub(in crate::store) fn activate_compute_attempt_at_on(
    conn: &Connection,
    request: &ActivateComputeAttemptRequest,
    activated_at: &str,
) -> Result<ComputeAttemptActivationReceipt> {
    activate_compute_attempt_with_time_on(conn, request, Some(activated_at))
}

fn activate_compute_attempt_with_time_on(
    conn: &Connection,
    request: &ActivateComputeAttemptRequest,
    supplied_activated_at: Option<&str>,
) -> Result<ComputeAttemptActivationReceipt> {
    let request = normalize_activation(request)?;
    let idempotency_scope = format!("compute_attempt_activation:{}", request.provider_id);
    if let Some(receipt) = attempt_activation_on(
        conn,
        &idempotency_scope,
        &request.idempotency_key,
        Some(&request.request_digest),
    )? {
        return Ok(receipt);
    }
    if let Some(existing_lease_id) = conn
        .query_row(
            "SELECT lease_id FROM compute_attempt_activations WHERE job_id=(
                SELECT job_id FROM compute_reservations WHERE reservation_id=?1
             ) LIMIT 1",
            params![request.reservation_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        bail!("该 Job 已存在 Attempt 激活回执：{existing_lease_id}");
    }

    let source_reservation = current_registered_reservation_on(conn, &request.reservation_id)?
        .ok_or_else(|| anyhow!("Attempt 激活引用的 Reservation 不存在"))?;
    ensure_reservation_matches(&request, &source_reservation)?;
    let source_job = current_registered_job_on(conn, &source_reservation.reservation.job.job_id)?
        .ok_or_else(|| anyhow!("Attempt 激活引用的 Job 不存在"))?;
    ensure_job_matches(&request, &source_reservation, &source_job)?;
    ensure_provider_and_offer_live(conn, &request, &source_reservation)?;

    let broker = broker_reserve_binding_on(
        conn,
        &request.reservation_id,
        &source_job.job.consumer_account_id,
    )?;
    ensure_broker_binding(&broker, &source_reservation, &source_job)?;
    let activated_at = activation_timestamp(
        &source_job.job.updated_at,
        &source_reservation.reservation.updated_at,
        supplied_activated_at,
    )?;
    ensure_budget_reserved(
        conn,
        &broker,
        &source_job.job.consumer_account_id,
        &activated_at,
    )?;
    ensure_lease_window(
        &request,
        &source_reservation.reservation.expires_at,
        &activated_at,
    )?;

    let source_claim = source_reservation.reservation.capacity_claim.clone();
    let lease = ComputeAttemptLease {
        schema: COMPUTE_ATTEMPT_LEASE_SCHEMA.to_string(),
        lease_id: request.lease_id.clone(),
        job_id: source_job.job.job_id.clone(),
        reservation_id: request.reservation_id.clone(),
        attempt_no: request.attempt_no,
        shard_id: request.shard_id.clone(),
        provider_id: request.provider_id.clone(),
        executor_id: request.executor_id.clone(),
        status: ATTEMPT_STATUS_STAGING.to_string(),
        fencing_generation: request.fencing_generation,
        lease_credential_ref: request.lease_credential_ref.clone(),
        lease_credential_hint: request.lease_credential_hint.clone(),
        latest_checkpoint: None,
        issued_at: activated_at.clone(),
        last_heartbeat_at: None,
        expires_at: request.expires_at.clone(),
        hard_deadline_at: request.hard_deadline_at.clone(),
        terminal_reason_code: None,
    };
    let active_capacity = activate_reservation_capacity_claim_on(
        conn,
        ActivateReservationCapacityClaim {
            claim_id: source_claim.claim_id.clone(),
            expected_revision: source_claim.claim_revision,
            expected_digest: source_claim.claim_digest.clone(),
            offer: ComputeCapacityOfferBinding {
                offer_id: source_reservation.reservation.offer.offer_id.clone(),
                offer_version: source_reservation.reservation.offer.offer_version,
                offer_digest: source_reservation.reservation.offer.offer_digest.clone(),
            },
            job_id: source_job.job.job_id.clone(),
            reservation_id: request.reservation_id.clone(),
            attempt_lease_id: lease.lease_id.clone(),
            fencing_generation: lease.fencing_generation,
            request_digest: request.request_digest.clone(),
            idempotency_scope: idempotency_scope.clone(),
            idempotency_key: request.idempotency_key.clone(),
            activated_at: activated_at.clone(),
        },
    )?;

    let mut running_job = source_job.job.clone();
    running_job.status = JOB_STATUS_RUNNING.to_string();
    running_job.updated_at = activated_at.clone();
    let running_job = register_compute_job_on(conn, &running_job, source_job.revision)?;

    let mut active_reservation = source_reservation.reservation.clone();
    active_reservation.job = ComputeJobVersionBinding {
        job_id: running_job.job.job_id.clone(),
        job_revision: running_job.revision,
        job_digest: running_job.job_digest.clone(),
    };
    active_reservation.capacity_claim = active_capacity.claim.clone();
    active_reservation.updated_at = activated_at.clone();
    let active_reservation =
        register_compute_reservation_on(conn, &active_reservation, source_reservation.revision)?;
    let lease_digest = compute_attempt_lease_digest(&lease)?;
    let receipt = persist_attempt_activation_on(
        conn,
        &request,
        &idempotency_scope,
        &lease,
        &lease_digest,
        &source_job,
        &running_job,
        &source_reservation,
        &active_reservation,
        &source_claim,
        &active_capacity,
        &broker,
        &activated_at,
    )?;
    initialize_compute_attempt_lease_state_on(
        conn,
        &source_job.job.consumer_account_id,
        &lease,
        &lease_digest,
        &request.activated_by_user_id,
        &activated_at,
    )?;
    Ok(receipt)
}

pub(super) fn compute_attempt_activation_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<ComputeAttemptActivationReceipt> {
    if lease_id.is_empty() || lease_id.trim() != lease_id {
        bail!("Attempt Lease ID 无效");
    }
    attempt_activation_on(conn, "", lease_id, None)?
        .ok_or_else(|| anyhow!("Attempt 激活回执不存在"))
}
