use anyhow::{bail, Result};
use chrono::DateTime;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        execution::{ComputeJob, ComputeReservation},
        receipts::{
            ComputeAttestationEvidence, ComputeExecutionReceipt, ComputeExecutionUsage,
            ComputeVerificationDecision, COMPUTE_EXECUTION_RECEIPT_SCHEMA,
            VERIFICATION_STATUS_ACCEPTED,
        },
    },
    store::{
        ComputeAttemptActivationReceipt, ComputeAttemptConsumerReviewReceipt,
        ComputeAttemptPlatformObservationReceipt, ComputeAttemptTerminalCandidateReceipt,
        ComputeAttemptUsageDeclarationReceipt, ComputeAttemptVerificationDecisionReceipt,
        ComputeJobRegistrationReceipt, ComputeReservationRegistrationReceipt,
    },
};

use super::IssueComputeAttemptExecutionReceiptRequest;

mod audit;
mod persistence;

pub(super) use persistence::{
    execution_receipt_by_idempotency_on, execution_receipt_by_lease_on,
    execution_receipt_by_verification_on,
};

#[derive(Debug, Clone)]
pub(super) struct StoredExecutionReceipt {
    pub execution_receipt_id: String,
    pub verification_decision_id: String,
    pub verification_event_digest: String,
    pub lease_id: String,
    pub receipt_digest: String,
    pub receipt: ComputeExecutionReceipt,
    pub request_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub issued_by_user_id: String,
    pub issued_at: String,
    pub created_at: String,
}

pub(super) fn normalize_execution_receipt_request(
    input: &IssueComputeAttemptExecutionReceiptRequest,
) -> Result<IssueComputeAttemptExecutionReceiptRequest> {
    for (label, value, max_len) in [
        ("Attempt Lease ID", input.lease_id.as_str(), 200),
        (
            "Verification 决定 ID",
            input.expected_verification_decision_id.as_str(),
            200,
        ),
        (
            "Verification 事件摘要",
            input.expected_verification_event_digest.as_str(),
            64,
        ),
        ("幂等键", input.idempotency_key.as_str(), 200),
        (
            "Execution Receipt 签发用户",
            input.issued_by_user_id.as_str(),
            200,
        ),
    ] {
        validate_exact(label, value, max_len)?;
    }
    validate_digest(
        "Verification 事件摘要",
        &input.expected_verification_event_digest,
    )?;
    let mut normalized = input.clone();
    normalized.lease_id = normalized.lease_id.trim().to_string();
    normalized.expected_verification_decision_id = normalized
        .expected_verification_decision_id
        .trim()
        .to_string();
    normalized.expected_verification_event_digest = normalized
        .expected_verification_event_digest
        .trim()
        .to_ascii_lowercase();
    normalized.idempotency_key = normalized.idempotency_key.trim().to_string();
    normalized.issued_by_user_id = normalized.issued_by_user_id.trim().to_string();
    Ok(normalized)
}

