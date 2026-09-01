const RESOLVER: &str = include_str!("../capacity_future.rs");
const OWNERS: &str = include_str!("../capacity_future/owners.rs");
const CAPACITY: &str = include_str!("../capacity_future/capacity.rs");
const INSTRUMENT_HISTORY: &str = include_str!("../../compute_capacity_instruments/historical.rs");
const INSTRUMENT_READ: &str = include_str!("../../compute_capacity_instruments/read.rs");
const ALLOCATION_HISTORY: &str = include_str!("../../compute_delivery_allocations/historical.rs");
const ALLOCATION_READ: &str = include_str!("../../compute_delivery_allocations/read.rs");
const ALLOCATION_AUDIT: &str = include_str!("../../compute_delivery_allocations/read/audit.rs");
const OFFER_PUBLICATIONS: &str = include_str!("../../compute_offer_publications.rs");
const PRICE_SNAPSHOTS: &str = include_str!("../../compute_price_snapshot_registry.rs");
const ATTEMPT_EXECUTION_RECEIPTS: &str = include_str!("../../compute_attempt_execution_receipts.rs");
const ATTEMPT_VERIFICATIONS: &str = include_str!("../../compute_attempt_verifications.rs");
const ATTEMPT_SETTLEMENTS: &str = include_str!("../../compute_attempt_settlements.rs");
const ATTEMPT_SETTLEMENT_SUPPORT: &str =
    include_str!("../../compute_attempt_settlements/support.rs");
const ATTEMPT_RELEASES: &str = include_str!("../../compute_attempt_settlement_releases.rs");
const ATTEMPT_RELEASE_SUPPORT: &str =
    include_str!("../../compute_attempt_settlement_releases/support.rs");
const EXECUTION_RESOLVER: &str = include_str!("../execution.rs");
const VERIFICATION_RESOLVER: &str = include_str!("../verification.rs");
const SETTLEMENT_RESOLVER: &str = include_str!("../settlement.rs");
const RELEASE_RESOLVER: &str = include_str!("../release.rs");
const COMMITMENT_VALIDATION: &str =
    include_str!("../../compute_capacity_commitments/validation.rs");
const VERIFICATION_AUDIT: &str =
    include_str!("../../compute_attempt_verifications/support/audit.rs");
const SETTLEMENT_AUDIT: &str = include_str!("../../compute_attempt_settlements/support/audit.rs");
const AUTHORITY: &str = include_str!(
    "../../../../../docs/distributed-compute/capacity-future-settlement-lineage-authority.md"
);
const ACCEPTANCE: &str = include_str!(
    "../../../../../docs/distributed-compute/capacity-future-settlement-lineage-acceptance.md"
);

#[test]
fn capacity_future_retained_resolver_is_lease_rooted_and_single_snapshot() {
    let facade = section(
        RESOLVER,
        "pub(crate) fn resolve_compute_capacity_future_settlement_lineage_for_lease(",
        "pub(super) fn resolve_capacity_future_settlement_lineage_on(",
    );
    assert!(facade.contains("lease_id: &str"));
    assert!(!facade.contains("instrument_id:"));
    assert!(!facade.contains("settlement_receipt_id:"));
    assert_eq!(
        facade
            .matches("transaction_with_behavior(TransactionBehavior::Deferred)")
            .count(),
        1
    );
    assert!(facade.contains("resolve_capacity_future_settlement_lineage_on(&tx, lease_id)"));
    assert!(RESOLVER.contains("build_compute_capacity_future_settlement_lineage(&sources)"));
}

#[test]
fn capacity_future_seal_is_private_non_serde_and_scope_carrying() {
    let seal = section(
        RESOLVER,
        "mod owners;",
        "pub(crate) enum ComputeCapacityFutureSettlementLineageResolveError",
    );
    for field in [
        "canonical_json: String,",
        "lineage_digest: String,",
        "access_scope: FederationHistoricalLineageAccessScope,",
    ] {
        assert!(
            seal.lines().any(|line| line.trim() == field),
            "missing private sealed field {field}"
        );
    }
    for forbidden in ["#[derive", "Clone", "Serialize", "Deserialize"] {
        assert!(!seal.contains(forbidden), "seal leaked {forbidden}");
    }
    for forbidden in ["Clone for", "Serialize for", "Deserialize for"] {
        assert!(!RESOLVER.contains(&format!(
            "{forbidden} ValidatedComputeCapacityFutureSettlementLineageV1"
        )));
    }
    let constructor = section(
        RESOLVER,
        "impl ValidatedComputeCapacityFutureSettlementLineageV1 {",
        "pub(crate) fn canonical_json(&self)",
    );
    assert!(constructor.contains("\n    fn from_projected("));
    assert!(RESOLVER.contains("permits_user(&self, user_id: &str)"));
    assert!(RESOLVER.contains("belongs_to_project(&self, project_id: &str)"));
}

