use super::{
    ATTEMPT_FINALIZATION_AUDIT, ATTEMPT_SETTLEMENT_AUDIT, EXECUTION_RESOLVER, SETTLEMENT_RESOLVER,
};

const ATTEMPT_TERMINALS: &str = include_str!("../../compute_attempt_terminals.rs");
const ATTEMPT_TERMINAL_FINAL_USAGE: &str =
    include_str!("../../compute_attempt_terminals/final_usage.rs");
const ATTEMPT_CONSUMER_REVIEWS: &str = include_str!("../../compute_attempt_consumer_reviews.rs");
const ATTEMPT_PLATFORM_OBSERVATIONS: &str =
    include_str!("../../compute_attempt_platform_observations.rs");
const ATTEMPT_VERIFICATIONS: &str = include_str!("../../compute_attempt_verifications.rs");
const ATTEMPT_EXECUTION_RECEIPTS: &str =
    include_str!("../../compute_attempt_execution_receipts.rs");
const ATTEMPT_EXECUTION_RECEIPT_AUDIT: &str =
    include_str!("../../compute_attempt_execution_receipts/support/audit.rs");
const ATTEMPT_ACTIVATIONS: &str = include_str!("../../compute_attempt_activations.rs");
const ATTEMPT_ACTIVATION_ROWS: &str = include_str!("../../compute_attempt_activations/rows.rs");
const ATTEMPT_LEASES: &str = include_str!("../../compute_attempt_leases.rs");
const OFFER_REGISTRY: &str = include_str!("../../compute_offer_registry.rs");
const OFFER_CAPACITY: &str = include_str!("../../compute_offer_registry/capacity.rs");
const CAPACITY_ROWS: &str = include_str!("../../compute_capacity_rows.rs");
const CAPACITY_CLAIM_ROWS: &str = include_str!("../../compute_capacity_claim_rows.rs");
const PRICE_SNAPSHOT_REGISTRY: &str = include_str!("../../compute_price_snapshot_registry.rs");
const JOB_REGISTRY: &str = include_str!("../../compute_job_registry.rs");
const JOB_AUDIT: &str = include_str!("../../compute_job_registry/audit.rs");
const RESERVATION_REGISTRY: &str = include_str!("../../compute_reservation_registry.rs");
const RESERVATION_AUDIT: &str = include_str!("../../compute_reservation_registry/audit.rs");
const RESERVATION_DEPENDENCIES: &str =
    include_str!("../../compute_reservation_registry/dependencies.rs");
const DELIVERY_ALLOCATION_READ: &str = include_str!("../../compute_delivery_allocations/read.rs");
const DELIVERY_ALLOCATION_AUDIT: &str =
    include_str!("../../compute_delivery_allocations/read/audit.rs");
const CAPACITY_COMMITMENTS: &str = include_str!("../../compute_capacity_commitments.rs");
const CAPACITY_COMMITMENT_READ: &str = include_str!("../../compute_capacity_commitments/read.rs");

#[test]
fn v189_through_v193_historical_chain_uses_exact_retained_evidence() {
    assert!(ATTEMPT_TERMINALS.contains("fn compute_attempt_historical_terminal_candidate_on("));
    let historical_candidate = item_source(
        ATTEMPT_TERMINAL_FINAL_USAGE,
        "fn historical_terminal_candidate_receipt_on(",
    );
    assert!(historical_candidate.contains("compute_attempt_usage_declaration_on("));
    assert!(historical_candidate.contains("receipt.final_usage_sequence_no"));
    assert!(!historical_candidate.contains("latest_compute_attempt_usage_declaration_on"));

    assert!(ATTEMPT_CONSUMER_REVIEWS.contains("fn compute_attempt_historical_consumer_review_on("));
    assert!(ATTEMPT_CONSUMER_REVIEWS.contains("compute_attempt_historical_terminal_candidate_on"));
    assert!(ATTEMPT_PLATFORM_OBSERVATIONS
        .contains("fn compute_attempt_historical_platform_observation_on("));
    assert!(
        ATTEMPT_PLATFORM_OBSERVATIONS.contains("compute_attempt_historical_terminal_candidate_on")
    );
    assert!(
        ATTEMPT_VERIFICATIONS.contains("fn compute_attempt_historical_verification_decision_on(")
    );
    for historical_source in [
        "compute_attempt_historical_terminal_candidate_on",
        "compute_attempt_historical_consumer_review_on",
        "compute_attempt_historical_platform_observation_on",
        "registered_historical_reservation_version_on",
    ] {
        assert!(ATTEMPT_VERIFICATIONS.contains(historical_source));
    }

    let by_id = item_source(
        ATTEMPT_EXECUTION_RECEIPTS,
        "fn compute_attempt_execution_receipt_by_id_on(",
    );
    assert!(by_id.contains("execution_receipt_historical_envelope_on"));
    let historical_envelope = item_source(
        ATTEMPT_EXECUTION_RECEIPTS,
        "fn execution_receipt_historical_envelope_on(",
    );
    assert!(historical_envelope
        .contains("execution_receipt_envelope_with_source_policy_on(conn, stored, false, true)"));
    for historical_source in [
        "compute_attempt_historical_verification_decision_on",
        "compute_attempt_historical_terminal_candidate_on",
        "compute_attempt_historical_consumer_review_on",
        "compute_attempt_historical_platform_observation_on",
        "compute_attempt_historical_activation_sources_on",
        "registered_historical_job_version_on",
        "registered_historical_reservation_version_on",
    ] {
        assert!(ATTEMPT_EXECUTION_RECEIPTS.contains(historical_source));
    }
    assert!(ATTEMPT_EXECUTION_RECEIPTS.contains("candidate.final_usage_sequence_no"));
}

