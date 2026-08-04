use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{execution::ComputeReservation, receipts::ComputeMeterReading},
    store::{
        ComputeAttemptConsumerReviewReceipt, ComputeAttemptPlatformObservationReceipt,
        ComputeAttemptTerminalCandidateReceipt, ComputeAttemptUsageDeclarationReceipt,
        ComputeReservationRegistrationReceipt,
    },
};

use super::{
    DecideComputeAttemptVerificationRequest, VERIFICATION_DECISION_ACCEPTED,
    VERIFICATION_DECISION_DISPUTED, VERIFICATION_DECISION_REJECTED,
    VERIFICATION_POLICY_CONSERVATIVE_MIN_V1,
};

mod audit;
mod persistence;

pub(super) use persistence::{
    verification_decision_by_candidate_on, verification_decision_by_idempotency_on,
    verification_decision_by_lease_on,
};

const MAX_REASON_CODES: usize = 16;

#[derive(Debug, Clone)]
pub(super) struct StoredVerificationDecision {
    pub verification_decision_id: String,
    pub terminal_candidate_id: String,
    pub terminal_candidate_event_digest: String,
    pub consumer_review_id: String,
    pub consumer_review_event_digest: String,
    pub platform_observation_id: String,
    pub platform_observation_event_digest: String,
    pub lease_id: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub decision: String,
    pub reason_codes: Vec<String>,
    pub reason_codes_digest: String,
    pub decision_ref: String,
    pub verified_usage: Vec<ComputeMeterReading>,
    pub verified_usage_digest: String,
    pub compensable_usage: Vec<ComputeMeterReading>,
    pub compensable_usage_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub decided_by_user_id: String,
    pub decided_at: String,
    pub created_at: String,
}

pub(super) fn normalize_verification_request(
    input: &DecideComputeAttemptVerificationRequest,
) -> Result<DecideComputeAttemptVerificationRequest> {
    for (label, value, max_len) in [
        ("Attempt Lease ID", input.lease_id.as_str(), 200),
        (
            "Provider 候选 ID",
            input.expected_terminal_candidate_id.as_str(),
            200,
        ),
        (
            "Provider 候选事件摘要",
            input.expected_terminal_candidate_event_digest.as_str(),
            64,
        ),
        (
            "消费者审核 ID",
            input.expected_consumer_review_id.as_str(),
            200,
        ),
        (
            "消费者审核事件摘要",
            input.expected_consumer_review_event_digest.as_str(),
            64,
        ),
        (
            "平台观测 ID",
            input.expected_platform_observation_id.as_str(),
            200,
        ),
        (
            "平台观测事件摘要",
            input.expected_platform_observation_event_digest.as_str(),
            64,
        ),
        ("Verification policy ID", input.policy_id.as_str(), 100),
        ("Verification 决定", input.decision.as_str(), 40),
        ("Verification 决定引用", input.decision_ref.as_str(), 1000),
        ("幂等键", input.idempotency_key.as_str(), 200),
        (
            "Verification 操作者",
            input.decided_by_user_id.as_str(),
            200,
        ),
    ] {
        validate_exact(label, value, max_len)?;
    }
    for (label, digest) in [
        (
            "Provider 候选事件摘要",
            input.expected_terminal_candidate_event_digest.as_str(),
        ),
        (
            "消费者审核事件摘要",
            input.expected_consumer_review_event_digest.as_str(),
        ),
        (
            "平台观测事件摘要",
            input.expected_platform_observation_event_digest.as_str(),
        ),
    ] {
        validate_digest(label, digest)?;
    }
    if input.policy_id != VERIFICATION_POLICY_CONSERVATIVE_MIN_V1 || input.policy_version != 1 {
        bail!("v192 只支持 conservative_min_v1 policy version 1");
    }
    if !matches!(
        input.decision.as_str(),
        VERIFICATION_DECISION_ACCEPTED
            | VERIFICATION_DECISION_REJECTED
            | VERIFICATION_DECISION_DISPUTED
    ) {
        bail!("Verification 决定只允许 accepted、rejected 或 disputed");
    }
    if input.reason_codes.is_empty() || input.reason_codes.len() > MAX_REASON_CODES {
        bail!("Verification reason_codes 必须包含 1 至 16 项");
    }
    let mut normalized = input.clone();
    normalized.lease_id = normalized.lease_id.trim().to_string();
    normalized.expected_terminal_candidate_id =
        normalized.expected_terminal_candidate_id.trim().to_string();
    normalized.expected_terminal_candidate_event_digest = normalized
        .expected_terminal_candidate_event_digest
        .trim()
        .to_ascii_lowercase();
    normalized.expected_consumer_review_id =
        normalized.expected_consumer_review_id.trim().to_string();
    normalized.expected_consumer_review_event_digest = normalized
        .expected_consumer_review_event_digest
        .trim()
        .to_ascii_lowercase();
    normalized.expected_platform_observation_id = normalized
        .expected_platform_observation_id
        .trim()
        .to_string();
    normalized.expected_platform_observation_event_digest = normalized
        .expected_platform_observation_event_digest
        .trim()
        .to_ascii_lowercase();
    normalized.policy_id = normalized.policy_id.trim().to_string();
    normalized.decision = normalized.decision.trim().to_ascii_lowercase();
    normalized.decision_ref = normalized.decision_ref.trim().to_string();
    normalized.idempotency_key = normalized.idempotency_key.trim().to_string();
    normalized.decided_by_user_id = normalized.decided_by_user_id.trim().to_string();
    let mut reason_codes = BTreeSet::new();
    for reason_code in &input.reason_codes {
        validate_exact("Verification reason code", reason_code, 100)?;
        reason_codes.insert(reason_code.trim().to_ascii_lowercase());
    }
    normalized.reason_codes = reason_codes.into_iter().collect();
    Ok(normalized)
}

