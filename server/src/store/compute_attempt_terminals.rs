use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::compute_federation::{
    capacity::ComputeCapacityClaimState,
    execution::{ATTEMPT_STATUS_RUNNING, JOB_STATUS_RUNNING, RESERVATION_STATUS_ACTIVE},
    provider::{PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_DRAINING},
    workload::ComputeArtifactRef,
};

use super::{
    compute_attempt_activations::compute_attempt_activation_on,
    compute_attempt_leases::{current_lease_state_on, StoredLeaseState},
    compute_attempt_usage::latest_compute_attempt_usage_declaration_on,
    compute_capacity_claim_rows::stored_claim_on,
    compute_job_registry::current_registered_job_on,
    compute_provider_registry::current_registered_provider_on,
    compute_reservation_registry::current_registered_reservation_on,
    new_id, Store,
};

mod support;

use support::{
    artifacts_digest, candidate_by_idempotency_on, candidate_by_lease_on,
    list_pending_consumer_review_candidates_on, normalize_terminal_request, terminal_event_digest,
    terminal_request_digest,
};

pub(crate) const COMPUTE_ATTEMPT_TERMINAL_CANDIDATE_SCHEMA: &str =
    "compute_federation.attempt_terminal_candidate.v1";
pub(crate) const TERMINAL_OUTCOME_SUCCEEDED: &str = "succeeded";
pub(crate) const TERMINAL_OUTCOME_FAILED: &str = "failed";
pub(crate) const TERMINAL_OUTCOME_CANCELED: &str = "canceled";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeDeclaredResultArtifactInput {
    pub artifact_id: String,
    pub digest_algorithm: String,
    pub digest: String,
    pub media_type: String,
    pub size_bytes: i64,
    pub location_ref: String,
    pub encryption_profile: Option<String>,
}

