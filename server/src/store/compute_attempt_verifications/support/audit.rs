use anyhow::{bail, Result};

use crate::{
    compute_federation::execution::ComputeReservation,
    store::{
        ComputeAttemptConsumerReviewReceipt, ComputeAttemptPlatformObservationReceipt,
        ComputeAttemptTerminalCandidateReceipt, ComputeAttemptUsageDeclarationReceipt,
    },
};

use super::{
    build_policy_usage, ensure_expected_binding, ensure_policy_decision,
    normalize_verification_request, reason_codes_digest, verification_event_digest,
    verification_request_digest, verification_usage_digest, StoredVerificationDecision,
};
use crate::store::compute_attempt_verifications::{
    ComputeAttemptVerificationDecisionReceipt, DecideComputeAttemptVerificationRequest,
    COMPUTE_ATTEMPT_VERIFICATION_DECISION_SCHEMA, VERIFICATION_DECISION_ACCEPTED,
    VERIFICATION_DECISION_REJECTED,
};

impl StoredVerificationDecision {
    pub(super) fn into_receipt(
        self,
        candidate: &ComputeAttemptTerminalCandidateReceipt,
        consumer_review: &ComputeAttemptConsumerReviewReceipt,
        platform_observation: &ComputeAttemptPlatformObservationReceipt,
        provider_usage: &ComputeAttemptUsageDeclarationReceipt,
        reservation: &ComputeReservation,
        replayed: bool,
    ) -> Result<ComputeAttemptVerificationDecisionReceipt> {
        audit_verification_decision(
            &self,
            candidate,
            consumer_review,
            platform_observation,
            provider_usage,
            reservation,
        )?;
        Ok(ComputeAttemptVerificationDecisionReceipt {
            schema: COMPUTE_ATTEMPT_VERIFICATION_DECISION_SCHEMA,
            verification_decision_id: self.verification_decision_id,
            terminal_candidate_id: self.terminal_candidate_id,
            terminal_candidate_event_digest: self.terminal_candidate_event_digest,
            consumer_review_id: self.consumer_review_id,
            consumer_review_event_digest: self.consumer_review_event_digest,
            platform_observation_id: self.platform_observation_id,
            platform_observation_event_digest: self.platform_observation_event_digest,
            lease_id: self.lease_id,
            provider_id: candidate.provider_id.clone(),
            consumer_account_id: candidate.consumer_account_id.clone(),
            source_lease_revision: candidate.source_lease_revision,
            source_lease_digest: candidate.source_lease_digest.clone(),
            fencing_generation: candidate.fencing_generation,
            job_id: candidate.job_id.clone(),
            job_revision: candidate.job_revision,
            job_digest: candidate.job_digest.clone(),
            reservation_id: candidate.reservation_id.clone(),
            reservation_revision: candidate.reservation_revision,
            reservation_digest: candidate.reservation_digest.clone(),
            capacity_claim_id: candidate.capacity_claim_id.clone(),
            capacity_claim_revision: candidate.capacity_claim_revision,
            capacity_claim_digest: candidate.capacity_claim_digest.clone(),
            final_usage_snapshot_id: candidate.final_usage_snapshot_id.clone(),
            final_usage_sequence_no: candidate.final_usage_sequence_no,
            final_provider_usage_digest: provider_usage.cumulative_usage_digest.clone(),
            platform_observed_usage_digest: platform_observation
                .cumulative_observed_usage_digest
                .clone(),
            candidate_outcome: candidate.outcome.clone(),
            consumer_decision: consumer_review.decision.clone(),
            observed_outcome: platform_observation.observed_outcome.clone(),
            policy_id: self.policy_id,
            policy_version: self.policy_version,
            decision: self.decision.clone(),
            reason_codes: self.reason_codes,
            reason_codes_digest: self.reason_codes_digest,
            decision_ref: self.decision_ref,
            verified_usage: self.verified_usage,
            verified_usage_digest: self.verified_usage_digest,
            compensable_usage: self.compensable_usage,
            compensable_usage_digest: self.compensable_usage_digest,
            request_digest: self.request_digest,
            event_digest: self.event_digest,
            decided_by_user_id: self.decided_by_user_id,
            decided_at: self.decided_at,
            verification_effect: match self.decision.as_str() {
                VERIFICATION_DECISION_ACCEPTED => "verified_usage_recorded",
                VERIFICATION_DECISION_REJECTED => "rejection_recorded",
                _ => "dispute_recorded",
            },
            execution_receipt_effect: "none",
            lease_effect: "unchanged",
            job_effect: "unchanged",
            capacity_effect: "unchanged",
            reservation_effect: "unchanged",
            money_effect: "preauthorization_unchanged",
            replayed,
        })
    }
}

