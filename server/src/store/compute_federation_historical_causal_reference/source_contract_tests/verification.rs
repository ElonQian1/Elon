use crate::compute_federation::{
    federation_historical_causal_reference::{
        ConsumerReviewRef, ExecutionReceiptRef, ExecutionVerificationSourceLineageV1,
        PlatformObservationRef, ProviderDeclaredUsageRef, TerminalCandidateRef,
        VerificationDecisionRef,
    },
    receipts::{ComputeAttestationEvidence, ComputeMeterReading, ComputeVerificationDecision},
};

use super::super::verification_refs::{
    validate_execution_verification_source_links, ExecutionVerificationSourceLinkFacts,
    FinalProviderUsageRef,
};

const STORE_FACADE: &str = include_str!("../../compute_federation_historical_causal_reference.rs");
const VERIFICATION_RESOLVER: &str = include_str!("../verification.rs");
const VERIFICATION_REFS: &str = include_str!("../verification_refs.rs");
const V188_OWNER: &str = include_str!("../../compute_attempt_usage.rs");
const V188_MIGRATION: &str = include_str!("../../../compute_attempt_usage_migration.rs");
const V189_OWNER: &str = include_str!("../../compute_attempt_terminals.rs");
const V190_OWNER: &str = include_str!("../../compute_attempt_consumer_reviews.rs");
const V191_OWNER: &str = include_str!("../../compute_attempt_platform_observations.rs");
const V192_OWNER: &str = include_str!("../../compute_attempt_verifications.rs");
const V193_OWNER: &str = include_str!("../../compute_attempt_execution_receipts.rs");
const V193_AUDIT: &str = include_str!("../../compute_attempt_execution_receipts/support/audit.rs");

#[test]
fn verification_resolver_starts_from_v193_and_reuses_one_execution_scope_linearly() {
    let facade = method_source(
        STORE_FACADE,
        "pub(crate) fn resolve_compute_execution_verification_source_lineage_for_lease(",
    );
    assert_eq!(
        facade
            .matches("transaction_with_behavior(TransactionBehavior::Deferred)")
            .count(),
        1
    );
    assert!(
        facade.contains("compute_attempt_historical_execution_receipt_by_lease_on(&tx, lease_id)")
    );
    assert!(facade.contains(
        "verification::resolve_execution_verification_source_lineage_on(&tx, &receipt)?"
    ));
    for forbidden in [
        "compute_attempt_usage_declaration_on",
        "compute_attempt_historical_terminal_candidate_on",
        "compute_attempt_historical_consumer_review_on",
        "compute_attempt_historical_platform_observation_on",
        "compute_attempt_historical_verification_decision_on",
    ] {
        assert!(
            !facade.contains(forbidden),
            "Store facade must leave evidence reads to the same-snapshot child: {forbidden}"
        );
    }

    let resolver = function_tail(
        VERIFICATION_RESOLVER,
        "pub(super) fn resolve_execution_verification_source_lineage_on(",
    );
    let rebuild = resolver
        .find("execution::resolve_execution_source_lineage_on(")
        .expect("verification resolver must rebuild the exact execution carrier");
    let evidence = resolver
        .find("compute_attempt_historical_terminal_candidate_on(conn, lease_id)")
        .expect("verification resolver must read v189 after rebuilding execution");
    assert!(rebuild < evidence);
    for required in [
        "rebuilt_execution.kind()",
        "FederationHistoricalLineageKindV1::ExecutionSourceV1",
        "rebuilt_execution.into_lineage_digest_and_access_scope()",
        "compute_attempt_usage_declaration_on(conn, lease_id, candidate.final_usage_sequence_no)",
        "compute_attempt_historical_consumer_review_on(conn, lease_id)",
        "compute_attempt_historical_platform_observation_on(conn, lease_id)",
        "compute_attempt_historical_verification_decision_on(conn, lease_id)",
        "validate_execution_verification_source_links(&facts)?",
        "build_execution_verification_source_carrier(facts.lineage)?",
    ] {
        assert!(
            resolver.contains(required),
            "missing verification proof: {required}"
        );
    }
    for forbidden in [
        "current_",
        "latest_",
        "caller",
        "TransactionBehavior::",
        "INSERT INTO",
        "UPDATE ",
        "DELETE FROM",
        "Utc::now",
        "new_id",
    ] {
        assert!(
            !resolver.contains(forbidden),
            "verification resolver must not use fallback/write authority: {forbidden}"
        );
    }
}