#[test]
fn owners_are_rebuilt_from_historical_store_authorities() {
    for marker in [
        "compute_attempt_historical_settlement_by_lease_on(conn, lease_id)",
        "registered_historical_price_snapshot_on(",
        "compute_attempt_historical_execution_receipt_by_lease_on(conn, lease_id)",
        "resolve_execution_source_lineage_on(",
        "resolve_execution_verification_source_lineage_on(conn, &execution_receipt)",
        "resolve_settlement_source_lineage_on(",
        "compute_attempt_historical_settlement_release_by_lease_on(conn, lease_id)",
        "audited_historical_delivery_allocation_settlement_source_on(",
        "audited_historical_capacity_instrument_settlement_source_on(",
    ] {
        assert!(
            OWNERS.contains(marker),
            "missing retained owner seam {marker}"
        );
    }
    assert!(OWNERS.contains("price_snapshot.pricing_mode != PRICING_MODE_CAPACITY_FUTURE"));
    assert!(OWNERS.contains("return Ok(None)"));
    assert!(OWNERS.contains("settlement_scope.ensure_same_as(&execution_scope)"));
    assert!(OWNERS.contains("settlement_scope.ensure_same_as(&verification_scope)"));
}

#[test]
fn retired_instrument_and_exercised_allocation_use_historical_readers() {
    for marker in [
        "instrument_by_id_on(conn, instrument_id)",
        "activation_by_instrument_on(conn, instrument_id)",
        "historical_adoption_by_exact_offer_on(",
        "audited_historical_compute_offer_publication_on(conn, offer_id)",
    ] {
        assert!(INSTRUMENT_HISTORY.contains(marker));
    }
    for forbidden in [
        "currentness_on",
        "require_current_capacity_instrument_adoption_on",
        "adoption_by_offer_on(conn, offer_id)",
    ] {
        assert!(!INSTRUMENT_HISTORY.contains(forbidden));
    }
    let exact_adoption = section(
        INSTRUMENT_READ,
        "pub(super) fn historical_adoption_by_exact_offer_on(",
        "pub(super) fn adoption_by_idempotency_on(",
    );
    assert!(exact_adoption.contains("SELECT COUNT(*)"));
    assert!(exact_adoption.contains("WHERE offer_id=?1 AND offer_version=?2 AND offer_digest=?3"));
    assert!(ALLOCATION_HISTORY
        .contains("persisted_historical_delivery_allocation_reservation_authority_on("));
    assert!(ALLOCATION_HISTORY.contains("raw_terminal_by_grant_on(conn, &grant)"));
    assert!(ALLOCATION_HISTORY
        .contains("audit_historical_exercise_consumers_on(conn, &grant, &terminal)"));
    let exercise_audit = section(
        ALLOCATION_AUDIT,
        "pub(in crate::store::compute_delivery_allocations) fn audit_historical_exercise_consumers_on(",
        "pub(super) fn validate_non_exercise_terminal(",
    );
    for marker in [
        "audit_exercise_consumers_with_dependency_policy_on(conn, grant, terminal, true)",
        "registered_historical_reservation_version_on(",
        "registered_historical_job_version_on(",
        "broker_reserve_binding_on(",
        "broker.capacity_claim.claim_revision != evidence.reservation_claim.claim_revision",
        "broker.source_job.job_id != grant.job.job_id",
        "broker.reserved_job.job_id != grant.job.job_id",
    ] {
        assert!(exercise_audit.contains(marker));
    }
    let historical_publication = section(
        OFFER_PUBLICATIONS,
        "pub(in crate::store) fn audited_historical_compute_offer_publication_on(",
        "#[derive(Debug, Clone)]",
    );
    assert!(historical_publication
        .contains("audit_publication_with_offer_policy_on(conn, &stored, true)"));
    assert!(OFFER_PUBLICATIONS.contains("registered_historical_offer_version_on("));
}

