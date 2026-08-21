use crate::compute_federation::federation_historical_causal_reference::{
    AttemptSettlementRef, SettlementReleaseGateV1, SettlementReleasePostingRef,
    SettlementReleaseRef, SettlementReleaseSourceLineageV1, SettlementSourcePostingRef,
};

use super::super::release_refs::{
    validate_settlement_release_source_links, SettlementReleaseSourceLinkFacts,
};

const STORE_FACADE: &str = include_str!("../../compute_federation_historical_causal_reference.rs");
const RELEASE_RESOLVER: &str = include_str!("../release.rs");
const RELEASE_REFS: &str = include_str!("../release_refs.rs");
const V195_AUDIT: &str = include_str!("../../compute_attempt_settlements/support/audit.rs");
const V196_OWNER: &str = include_str!("../../compute_attempt_settlement_challenges.rs");
const V196_SUPPORT: &str = include_str!("../../compute_attempt_settlement_challenges/support.rs");
const V196_AUDIT: &str =
    include_str!("../../compute_attempt_settlement_challenges/support/audit.rs");
const V197_OWNER: &str = include_str!("../../compute_attempt_settlement_challenge_resolutions.rs");
const V197_SUPPORT: &str =
    include_str!("../../compute_attempt_settlement_challenge_resolutions/support.rs");
const V197_AUDIT: &str =
    include_str!("../../compute_attempt_settlement_challenge_resolutions/support/audit.rs");
const V199_OWNER: &str = include_str!("../../compute_attempt_settlement_corrections.rs");
const V199_SUPPORT: &str = include_str!("../../compute_attempt_settlement_corrections/support.rs");
const V199_AUDIT: &str =
    include_str!("../../compute_attempt_settlement_corrections/support/audit.rs");
const V198_OWNER: &str = include_str!("../../compute_attempt_settlement_releases.rs");
const V198_SUPPORT: &str = include_str!("../../compute_attempt_settlement_releases/support.rs");
const V198_AUDIT: &str = include_str!("../../compute_attempt_settlement_releases/support/audit.rs");
const V198_HISTORICAL: &str =
    include_str!("../../compute_attempt_settlement_releases/historical.rs");
const V196_MIGRATION: &str = include_str!("../../../compute_settlement_challenge_migration.rs");
const V197_MIGRATION: &str =
    include_str!("../../../compute_settlement_challenge_resolution_migration.rs");
const V199_MIGRATION: &str = include_str!("../../../compute_settlement_correction_migration.rs");
const V198_MIGRATION: &str = include_str!("../../../compute_settlement_release_migration.rs");

#[test]
fn release_resolver_is_one_deferred_by_lease_historical_chain() {
    let facade = item_source(
        STORE_FACADE,
        "fn resolve_compute_settlement_release_source_lineage_for_lease(",
    );
    assert!(facade.contains("transaction_with_behavior(TransactionBehavior::Deferred)"));
    assert!(
        facade.contains("compute_attempt_historical_settlement_release_by_lease_on(&tx, lease_id)")
    );
    assert!(facade.contains("release::resolve_settlement_release_source_lineage_on(&tx, &release)"));

    for required in [
        "compute_attempt_historical_settlement_by_lease_on",
        "resolve_settlement_source_lineage_on",
        "into_lineage_digest_and_access_scope",
        "compute_attempt_historical_settlement_challenge_by_lease_on",
        "compute_attempt_historical_settlement_challenge_resolution_by_challenge_on",
        "compute_attempt_historical_settlement_correction_by_resolution_on",
        "SettlementReleaseGateV1::NoChallenge",
        "SettlementReleaseGateV1::ResolvedChallenge",
        "SettlementReleaseGateV1::AcceptedCorrected",
        "resolution_action: settlement_challenge_resolution_action(&resolution.action)?",
        "settlement_correction_posting_ref",
    ] {
        assert!(
            RELEASE_RESOLVER.contains(required),
            "missing release proof: {required}"
        );
    }
    for forbidden in [
        "compute_attempt_settlement_on(",
        "compute_settlement_challenge_on(",
        "compute_settlement_challenge_resolution_on(",
        "compute_settlement_correction_on(",
        "compute_settlement_release_on(",
        "compute_settlement_release_optional_on(",
        "current_",
        "latest_",
        "TransactionBehavior::Immediate",
        "INSERT INTO",
        "UPDATE ",
        "DELETE FROM",
    ] {
        assert!(
            !RELEASE_RESOLVER.contains(forbidden),
            "release resolver must not use current/write fallback: {forbidden}"
        );
    }
    for abi_field in [
        "settlement_challenge_id: challenge_id.to_string()",
        "settlement_challenge_event_digest: event_digest.to_string()",
        "settlement_challenge_resolution_id: resolution_id.to_string()",
        "settlement_challenge_resolution_event_digest: event_digest.to_string()",
        "settlement_correction_id: correction_id.to_string()",
        "settlement_correction_event_digest: event_digest.to_string()",
    ] {
        assert!(
            RELEASE_REFS.contains(abi_field),
            "missing stable ABI field: {abi_field}"
        );
    }
    for stale_abi in [
        "SettlementChallengeResolutionActionV1::Accepted",
        "        challenge_id: challenge_id.to_string()",
        "        challenge_event_digest: event_digest.to_string()",
        "        resolution_id: resolution_id.to_string()",
        "        resolution_event_digest: event_digest.to_string()",
        "        correction_id: correction_id.to_string()",
        "        correction_event_digest: event_digest.to_string()",
    ] {
        assert!(
            !RELEASE_REFS.contains(stale_abi),
            "stale settlement release ABI mapping survived: {stale_abi}"
        );
    }
}

