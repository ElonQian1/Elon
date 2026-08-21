use crate::compute_federation::federation_historical_causal_reference::build_execution_source_carrier;

use super::source_refs::{validate_execution_source_links, validate_settlement_source_links};

mod fixtures;
mod owner_contracts;
mod release;
mod verification;

use fixtures::{audit_legacy_pool, execution_facts, legacy_pool_facts, settlement_facts};

const STORE_FACADE: &str = include_str!("../compute_federation_historical_causal_reference.rs");
const EXECUTION_RESOLVER: &str = include_str!("execution.rs");
const SETTLEMENT_RESOLVER: &str = include_str!("settlement.rs");
const RELEASE_RESOLVER: &str = include_str!("release.rs");
const RELEASE_REFS: &str = include_str!("release_refs.rs");
const SOURCE_REFS: &str = include_str!("source_refs.rs");
const VERIFICATION_RESOLVER: &str = include_str!("verification.rs");
const VERIFICATION_REFS: &str = include_str!("verification_refs.rs");
const CAPACITY_POOL_QUERIES: &str = include_str!("../compute_capacity_pool_queries.rs");
const ATTEMPT_EXECUTION_RECEIPTS: &str = include_str!("../compute_attempt_execution_receipts.rs");
const ATTEMPT_SETTLEMENTS: &str = include_str!("../compute_attempt_settlements.rs");
const ATTEMPT_SETTLEMENT_SUPPORT: &str = include_str!("../compute_attempt_settlements/support.rs");
const ATTEMPT_SETTLEMENT_AUDIT: &str =
    include_str!("../compute_attempt_settlements/support/audit.rs");
const ATTEMPT_FINALIZATIONS: &str = include_str!("../compute_attempt_finalizations.rs");
const ATTEMPT_FINALIZATION_AUDIT: &str =
    include_str!("../compute_attempt_finalizations/support/audit.rs");
const DOMAIN_FACADE: &str =
    include_str!("../../compute_federation/federation_historical_causal_reference.rs");
const DOMAIN_TYPES: &str =
    include_str!("../../compute_federation/federation_historical_causal_reference/types.rs");
const STORE_MIGRATIONS: &str = include_str!("../../store_migrations.rs");
const STORE_SCHEMA: &str = include_str!("../../store_schema.rs");
const ROUTER: &str = include_str!("../../router.rs");
const MAIN: &str = include_str!("../../main.rs");