#[test]
fn verification_projects_six_exact_owner_refs_and_all_v193_evidence() {
    for mapping in [
        "usage_snapshot_id: usage_snapshot_id.to_string()",
        "usage_sequence_no: positive_u64(\"Provider declared usage sequence\", usage_sequence_no)?",
        "cumulative_usage_digest: cumulative_usage_digest.to_string()",
        "usage_event_digest: usage_event_digest.to_string()",
        "terminal_candidate_id: terminal_candidate_id.to_string()",
        "terminal_candidate_event_digest: terminal_candidate_event_digest.to_string()",
        "consumer_review_id: consumer_review_id.to_string()",
        "consumer_review_event_digest: consumer_review_event_digest.to_string()",
        "platform_observation_id: platform_observation_id.to_string()",
        "platform_observation_event_digest: platform_observation_event_digest.to_string()",
        "cumulative_observed_usage_digest: cumulative_observed_usage_digest.to_string()",
        "verification_decision_id: verification_decision_id.to_string()",
        "verification_event_digest: verification_event_digest.to_string()",
        "verified_usage_digest: verified_usage_digest.to_string()",
        "compensable_usage_digest: compensable_usage_digest.to_string()",
    ] {
        assert!(
            VERIFICATION_REFS.contains(mapping),
            "missing exact verification ABI mapping: {mapping}"
        );
    }
    for evidence in [
        "audited_provider_declared_usage_lease_id",
        "candidate_final_usage != audited_final_usage",
        "consumer_review_terminal_candidate != facts.audited_terminal_candidate",
        "platform_observation_terminal_candidate != facts.audited_terminal_candidate",
        "verification_terminal_candidate != facts.audited_terminal_candidate",
        "verification_consumer_review != facts.audited_consumer_review",
        "verification_platform_observation != facts.audited_platform_observation",
        "execution_declared_usage != facts.audited_declared_usage",
        "execution_observed_usage != facts.audited_observed_usage",
        "execution_verified_usage != facts.audited_verified_usage",
        "execution_compensable_usage != facts.audited_compensable_usage",
        "execution_attestations != facts.expected_execution_attestations",
        "execution_verification != facts.expected_execution_verification",
    ] {
        assert!(
            VERIFICATION_REFS.contains(evidence),
            "missing verification equality: {evidence}"
        );
    }
    for attestation in [
        "evidence_kind: \"provider_terminal_candidate\".to_string()",
        "evidence_kind: \"consumer_review\".to_string()",
        "evidence_kind: \"platform_observation\".to_string()",
    ] {
        assert!(VERIFICATION_RESOLVER.contains(attestation));
    }
}

#[test]
fn v188_through_v193_reads_are_retained_owner_audits() {
    assert!(V188_OWNER.contains("pub(crate) fn compute_attempt_usage_declaration_on("));
    assert!(V188_OWNER.contains("declaration_by_sequence_on(conn, lease_id, sequence_no)?"));
    assert!(V188_MIGRATION.contains("UNIQUE(lease_id, sequence_no)"));
    assert!(V188_MIGRATION.contains("trg_compute_attempt_usage_declarations_no_update"));
    assert!(V188_MIGRATION.contains("trg_compute_attempt_usage_declarations_no_delete"));
    for (owner, seam) in [
        (
            V189_OWNER,
            "fn compute_attempt_historical_terminal_candidate_on(",
        ),
        (
            V190_OWNER,
            "fn compute_attempt_historical_consumer_review_on(",
        ),
        (
            V191_OWNER,
            "fn compute_attempt_historical_platform_observation_on(",
        ),
        (
            V192_OWNER,
            "fn compute_attempt_historical_verification_decision_on(",
        ),
        (
            V193_OWNER,
            "fn compute_attempt_historical_execution_receipt_by_lease_on(",
        ),
    ] {
        assert!(owner.contains(seam), "missing retained owner seam: {seam}");
    }
    assert!(V193_OWNER.contains("execution_receipt_historical_envelope_on(conn, stored)"));
    assert!(V193_AUDIT.contains("self.receipt != expected_receipt"));
    assert!(V193_AUDIT.contains("self.verification_event_digest != verification.event_digest"));
}