pub(super) fn ensure_expected_binding(
    input: &DecideComputeAttemptVerificationRequest,
    candidate: &ComputeAttemptTerminalCandidateReceipt,
    consumer_review: &ComputeAttemptConsumerReviewReceipt,
    platform_observation: &ComputeAttemptPlatformObservationReceipt,
) -> Result<()> {
    if candidate.terminal_candidate_id != input.expected_terminal_candidate_id
        || candidate.event_digest != input.expected_terminal_candidate_event_digest
        || consumer_review.consumer_review_id != input.expected_consumer_review_id
        || consumer_review.event_digest != input.expected_consumer_review_event_digest
        || platform_observation.platform_observation_id != input.expected_platform_observation_id
        || platform_observation.event_digest != input.expected_platform_observation_event_digest
    {
        bail!("Verification 必须绑定精确的 v189-v191 证据 ID 与事件摘要");
    }
    Ok(())
}

pub(super) fn ensure_evidence_binding(
    candidate: &ComputeAttemptTerminalCandidateReceipt,
    consumer_review: &ComputeAttemptConsumerReviewReceipt,
    platform_observation: &ComputeAttemptPlatformObservationReceipt,
    provider_usage: &ComputeAttemptUsageDeclarationReceipt,
    reservation: &ComputeReservationRegistrationReceipt,
) -> Result<()> {
    if consumer_review.terminal_candidate_id != candidate.terminal_candidate_id
        || consumer_review.terminal_candidate_event_digest != candidate.event_digest
        || platform_observation.terminal_candidate_id != candidate.terminal_candidate_id
        || platform_observation.terminal_candidate_event_digest != candidate.event_digest
        || provider_usage.snapshot_id != candidate.final_usage_snapshot_id
        || provider_usage.sequence_no != candidate.final_usage_sequence_no
        || provider_usage.cumulative_usage_digest != candidate.final_cumulative_usage_digest
    {
        bail!("Verification 证据链未绑定同一终态候选和最终用量快照");
    }
    let business_keys_match = [
        consumer_review.lease_id.as_str(),
        platform_observation.lease_id.as_str(),
        provider_usage.lease_id.as_str(),
    ]
    .iter()
    .all(|lease_id| *lease_id == candidate.lease_id);
    let causality_matches = consumer_review.provider_id == candidate.provider_id
        && platform_observation.provider_id == candidate.provider_id
        && provider_usage.provider_id == candidate.provider_id
        && consumer_review.consumer_account_id == candidate.consumer_account_id
        && platform_observation.consumer_account_id == candidate.consumer_account_id
        && provider_usage.consumer_account_id == candidate.consumer_account_id
        && consumer_review.job_id == candidate.job_id
        && platform_observation.job_id == candidate.job_id
        && provider_usage.job_id == candidate.job_id
        && consumer_review.reservation_id == candidate.reservation_id
        && platform_observation.reservation_id == candidate.reservation_id
        && provider_usage.reservation_id == candidate.reservation_id
        && consumer_review.capacity_claim_id == candidate.capacity_claim_id
        && platform_observation.capacity_claim_id == candidate.capacity_claim_id
        && provider_usage.capacity_claim_id == candidate.capacity_claim_id;
    if !business_keys_match || !causality_matches {
        bail!("Verification 证据链业务身份不一致");
    }
    if reservation.revision != candidate.reservation_revision
        || reservation.reservation_digest != candidate.reservation_digest
        || reservation.reservation.reservation_id != candidate.reservation_id
        || reservation.reservation.capacity_claim.claim_id != candidate.capacity_claim_id
        || reservation.reservation.capacity_claim.claim_revision
            != candidate.capacity_claim_revision
        || reservation.reservation.capacity_claim.claim_digest != candidate.capacity_claim_digest
    {
        bail!("Verification 绑定的 Reservation 历史版本与候选不一致");
    }
    Ok(())
}