#[test]
fn historical_activation_and_source_lease_do_not_depend_on_mutable_balances_or_heads() {
    assert!(ATTEMPT_ACTIVATIONS.contains("struct HistoricalComputeAttemptActivationSources"));
    let historical_read = item_source(
        ATTEMPT_ACTIVATION_ROWS,
        "fn historical_attempt_activation_sources_on(",
    );
    assert!(historical_read.contains("audit_stored_activation_with_source_policy_on"));
    assert!(historical_read.contains("true"));
    assert!(historical_read.contains("into_historical_sources"));

    let projection = item_source(ATTEMPT_ACTIVATION_ROWS, "fn into_historical_sources(");
    assert!(!projection.contains("balances_for_transaction_on"));
    assert!(!projection.contains("current_balances"));
    assert!(ATTEMPT_ACTIVATION_ROWS.contains("fn into_receipt("));
    assert!(ATTEMPT_ACTIVATION_ROWS.contains("balances_for_transaction_on"));
    assert!(ATTEMPT_ACTIVATION_ROWS.contains("registered_historical_job_version_on"));
    assert!(ATTEMPT_ACTIVATION_ROWS.contains("registered_historical_reservation_version_on"));

    let lease = item_source(
        ATTEMPT_LEASES,
        "fn audited_compute_attempt_lease_version_on(",
    );
    assert!(lease.contains("compute_attempt_historical_activation_sources_on"));
    assert!(lease.contains("registered_historical_job_version_on"));
    assert!(!lease.contains("current_lease_state_on"));
}

