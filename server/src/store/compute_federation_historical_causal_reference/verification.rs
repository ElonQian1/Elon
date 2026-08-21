use anyhow::{anyhow, bail, Result};
use rusqlite::Connection;

use crate::{
    compute_federation::{
        federation_historical_causal_reference::{
            build_execution_verification_source_carrier, ExecutionVerificationSourceLineageV1,
            FederationHistoricalLineageKindV1,
        },
        receipts::{ComputeAttestationEvidence, ComputeVerificationDecision},
    },
    store::{
        compute_attempt_consumer_reviews::compute_attempt_historical_consumer_review_on,
        compute_attempt_execution_receipts::ComputeAttemptExecutionReceiptEnvelope,
        compute_attempt_platform_observations::compute_attempt_historical_platform_observation_on,
        compute_attempt_terminals::compute_attempt_historical_terminal_candidate_on,
        compute_attempt_usage::compute_attempt_usage_declaration_on,
        compute_attempt_verifications::compute_attempt_historical_verification_decision_on,
    },
};

use super::{
    execution,
    source_refs::execution_receipt_ref,
    verification_refs::{
        consumer_review_ref, final_provider_usage_ref, platform_observation_ref,
        provider_declared_usage_ref, terminal_candidate_ref,
        validate_execution_verification_source_links, verification_decision_ref,
        ExecutionVerificationSourceLinkFacts,
    },
    ValidatedFederationHistoricalLineage,
};