pub(super) fn ensure_policy_decision(
    input: &DecideComputeAttemptVerificationRequest,
    candidate: &ComputeAttemptTerminalCandidateReceipt,
    consumer_review: &ComputeAttemptConsumerReviewReceipt,
    platform_observation: &ComputeAttemptPlatformObservationReceipt,
) -> Result<()> {
    if input.decision == VERIFICATION_DECISION_ACCEPTED
        && (consumer_review.decision != VERIFICATION_DECISION_ACCEPTED
            || candidate.outcome != platform_observation.observed_outcome
            || platform_observation.observed_outcome == "indeterminate")
    {
        bail!("accepted 只允许消费者接受且 Provider 与平台 outcome 一致的证据链");
    }
    Ok(())
}

pub(super) fn build_policy_usage(
    input: &DecideComputeAttemptVerificationRequest,
    provider_usage: &ComputeAttemptUsageDeclarationReceipt,
    platform_observation: &ComputeAttemptPlatformObservationReceipt,
    reservation: &ComputeReservation,
    decided_at: &str,
) -> Result<(Vec<ComputeMeterReading>, Vec<ComputeMeterReading>)> {
    let observed: BTreeMap<&str, i64> = platform_observation
        .cumulative_observed_usage
        .iter()
        .map(|reading| (reading.meter.as_str(), reading.quantity))
        .collect();
    let reserved: BTreeMap<&str, i64> = reservation
        .reserved_capacity
        .iter()
        .map(|line| (line.meter.as_str(), line.quantity))
        .collect();
    if observed.len() != provider_usage.cumulative_declared_usage.len()
        || reserved.len() != provider_usage.cumulative_declared_usage.len()
    {
        bail!("Verification policy 要求声明、观测与 Reservation meter 集合完全一致");
    }
    let source_id = format!("{}@{}", input.policy_id, input.policy_version);
    let mut verified = Vec::with_capacity(provider_usage.cumulative_declared_usage.len());
    let mut compensable = Vec::with_capacity(provider_usage.cumulative_declared_usage.len());
    for declared in &provider_usage.cumulative_declared_usage {
        let observed_quantity = *observed
            .get(declared.meter.as_str())
            .ok_or_else(|| anyhow::anyhow!("平台观测缺少 meter {}", declared.meter))?;
        let reserved_quantity = *reserved
            .get(declared.meter.as_str())
            .ok_or_else(|| anyhow::anyhow!("Reservation 缺少 meter {}", declared.meter))?;
        let verified_quantity = if input.decision == VERIFICATION_DECISION_ACCEPTED {
            declared.quantity.min(observed_quantity)
        } else {
            0
        };
        let compensable_quantity = verified_quantity.min(reserved_quantity);
        verified.push(build_reading(
            "verified_usage",
            &declared.meter,
            verified_quantity,
            "verification_policy",
            &source_id,
            decided_at,
        )?);
        compensable.push(build_reading(
            "compensable_usage",
            &declared.meter,
            compensable_quantity,
            "compensation_policy",
            &source_id,
            decided_at,
        )?);
    }
    Ok((verified, compensable))
}