impl From<ComputeDeclaredResultArtifactInput> for ComputeArtifactRef {
    fn from(value: ComputeDeclaredResultArtifactInput) -> Self {
        Self {
            artifact_id: value.artifact_id,
            digest_algorithm: value.digest_algorithm,
            digest: value.digest,
            media_type: value.media_type,
            size_bytes: value.size_bytes,
            location_ref: value.location_ref,
            encryption_profile: value.encryption_profile,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DeclareComputeAttemptTerminalCandidateRequest {
    pub lease_id: String,
    pub provider_id: String,
    pub expected_lease_revision: i64,
    pub expected_lease_digest: String,
    pub expected_fencing_generation: i64,
    pub final_usage_snapshot_id: String,
    pub final_usage_sequence_no: i64,
    pub final_cumulative_usage_digest: String,
    pub executor_terminal_ref: String,
    pub outcome: String,
    pub reason_code: String,
    pub diagnostic_ref: Option<String>,
    pub output_digest: Option<String>,
    pub result_artifacts: Vec<ComputeDeclaredResultArtifactInput>,
    pub idempotency_key: String,
    pub declared_by_user_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptTerminalCandidateReceipt {
    pub schema: &'static str,
    pub terminal_candidate_id: String,
    pub lease_id: String,
    pub provider_id: String,
    pub consumer_account_id: String,
    pub source_lease_revision: i64,
    pub source_lease_digest: String,
    pub fencing_generation: i64,
    pub job_id: String,
    pub job_revision: i64,
    pub job_digest: String,
    pub reservation_id: String,
    pub reservation_revision: i64,
    pub reservation_digest: String,
    pub capacity_claim_id: String,
    pub capacity_claim_revision: i64,
    pub capacity_claim_digest: String,
    pub final_usage_snapshot_id: String,
    pub final_usage_sequence_no: i64,
    pub final_cumulative_usage_digest: String,
    pub executor_terminal_ref: String,
    pub outcome: String,
    pub reason_code: String,
    pub diagnostic_ref: Option<String>,
    pub output_digest: Option<String>,
    pub result_artifacts: Vec<ComputeArtifactRef>,
    pub result_artifacts_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub declared_by_user_id: String,
    pub declared_at: String,
    pub verification_status: &'static str,
    pub execution_effect: &'static str,
    pub lease_effect: &'static str,
    pub job_effect: &'static str,
    pub capacity_effect: &'static str,
    pub reservation_effect: &'static str,
    pub money_effect: &'static str,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn declare_compute_attempt_terminal_candidate(
        &self,
        input: &DeclareComputeAttemptTerminalCandidateRequest,
    ) -> Result<ComputeAttemptTerminalCandidateReceipt> {
        let input = normalize_terminal_request(input)?;
        let request_digest = terminal_request_digest(&input)?;
        let idempotency_scope = format!("compute_attempt_terminal:{}", input.provider_id);
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            candidate_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同 Attempt 终态候选幂等键不能用于不同请求");
            }
            let receipt = stored.into_receipt(true)?;
            tx.commit()?;
            return Ok(receipt);
        }
        if let Some(stored) = candidate_by_lease_on(&tx, &input.lease_id)? {
            if stored.request_digest != request_digest {
                bail!("同一 Attempt Lease 已绑定另一份终态候选");
            }
            let receipt = stored.into_receipt(true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let current = current_lease_state_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("Attempt Lease 当前状态不存在"))?;
        ensure_live_running_lease(&tx, &input, &current)?;
        let activation = compute_attempt_activation_on(&tx, &input.lease_id)?;
        let job = current_registered_job_on(&tx, &current.lease.job_id)?
            .ok_or_else(|| anyhow!("Attempt 对应 Job 不存在"))?;
        let reservation = current_registered_reservation_on(&tx, &current.lease.reservation_id)?
            .ok_or_else(|| anyhow!("Attempt 对应 Reservation 不存在"))?;
        let claim = stored_claim_on(&tx, &activation.active_claim.claim_id)?
            .ok_or_else(|| anyhow!("Attempt 对应 Capacity Claim 不存在"))?;
        ensure_active_bindings(&current, &activation, &job, &reservation, &claim)?;

        let usage = latest_compute_attempt_usage_declaration_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("终态候选必须先登记最终累计用量快照"))?;
        ensure_final_usage_binding(&input, &current, &usage, &job, &reservation, &claim)?;
        let result_artifacts: Vec<ComputeArtifactRef> = input
            .result_artifacts
            .clone()
            .into_iter()
            .map(Into::into)
            .collect();
        support::ensure_output_contract(&input, &result_artifacts, &job.job.workload.output)?;
        let result_artifacts_digest = artifacts_digest(&result_artifacts)?;
        let declared_at = Utc::now().to_rfc3339();
        let terminal_candidate_id = new_id("compute_attempt_terminal");
        let event_digest = terminal_event_digest(
            &terminal_candidate_id,
            &input,
            &current,
            &job,
            &reservation,
            &claim,
            &result_artifacts_digest,
            &request_digest,
            &declared_at,
        )?;

        tx.execute(
            "INSERT INTO compute_attempt_terminal_candidates (
                terminal_candidate_id, lease_id, provider_id, consumer_account_id,
                source_lease_revision, source_lease_digest, source_lease_status,
                fencing_generation, job_id, job_revision, job_digest,
                reservation_id, reservation_revision, reservation_digest,
                capacity_claim_id, capacity_claim_revision, capacity_claim_digest,
                final_usage_snapshot_id, final_usage_sequence_no,
                final_cumulative_usage_digest, executor_terminal_ref, outcome,
                reason_code, diagnostic_ref, output_digest, result_artifacts_json,
                result_artifacts_digest, request_digest, event_digest,
                idempotency_scope, idempotency_key, declared_by_user_id,
                declared_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'running', ?7, ?8, ?9, ?10,
                       ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21,
                       ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?32)",
            params![
                terminal_candidate_id,
                input.lease_id,
                input.provider_id,
                current.consumer_account_id,
                current.lease_revision,
                current.lease_digest,
                current.lease.fencing_generation,
                job.job.job_id,
                job.revision,
                job.job_digest,
                reservation.reservation.reservation_id,
                reservation.revision,
                reservation.reservation_digest,
                claim.claim_id,
                claim.revision,
                claim.claim_digest,
                input.final_usage_snapshot_id,
                input.final_usage_sequence_no,
                input.final_cumulative_usage_digest,
                input.executor_terminal_ref,
                input.outcome,
                input.reason_code,
                input.diagnostic_ref,
                input.output_digest,
                serde_json::to_string(&result_artifacts)?,
                result_artifacts_digest,
                request_digest,
                event_digest,
                idempotency_scope,
                input.idempotency_key,
                input.declared_by_user_id,
                declared_at,
            ],
        )?;
        let receipt = candidate_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
            .ok_or_else(|| anyhow!("Attempt 终态候选写入后不可见"))?
            .into_receipt(false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_attempt_terminal_candidate(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptTerminalCandidateReceipt> {
        compute_attempt_terminal_candidate_on(&*self.conn()?, lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无终态候选"))
    }

    pub(crate) fn list_compute_attempt_terminal_candidates_pending_consumer_review(
        &self,
        consumer_account_id: &str,
        limit: usize,
    ) -> Result<Vec<ComputeAttemptTerminalCandidateReceipt>> {
        support::validate_exact("消费者账户 ID", consumer_account_id, 200)?;
        list_pending_consumer_review_candidates_on(
            &*self.conn()?,
            consumer_account_id,
            limit.clamp(1, 100),
        )?
        .into_iter()
        .map(|stored| stored.into_receipt(false))
        .collect()
    }
}

pub(crate) fn compute_attempt_terminal_candidate_on(
    conn: &rusqlite::Connection,
    lease_id: &str,
) -> Result<Option<ComputeAttemptTerminalCandidateReceipt>> {
    support::validate_exact("Attempt Lease ID", lease_id, 200)?;
    candidate_by_lease_on(conn, lease_id)?
        .map(|stored| stored.into_receipt(false))
        .transpose()
}

fn ensure_live_running_lease(
    conn: &rusqlite::Connection,
    input: &DeclareComputeAttemptTerminalCandidateRequest,
    current: &StoredLeaseState,
) -> Result<()> {
    let provider = current_registered_provider_on(conn, &input.provider_id)?
        .ok_or_else(|| anyhow!("Attempt Lease Provider 不存在"))?;
    if provider.provider.owner_account_id != input.declared_by_user_id
        || current.provider_id != input.provider_id
        || !matches!(
            provider.provider.status.as_str(),
            PROVIDER_STATUS_ACTIVE | PROVIDER_STATUS_DRAINING
        )
    {
        bail!("只有当前 Provider 所有者可为 active/draining Provider 声明终态候选");
    }
    if current.lease_revision != input.expected_lease_revision
        || current.lease_digest != input.expected_lease_digest
        || current.lease.fencing_generation != input.expected_fencing_generation
        || current.lease.status != ATTEMPT_STATUS_RUNNING
        || current.lease.last_heartbeat_at.is_none()
    {
        bail!("终态候选必须绑定当前 running Lease 的精确版本、摘要和 fencing 代次");
    }
    let now = Utc::now();
    let expires_at = DateTime::parse_from_rfc3339(&current.lease.expires_at)?.with_timezone(&Utc);
    let hard_deadline =
        DateTime::parse_from_rfc3339(&current.lease.hard_deadline_at)?.with_timezone(&Utc);
    if now >= expires_at || now >= hard_deadline {
        bail!("已过期的 Attempt Lease 不能登记终态候选");
    }
    Ok(())
}

fn ensure_active_bindings(
    current: &StoredLeaseState,
    activation: &super::ComputeAttemptActivationReceipt,
    job: &super::compute_job_registry::ComputeJobRegistrationReceipt,
    reservation: &super::compute_reservation_registry::ComputeReservationRegistrationReceipt,
    claim: &crate::compute_federation::capacity::ComputeCapacityClaim,
) -> Result<()> {
    if activation.lease.lease_id != current.lease.lease_id
        || activation.lease.fencing_generation != current.lease.fencing_generation
        || activation.running_job.job_id != job.job.job_id
        || activation.running_job.job_revision != job.revision
        || activation.running_job.job_digest != job.job_digest
        || job.job.status != JOB_STATUS_RUNNING
        || activation.active_reservation_revision != reservation.revision
        || activation.active_reservation_digest != reservation.reservation_digest
        || reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || activation.active_claim.claim_id != claim.claim_id
        || activation.active_claim.claim_revision != claim.revision
        || activation.active_claim.claim_digest != claim.claim_digest
        || claim.state != ComputeCapacityClaimState::Active
        || reservation.reservation.capacity_claim.claim_id != claim.claim_id
        || reservation.reservation.capacity_claim.claim_revision != claim.revision
        || reservation.reservation.capacity_claim.claim_digest != claim.claim_digest
        || current.consumer_account_id != job.job.consumer_account_id
    {
        bail!("Attempt 终态候选引用的 Job、Reservation 或 Capacity Claim 已漂移");
    }
    Ok(())
}

fn ensure_final_usage_binding(
    input: &DeclareComputeAttemptTerminalCandidateRequest,
    current: &StoredLeaseState,
    usage: &super::ComputeAttemptUsageDeclarationReceipt,
    job: &super::compute_job_registry::ComputeJobRegistrationReceipt,
    reservation: &super::compute_reservation_registry::ComputeReservationRegistrationReceipt,
    claim: &crate::compute_federation::capacity::ComputeCapacityClaim,
) -> Result<()> {
    if usage.snapshot_id != input.final_usage_snapshot_id
        || usage.sequence_no != input.final_usage_sequence_no
        || usage.cumulative_usage_digest != input.final_cumulative_usage_digest
        || usage.lease_id != current.lease.lease_id
        || usage.provider_id != input.provider_id
        || usage.consumer_account_id != current.consumer_account_id
        || usage.source_lease_revision != current.lease_revision
        || usage.source_lease_digest != current.lease_digest
        || usage.fencing_generation != current.lease.fencing_generation
        || usage.job_id != job.job.job_id
        || usage.job_revision != job.revision
        || usage.job_digest != job.job_digest
        || usage.reservation_id != reservation.reservation.reservation_id
        || usage.reservation_revision != reservation.revision
        || usage.reservation_digest != reservation.reservation_digest
        || usage.capacity_claim_id != claim.claim_id
        || usage.capacity_claim_revision != claim.revision
        || usage.capacity_claim_digest != claim.claim_digest
    {
        bail!("终态候选必须绑定当前 Lease 最新且精确的累计声明用量快照");
    }
    Ok(())
}