#[test]
fn v195_through_v199_and_v198_have_distinct_historical_audit_seams() {
    for (owner, declaration) in [
        (
            V196_OWNER,
            "fn compute_attempt_historical_settlement_challenge_by_lease_on(",
        ),
        (
            V197_OWNER,
            "fn compute_attempt_historical_settlement_challenge_resolution_by_challenge_on(",
        ),
        (
            V199_OWNER,
            "fn compute_attempt_historical_settlement_correction_by_resolution_on(",
        ),
        (
            V198_OWNER,
            "fn compute_attempt_historical_settlement_release_by_lease_on(",
        ),
    ] {
        assert!(
            owner.contains(declaration),
            "missing owner seam: {declaration}"
        );
    }
    for (support, exact_query, migration, exact_one_fence) in [
        (
            V196_SUPPORT,
            "challenge_query(conn, \"WHERE lease_id=?1\", params![lease_id])",
            V196_MIGRATION,
            "lease_id                        TEXT NOT NULL UNIQUE",
        ),
        (
            V197_SUPPORT,
            "resolution_query(conn, \"WHERE challenge_id=?1\", params![challenge_id])",
            V197_MIGRATION,
            "challenge_id                   TEXT NOT NULL UNIQUE",
        ),
        (
            V199_SUPPORT,
            "correction_query(conn, \"WHERE resolution_id=?1\", params![resolution_id])",
            V199_MIGRATION,
            "resolution_id                       TEXT NOT NULL UNIQUE",
        ),
        (
            V198_SUPPORT,
            "release_query(conn, \"WHERE lease_id=?1\", params![lease_id])",
            V198_MIGRATION,
            "lease_id                            TEXT NOT NULL UNIQUE",
        ),
    ] {
        assert!(
            support.contains(exact_query),
            "missing exact owner query: {exact_query}"
        );
        assert!(
            migration.contains(exact_one_fence),
            "missing exact-one owner fence: {exact_one_fence}"
        );
    }
    for (audit, wrapper, policy_call) in [
        (
            V196_AUDIT,
            "fn audited_historical_challenge_on(",
            "audited_challenge_with_head_policy_on(conn, stored, false, false)",
        ),
        (
            V197_AUDIT,
            "fn audited_historical_resolution_on(",
            "audited_resolution_with_head_policy_on(conn, stored, false, false)",
        ),
        (
            V199_AUDIT,
            "fn audited_historical_correction_on(",
            "audited_correction_with_head_policy_on(conn, stored, false, false)",
        ),
        (
            V198_AUDIT,
            "fn audited_historical_release_on(",
            "audited_release_with_head_policy_on(conn, stored, false, false)",
        ),
    ] {
        assert!(audit.contains(wrapper));
        assert!(audit.contains(policy_call));
    }
    assert!(V195_AUDIT.contains("audit_posting(conn, &receipt, require_current_heads)?"));
    assert!(V195_AUDIT.contains("if require_current_heads"));
    assert!(V199_AUDIT.contains("if require_current_heads"));
    assert!(V198_AUDIT.contains("if require_current_heads"));
    let raw_owner_contracts: [(&str, &[&str]); 4] = [
        (
            V196_AUDIT,
            &[
                "stored.request_json != serde_json::to_string(&request)?",
                "stored.evidence_refs_json != serde_json::to_string(&receipt.evidence_refs)?",
                "stored.receipt_json != serde_json::to_string(&receipt)?",
            ],
        ),
        (
            V197_AUDIT,
            &[
                "stored.request_json != serde_json::to_string(&request)?",
                "stored.receipt_json != serde_json::to_string(&receipt)?",
            ],
        ),
        (
            V199_AUDIT,
            &[
                "stored.request_json != serde_json::to_string(&request)?",
                "stored.evidence_refs_json != serde_json::to_string(&receipt.evidence_refs)?",
                "stored.receipt_json != serde_json::to_string(&receipt)?",
            ],
        ),
        (
            V198_AUDIT,
            &[
                "stored.request_json != serde_json::to_string(&request)?",
                "stored.challenge_gate_json != serde_json::to_string(&receipt.challenge_gate)?",
                "stored.receipt_json != serde_json::to_string(&receipt)?",
            ],
        ),
    ];
    for (audit, raw_owner_projection) in raw_owner_contracts {
        for projection in raw_owner_projection {
            assert!(
                audit.contains(projection),
                "missing raw owner audit: {projection}"
            );
        }
    }

    for required in [
        "compute_attempt_historical_settlement_challenge_by_lease_on",
        "compute_attempt_historical_settlement_challenge_resolution_by_challenge_on",
        "compute_attempt_historical_settlement_correction_by_resolution_on",
        "historical_release_amounts",
    ] {
        assert!(V198_HISTORICAL.contains(required));
    }
    for forbidden in [
        "settlement_challenge_gate_on",
        "compute_settlement_account_balances",
        "compute_settlement_release_on",
        "current_",
        "latest_",
    ] {
        assert!(!V198_HISTORICAL.contains(forbidden));
    }
}