#[test]
fn resolver_source_uses_exact_owner_seams_inside_deferred_read_snapshots() {
    assert!(STORE_FACADE.contains("resolve_compute_execution_source_lineage"));
    assert!(
        STORE_FACADE.contains("resolve_compute_execution_verification_source_lineage_for_lease")
    );
    assert!(STORE_FACADE.contains("resolve_compute_settlement_source_lineage"));
    assert_eq!(
        STORE_FACADE
            .matches("transaction_with_behavior(TransactionBehavior::Deferred)")
            .count(),
        6
    );

    let resolver_source = resolver_source();
    for owner_seam in [
        "compute_attempt_historical_activation_sources_on",
        "compute_attempt_execution_receipt_by_id_on",
        "audited_compute_attempt_lease_version_on",
        "compute_attempt_historical_terminal_candidate_on",
        "compute_attempt_usage_declaration_on",
        "compute_attempt_historical_consumer_review_on",
        "compute_attempt_historical_platform_observation_on",
        "compute_attempt_historical_verification_decision_on",
        "stored_claim_version_on",
        "audited_compute_capacity_pool_version_on",
        "registered_historical_job_version_on",
        "registered_historical_offer_version_on",
        "registered_historical_price_snapshot_on",
        "registered_provider_version_on",
        "registered_historical_reservation_version_on",
        "compute_attempt_historical_finalization_on",
        "compute_attempt_settlement_by_receipt_id_on",
    ] {
        assert!(
            resolver_source.contains(owner_seam),
            "missing exact retained owner seam: {owner_seam}"
        );
    }

    for forbidden in [
        "TransactionBehavior::Immediate",
        ".execute(",
        ".execute_batch(",
        "INSERT INTO",
        "UPDATE ",
        "DELETE FROM",
        "CREATE TABLE",
        "CREATE TRIGGER",
        "apply_migrations",
        "NodeComputeRun",
        "current_lease",
        "current_lease_state_on",
        "current_registered_provider_on",
        "current_registered_offer_on",
        "current_registered_job_on",
        "current_registered_reservation_on",
        "current_capacity_pool_on",
        "stored_claim_on(",
        "compute_attempt_execution_receipt_on(",
        "compute_attempt_settlement_on(",
        "Utc::now",
        "new_id",
        "random",
    ] {
        assert!(
            !resolver_source.contains(forbidden),
            "historical resolver must not contain writer/current fallback: {forbidden}"
        );
    }

    assert!(CAPACITY_POOL_QUERIES
        .contains("pub(in crate::store) fn audit_legacy_compute_capacity_pool_digests("));
    assert_eq!(
        CAPACITY_POOL_QUERIES
            .matches("audit_legacy_compute_capacity_pool_digests(")
            .count(),
        3
    );
    assert!(ATTEMPT_SETTLEMENTS.contains("compute_attempt_settlement_by_receipt_id_on"));
    assert!(ATTEMPT_SETTLEMENTS.contains("stored.into_historical_receipt(conn)"));
    assert!(ATTEMPT_SETTLEMENT_SUPPORT.contains("fn into_historical_receipt("));
    assert!(
        ATTEMPT_SETTLEMENT_SUPPORT.contains("audit::audited_historical_settlement_on(conn, self)")
    );
    assert!(ATTEMPT_SETTLEMENT_AUDIT
        .contains("audited_settlement_with_head_policy_on(conn, stored, false, false)"));
    assert!(ATTEMPT_SETTLEMENT_AUDIT
        .contains("audited_settlement_with_head_policy_on(conn, stored, replayed, true)"));
    assert!(ATTEMPT_SETTLEMENT_AUDIT.contains("if require_current_heads"));
    assert!(ATTEMPT_SETTLEMENT_AUDIT
        .contains("compute_attempt_historical_finalization_on(conn, &stored.lease_id)?"));
    assert!(SETTLEMENT_RESOLVER.contains("compute_attempt_historical_finalization_on"));
    assert!(!SETTLEMENT_RESOLVER.contains("compute_attempt_finalization_on"));
    assert!(ATTEMPT_FINALIZATIONS.contains("fn compute_attempt_historical_finalization_on("));
    assert!(ATTEMPT_FINALIZATIONS.contains("stored.into_historical_receipt(conn)"));
    assert!(ATTEMPT_FINALIZATION_AUDIT
        .contains("self.into_receipt_with_head_policy(conn, false, false)"));
    assert!(ATTEMPT_FINALIZATION_AUDIT
        .contains("self.into_receipt_with_head_policy(conn, replayed, true)"));
}

