use anyhow::{bail, Result};

use crate::compute_federation::{
    federation_historical_causal_reference::{
        ConsumerReviewRef, ExecutionReceiptRef, ExecutionVerificationSourceLineageV1,
        PlatformObservationRef, ProviderDeclaredUsageRef, TerminalCandidateRef,
        VerificationDecisionRef,
    },
    receipts::{ComputeAttestationEvidence, ComputeMeterReading, ComputeVerificationDecision},
};

use super::source_refs::positive_u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FinalProviderUsageRef {
    pub(super) lease_id: String,
    pub(super) usage_snapshot_id: String,
    pub(super) usage_sequence_no: u64,
    pub(super) cumulative_usage_digest: String,
}

#[derive(Clone)]
pub(super) struct ExecutionVerificationSourceLinkFacts {
    pub(super) lineage: ExecutionVerificationSourceLineageV1,
    pub(super) rebuilt_execution_receipt: ExecutionReceiptRef,
    pub(super) rebuilt_execution_lineage_digest: String,
    pub(super) audited_provider_declared_usage: ProviderDeclaredUsageRef,
    pub(super) audited_provider_declared_usage_lease_id: String,
    pub(super) audited_terminal_candidate: TerminalCandidateRef,
    pub(super) audited_consumer_review: ConsumerReviewRef,
    pub(super) audited_platform_observation: PlatformObservationRef,
    pub(super) audited_verification_decision: VerificationDecisionRef,
    pub(super) candidate_final_usage: FinalProviderUsageRef,
    pub(super) consumer_review_terminal_candidate: TerminalCandidateRef,
    pub(super) consumer_review_final_usage: FinalProviderUsageRef,
    pub(super) platform_observation_terminal_candidate: TerminalCandidateRef,
    pub(super) platform_observation_final_usage: FinalProviderUsageRef,
    pub(super) verification_terminal_candidate: TerminalCandidateRef,
    pub(super) verification_consumer_review: ConsumerReviewRef,
    pub(super) verification_platform_observation: PlatformObservationRef,
    pub(super) verification_final_usage: FinalProviderUsageRef,
    pub(super) verification_platform_observed_usage_digest: String,
    pub(super) execution_verification_decision_id: String,
    pub(super) execution_verification_event_digest: String,
    pub(super) execution_declared_usage: Vec<ComputeMeterReading>,
    pub(super) audited_declared_usage: Vec<ComputeMeterReading>,
    pub(super) execution_observed_usage: Vec<ComputeMeterReading>,
    pub(super) audited_observed_usage: Vec<ComputeMeterReading>,
    pub(super) execution_verified_usage: Vec<ComputeMeterReading>,
    pub(super) audited_verified_usage: Vec<ComputeMeterReading>,
    pub(super) execution_compensable_usage: Vec<ComputeMeterReading>,
    pub(super) audited_compensable_usage: Vec<ComputeMeterReading>,
    pub(super) execution_attestations: Vec<ComputeAttestationEvidence>,
    pub(super) expected_execution_attestations: Vec<ComputeAttestationEvidence>,
    pub(super) execution_verification: ComputeVerificationDecision,
    pub(super) expected_execution_verification: ComputeVerificationDecision,
}

