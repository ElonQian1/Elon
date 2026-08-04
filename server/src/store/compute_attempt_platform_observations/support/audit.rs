use anyhow::{bail, Result};

use super::{
    build_observed_readings, digest_json, evidence_refs_digest,
    normalize_platform_observation_request, observation_request_digest, observed_usage_digest,
    quantities_by_meter, variance_meters_digest, StoredPlatformObservation,
};
use crate::store::compute_attempt_platform_observations::{
    ComputeObservedUsageInput, ObserveComputeAttemptTerminalCandidateRequest,
    COMPUTE_ATTEMPT_PLATFORM_OBSERVATION_SCHEMA,
};

pub(super) fn audit_platform_observation(stored: &StoredPlatformObservation) -> Result<()> {
    if stored.created_at != stored.observed_at
        || stored.cumulative_observed_usage_digest
            != observed_usage_digest(&stored.cumulative_observed_usage)?
        || stored.variance_meters_digest != variance_meters_digest(&stored.variance_meters)?
        || stored.evidence_refs_digest != evidence_refs_digest(&stored.evidence_refs)?
    {
        bail!("平台终态观测基础字段审计失败");
    }
    let quantities = quantities_by_meter(&stored.cumulative_observed_usage);
    let request =
        normalize_platform_observation_request(&ObserveComputeAttemptTerminalCandidateRequest {
            lease_id: stored.lease_id.clone(),
            expected_terminal_candidate_id: stored.terminal_candidate_id.clone(),
            expected_terminal_candidate_event_digest: stored
                .terminal_candidate_event_digest
                .clone(),
            observation_source: stored.observation_source.clone(),
            observer_ref: stored.observer_ref.clone(),
            observed_outcome: stored.observed_outcome.clone(),
            cumulative_observed_usage: stored
                .cumulative_observed_usage
                .iter()
                .map(|reading| ComputeObservedUsageInput {
                    meter: reading.meter.clone(),
                    cumulative_quantity: reading.quantity,
                })
                .collect(),
            evidence_refs: stored.evidence_refs.clone(),
            idempotency_key: stored.idempotency_key.clone(),
            observed_by_user_id: stored.observed_by_user_id.clone(),
        })?;
    if quantities.len() != stored.cumulative_observed_usage.len()
        || stored.cumulative_observed_usage
            != build_observed_readings(&request, &stored.observed_at)?
        || stored.idempotency_scope
            != format!(
                "compute_attempt_platform_observation:{}",
                stored.observed_by_user_id
            )
        || stored.request_digest != observation_request_digest(&request)?
    {
        bail!("平台终态观测请求审计失败");
    }
    let event_digest = digest_json(&serde_json::json!({
        "schema":COMPUTE_ATTEMPT_PLATFORM_OBSERVATION_SCHEMA,
        "platform_observation_id":stored.platform_observation_id,
        "terminal_candidate_id":stored.terminal_candidate_id,
        "terminal_candidate_event_digest":stored.terminal_candidate_event_digest,
        "lease_id":stored.lease_id,
        "provider_id":stored.provider_id,
        "consumer_account_id":stored.consumer_account_id,
        "source_lease_revision":stored.source_lease_revision,
        "source_lease_digest":stored.source_lease_digest,
        "fencing_generation":stored.fencing_generation,
        "job_id":stored.job_id,
        "job_revision":stored.job_revision,
        "job_digest":stored.job_digest,
        "reservation_id":stored.reservation_id,
        "reservation_revision":stored.reservation_revision,
        "reservation_digest":stored.reservation_digest,
        "capacity_claim_id":stored.capacity_claim_id,
        "capacity_claim_revision":stored.capacity_claim_revision,
        "capacity_claim_digest":stored.capacity_claim_digest,
        "final_usage_snapshot_id":stored.final_usage_snapshot_id,
        "final_usage_sequence_no":stored.final_usage_sequence_no,
        "final_provider_usage_digest":stored.final_provider_usage_digest,
        "candidate_outcome":stored.candidate_outcome,
        "observation_source":stored.observation_source,
        "observer_ref":stored.observer_ref,
        "observed_outcome":stored.observed_outcome,
        "cumulative_observed_usage_digest":stored.cumulative_observed_usage_digest,
        "variance_meters_digest":stored.variance_meters_digest,
        "evidence_refs_digest":stored.evidence_refs_digest,
        "request_digest":stored.request_digest,
        "observed_by_user_id":stored.observed_by_user_id,
        "observed_at":stored.observed_at,
    }))?;
    if stored.event_digest != event_digest {
        bail!("平台终态观测事件摘要审计失败");
    }
    Ok(())
}
