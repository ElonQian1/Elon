use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use rusqlite::{params, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::compute_federation::receipts::ComputeMeterReading;

use super::{
    compute_attempt_terminals::compute_attempt_terminal_candidate_on,
    compute_attempt_usage::compute_attempt_usage_declaration_on, new_id, Store,
};

mod support;

use support::{
    build_observed_readings, ensure_candidate_binding, ensure_observed_usage_binding,
    ensure_provider_usage_binding, evidence_refs_digest, normalize_platform_observation_request,
    observation_event_digest, observation_request_digest, observed_usage_digest,
    platform_observation_by_candidate_on, platform_observation_by_idempotency_on,
    platform_observation_by_lease_on, variance_meters, variance_meters_digest,
    StoredPlatformObservation,
};

pub(crate) const COMPUTE_ATTEMPT_PLATFORM_OBSERVATION_SCHEMA: &str =
    "compute_federation.attempt_platform_observation.v1";
pub(crate) const OBSERVATION_SOURCE_CONTROL_PLANE: &str = "control_plane";
pub(crate) const OBSERVATION_SOURCE_TRANSPORT_GATEWAY: &str = "transport_gateway";
pub(crate) const OBSERVATION_SOURCE_SERVER_METERING: &str = "server_metering";
pub(crate) const OBSERVED_OUTCOME_SUCCEEDED: &str = "succeeded";
pub(crate) const OBSERVED_OUTCOME_FAILED: &str = "failed";
pub(crate) const OBSERVED_OUTCOME_CANCELED: &str = "canceled";
pub(crate) const OBSERVED_OUTCOME_INDETERMINATE: &str = "indeterminate";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeObservedUsageInput {
    pub meter: String,
    pub cumulative_quantity: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct ObserveComputeAttemptTerminalCandidateRequest {
    pub lease_id: String,
    pub expected_terminal_candidate_id: String,
    pub expected_terminal_candidate_event_digest: String,
    pub observation_source: String,
    pub observer_ref: String,
    pub observed_outcome: String,
    pub cumulative_observed_usage: Vec<ComputeObservedUsageInput>,
    pub evidence_refs: Vec<String>,
    pub idempotency_key: String,
    pub observed_by_user_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptPlatformObservationReceipt {
    pub schema: &'static str,
    pub platform_observation_id: String,
    pub terminal_candidate_id: String,
    pub terminal_candidate_event_digest: String,
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
    pub final_provider_usage_digest: String,
    pub candidate_outcome: String,
    pub observation_source: String,
    pub observer_ref: String,
    pub observed_outcome: String,
    pub cumulative_observed_usage: Vec<ComputeMeterReading>,
    pub cumulative_observed_usage_digest: String,
    pub variance_meters: Vec<String>,
    pub variance_meters_digest: String,
    pub evidence_refs: Vec<String>,
    pub evidence_refs_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub observed_by_user_id: String,
    pub observed_at: String,
    pub evidence_status: &'static str,
    pub observation_effect: &'static str,
    pub verification_effect: &'static str,
    pub lease_effect: &'static str,
    pub job_effect: &'static str,
    pub capacity_effect: &'static str,
    pub reservation_effect: &'static str,
    pub money_effect: &'static str,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn observe_compute_attempt_terminal_candidate(
        &self,
        input: &ObserveComputeAttemptTerminalCandidateRequest,
    ) -> Result<ComputeAttemptPlatformObservationReceipt> {
        let input = normalize_platform_observation_request(input)?;
        let request_digest = observation_request_digest(&input)?;
        let idempotency_scope = format!(
            "compute_attempt_platform_observation:{}",
            input.observed_by_user_id
        );
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            platform_observation_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同平台观测幂等键不能用于不同请求");
            }
            let receipt = platform_observation_receipt_on(&tx, stored, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let candidate = compute_attempt_terminal_candidate_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无 Provider 终态候选"))?;
        if candidate.terminal_candidate_id != input.expected_terminal_candidate_id
            || candidate.event_digest != input.expected_terminal_candidate_event_digest
        {
            bail!("平台观测必须绑定精确的 Provider 终态候选 ID 与事件摘要");
        }
        let provider_usage = compute_attempt_usage_declaration_on(
            &tx,
            &candidate.lease_id,
            candidate.final_usage_sequence_no,
        )?
        .ok_or_else(|| anyhow!("终态候选绑定的最终 Provider 用量快照不存在"))?;
        ensure_provider_usage_binding(&candidate, &provider_usage)?;

        if let Some(stored) =
            platform_observation_by_candidate_on(&tx, &candidate.terminal_candidate_id)?
        {
            if stored.request_digest != request_digest {
                bail!("同一 Provider 终态候选已绑定另一份平台观测证据");
            }
            let receipt = platform_observation_receipt_on(&tx, stored, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        support::ensure_exact_meter_set(
            &input.cumulative_observed_usage,
            &provider_usage.cumulative_declared_usage,
        )?;
        let observed_at = Utc::now().to_rfc3339();
        let observed_usage = build_observed_readings(&input, &observed_at)?;
        let observed_usage_digest = observed_usage_digest(&observed_usage)?;
        let variance_meters = variance_meters(
            &input.cumulative_observed_usage,
            &provider_usage.cumulative_declared_usage,
        )?;
        let variance_meters_digest = variance_meters_digest(&variance_meters)?;
        let evidence_refs_digest = evidence_refs_digest(&input.evidence_refs)?;
        let platform_observation_id = new_id("compute_attempt_platform_observation");
        let event_digest = observation_event_digest(
            &platform_observation_id,
            &input,
            &candidate,
            &provider_usage,
            &observed_usage_digest,
            &variance_meters_digest,
            &evidence_refs_digest,
            &request_digest,
            &observed_at,
        )?;

        tx.execute(
            "INSERT INTO compute_attempt_platform_observations (
                platform_observation_id, terminal_candidate_id,
                terminal_candidate_event_digest, lease_id, provider_id,
                consumer_account_id, source_lease_revision, source_lease_digest,
                fencing_generation, job_id, job_revision, job_digest,
                reservation_id, reservation_revision, reservation_digest,
                capacity_claim_id, capacity_claim_revision, capacity_claim_digest,
                final_usage_snapshot_id, final_usage_sequence_no,
                final_provider_usage_digest, candidate_outcome, observation_source,
                observer_ref, observed_outcome, cumulative_observed_usage_json,
                cumulative_observed_usage_digest, variance_meters_json,
                variance_meters_digest, evidence_refs_json, evidence_refs_digest,
                request_digest, event_digest, idempotency_scope, idempotency_key,
                observed_by_user_id, observed_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23,
                       ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34,
                       ?35, ?36, ?37, ?37)",
            params![
                platform_observation_id,
                candidate.terminal_candidate_id,
                candidate.event_digest,
                candidate.lease_id,
                candidate.provider_id,
                candidate.consumer_account_id,
                candidate.source_lease_revision,
                candidate.source_lease_digest,
                candidate.fencing_generation,
                candidate.job_id,
                candidate.job_revision,
                candidate.job_digest,
                candidate.reservation_id,
                candidate.reservation_revision,
                candidate.reservation_digest,
                candidate.capacity_claim_id,
                candidate.capacity_claim_revision,
                candidate.capacity_claim_digest,
                candidate.final_usage_snapshot_id,
                candidate.final_usage_sequence_no,
                candidate.final_cumulative_usage_digest,
                candidate.outcome,
                input.observation_source,
                input.observer_ref,
                input.observed_outcome,
                serde_json::to_string(&observed_usage)?,
                observed_usage_digest,
                serde_json::to_string(&variance_meters)?,
                variance_meters_digest,
                serde_json::to_string(&input.evidence_refs)?,
                evidence_refs_digest,
                request_digest,
                event_digest,
                idempotency_scope,
                input.idempotency_key,
                input.observed_by_user_id,
                observed_at,
            ],
        )?;

        let stored = platform_observation_by_idempotency_on(
            &tx,
            &idempotency_scope,
            &input.idempotency_key,
        )?
        .ok_or_else(|| anyhow!("平台终态观测写入后不可见"))?;
        let receipt = platform_observation_receipt_on(&tx, stored, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_attempt_platform_observation(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptPlatformObservationReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        compute_attempt_platform_observation_on(&*conn, lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无平台终态观测证据"))
    }
}

pub(crate) fn compute_attempt_platform_observation_on(
    conn: &rusqlite::Connection,
    lease_id: &str,
) -> Result<Option<ComputeAttemptPlatformObservationReceipt>> {
    let Some(stored) = platform_observation_by_lease_on(conn, lease_id)? else {
        return Ok(None);
    };
    Ok(Some(platform_observation_receipt_on(conn, stored, false)?))
}

fn platform_observation_receipt_on(
    conn: &rusqlite::Connection,
    stored: StoredPlatformObservation,
    replayed: bool,
) -> Result<ComputeAttemptPlatformObservationReceipt> {
    let candidate = compute_attempt_terminal_candidate_on(conn, &stored.lease_id)?
        .ok_or_else(|| anyhow!("平台观测引用的 Provider 候选不存在"))?;
    let provider_usage = compute_attempt_usage_declaration_on(
        conn,
        &stored.lease_id,
        stored.final_usage_sequence_no,
    )?
    .ok_or_else(|| anyhow!("平台观测引用的 Provider 用量快照不存在"))?;
    ensure_candidate_binding(&stored, &candidate)?;
    ensure_provider_usage_binding(&candidate, &provider_usage)?;
    ensure_observed_usage_binding(&stored, &provider_usage)?;
    stored.into_receipt(replayed)
}