pub(super) fn validate_execution_verification_source_links(
    facts: &ExecutionVerificationSourceLinkFacts,
) -> Result<()> {
    let lineage = &facts.lineage;
    if lineage.execution_receipt != facts.rebuilt_execution_receipt
        || lineage.execution_lineage_digest != facts.rebuilt_execution_lineage_digest
        || lineage.provider_declared_usage != facts.audited_provider_declared_usage
        || lineage.terminal_candidate != facts.audited_terminal_candidate
        || lineage.consumer_review != facts.audited_consumer_review
        || lineage.platform_observation != facts.audited_platform_observation
        || lineage.verification_decision != facts.audited_verification_decision
    {
        bail!("execution verification source 的七键 lineage 与 retained owners 不一致");
    }

    let audited_final_usage = FinalProviderUsageRef {
        lease_id: facts.audited_provider_declared_usage_lease_id.clone(),
        usage_snapshot_id: facts
            .audited_provider_declared_usage
            .usage_snapshot_id
            .clone(),
        usage_sequence_no: facts.audited_provider_declared_usage.usage_sequence_no,
        cumulative_usage_digest: facts
            .audited_provider_declared_usage
            .cumulative_usage_digest
            .clone(),
    };
    if facts.candidate_final_usage != audited_final_usage
        || facts.consumer_review_final_usage != audited_final_usage
        || facts.platform_observation_final_usage != audited_final_usage
        || facts.verification_final_usage != audited_final_usage
        || facts.consumer_review_terminal_candidate != facts.audited_terminal_candidate
        || facts.platform_observation_terminal_candidate != facts.audited_terminal_candidate
        || facts.verification_terminal_candidate != facts.audited_terminal_candidate
        || facts.verification_consumer_review != facts.audited_consumer_review
        || facts.verification_platform_observation != facts.audited_platform_observation
        || facts.verification_platform_observed_usage_digest
            != facts
                .audited_platform_observation
                .cumulative_observed_usage_digest
    {
        bail!("execution verification source 的 v188-v192 evidence cross-link 不一致");
    }

    if facts.execution_verification_decision_id
        != facts.audited_verification_decision.verification_decision_id
        || facts.execution_verification_event_digest
            != facts
                .audited_verification_decision
                .verification_event_digest
        || facts.execution_declared_usage != facts.audited_declared_usage
        || facts.execution_observed_usage != facts.audited_observed_usage
        || facts.execution_verified_usage != facts.audited_verified_usage
        || facts.execution_compensable_usage != facts.audited_compensable_usage
        || facts.execution_attestations != facts.expected_execution_attestations
        || facts.execution_verification != facts.expected_execution_verification
    {
        bail!("execution verification source 的 v193 evidence carrier 与 v188-v192 不一致");
    }
    Ok(())
}

pub(super) fn provider_declared_usage_ref(
    usage_snapshot_id: &str,
    usage_sequence_no: i64,
    cumulative_usage_digest: &str,
    usage_event_digest: &str,
) -> Result<ProviderDeclaredUsageRef> {
    Ok(ProviderDeclaredUsageRef {
        usage_snapshot_id: usage_snapshot_id.to_string(),
        usage_sequence_no: positive_u64("Provider declared usage sequence", usage_sequence_no)?,
        cumulative_usage_digest: cumulative_usage_digest.to_string(),
        usage_event_digest: usage_event_digest.to_string(),
    })
}

pub(super) fn final_provider_usage_ref(
    lease_id: &str,
    usage_snapshot_id: &str,
    usage_sequence_no: i64,
    cumulative_usage_digest: &str,
) -> Result<FinalProviderUsageRef> {
    Ok(FinalProviderUsageRef {
        lease_id: lease_id.to_string(),
        usage_snapshot_id: usage_snapshot_id.to_string(),
        usage_sequence_no: positive_u64("Final provider usage sequence", usage_sequence_no)?,
        cumulative_usage_digest: cumulative_usage_digest.to_string(),
    })
}

pub(super) fn terminal_candidate_ref(
    terminal_candidate_id: &str,
    terminal_candidate_event_digest: &str,
) -> TerminalCandidateRef {
    TerminalCandidateRef {
        terminal_candidate_id: terminal_candidate_id.to_string(),
        terminal_candidate_event_digest: terminal_candidate_event_digest.to_string(),
    }
}

pub(super) fn consumer_review_ref(
    consumer_review_id: &str,
    consumer_review_event_digest: &str,
) -> ConsumerReviewRef {
    ConsumerReviewRef {
        consumer_review_id: consumer_review_id.to_string(),
        consumer_review_event_digest: consumer_review_event_digest.to_string(),
    }
}

pub(super) fn platform_observation_ref(
    platform_observation_id: &str,
    platform_observation_event_digest: &str,
    cumulative_observed_usage_digest: &str,
) -> PlatformObservationRef {
    PlatformObservationRef {
        platform_observation_id: platform_observation_id.to_string(),
        platform_observation_event_digest: platform_observation_event_digest.to_string(),
        cumulative_observed_usage_digest: cumulative_observed_usage_digest.to_string(),
    }
}

pub(super) fn verification_decision_ref(
    verification_decision_id: &str,
    verification_event_digest: &str,
    verified_usage_digest: &str,
    compensable_usage_digest: &str,
) -> VerificationDecisionRef {
    VerificationDecisionRef {
        verification_decision_id: verification_decision_id.to_string(),
        verification_event_digest: verification_event_digest.to_string(),
        verified_usage_digest: verified_usage_digest.to_string(),
        compensable_usage_digest: compensable_usage_digest.to_string(),
    }
}