#[test]
fn by_lease_adoption_uses_historical_owners_and_private_access_scope() {
    assert!(ATTEMPT_EXECUTION_RECEIPTS
        .contains("fn compute_attempt_historical_execution_receipt_by_lease_on("));
    assert!(ATTEMPT_EXECUTION_RECEIPTS
        .contains("execution_receipt_historical_envelope_on(conn, stored)"));
    assert!(ATTEMPT_SETTLEMENTS.contains("fn compute_attempt_historical_settlement_by_lease_on("));
    assert!(ATTEMPT_SETTLEMENTS.contains("stored.into_historical_receipt(conn)"));
    assert!(STORE_FACADE.contains("resolve_compute_execution_source_lineage_for_lease"));
    assert!(STORE_FACADE.contains("resolve_compute_settlement_source_lineage_for_lease"));
    assert!(STORE_FACADE
        .contains("compute_attempt_historical_execution_receipt_by_lease_on(&tx, lease_id)"));
    assert!(
        STORE_FACADE.contains("compute_attempt_historical_settlement_by_lease_on(&tx, lease_id)")
    );

    let marker = "struct FederationHistoricalLineageAccessScope";
    let start = STORE_FACADE
        .find(marker)
        .expect("Store must own the private historical access scope");
    let prefix = STORE_FACADE[..start]
        .lines()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!prefix.contains("#[derive"));
    assert!(!STORE_FACADE.contains("Serialize for FederationHistoricalLineageAccessScope"));
    assert!(!STORE_FACADE.contains("Deserialize for FederationHistoricalLineageAccessScope"));
    assert!(!STORE_FACADE.contains("Clone for FederationHistoricalLineageAccessScope"));
    assert!(EXECUTION_RESOLVER.contains("&job.job.consumer_account_id"));
    assert!(EXECUTION_RESOLVER.contains("job.job.project_id.as_deref()"));
    assert!(EXECUTION_RESOLVER.contains("&provider.provider.owner_account_id"));
    assert!(SETTLEMENT_RESOLVER.contains("access_scope.ensure_job_matches("));
    assert!(SETTLEMENT_RESOLVER
        .contains("access_scope.ensure_same_as(rebuilt_execution.access_scope())?"));
}

#[test]
fn capacity_pool_owner_audit_rejects_policy_and_pool_digest_drift() {
    let (binding, meter_policies) = legacy_pool_facts();
    assert!(audit_legacy_pool(&binding, &meter_policies).is_ok());

    let (binding, mut policy_drift) = legacy_pool_facts();
    policy_drift[0].policy_digest = "policy-digest-b".to_string();
    assert!(audit_legacy_pool(&binding, &policy_drift).is_err());

    let (mut pool_drift, meter_policies) = legacy_pool_facts();
    pool_drift.pool_digest = "pool-digest-b".to_string();
    assert!(audit_legacy_pool(&pool_drift, &meter_policies).is_err());
}

#[test]
fn settlement_rebuilds_execution_and_store_alone_seals_the_validated_view() {
    assert!(EXECUTION_RESOLVER.contains("resolve_execution_source_lineage_on"));
    assert!(EXECUTION_RESOLVER.contains("validate_root_pair("));
    assert!(SETTLEMENT_RESOLVER.contains("resolve_settlement_source_lineage_on"));
    assert!(SETTLEMENT_RESOLVER.contains("validate_root_triple("));
    assert!(SETTLEMENT_RESOLVER.contains("resolve_execution_source_lineage_on("));
    assert!(SETTLEMENT_RESOLVER
        .contains("execution_lineage_digest: rebuilt_execution.lineage_digest().to_string()"));
    assert!(!SETTLEMENT_RESOLVER.contains("execution_lineage_digest: settlement"));

    let marker = "pub(crate) struct ValidatedFederationHistoricalLineage";
    let start = STORE_FACADE
        .find(marker)
        .expect("Store facade must own the sealed validated view");
    let tail = &STORE_FACADE[start..];
    let end = tail
        .find("\n}")
        .expect("validated view declaration must be bounded");
    let declaration = &tail[..end];
    for field in declaration.lines().skip(1) {
        assert!(
            !field.trim_start().starts_with("pub"),
            "validated view fields must remain private: {field}"
        );
    }
    let prefix = STORE_FACADE[..start]
        .lines()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!prefix.contains("#[derive"));
    assert!(STORE_FACADE.contains("fn from_carrier("));
    assert!(!STORE_FACADE.contains("pub(crate) fn from_carrier("));
    assert!(STORE_FACADE.contains("validate_federation_historical_causal_reference(&carrier)?;"));
    assert!(!STORE_FACADE.contains("impl Clone for ValidatedFederationHistoricalLineage"));
    assert!(!STORE_FACADE.contains("Serialize for ValidatedFederationHistoricalLineage"));
    assert!(!STORE_FACADE.contains("Deserialize for ValidatedFederationHistoricalLineage"));
    assert!(STORE_FACADE.contains("pub(crate) fn canonical_json(&self) -> &str"));
    assert!(STORE_FACADE.contains("pub(crate) fn lineage_digest(&self) -> &str"));
    assert!(STORE_FACADE.contains("pub(crate) fn kind(&self)"));

    let domain_source = format!("{DOMAIN_FACADE}\n{DOMAIN_TYPES}");
    assert!(!domain_source.contains("ValidatedFederationHistoricalLineage"));
    assert!(domain_source.contains("UntrustedFederationHistoricalCausalReferenceEnvelopeV1"));
    for boundary in [ROUTER, MAIN, STORE_MIGRATIONS, STORE_SCHEMA] {
        assert!(!boundary.contains("UntrustedFederationHistoricalCausalReferenceEnvelopeV1"));
    }
    for persistence_root in [STORE_MIGRATIONS, STORE_SCHEMA] {
        assert!(
            !persistence_root.contains("compute_federation.core_historical_causal_reference.v1")
        );
        assert!(!persistence_root.contains("federation_historical_causal_reference"));
    }

    let first = build_execution_source_carrier(execution_facts().lineage).unwrap();
    let second = build_execution_source_carrier(execution_facts().lineage).unwrap();
    assert_eq!(first.lineage_digest(), second.lineage_digest());
    assert_eq!(
        first.canonical_json().unwrap(),
        second.canonical_json().unwrap()
    );
}

