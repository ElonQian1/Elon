use anyhow::{bail, Result};

use super::{
    artifacts_digest, digest_json, normalize_terminal_request, terminal_request_digest,
    StoredTerminalCandidate,
};
use crate::store::compute_attempt_terminals::{
    ComputeDeclaredResultArtifactInput, DeclareComputeAttemptTerminalCandidateRequest,
    COMPUTE_ATTEMPT_TERMINAL_CANDIDATE_SCHEMA,
};

pub(super) fn audit_candidate(stored: &StoredTerminalCandidate) -> Result<()> {
    if stored.source_lease_status != "running"
        || stored.created_at != stored.declared_at
        || stored.result_artifacts_digest != artifacts_digest(&stored.result_artifacts)?
    {
        bail!("Attempt 终态候选基础字段审计失败");
    }
    let request = DeclareComputeAttemptTerminalCandidateRequest {
        lease_id: stored.lease_id.clone(),
        provider_id: stored.provider_id.clone(),
        expected_lease_revision: stored.source_lease_revision,
        expected_lease_digest: stored.source_lease_digest.clone(),
        expected_fencing_generation: stored.fencing_generation,
        final_usage_snapshot_id: stored.final_usage_snapshot_id.clone(),
        final_usage_sequence_no: stored.final_usage_sequence_no,
        final_cumulative_usage_digest: stored.final_cumulative_usage_digest.clone(),
        executor_terminal_ref: stored.executor_terminal_ref.clone(),
        outcome: stored.outcome.clone(),
        reason_code: stored.reason_code.clone(),
        diagnostic_ref: stored.diagnostic_ref.clone(),
        output_digest: stored.output_digest.clone(),
        result_artifacts: stored
            .result_artifacts
            .iter()
            .cloned()
            .map(|artifact| ComputeDeclaredResultArtifactInput {
                artifact_id: artifact.artifact_id,
                digest_algorithm: artifact.digest_algorithm,
                digest: artifact.digest,
                media_type: artifact.media_type,
                size_bytes: artifact.size_bytes,
                location_ref: artifact.location_ref,
                encryption_profile: artifact.encryption_profile,
            })
            .collect(),
        idempotency_key: stored.idempotency_key.clone(),
        declared_by_user_id: stored.declared_by_user_id.clone(),
    };
    let request = normalize_terminal_request(&request)?;
    if stored.idempotency_scope != format!("compute_attempt_terminal:{}", stored.provider_id)
        || stored.request_digest != terminal_request_digest(&request)?
    {
        bail!("Attempt 终态候选请求审计失败");
    }
    let event_digest = digest_json(&serde_json::json!({
        "schema":COMPUTE_ATTEMPT_TERMINAL_CANDIDATE_SCHEMA,
        "terminal_candidate_id":stored.terminal_candidate_id,
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
        "final_cumulative_usage_digest":stored.final_cumulative_usage_digest,
        "executor_terminal_ref":stored.executor_terminal_ref,
        "outcome":stored.outcome,
        "reason_code":stored.reason_code,
        "diagnostic_ref":stored.diagnostic_ref,
        "output_digest":stored.output_digest,
        "result_artifacts_digest":stored.result_artifacts_digest,
        "request_digest":stored.request_digest,
        "declared_by_user_id":stored.declared_by_user_id,
        "declared_at":stored.declared_at,
    }))?;
    if stored.event_digest != event_digest {
        bail!("Attempt 终态候选事件摘要审计失败");
    }
    Ok(())
}