#[test]
fn capacity_and_usage_equations_reuse_native_owner_formulas() {
    assert!(CAPACITY.contains("validate_contract_multiple(&quantities"));
    assert!(CAPACITY.contains("line.bucket.quantum_units != unit.unit_size"));
    assert!(CAPACITY.contains("parent.lines == parent_result.lines"));
    assert!(CAPACITY.contains("parent.lines == child.lines"));
    assert!(COMMITMENT_VALIDATION.contains("pub(in crate::store) fn validate_contract_multiple("));
    assert!(
        VERIFICATION_AUDIT.contains("verification_usage_digest(\"verified\", &expected_verified)")
    );
    assert!(SETTLEMENT_AUDIT.contains("calculate_settlement(&snapshot, &execution.receipt"));
    assert!(!OWNERS.contains("verified_usage_digest =="));
    assert!(!OWNERS.contains("compensable_usage_digest =="));
}

#[test]
fn capacity_future_historical_owner_chain_reaches_native_audits() {
    for marker in [
        "SELECT COUNT(*), MIN(grant_id), MIN(reservation_claim_id)",
        "WHERE terminal_status='exercised' AND reservation_id=?1",
        "owner_count != 1",
        "stored_claim_id != claim_id",
    ] {
        assert!(
            ALLOCATION_READ.contains(marker),
            "missing v228 owner fence {marker}"
        );
    }
    for (source, marker) in [
        (
            PRICE_SNAPSHOTS,
            "audited_price_snapshot_with_offer_policy_on(conn, &stored, true)",
        ),
        (
            ATTEMPT_EXECUTION_RECEIPTS,
            "execution_receipt_historical_envelope_on(conn, stored)",
        ),
        (
            ATTEMPT_VERIFICATIONS,
            "verification_decision_receipt_with_source_policy_on(conn, stored, false, true)",
        ),
        (ATTEMPT_SETTLEMENTS, "stored.into_historical_receipt(conn)"),
        (
            ATTEMPT_SETTLEMENT_SUPPORT,
            "audit::audited_historical_settlement_on(conn, self)",
        ),
        (ATTEMPT_RELEASES, "stored.into_historical_receipt(conn)"),
        (
            ATTEMPT_RELEASE_SUPPORT,
            "audit::audited_historical_release_on(conn, self)",
        ),
        (
            EXECUTION_RESOLVER,
            "compute_attempt_execution_receipt_by_id_on(conn, execution_receipt_id)",
        ),
        (
            VERIFICATION_RESOLVER,
            "compute_attempt_historical_verification_decision_on(conn, lease_id)",
        ),
        (
            SETTLEMENT_RESOLVER,
            "compute_attempt_settlement_by_receipt_id_on(conn, settlement_receipt_id)",
        ),
        (
            RELEASE_RESOLVER,
            "compute_attempt_historical_settlement_by_lease_on(conn, &release.lease_id)",
        ),
    ] {
        assert!(source.contains(marker), "historical owner chain lost {marker}");
    }
    assert!(VERIFICATION_AUDIT.contains("audit_verification_decision("));
    assert!(VERIFICATION_AUDIT
        .contains("verification_usage_digest(\"verified\", &expected_verified)"));
    assert!(SETTLEMENT_AUDIT.contains("audited_historical_settlement_on("));
    assert!(SETTLEMENT_AUDIT.contains("calculate_settlement(&snapshot, &execution.receipt"));
    assert!(SETTLEMENT_AUDIT.contains(".settlement_account_id"));
    assert!(SETTLEMENT_AUDIT
        .contains(".unwrap_or(provider.provider.owner_account_id.as_str())"));
    assert!(SETTLEMENT_AUDIT
        .contains("receipt.settlement.provider_account_id != provider_account_id"));
    for source in [
        EXECUTION_RESOLVER,
        VERIFICATION_RESOLVER,
        SETTLEMENT_RESOLVER,
        RELEASE_RESOLVER,
    ] {
        assert_static_read_only(source);
    }
}