pub(super) fn resolve_execution_verification_source_lineage_on(
    conn: &Connection,
    execution: &ComputeAttemptExecutionReceiptEnvelope,
) -> Result<ValidatedFederationHistoricalLineage> {
    let rebuilt_execution = execution::resolve_execution_source_lineage_on(
        conn,
        &execution.receipt.receipt_id,
        &execution.receipt.receipt_digest,
    )?;
    if rebuilt_execution.kind() != FederationHistoricalLineageKindV1::ExecutionSourceV1 {
        bail!("execution verification source root did not rebuild as execution_source_v1");
    }
    let (execution_lineage_digest, access_scope) =
        rebuilt_execution.into_lineage_digest_and_access_scope();

    let lease_id = &execution.receipt.attempt_lease_id;
    let candidate =
        compute_attempt_historical_terminal_candidate_on(conn, lease_id)?.ok_or_else(|| {
            anyhow!("execution verification source v189 terminal candidate is absent")
        })?;
    let provider_usage =
        compute_attempt_usage_declaration_on(conn, lease_id, candidate.final_usage_sequence_no)?
            .ok_or_else(|| {
                anyhow!("execution verification source v188 provider declared usage is absent")
            })?;
    let consumer_review = compute_attempt_historical_consumer_review_on(conn, lease_id)?
        .ok_or_else(|| anyhow!("execution verification source v190 consumer review is absent"))?;
    let platform_observation = compute_attempt_historical_platform_observation_on(conn, lease_id)?
        .ok_or_else(|| {
            anyhow!("execution verification source v191 platform observation is absent")
        })?;
    let verification = compute_attempt_historical_verification_decision_on(conn, lease_id)?
        .ok_or_else(|| {
            anyhow!("execution verification source v192 verification decision is absent")
        })?;

    let execution_receipt = execution_receipt_ref(
        &execution.receipt.receipt_id,
        &execution.receipt.receipt_digest,
    );
    let provider_declared_usage = provider_declared_usage_ref(
        &provider_usage.snapshot_id,
        provider_usage.sequence_no,
        &provider_usage.cumulative_usage_digest,
        &provider_usage.event_digest,
    )?;
    let terminal_candidate =
        terminal_candidate_ref(&candidate.terminal_candidate_id, &candidate.event_digest);
    let consumer_review_lineage = consumer_review_ref(
        &consumer_review.consumer_review_id,
        &consumer_review.event_digest,
    );
    let platform_observation_lineage = platform_observation_ref(
        &platform_observation.platform_observation_id,
        &platform_observation.event_digest,
        &platform_observation.cumulative_observed_usage_digest,
    );
    let verification_decision = verification_decision_ref(
        &verification.verification_decision_id,
        &verification.event_digest,
        &verification.verified_usage_digest,
        &verification.compensable_usage_digest,
    );
    let lineage = ExecutionVerificationSourceLineageV1 {
        execution_receipt: execution_receipt.clone(),
        execution_lineage_digest: execution_lineage_digest.clone(),
        provider_declared_usage: provider_declared_usage.clone(),
        terminal_candidate: terminal_candidate.clone(),
        consumer_review: consumer_review_lineage.clone(),
        platform_observation: platform_observation_lineage.clone(),
        verification_decision: verification_decision.clone(),
    };

    let facts = ExecutionVerificationSourceLinkFacts {
        rebuilt_execution_receipt: execution_receipt,
        rebuilt_execution_lineage_digest: execution_lineage_digest,
        audited_provider_declared_usage: provider_declared_usage,
        audited_provider_declared_usage_lease_id: provider_usage.lease_id.clone(),
        audited_terminal_candidate: terminal_candidate,
        audited_consumer_review: consumer_review_lineage,
        audited_platform_observation: platform_observation_lineage,
        audited_verification_decision: verification_decision,
        candidate_final_usage: final_provider_usage_ref(
            &candidate.lease_id,
            &candidate.final_usage_snapshot_id,
            candidate.final_usage_sequence_no,
            &candidate.final_cumulative_usage_digest,
        )?,
        consumer_review_terminal_candidate: terminal_candidate_ref(
            &consumer_review.terminal_candidate_id,
            &consumer_review.terminal_candidate_event_digest,
        ),
        consumer_review_final_usage: final_provider_usage_ref(
            &consumer_review.lease_id,
            &consumer_review.final_usage_snapshot_id,
            consumer_review.final_usage_sequence_no,
            &consumer_review.final_cumulative_usage_digest,
        )?,
        platform_observation_terminal_candidate: terminal_candidate_ref(
            &platform_observation.terminal_candidate_id,
            &platform_observation.terminal_candidate_event_digest,
        ),
        platform_observation_final_usage: final_provider_usage_ref(
            &platform_observation.lease_id,
            &platform_observation.final_usage_snapshot_id,
            platform_observation.final_usage_sequence_no,
            &platform_observation.final_provider_usage_digest,
        )?,
        verification_terminal_candidate: terminal_candidate_ref(
            &verification.terminal_candidate_id,
            &verification.terminal_candidate_event_digest,
        ),
        verification_consumer_review: consumer_review_ref(
            &verification.consumer_review_id,
            &verification.consumer_review_event_digest,
        ),
        verification_platform_observation: platform_observation_ref(
            &verification.platform_observation_id,
            &verification.platform_observation_event_digest,
            &verification.platform_observed_usage_digest,
        ),
        verification_final_usage: final_provider_usage_ref(
            &verification.lease_id,
            &verification.final_usage_snapshot_id,
            verification.final_usage_sequence_no,
            &verification.final_provider_usage_digest,
        )?,
        verification_platform_observed_usage_digest: verification
            .platform_observed_usage_digest
            .clone(),
        execution_verification_decision_id: execution.verification_decision_id.clone(),
        execution_verification_event_digest: execution.verification_event_digest.clone(),
        execution_declared_usage: execution.receipt.usage.declared_usage.clone(),
        audited_declared_usage: provider_usage.cumulative_declared_usage.clone(),
        execution_observed_usage: execution.receipt.usage.observed_usage.clone(),
        audited_observed_usage: platform_observation.cumulative_observed_usage.clone(),
        execution_verified_usage: execution.receipt.usage.verified_usage.clone(),
        audited_verified_usage: verification.verified_usage.clone(),
        execution_compensable_usage: execution.receipt.usage.compensable_usage.clone(),
        audited_compensable_usage: verification.compensable_usage.clone(),
        execution_attestations: execution.receipt.attestations.clone(),
        expected_execution_attestations: vec![
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
        ],
        execution_verification: execution.receipt.verification.clone(),
        expected_execution_verification: ComputeVerificationDecision {
            status: verification.decision.clone(),
            policy_id: verification.policy_id.clone(),
            policy_version: verification.policy_version,
            reason_codes: verification.reason_codes.clone(),
            duplicate_receipt_ids: Vec::new(),
            challenge_receipt_ids: Vec::new(),
            decision_digest: verification.event_digest.clone(),
            decided_at: Some(verification.decided_at.clone()),
        },
        lineage,
    };
    validate_execution_verification_source_links(&facts)?;
    ValidatedFederationHistoricalLineage::from_carrier(
        build_execution_verification_source_carrier(facts.lineage)?,
        access_scope,
    )
}