#[test]
fn verification_cross_owner_splices_are_constructible_and_fail_closed() {
    let facts = verification_facts();
    validate_execution_verification_source_links(&facts).unwrap();

    let mut lineage_receipt = facts.clone();
    lineage_receipt
        .lineage
        .execution_receipt
        .execution_receipt_digest = "receipt-b".into();
    let mut lineage_digest = facts.clone();
    lineage_digest.lineage.execution_lineage_digest = "lineage-b".into();
    let mut lineage_usage = facts.clone();
    lineage_usage
        .lineage
        .provider_declared_usage
        .usage_event_digest = "usage-event-b".into();
    let mut lineage_candidate = facts.clone();
    lineage_candidate
        .lineage
        .terminal_candidate
        .terminal_candidate_event_digest = "candidate-event-b".into();
    let mut lineage_review = facts.clone();
    lineage_review
        .lineage
        .consumer_review
        .consumer_review_event_digest = "review-event-b".into();
    let mut lineage_observation = facts.clone();
    lineage_observation
        .lineage
        .platform_observation
        .platform_observation_event_digest = "observation-event-b".into();
    let mut lineage_verification = facts.clone();
    lineage_verification
        .lineage
        .verification_decision
        .verification_event_digest = "verification-event-b".into();
    let mut candidate_usage = facts.clone();
    candidate_usage.candidate_final_usage.usage_sequence_no += 1;
    let mut candidate_usage_lease = facts.clone();
    candidate_usage_lease.candidate_final_usage.lease_id = "lease-b".into();
    let mut review_candidate = facts.clone();
    review_candidate
        .consumer_review_terminal_candidate
        .terminal_candidate_id = "candidate-b".into();
    let mut review_usage = facts.clone();
    review_usage
        .consumer_review_final_usage
        .cumulative_usage_digest = "usage-b".into();
    let mut observation_candidate = facts.clone();
    observation_candidate
        .platform_observation_terminal_candidate
        .terminal_candidate_event_digest = "candidate-event-b".into();
    let mut observation_usage = facts.clone();
    observation_usage
        .platform_observation_final_usage
        .usage_snapshot_id = "usage-b".into();
    let mut verification_candidate = facts.clone();
    verification_candidate
        .verification_terminal_candidate
        .terminal_candidate_id = "candidate-b".into();
    let mut verification_review = facts.clone();
    verification_review
        .verification_consumer_review
        .consumer_review_id = "review-b".into();
    let mut verification_observation = facts.clone();
    verification_observation
        .verification_platform_observation
        .platform_observation_id = "observation-b".into();
    let mut verification_usage = facts.clone();
    verification_usage
        .verification_final_usage
        .cumulative_usage_digest = "usage-b".into();
    let mut verification_observed_digest = facts.clone();
    verification_observed_digest.verification_platform_observed_usage_digest = "observed-b".into();
    let mut execution_verification_id = facts.clone();
    execution_verification_id.execution_verification_decision_id = "verification-b".into();
    let mut execution_verification_event = facts.clone();
    execution_verification_event.execution_verification_event_digest =
        "verification-event-b".into();
    let mut declared_usage = facts.clone();
    declared_usage.execution_declared_usage[0].quantity += 1;
    let mut observed_usage = facts.clone();
    observed_usage.execution_observed_usage[0].quantity += 1;
    let mut verified_usage = facts.clone();
    verified_usage.execution_verified_usage[0].quantity += 1;
    let mut compensable_usage = facts.clone();
    compensable_usage.execution_compensable_usage[0].quantity += 1;
    let mut attestation = facts.clone();
    attestation.execution_attestations[0].evidence_digest = "candidate-event-b".into();
    let mut receipt_verification = facts.clone();
    receipt_verification.execution_verification.decision_digest = "verification-event-b".into();

    for (case, drifted) in [
        ("lineage_receipt", lineage_receipt),
        ("lineage_digest", lineage_digest),
        ("lineage_usage", lineage_usage),
        ("lineage_candidate", lineage_candidate),
        ("lineage_review", lineage_review),
        ("lineage_observation", lineage_observation),
        ("lineage_verification", lineage_verification),
        ("candidate_usage", candidate_usage),
        ("candidate_usage_lease", candidate_usage_lease),
        ("review_candidate", review_candidate),
        ("review_usage", review_usage),
        ("observation_candidate", observation_candidate),
        ("observation_usage", observation_usage),
        ("verification_candidate", verification_candidate),
        ("verification_review", verification_review),
        ("verification_observation", verification_observation),
        ("verification_usage", verification_usage),
        ("verification_observed_digest", verification_observed_digest),
        ("execution_verification_id", execution_verification_id),
        ("execution_verification_event", execution_verification_event),
        ("declared_usage", declared_usage),
        ("observed_usage", observed_usage),
        ("verified_usage", verified_usage),
        ("compensable_usage", compensable_usage),
        ("attestation", attestation),
        ("receipt_verification", receipt_verification),
    ] {
        assert!(
            validate_execution_verification_source_links(&drifted).is_err(),
            "verification splice must fail closed: {case}"
        );
    }
}

