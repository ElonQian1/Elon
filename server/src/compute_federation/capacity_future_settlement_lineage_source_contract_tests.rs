const DOMAIN_ROOT: &str = include_str!("capacity_future_settlement_lineage.rs");
const DOMAIN_TYPES: &str = include_str!("capacity_future_settlement_lineage/types.rs");
const DOMAIN_CANONICAL: &str = include_str!("capacity_future_settlement_lineage/canonical.rs");
const DOMAIN_VALIDATION: &str = include_str!("capacity_future_settlement_lineage/validation.rs");
const SOURCE_EQUATIONS: &str =
    include_str!("capacity_future_settlement_lineage/source_equations.rs");
const SETTLEMENT_EQUATIONS: &str =
    include_str!("capacity_future_settlement_lineage/settlement_equations.rs");
const SOURCE_INPUTS: &str = include_str!("capacity_future_settlement_lineage/source_inputs.rs");
const SOURCE_SUPPORT: &str = include_str!("capacity_future_settlement_lineage/source_support.rs");
const AUTHORITY: &str = include_str!(
    "../../../docs/distributed-compute/capacity-future-settlement-lineage-authority.md"
);
const ACCEPTANCE: &str = include_str!(
    "../../../docs/distributed-compute/capacity-future-settlement-lineage-acceptance.md"
);

#[test]
fn capacity_future_bridge_has_an_independent_reference_only_digest_domain() {
    for marker in [
        "compute_federation.capacity_future_settlement_lineage_bridge.v1",
        "capacity_future_settlement_bridge_v1",
        "ELON-COMPUTE-CAPACITY-FUTURE-SETTLEMENT-LINEAGE-BRIDGE-V1",
        "rfc8785_jcs",
        "sha256",
        "retained_references_only",
    ] {
        assert!(DOMAIN_TYPES.contains(marker), "missing ABI marker {marker}");
    }
    assert!(DOMAIN_CANONICAL.contains("digest.update([0])"));
    assert!(DOMAIN_CANONICAL.contains("lineage_digest"));
    assert!(DOMAIN_CANONICAL.contains("canonical_json(&envelope)? == value"));
    assert!(DOMAIN_ROOT.contains("does not create capacity"));
    assert!(DOMAIN_ROOT.contains("verify usage"));
}

#[test]
fn capacity_future_bridge_reuses_core_refs_without_extending_f0_profiles() {
    for marker in [
        "AttemptSettlementRef",
        "CapacityClaimVersionRef",
        "ExecutionReceiptRef",
        "JobVersionRef",
        "OfferVersionRef",
        "PriceSnapshotRef",
        "ReservationVersionRef",
        "SettlementReleaseRef",
        "VerificationDecisionRef",
    ] {
        assert!(DOMAIN_TYPES.contains(marker), "missing reused ref {marker}");
    }
    for forbidden in [
        "struct ProviderVersionRef",
        "struct ComputePriceSnapshot",
        "struct ComputeExecutionReceipt",
        "struct ComputeSettlementReceipt",
        "struct ComputeAttemptLease",
        "FederationHistoricalLineageKindV1",
    ] {
        assert!(
            !DOMAIN_TYPES.contains(forbidden),
            "duplicated core ABI {forbidden}"
        );
    }
    assert!(!SOURCE_EQUATIONS.contains("build_execution_source_carrier"));
    assert!(!SOURCE_EQUATIONS.contains("build_settlement_source_carrier"));
}

#[test]
fn economic_history_is_a_closed_non_nullable_choice() {
    for marker in [
        "PendingSettlementSourceV1",
        "AvailableReleaseSourceV1",
        "economic_stage",
        "settlement_release_lineage_digest",
    ] {
        assert!(DOMAIN_TYPES.contains(marker));
    }
    assert!(!DOMAIN_TYPES.contains("Option<"));
    assert!(SOURCE_SUPPORT.contains("PendingSettlementSource { .. } => None"));
    assert!(SOURCE_SUPPORT.contains("requires settlement_release_source_v1"));
    assert!(AUTHORITY.contains("不得从 pending 分支缺少 release 推导当前仍为 pending"));
}

#[test]
fn source_equations_keep_v192_and_v195_usage_digest_domains_distinct() {
    for marker in [
        "validate_compute_capacity_instrument(",
        "validate_compute_capacity_instrument_activation_receipt(",
        "validate_compute_capacity_instrument_offer_adoption_receipt(",
        "capacity_commitment",
        "delivery_allocation_grant",
        "delivery_allocation_exercise",
        "execution_source.lineage_digest()",
        "execution_verification_source",
        "execution_receipt.receipt_id",
        "execution_receipt.verification.status",
        "execution_receipt.verification.decision_digest",
        "verification_event_digest",
        "settlement_usage_digests",
        "verified_usage_digest: sources",
        "compensable_usage_digest: sources",
        "settlement_source_carrier(sources).lineage_digest()",
    ] {
        assert!(
            SOURCE_EQUATIONS.contains(marker) || SETTLEMENT_EQUATIONS.contains(marker),
            "missing source equation {marker}"
        );
    }
    for marker in [
        "UntrustedCapacityFutureAttemptSettlementAuditView",
        "settlement_event_digest",
        "budget_reservation_id",
        "source_job",
        "terminal_job",
    ] {
        assert!(
            SOURCE_INPUTS.contains(marker),
            "missing v195 audit input {marker}"
        );
    }
    for marker in [
        "pricing_mode == PRICING_MODE_CAPACITY_FUTURE",
        "delivery_window == sources.instrument.delivery_window",
        "exercise.parent_claim_id == commitment.claim.claim_id",
        "execution.capacity_claim.claim_id == exercise.reservation_claim.claim_id",
        "settlement.terminal_reservation.reservation_id",
    ] {
        assert!(
            SOURCE_EQUATIONS.contains(marker) || SETTLEMENT_EQUATIONS.contains(marker),
            "missing root fence {marker}"
        );
    }
    for source in [SOURCE_EQUATIONS, SETTLEMENT_EQUATIONS] {
        assert!(!source.contains("verification.verification_decision.verified_usage_digest"));
        assert!(!source.contains("verification.verification_decision.compensable_usage_digest"));
    }
}

#[test]
fn bridge_keeps_all_runtime_and_economic_effects_closed() {
    for marker in [
        "capacity_effect",
        "verification_effect",
        "settlement_effect",
        "money_effect",
        "withdrawal_effect",
    ] {
        assert!(DOMAIN_TYPES.contains(marker));
        assert!(DOMAIN_VALIDATION.contains(marker));
    }
    for forbidden in [
        "rusqlite",
        "axum",
        "Store",
        "migration",
        "INSERT INTO",
        "UPDATE ",
        "DELETE FROM",
        "withdrawn",
        "external_paid",
    ] {
        assert!(!DOMAIN_ROOT.contains(forbidden));
        assert!(!DOMAIN_TYPES.contains(forbidden));
        assert!(!SOURCE_EQUATIONS.contains(forbidden));
        assert!(!SETTLEMENT_EQUATIONS.contains(forbidden));
        assert!(!SOURCE_INPUTS.contains(forbidden));
    }
    for marker in [
        "implementation_uncompiled",
        "implementation_unrun",
        "passed=0",
        "failed=0",
        "migration/table/writer=none/none/none",
    ] {
        assert!(
            ACCEPTANCE.contains(marker),
            "missing evidence boundary {marker}"
        );
    }
}