#[test]
fn job_reservation_snapshot_offer_and_delivery_authorities_are_retained_only() {
    for resolver in [EXECUTION_RESOLVER, SETTLEMENT_RESOLVER] {
        for owner in [
            "registered_historical_job_version_on",
            "registered_historical_reservation_version_on",
            "registered_historical_price_snapshot_on",
            "registered_historical_offer_version_on",
        ] {
            assert!(resolver.contains(owner));
        }
    }

    let historical_offer =
        item_source(OFFER_REGISTRY, "fn registered_historical_offer_version_on(");
    assert!(historical_offer
        .contains("audited_offer_with_capacity_policy_on(conn, None, &stored, false)"));
    let ordinary_offer = item_source(OFFER_REGISTRY, "fn audited_offer_on(");
    assert!(ordinary_offer
        .contains("audited_offer_with_capacity_policy_on(conn, projection, stored, true)"));
    assert!(OFFER_CAPACITY.contains("ensure_offer_immutable_bucket_references_on"));
    let immutable_bucket = item_source(
        OFFER_CAPACITY,
        "fn ensure_offer_immutable_bucket_references_on(",
    );
    assert!(immutable_bucket.contains("stored_bucket_reference_on"));
    assert!(immutable_bucket.contains("stored.binding != capacity.bucket"));
    assert!(immutable_bucket.contains("stored.starts_at"));
    assert!(immutable_bucket.contains("stored.ends_at"));
    for mutable_fact in [
        "stored_bucket_on",
        ".balance",
        "issued_units",
        "available_units",
    ] {
        assert!(!immutable_bucket.contains(mutable_fact));
    }
    let immutable_bucket_row = item_source(CAPACITY_ROWS, "fn stored_bucket_reference_on(");
    for mutable_column in [
        "status",
        "issued_units",
        "available_units",
        "reserved_units",
    ] {
        assert!(!immutable_bucket_row.contains(mutable_column));
    }

    assert!(PRICE_SNAPSHOT_REGISTRY.contains("fn registered_historical_price_snapshot_on("));
    assert!(PRICE_SNAPSHOT_REGISTRY.contains("registered_historical_offer_version_on"));
    assert!(JOB_REGISTRY.contains("fn registered_historical_job_version_on("));
    assert!(JOB_AUDIT.contains("fn audited_historical_job_on("));
    assert!(JOB_REGISTRY.contains("registered_historical_offer_version_on"));
    assert!(JOB_REGISTRY.contains("registered_historical_price_snapshot_on"));
    let ordinary_reservation = item_source(
        RESERVATION_REGISTRY,
        "fn registered_reservation_version_on(",
    );
    assert!(ordinary_reservation.contains("audited_reservation_on(conn, None, &stored)"));
    let historical_reservation = item_source(
        RESERVATION_REGISTRY,
        "fn registered_historical_reservation_version_on(",
    );
    assert!(historical_reservation.contains("audited_historical_reservation_on(conn, &stored)"));
    assert!(RESERVATION_AUDIT.contains(
        "audited_reservation_with_dependency_policy_on(conn, projection, stored, None, false)"
    ));
    assert!(RESERVATION_AUDIT
        .contains("audited_reservation_with_dependency_policy_on(conn, None, stored, None, true)"));
    assert!(RESERVATION_AUDIT.contains("registered_historical_dependencies_on"));
    for historical_dependency in [
        "registered_historical_job_version_on",
        "registered_historical_offer_version_on",
        "registered_historical_price_snapshot_on",
        "persisted_historical_delivery_allocation_reservation_authority_on",
    ] {
        assert!(RESERVATION_DEPENDENCIES.contains(historical_dependency));
    }

    let historical_grant = item_source(DELIVERY_ALLOCATION_READ, "fn historical_grant_by_id_on(");
    assert!(
        historical_grant.contains("grant_by_id_with_dependency_policy_on(conn, grant_id, true)")
    );
    let historical_authority = item_source(
        DELIVERY_ALLOCATION_READ,
        "fn persisted_historical_delivery_allocation_reservation_authority_on(",
    );
    assert!(historical_authority.contains("historical_grant_by_id_on"));
    assert!(historical_authority.contains("historical_reservation_authority_from_terminal_on"));
    assert!(DELIVERY_ALLOCATION_AUDIT.contains("audit_historical_grant_dependencies_on"));
    assert!(DELIVERY_ALLOCATION_AUDIT.contains("audited_historical_capacity_commitment_source_on"));
    assert!(DELIVERY_ALLOCATION_AUDIT.contains("registered_historical_job_version_on"));
    assert!(DELIVERY_ALLOCATION_AUDIT.contains(
        "reservation_authority_from_terminal_with_parent_policy_on(conn, grant, terminal, false)"
    ));
    assert!(CAPACITY_COMMITMENTS.contains("fn audited_historical_capacity_commitment_source_on("));
    assert!(CAPACITY_COMMITMENTS.contains("audit_historical_immutable_dependencies_on"));
    assert!(CAPACITY_COMMITMENTS.contains("registered_historical_offer_version_on"));
    assert!(CAPACITY_COMMITMENTS.contains("registered_historical_price_snapshot_on"));
    assert!(CAPACITY_COMMITMENT_READ.contains("historical_commitment_by_id_on"));
    assert!(CAPACITY_COMMITMENT_READ
        .contains("commitment_by_id_with_dependency_policy_on(conn, commitment_id, true)"));
}

#[test]
fn v193_v194_v195_owner_reads_reject_raw_json_shape_drift() {
    assert!(OFFER_REGISTRY.contains("stored.offer_json != serde_json::to_string(&offer)?"));
    assert!(PRICE_SNAPSHOT_REGISTRY
        .contains("stored.snapshot_json != serde_json::to_string(&snapshot)?"));
    assert!(JOB_AUDIT.contains("stored.job_json != serde_json::to_string(&job)?"));
    assert!(RESERVATION_AUDIT
        .contains("stored.reservation_json != serde_json::to_string(&reservation)?"));
    assert!(CAPACITY_CLAIM_ROWS.contains("claim_json != serde_json::to_string(&claim)?"));
    assert!(ATTEMPT_EXECUTION_RECEIPT_AUDIT
        .contains("self.receipt_json != serde_json::to_string(&self.receipt)?"));
    assert!(ATTEMPT_FINALIZATION_AUDIT
        .contains("self.request_json != serde_json::to_string(&normalized)?"));
    assert!(ATTEMPT_FINALIZATION_AUDIT
        .contains("self.receipt_json != serde_json::to_string(&self.receipt)?"));
    assert!(ATTEMPT_SETTLEMENT_AUDIT
        .contains("stored.request_json != serde_json::to_string(&request)?"));
    assert!(ATTEMPT_SETTLEMENT_AUDIT
        .contains("stored.receipt_json != serde_json::to_string(&receipt)?"));
}

fn item_source<'a>(source: &'a str, marker: &str) -> &'a str {
    let tail = &source[source.find(marker).expect("source item marker must exist")..];
    let end = tail
        .find("\n}")
        .expect("source item must end at module indentation")
        + 2;
    &tail[..end]
}