#[test]
fn execution_cross_object_splices_are_constructible_and_fail_closed() {
    assert!(validate_execution_source_links(&execution_facts()).is_ok());

    let mut cross_provider = execution_facts();
    cross_provider.offer_provider.provider_id = "provider-b".to_string();
    let mut audited_provider_drift = execution_facts();
    audited_provider_drift.audited_provider.provider_digest = "provider-digest-b".to_string();
    let mut audited_pool_drift = execution_facts();
    audited_pool_drift.audited_pool.pool_digest = "pool-digest-b".to_string();
    let mut audited_offer_drift = execution_facts();
    audited_offer_drift.audited_offer.offer_digest = "offer-digest-b".to_string();
    let mut audited_v193_drift = execution_facts();
    audited_v193_drift
        .audited_execution_receipt
        .execution_receipt_digest = "execution-digest-b".to_string();
    let mut cross_pool = execution_facts();
    cross_pool.pool_from_claim.pool_digest = "pool-digest-b".to_string();
    let mut cross_offer = execution_facts();
    cross_offer.snapshot_offer.offer_id = "offer-b".to_string();
    let mut cross_snapshot = execution_facts();
    cross_snapshot.reservation_snapshot.price_snapshot_digest = "snapshot-digest-b".to_string();
    let mut delivery_window_drift = execution_facts();
    delivery_window_drift.claim_delivery_window.window_digest = "window-digest-b".to_string();
    let mut cross_job = execution_facts();
    cross_job.candidate_job.job_digest = "job-digest-b".to_string();
    let mut cross_reservation = execution_facts();
    cross_reservation
        .verification_reservation
        .reservation_revision += 1;
    let mut cross_claim = execution_facts();
    cross_claim.reservation_claim.claim_id = "claim-b".to_string();
    let mut current_or_terminal_lease = execution_facts();
    current_or_terminal_lease.audited_lease.lease_revision += 1;
    current_or_terminal_lease.audited_lease.lease_digest = "lease-digest-b".to_string();
    let mut v193_fencing_drift = execution_facts();
    v193_fencing_drift.receipt_fencing_generation += 1;

    for (case, facts) in [
        ("provider", cross_provider),
        ("audited_provider", audited_provider_drift),
        ("audited_pool", audited_pool_drift),
        ("audited_offer", audited_offer_drift),
        ("audited_v193", audited_v193_drift),
        ("pool", cross_pool),
        ("offer", cross_offer),
        ("snapshot", cross_snapshot),
        ("delivery_window", delivery_window_drift),
        ("job", cross_job),
        ("reservation", cross_reservation),
        ("claim", cross_claim),
        ("source_lease", current_or_terminal_lease),
        ("v193_fencing", v193_fencing_drift),
    ] {
        assert!(
            validate_execution_source_links(&facts).is_err(),
            "execution splice must fail closed: {case}"
        );
    }
}

