use sha2::{Digest, Sha256};

const MIGRATION: &str =
    include_str!("../../store_migrations/compute_external_pool_adapter_route_source_projection.rs");
const STORE_MIGRATIONS: &str = include_str!("../../store_migrations.rs");
const V221_SOURCE: &str =
    include_str!("../../compute_external_pool_onboarding_migration/source_trigger.rs");
const V254_FENCES: &str = include_str!(
    "../../store_migrations/compute_external_pool_provider_activation_candidate/guards/fences.rs"
);

#[test]
fn route_source_projection_migration_is_atomic_schema_only_and_fail_closed() {
    assert_ordered(
        MIGRATION,
        &[
            "TransactionBehavior::Immediate",
            "reject_existing_external_pool_routes(&transaction)?",
            "require_v254_fences(&transaction)?",
            "replace_exact_source_trigger(&transaction)?",
            "transaction.commit()?",
        ],
    );
    for required in [
        "WHERE source_kind='external_pool_onboarding'",
        "OR provider_kind='external_pool'",
        "V271 refuses existing external_pool route authorization rows",
        "V271 requires all 18 V254 deny fences",
        "DROP TRIGGER IF EXISTS trg_compute_route_authorization_exact_source",
        "CREATE TRIGGER trg_compute_route_authorization_exact_source",
    ] {
        assert!(
            MIGRATION.contains(required),
            "missing V271 guard {required}"
        );
    }
    for forbidden in [
        "CREATE TABLE",
        "CREATE VIEW",
        "create_scalar_function",
        "/api/",
        "INSERT INTO compute_providers",
        "UPDATE compute_providers",
        "compute_capacity_pools pool",
        "compute_offers offer",
    ] {
        assert!(
            !MIGRATION.contains(forbidden),
            "V271 migration gained out-of-scope authority {forbidden}"
        );
    }
}

#[test]
fn route_source_projection_preserves_generic_sources_and_replaces_logical_equality() {
    let trigger_start = "CREATE TRIGGER trg_compute_route_authorization_exact_source";
    let generic_start = "(NEW.source_kind='provider_activation_application'";
    let external_start = ")) OR (NEW.source_kind='external_pool_onboarding'";
    assert_eq!(
        source_between(V221_SOURCE, trigger_start, generic_start),
        source_between(MIGRATION, trigger_start, generic_start),
        "V271 changed the common credential/Adapter/actor source guard"
    );
    assert_eq!(
        source_between(V221_SOURCE, generic_start, external_start),
        source_between(MIGRATION, generic_start, external_start),
        "V271 changed a generic route source branch"
    );
    for source_kind in [
        "NEW.source_kind='provider_activation_application'",
        "NEW.source_kind='provider_recovery_application'",
    ] {
        assert!(V221_SOURCE.contains(source_kind));
        assert!(MIGRATION.contains(source_kind));
    }
    assert!(V221_SOURCE.contains("source.adapter_id=NEW.adapter_id"));
    assert!(!MIGRATION.contains("source.adapter_id=NEW.adapter_id"));
    for required in [
        "binding.application_id=source.application_id",
        "binding.application_digest=source.application_digest",
        "binding.provider_policy_revision=source.target_provider_policy_revision",
        "binding.provider_digest=source.target_provider_digest",
        "binding.adapter_id=source.adapter_id",
        "binding.route_adapter_projection_id<>source.adapter_id",
        "binding.route_adapter_projection_id=NEW.adapter_id",
        "candidate.logical_adapter_id=binding.adapter_id",
        "candidate.route_adapter_projection_id=NEW.adapter_id",
        "candidate.release_version=NEW.adapter_release_version",
        "candidate.implementation_digest=NEW.implementation_digest",
        "candidate.adapter_config_revision=NEW.adapter_config_revision",
        "candidate.adapter_config_digest=NEW.adapter_config_digest",
        "candidate.service_actor_id=NEW.verified_by_service_actor_id",
        "candidate.logical_adapter_binding_digest=NEW.route_binding_digest",
        "candidate.logical_adapter_binding_digest=NEW.adapter_binding_digest",
        "$.candidate.logical_adapter_binding_digest",
        "$.candidate.logical_projection_compatibility_digest",
    ] {
        assert!(
            MIGRATION.contains(required),
            "V271 lost exact logical-to-projection mapping {required}"
        );
    }
    assert!(!MIGRATION.contains("candidate.capability_set_digest=NEW.capability_set_digest"));
    for required in [
        "JOIN compute_route_adapter_versions projected_adapter",
        "projected_adapter.adapter_id=NEW.adapter_id",
        "projected_adapter.adapter_revision=NEW.adapter_revision",
        "projected_adapter.adapter_digest=NEW.adapter_registry_digest",
        "projected_adapter.supported_capabilities_json=release.supported_capabilities_json",
        "json_array_length(projected_adapter.supported_capabilities_json)=6",
    ] {
        assert!(
            MIGRATION.contains(required),
            "V271 lost locally scoped projected Adapter root {required}"
        );
    }
    for (ordinal, capability) in [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ]
    .into_iter()
    .enumerate()
    {
        assert!(MIGRATION.contains(&format!(
            "json_extract(projected_adapter.supported_capabilities_json,'$[{ordinal}].capability_id')='{capability}'"
        )));
    }
}