fn audit_verification_decision(
    stored: &StoredVerificationDecision,
    candidate: &ComputeAttemptTerminalCandidateReceipt,
    consumer_review: &ComputeAttemptConsumerReviewReceipt,
    platform_observation: &ComputeAttemptPlatformObservationReceipt,
    provider_usage: &ComputeAttemptUsageDeclarationReceipt,
    reservation: &ComputeReservation,
) -> Result<()> {
    let request = normalize_verification_request(&DecideComputeAttemptVerificationRequest {
        lease_id: stored.lease_id.clone(),
        expected_terminal_candidate_id: stored.terminal_candidate_id.clone(),
        expected_terminal_candidate_event_digest: stored.terminal_candidate_event_digest.clone(),
        expected_consumer_review_id: stored.consumer_review_id.clone(),
        expected_consumer_review_event_digest: stored.consumer_review_event_digest.clone(),
        expected_platform_observation_id: stored.platform_observation_id.clone(),
        expected_platform_observation_event_digest: stored
            .platform_observation_event_digest
            .clone(),
        policy_id: stored.policy_id.clone(),
        policy_version: stored.policy_version,
        decision: stored.decision.clone(),
        reason_codes: stored.reason_codes.clone(),
        decision_ref: stored.decision_ref.clone(),
        idempotency_key: stored.idempotency_key.clone(),
        decided_by_user_id: stored.decided_by_user_id.clone(),
    })?;
    ensure_expected_binding(&request, candidate, consumer_review, platform_observation)?;
    ensure_policy_decision(&request, candidate, consumer_review, platform_observation)?;
    if stored.idempotency_scope
        != format!("compute_attempt_verification:{}", stored.decided_by_user_id)
        || stored.created_at != stored.decided_at
    {
        bail!("Verification 决定幂等范围或时间字段被篡改");
    }
    let expected_reason_codes_digest = reason_codes_digest(&request.reason_codes)?;
    let (expected_verified, expected_compensable) = build_policy_usage(
        &request,
        provider_usage,
        platform_observation,
        reservation,
        &stored.decided_at,
    )?;
    let expected_verified_digest = verification_usage_digest("verified", &expected_verified)?;
    let expected_compensable_digest =
        verification_usage_digest("compensable", &expected_compensable)?;
    let expected_request_digest = verification_request_digest(&request)?;
    let expected_event_digest = verification_event_digest(
        &stored.verification_decision_id,
        &request,
        candidate,
        consumer_review,
        platform_observation,
        &expected_reason_codes_digest,
        &expected_verified_digest,
        &expected_compensable_digest,
        &expected_request_digest,
        &stored.decided_at,
    )?;
    if stored.reason_codes != request.reason_codes
        || stored.reason_codes_digest != expected_reason_codes_digest
        || stored.verified_usage != expected_verified
        || stored.verified_usage_digest != expected_verified_digest
        || stored.compensable_usage != expected_compensable
        || stored.compensable_usage_digest != expected_compensable_digest
        || stored.request_digest != expected_request_digest
        || stored.event_digest != expected_event_digest
    {
        bail!("Verification 决定审计摘要或策略用量不一致");
    }
    Ok(())
}