fn build_reading(
    purpose: &str,
    meter: &str,
    quantity: i64,
    source_kind: &str,
    source_id: &str,
    observed_at: &str,
) -> Result<ComputeMeterReading> {
    let reading_digest = sha256_json(&json!({
        "purpose": purpose,
        "meter": meter,
        "quantity": quantity,
        "source_kind": source_kind,
        "source_id": source_id,
        "observed_at": observed_at,
    }))?;
    Ok(ComputeMeterReading {
        meter: meter.to_string(),
        quantity,
        source_kind: source_kind.to_string(),
        source_id: source_id.to_string(),
        reading_digest,
        observed_at: observed_at.to_string(),
    })
}

pub(super) fn reason_codes_digest(reason_codes: &[String]) -> Result<String> {
    sha256_json(&json!({
        "purpose": "compute_attempt_verification_reason_codes",
        "reason_codes": reason_codes,
    }))
}

pub(super) fn verification_usage_digest(
    usage_kind: &str,
    readings: &[ComputeMeterReading],
) -> Result<String> {
    sha256_json(&json!({
        "purpose": "compute_attempt_verification_usage",
        "usage_kind": usage_kind,
        "readings": readings,
    }))
}

pub(super) fn verification_request_digest(
    input: &DecideComputeAttemptVerificationRequest,
) -> Result<String> {
    sha256_json(&json!({
        "purpose": "compute_attempt_verification_request",
        "lease_id": input.lease_id,
        "terminal_candidate_id": input.expected_terminal_candidate_id,
        "terminal_candidate_event_digest": input.expected_terminal_candidate_event_digest,
        "consumer_review_id": input.expected_consumer_review_id,
        "consumer_review_event_digest": input.expected_consumer_review_event_digest,
        "platform_observation_id": input.expected_platform_observation_id,
        "platform_observation_event_digest": input.expected_platform_observation_event_digest,
        "policy_id": input.policy_id,
        "policy_version": input.policy_version,
        "decision": input.decision,
        "reason_codes": input.reason_codes,
        "decision_ref": input.decision_ref,
        "idempotency_key": input.idempotency_key,
        "decided_by_user_id": input.decided_by_user_id,
    }))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn verification_event_digest(
    verification_decision_id: &str,
    input: &DecideComputeAttemptVerificationRequest,
    candidate: &ComputeAttemptTerminalCandidateReceipt,
    consumer_review: &ComputeAttemptConsumerReviewReceipt,
    platform_observation: &ComputeAttemptPlatformObservationReceipt,
    reason_codes_digest: &str,
    verified_usage_digest: &str,
    compensable_usage_digest: &str,
    request_digest: &str,
    decided_at: &str,
) -> Result<String> {
    sha256_json(&json!({
        "purpose": "compute_attempt_verification_decision",
        "verification_decision_id": verification_decision_id,
        "lease_id": candidate.lease_id,
        "terminal_candidate_id": candidate.terminal_candidate_id,
        "terminal_candidate_event_digest": candidate.event_digest,
        "consumer_review_id": consumer_review.consumer_review_id,
        "consumer_review_event_digest": consumer_review.event_digest,
        "platform_observation_id": platform_observation.platform_observation_id,
        "platform_observation_event_digest": platform_observation.event_digest,
        "policy_id": input.policy_id,
        "policy_version": input.policy_version,
        "decision": input.decision,
        "reason_codes_digest": reason_codes_digest,
        "verified_usage_digest": verified_usage_digest,
        "compensable_usage_digest": compensable_usage_digest,
        "request_digest": request_digest,
        "decided_by_user_id": input.decided_by_user_id,
        "decided_at": decided_at,
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