#[test]
fn route_source_projection_requires_the_exact_current_candidate_lineage() {
    for required in [
        "current_release.current_status='release_current'",
        "candidate.candidate_status='candidate_current_not_activation_ready'",
        "candidate.activation_closure_status='activation_closure_not_implemented'",
        "candidate.provider_status='registering'",
        "provider.status='registering'",
        "provider.current_policy_revision=candidate.provider_policy_revision",
        "provider.current_provider_digest=candidate.provider_digest",
        "delegation.sequence=candidate.sequence",
        "candidate.checked_at<=NEW.authenticated_at",
        "later.sequence>candidate.sequence",
        "revoked.delegation_id=candidate.delegation_id",
        "revoked.candidate_id=candidate.candidate_id",
        "compute_external_pool_adapter_installation_terminal_receipts",
        "compute_external_pool_adapter_adoption_terminal_receipts",
        "candidate.provider_binding_id=binding.provider_binding_id",
        "candidate.provider_binding_digest=binding.provider_binding_digest",
        "candidate.registry_release_id=binding.registry_release_id",
        "candidate.registry_release_digest=binding.registry_release_digest",
        "candidate.logical_adapter_binding_digest=NEW.route_binding_digest",
        "candidate.route_adapter_projection_id=NEW.adapter_id",
        "delegation.service_actor_kind='platform_dispatch_service'",
        "delegation.issued_at<=NEW.authenticated_at",
    ] {
        assert!(
            MIGRATION.contains(required),
            "V271 lost current candidate gate {required}"
        );
    }
    for json_root in [
        "$.provider_id",
        "$.provider_kind",
        "$.owner_account_id",
        "$.status",
        "$.policy_revision",
        "$.adapter.adapter_id",
        "$.adapter.adapter_version",
        "$.adapter.config_revision",
        "$.adapter.config_digest",
    ] {
        assert!(
            MIGRATION.contains(json_root),
            "V271 lost Provider version root {json_root}"
        );
    }
}

#[test]
fn route_source_projection_registers_v271_and_preserves_all_v254_fences() {
    assert!(STORE_MIGRATIONS.contains("mod compute_external_pool_adapter_route_source_projection;"));
    assert!(STORE_MIGRATIONS
        .contains("compute_external_pool_adapter_route_source_projection::migration_v271"));
    assert_eq!(
        V254_FENCES.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        18
    );
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
    );
    assert!(!MIGRATION.contains("DROP TRIGGER IF EXISTS v254_"));
    assert!(!MIGRATION.contains("create_scalar_function"));
    assert!(!MIGRATION.contains("logical_projection_compatibility_digest("));
}

fn assert_ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered source marker {needle}"));
        cursor += offset + needle.len();
    }
}

fn source_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap()
        .1
        .split_once(end)
        .unwrap()
        .0
}