pub(super) fn ensure_expected_verification(
    input: &IssueComputeAttemptExecutionReceiptRequest,
    verification: &ComputeAttemptVerificationDecisionReceipt,
) -> Result<()> {
    if verification.verification_decision_id != input.expected_verification_decision_id
        || verification.event_digest != input.expected_verification_event_digest
    {
        bail!("Execution Receipt 必须绑定精确 Verification 决定 ID 与事件摘要");
    }
    if verification.decision != VERIFICATION_STATUS_ACCEPTED {
        bail!("只有 accepted Verification 决定可以生成 Execution Receipt");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn ensure_receipt_sources(
    verification: &ComputeAttemptVerificationDecisionReceipt,
    candidate: &ComputeAttemptTerminalCandidateReceipt,
    consumer_review: &ComputeAttemptConsumerReviewReceipt,
    platform_observation: &ComputeAttemptPlatformObservationReceipt,
    provider_usage: &ComputeAttemptUsageDeclarationReceipt,
    activation: &ComputeAttemptActivationReceipt,
    job: &ComputeJobRegistrationReceipt,
    reservation: &ComputeReservationRegistrationReceipt,
) -> Result<()> {
    if verification.decision != VERIFICATION_STATUS_ACCEPTED
        || verification.terminal_candidate_id != candidate.terminal_candidate_id
        || verification.terminal_candidate_event_digest != candidate.event_digest
        || verification.consumer_review_id != consumer_review.consumer_review_id
        || verification.consumer_review_event_digest != consumer_review.event_digest
        || verification.platform_observation_id != platform_observation.platform_observation_id
        || verification.platform_observation_event_digest != platform_observation.event_digest
        || verification.final_usage_snapshot_id != provider_usage.snapshot_id
        || verification.final_usage_sequence_no != provider_usage.sequence_no
        || verification.final_provider_usage_digest != provider_usage.cumulative_usage_digest
    {
        bail!("Execution Receipt 的 v188-v192 证据链不一致");
    }
    if activation.lease.lease_id != verification.lease_id
        || activation.lease.job_id != verification.job_id
        || activation.lease.reservation_id != verification.reservation_id
        || activation.lease.provider_id != verification.provider_id
        || activation.lease.fencing_generation != verification.fencing_generation
        || job.revision != verification.job_revision
        || job.job_digest != verification.job_digest
        || job.job.job_id != verification.job_id
        || reservation.revision != verification.reservation_revision
        || reservation.reservation_digest != verification.reservation_digest
        || reservation.reservation.reservation_id != verification.reservation_id
        || reservation.reservation.offer.provider_id != verification.provider_id
        || reservation.reservation.capacity_claim.claim_id != verification.capacity_claim_id
        || reservation.reservation.capacity_claim.claim_revision
            != verification.capacity_claim_revision
        || reservation.reservation.capacity_claim.claim_digest != verification.capacity_claim_digest
    {
        bail!("Execution Receipt 的激活、Job 或 Reservation 因果链不一致");
    }
    if job.job.workload.runtime.is_none() {
        bail!("Execution Receipt 要求 Workload 固定 runtime/runner 摘要");
    }
    let started = DateTime::parse_from_rfc3339(&activation.activated_at)?;
    let finished = DateTime::parse_from_rfc3339(&candidate.declared_at)?;
    if finished < started {
        bail!("Execution Receipt 终态时间不能早于 Attempt 激活时间");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_execution_receipt(
    execution_receipt_id: &str,
    verification: &ComputeAttemptVerificationDecisionReceipt,
    candidate: &ComputeAttemptTerminalCandidateReceipt,
    consumer_review: &ComputeAttemptConsumerReviewReceipt,
    platform_observation: &ComputeAttemptPlatformObservationReceipt,
    provider_usage: &ComputeAttemptUsageDeclarationReceipt,
    activation: &ComputeAttemptActivationReceipt,
    job: &ComputeJob,
    reservation: &ComputeReservation,
    issued_at: &str,
) -> Result<ComputeExecutionReceipt> {
    let runtime = job
        .workload
        .runtime
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Execution Receipt 缺少 runtime 合同"))?;
    let input_digest = sha256_json(&json!({
        "purpose": "compute_execution_input_artifacts",
        "artifacts": job.workload.input_artifacts,
    }))?;
    let attestations = vec![
        ComputeAttestationEvidence {
            evidence_kind: "provider_terminal_candidate".to_string(),
            issuer: candidate.provider_id.clone(),
            evidence_digest: candidate.event_digest.clone(),
            artifact_ref: Some(candidate.executor_terminal_ref.clone()),
            observed_at: candidate.declared_at.clone(),
        },
        ComputeAttestationEvidence {
            evidence_kind: "consumer_review".to_string(),
            issuer: consumer_review.consumer_account_id.clone(),
            evidence_digest: consumer_review.event_digest.clone(),
            artifact_ref: Some(consumer_review.consumer_review_ref.clone()),
            observed_at: consumer_review.reviewed_at.clone(),
        },
        ComputeAttestationEvidence {
            evidence_kind: "platform_observation".to_string(),
            issuer: platform_observation.observed_by_user_id.clone(),
            evidence_digest: platform_observation.event_digest.clone(),
            artifact_ref: Some(platform_observation.observer_ref.clone()),
            observed_at: platform_observation.observed_at.clone(),
        },
    ];
    let mut receipt = ComputeExecutionReceipt {
        schema: COMPUTE_EXECUTION_RECEIPT_SCHEMA.to_string(),
        receipt_id: execution_receipt_id.to_string(),
        receipt_digest: String::new(),
        job_id: verification.job_id.clone(),
        reservation_id: verification.reservation_id.clone(),
        attempt_lease_id: verification.lease_id.clone(),
        attempt_no: activation.lease.attempt_no,
        fencing_generation: verification.fencing_generation,
        provider_id: verification.provider_id.clone(),
        executor_id: activation.lease.executor_id.clone(),
        offer_id: reservation.offer.offer_id.clone(),
        offer_version: reservation.offer.offer_version,
        offer_digest: reservation.offer.offer_digest.clone(),
        plugin_digest: runtime.plugin_digest.clone(),
        runner_digest: runtime.runner_digest.clone(),
        model_digest: job
            .workload
            .model
            .as_ref()
            .map(|model| model.model_digest.clone()),
        tokenizer_digest: job
            .workload
            .model
            .as_ref()
            .and_then(|model| model.tokenizer_digest.clone()),
        input_digest,
        output_digest: candidate.output_digest.clone(),
        result_artifacts: candidate.result_artifacts.clone(),
        execution_status: candidate.outcome.clone(),
        usage: ComputeExecutionUsage {
            declared_usage: provider_usage.cumulative_declared_usage.clone(),
            observed_usage: platform_observation.cumulative_observed_usage.clone(),
            verified_usage: verification.verified_usage.clone(),
            compensable_usage: verification.compensable_usage.clone(),
        },
        attestations,
        verification: ComputeVerificationDecision {
            status: verification.decision.clone(),
            policy_id: verification.policy_id.clone(),
            policy_version: verification.policy_version,
            reason_codes: verification.reason_codes.clone(),
            duplicate_receipt_ids: Vec::new(),
            challenge_receipt_ids: Vec::new(),
            decision_digest: verification.event_digest.clone(),
            decided_at: Some(verification.decided_at.clone()),
        },
        started_at: activation.activated_at.clone(),
        finished_at: candidate.declared_at.clone(),
        created_at: issued_at.to_string(),
    };
    receipt.receipt_digest = execution_receipt_digest(&receipt)?;
    Ok(receipt)
}

pub(super) fn execution_receipt_request_digest(
    input: &IssueComputeAttemptExecutionReceiptRequest,
) -> Result<String> {
    sha256_json(&json!({
        "purpose": "compute_attempt_execution_receipt_request",
        "lease_id": input.lease_id,
        "verification_decision_id": input.expected_verification_decision_id,
        "verification_event_digest": input.expected_verification_event_digest,
        "idempotency_key": input.idempotency_key,
        "issued_by_user_id": input.issued_by_user_id,
    }))
}

pub(super) fn execution_receipt_digest(receipt: &ComputeExecutionReceipt) -> Result<String> {
    let mut payload = receipt.clone();
    payload.receipt_digest.clear();
    sha256_json(&json!({
        "purpose": "compute_execution_receipt",
        "receipt": payload,
    }))
}

pub(super) fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > max_len {
        bail!("{label} 不能为空且长度不能超过 {max_len}");
    }
    if trimmed != value {
        bail!("{label} 不能包含首尾空白");
    }
    Ok(())
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label} 必须是 64 位十六进制摘要");
    }
    Ok(())
}

fn sha256_json(value: &impl Serialize) -> Result<String> {
    let encoded = serde_json::to_vec(value)?;
    Ok(hex::encode(Sha256::digest(encoded)))
}