#[test]
fn release_link_equations_reject_cross_owner_splices() {
    let facts = release_facts();
    validate_settlement_release_source_links(&facts).unwrap();

    let mut settlement = facts.clone();
    settlement
        .audited_attempt_settlement
        .settlement_event_digest = "settlement-event-b".into();
    let mut lineage_digest = facts.clone();
    lineage_digest.rebuilt_settlement_lineage_digest = "settlement-lineage-b".into();
    let mut source_posting = facts.clone();
    source_posting
        .audited_source_settlement_posting
        .settlement_posting_id = "settlement-posting-b".into();
    let mut gate = facts.clone();
    gate.audited_release_gate = SettlementReleaseGateV1::NoChallenge {
        challenge_gate_digest: "gate-b".into(),
    };
    let mut release = facts.clone();
    release.audited_settlement_release.settlement_release_id = "release-b".into();
    let mut release_posting = facts.clone();
    release_posting
        .audited_release_posting
        .settlement_release_posting_id = "release-posting-b".into();
    let mut lease = facts.clone();
    lease.release_lease_id = "lease-b".into();
    let mut consumer = facts.clone();
    consumer.release_consumer_account_id = "consumer-b".into();
    let mut provider = facts.clone();
    provider.release_provider_account_id = "provider-b".into();

    for (case, drifted) in [
        ("settlement", settlement),
        ("settlement_lineage", lineage_digest),
        ("source_posting", source_posting),
        ("gate", gate),
        ("release", release),
        ("release_posting", release_posting),
        ("lease", lease),
        ("consumer", consumer),
        ("provider", provider),
    ] {
        assert!(
            validate_settlement_release_source_links(&drifted).is_err(),
            "release splice must fail closed: {case}"
        );
    }
    assert!(RELEASE_REFS.contains("lineage.release_gate != facts.audited_release_gate"));
}

fn release_facts() -> SettlementReleaseSourceLinkFacts {
    let attempt_settlement = AttemptSettlementRef {
        settlement_receipt_id: "settlement-a".into(),
        settlement_receipt_digest: "settlement-digest-a".into(),
        settlement_event_digest: "settlement-event-a".into(),
    };
    let source_posting = SettlementSourcePostingRef {
        settlement_posting_id: "settlement-posting-a".into(),
        settlement_posting_digest: "settlement-posting-digest-a".into(),
    };
    let gate = SettlementReleaseGateV1::NoChallenge {
        challenge_gate_digest: "gate-a".into(),
    };
    let release = SettlementReleaseRef {
        settlement_release_id: "release-a".into(),
        settlement_release_event_digest: "release-event-a".into(),
    };
    let release_posting = SettlementReleasePostingRef {
        settlement_release_posting_id: "release-posting-a".into(),
        settlement_release_posting_digest: "release-posting-digest-a".into(),
    };
    let lineage = SettlementReleaseSourceLineageV1 {
        attempt_settlement: attempt_settlement.clone(),
        settlement_lineage_digest: "settlement-lineage-a".into(),
        source_settlement_posting: source_posting.clone(),
        release_gate: gate.clone(),
        settlement_release: release.clone(),
        release_posting: release_posting.clone(),
    };
    SettlementReleaseSourceLinkFacts {
        lineage,
        audited_attempt_settlement: attempt_settlement,
        rebuilt_settlement_lineage_digest: "settlement-lineage-a".into(),
        audited_source_settlement_posting: source_posting,
        audited_release_gate: gate,
        audited_settlement_release: release,
        audited_release_posting: release_posting,
        settlement_lease_id: "lease-a".into(),
        release_lease_id: "lease-a".into(),
        settlement_consumer_account_id: "consumer-a".into(),
        release_consumer_account_id: "consumer-a".into(),
        settlement_provider_account_id: "provider-a".into(),
        release_provider_account_id: "provider-a".into(),
    }
}

fn item_source<'a>(source: &'a str, marker: &str) -> &'a str {
    let tail = &source[source.find(marker).expect("source item marker must exist")..];
    let end = tail
        .find("\n}")
        .expect("source item must end at module indentation")
        + 2;
    &tail[..end]
}