fn verification_facts() -> ExecutionVerificationSourceLinkFacts {
    let execution_receipt = ExecutionReceiptRef {
        execution_receipt_id: "execution-a".into(),
        execution_receipt_digest: "execution-digest-a".into(),
    };
    let provider_usage = ProviderDeclaredUsageRef {
        usage_snapshot_id: "usage-a".into(),
        usage_sequence_no: 7,
        cumulative_usage_digest: "usage-digest-a".into(),
        usage_event_digest: "usage-event-a".into(),
    };
    let final_usage = FinalProviderUsageRef {
        lease_id: "lease-a".into(),
        usage_snapshot_id: "usage-a".into(),
        usage_sequence_no: 7,
        cumulative_usage_digest: "usage-digest-a".into(),
    };
    let terminal_candidate = TerminalCandidateRef {
        terminal_candidate_id: "candidate-a".into(),
        terminal_candidate_event_digest: "candidate-event-a".into(),
    };
    let consumer_review = ConsumerReviewRef {
        consumer_review_id: "review-a".into(),
        consumer_review_event_digest: "review-event-a".into(),
    };
    let platform_observation = PlatformObservationRef {
        platform_observation_id: "observation-a".into(),
        platform_observation_event_digest: "observation-event-a".into(),
        cumulative_observed_usage_digest: "observed-digest-a".into(),
    };
    let verification_decision = VerificationDecisionRef {
        verification_decision_id: "verification-a".into(),
        verification_event_digest: "verification-event-a".into(),
        verified_usage_digest: "verified-digest-a".into(),
        compensable_usage_digest: "compensable-digest-a".into(),
    };
    let declared_usage = vec![meter_reading("declared", 8)];
    let observed_usage = vec![meter_reading("observed", 7)];
    let verified_usage = vec![meter_reading("verified", 7)];
    let compensable_usage = vec![meter_reading("compensable", 6)];
    let attestations = vec![
        attestation("provider_terminal_candidate", "candidate-event-a"),
        attestation("consumer_review", "review-event-a"),
        attestation("platform_observation", "observation-event-a"),
    ];
    let receipt_verification = ComputeVerificationDecision {
        status: "accepted".into(),
        policy_id: "policy-a".into(),
        policy_version: 1,
        reason_codes: vec!["matched".into()],
        duplicate_receipt_ids: Vec::new(),
        challenge_receipt_ids: Vec::new(),
        decision_digest: "verification-event-a".into(),
        decided_at: Some("2026-08-22T00:00:00Z".into()),
    };
    ExecutionVerificationSourceLinkFacts {
        lineage: ExecutionVerificationSourceLineageV1 {
            execution_receipt: execution_receipt.clone(),
            execution_lineage_digest: "lineage-a".into(),
            provider_declared_usage: provider_usage.clone(),
            terminal_candidate: terminal_candidate.clone(),
            consumer_review: consumer_review.clone(),
            platform_observation: platform_observation.clone(),
            verification_decision: verification_decision.clone(),
        },
        rebuilt_execution_receipt: execution_receipt,
        rebuilt_execution_lineage_digest: "lineage-a".into(),
        audited_provider_declared_usage: provider_usage,
        audited_provider_declared_usage_lease_id: "lease-a".into(),
        audited_terminal_candidate: terminal_candidate.clone(),
        audited_consumer_review: consumer_review.clone(),
        audited_platform_observation: platform_observation.clone(),
        audited_verification_decision: verification_decision,
        candidate_final_usage: final_usage.clone(),
        consumer_review_terminal_candidate: terminal_candidate.clone(),
        consumer_review_final_usage: final_usage.clone(),
        platform_observation_terminal_candidate: terminal_candidate.clone(),
        platform_observation_final_usage: final_usage.clone(),
        verification_terminal_candidate: terminal_candidate,
        verification_consumer_review: consumer_review,
        verification_platform_observation: platform_observation,
        verification_final_usage: final_usage,
        verification_platform_observed_usage_digest: "observed-digest-a".into(),
        execution_verification_decision_id: "verification-a".into(),
        execution_verification_event_digest: "verification-event-a".into(),
        execution_declared_usage: declared_usage.clone(),
        audited_declared_usage: declared_usage,
        execution_observed_usage: observed_usage.clone(),
        audited_observed_usage: observed_usage,
        execution_verified_usage: verified_usage.clone(),
        audited_verified_usage: verified_usage,
        execution_compensable_usage: compensable_usage.clone(),
        audited_compensable_usage: compensable_usage,
        execution_attestations: attestations.clone(),
        expected_execution_attestations: attestations,
        execution_verification: receipt_verification.clone(),
        expected_execution_verification: receipt_verification,
    }
}

fn meter_reading(source: &str, quantity: i64) -> ComputeMeterReading {
    ComputeMeterReading {
        meter: "gpu_second".into(),
        quantity,
        source_kind: source.into(),
        source_id: format!("{source}-a"),
        reading_digest: format!("{source}-digest-a"),
        observed_at: "2026-08-22T00:00:00Z".into(),
    }
}

fn attestation(kind: &str, digest: &str) -> ComputeAttestationEvidence {
    ComputeAttestationEvidence {
        evidence_kind: kind.into(),
        issuer: format!("{kind}-issuer"),
        evidence_digest: digest.into(),
        artifact_ref: Some(format!("{kind}-artifact")),
        observed_at: "2026-08-22T00:00:00Z".into(),
    }
}

fn function_tail<'a>(source: &'a str, marker: &str) -> &'a str {
    &source[source
        .find(marker)
        .expect("source function marker must exist")..]
}

fn method_source<'a>(source: &'a str, marker: &str) -> &'a str {
    let tail = function_tail(source, marker);
    let after_marker = &tail[marker.len()..];
    let end = after_marker
        .find("\n    pub(crate) fn ")
        .map(|offset| marker.len() + offset)
        .unwrap_or(tail.len());
    &tail[..end]
}