#[test]
fn retained_bridge_remains_unrun_and_read_only() {
    let exact_adoption = section(
        INSTRUMENT_READ,
        "pub(super) fn historical_adoption_by_exact_offer_on(",
        "pub(super) fn adoption_by_idempotency_on(",
    );
    let historical_consumer = section(
        ALLOCATION_AUDIT,
        "pub(in crate::store::compute_delivery_allocations) fn audit_historical_exercise_consumers_on(",
        "pub(super) fn validate_non_exercise_terminal(",
    );
    for source in [
        RESOLVER,
        OWNERS,
        CAPACITY,
        INSTRUMENT_HISTORY,
        ALLOCATION_HISTORY,
        exact_adoption,
        historical_consumer,
    ] {
        assert_static_read_only(source);
    }
    assert_only_allowlisted_on_helpers(
        RESOLVER,
        &[
            "resolve_capacity_future_settlement_lineage_on",
            "resolve_capacity_future_settlement_owners_on",
        ],
    );
    assert_only_allowlisted_on_helpers(
        OWNERS,
        &[
            "audited_historical_capacity_instrument_settlement_source_on",
            "audited_historical_delivery_allocation_settlement_source_on",
            "compute_attempt_historical_execution_receipt_by_lease_on",
            "compute_attempt_historical_settlement_by_lease_on",
            "compute_attempt_historical_settlement_release_by_lease_on",
            "registered_historical_price_snapshot_on",
            "resolve_capacity_future_settlement_owners_on",
            "resolve_execution_source_lineage_on",
            "resolve_execution_verification_source_lineage_on",
            "resolve_settlement_release_source_lineage_on",
            "resolve_settlement_source_lineage_on",
        ],
    );
    assert_only_allowlisted_on_helpers(
        INSTRUMENT_HISTORY,
        &[
            "activation_by_instrument_on",
            "audited_historical_capacity_instrument_settlement_source_on",
            "audited_historical_compute_offer_publication_on",
            "historical_adoption_by_exact_offer_on",
            "instrument_by_id_on",
        ],
    );
    assert_only_allowlisted_on_helpers(
        ALLOCATION_HISTORY,
        &[
            "audit_historical_exercise_consumers_on",
            "audited_historical_delivery_allocation_settlement_source_on",
            "persisted_historical_delivery_allocation_reservation_authority_on",
            "raw_terminal_by_grant_on",
        ],
    );
    assert_only_allowlisted_on_helpers(
        exact_adoption,
        &["adoption_on", "historical_adoption_by_exact_offer_on"],
    );
    assert_only_allowlisted_on_helpers(
        historical_consumer,
        &[
            "audit_exercise_consumers_with_dependency_policy_on",
            "audit_historical_exercise_consumers_on",
            "broker_reserve_binding_on",
            "historical_reservation_authority_from_terminal_on",
            "registered_historical_job_version_on",
            "registered_historical_reservation_version_on",
            "registered_job_version_on",
            "registered_reservation_version_on",
            "reservation_authority_from_terminal_on",
        ],
    );
    for marker in [
        "store_resolver_source_written",
        "implementation_uncompiled",
        "implementation_unrun",
        "passed=0",
        "failed=0",
    ] {
        assert!(
            AUTHORITY.contains(marker) || ACCEPTANCE.contains(marker),
            "missing evidence boundary {marker}"
        );
    }
}

fn assert_static_read_only(source: &str) {
    let normalized = source
        .to_ascii_uppercase()
        .split_ascii_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for forbidden in [
        "INSERT INTO",
        "UPDATE ",
        "DELETE FROM",
        "REPLACE INTO",
        "CREATE TABLE",
        "ALTER TABLE",
        "DROP TABLE",
    ] {
        assert!(
            !normalized.contains(forbidden),
            "unexpected SQL effect {forbidden}"
        );
    }
    for forbidden in [
        "execute(",
        "execute_batch(",
        "insert_",
        "update_",
        "delete_",
        "upsert_",
        "persist_",
        "write_",
        "TransactionBehavior::Immediate",
        "TransactionBehavior::Exclusive",
    ] {
        assert!(
            !source.contains(forbidden),
            "unexpected effect API {forbidden}"
        );
    }
    for forbidden in ["_on ", "_on\n", "_on\r", "_on\t"] {
        assert!(
            !source.contains(forbidden),
            "non-canonical helper call {forbidden:?}"
        );
    }
}

fn assert_only_allowlisted_on_helpers(source: &str, allowed: &[&str]) {
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find("_on(") {
        let marker_start = cursor + relative;
        let name_end = marker_start + 3;
        let bytes = source.as_bytes();
        let mut name_start = marker_start;
        while name_start > 0
            && (bytes[name_start - 1].is_ascii_alphanumeric() || bytes[name_start - 1] == b'_')
        {
            name_start -= 1;
        }
        let name = &source[name_start..name_end];
        assert!(
            allowed.contains(&name),
            "non-allowlisted retained helper {name}"
        );
        cursor = name_end + 1;
    }
}

fn section<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start = source.find(start).expect("section start");
    let rest = &source[start..];
    let end = rest.find(end).expect("section end");
    &rest[..end]
}