#[test]
fn settlement_v193_v194_v195_splices_are_constructible_and_fail_closed() {
    assert!(validate_settlement_source_links(&settlement_facts()).is_ok());

    let mut cross_v193_v195 = settlement_facts();
    cross_v193_v195
        .settlement_execution_receipt
        .execution_receipt_digest = "execution-digest-b".to_string();
    let mut outer_settlement_drift = settlement_facts();
    outer_settlement_drift
        .audited_attempt_settlement
        .settlement_event_digest = "settlement-event-b".to_string();
    let mut copied_execution_digest = settlement_facts();
    copied_execution_digest.lineage.execution_lineage_digest = "lineage-digest-b".to_string();
    let mut finalization_drift = settlement_facts();
    finalization_drift
        .audited_finalization
        .finalization_event_digest = "finalization-event-b".to_string();
    let mut source_job_swap = settlement_facts();
    source_job_swap.settlement_source_job = source_job_swap.lineage.terminal_job.clone();
    let mut terminal_job_drift = settlement_facts();
    terminal_job_drift.settlement_terminal_job.job_digest = "terminal-job-digest-b".to_string();
    let mut terminal_reservation_drift = settlement_facts();
    terminal_reservation_drift
        .finalization_terminal_reservation
        .reservation_digest = "reservation-digest-b".to_string();
    let mut provider_drift = settlement_facts();
    provider_drift.settlement_provider.provider_digest = "provider-digest-b".to_string();
    let mut execution_provider_drift = settlement_facts();
    execution_provider_drift.execution_provider_id = "provider-b".to_string();
    let mut finalization_provider_drift = settlement_facts();
    finalization_provider_drift.finalization_provider_id = "provider-b".to_string();
    let mut audited_provider_drift = settlement_facts();
    audited_provider_drift.audited_provider.provider_digest = "provider-digest-b".to_string();
    let mut execution_lease_splice = settlement_facts();
    execution_lease_splice.execution_lease_id = "lease-b".to_string();
    let mut finalization_lease_splice = settlement_facts();
    finalization_lease_splice.finalization_lease_id = "lease-b".to_string();
    let mut released_balance = settlement_facts();
    released_balance.settlement_balance_state = "available".to_string();

    for (case, facts) in [
        ("v193_v195", cross_v193_v195),
        ("settlement_outer_event", outer_settlement_drift),
        ("execution_lineage_digest", copied_execution_digest),
        ("finalization", finalization_drift),
        ("source_terminal_job", source_job_swap),
        ("terminal_job", terminal_job_drift),
        ("terminal_reservation", terminal_reservation_drift),
        ("provider", provider_drift),
        ("execution_provider", execution_provider_drift),
        ("finalization_provider", finalization_provider_drift),
        ("audited_provider", audited_provider_drift),
        ("execution_lease", execution_lease_splice),
        ("finalization_lease", finalization_lease_splice),
        ("non_pending_balance", released_balance),
    ] {
        assert!(
            validate_settlement_source_links(&facts).is_err(),
            "settlement splice must fail closed: {case}"
        );
    }
}

fn resolver_source() -> String {
    [
        STORE_FACADE,
        EXECUTION_RESOLVER,
        SETTLEMENT_RESOLVER,
        RELEASE_RESOLVER,
        RELEASE_REFS,
        SOURCE_REFS,
        VERIFICATION_RESOLVER,
        VERIFICATION_REFS,
    ]
    .join("\n")
}
