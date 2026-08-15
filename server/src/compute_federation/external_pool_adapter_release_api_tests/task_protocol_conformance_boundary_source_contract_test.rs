use sha2::{Digest, Sha256};

const MIGRATION_ROOT: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance.rs"
);
const TABLES_INSTALLER: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/tables.rs"
);
const GUARDS_INSTALLER: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards.rs"
);
const ROOTS_INSTALLER: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/roots.rs"
);
const VIEW_INSTALLER: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/view.rs"
);
const RUN_TABLE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/tables/run_receipts.sql"
);
const REVOCATION_TABLE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/tables/revocations.sql"
);
const INDEXES: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/tables/indexes.sql"
);
const IMMUTABILITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/immutability.sql"
);
const NO_REPLACE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/no_replace.sql"
);
const PROJECTION: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/projection.sql"
);
const LINEAGE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/lineage.sql"
);
const ROOT_RELEASE_SECURITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/roots/release_security.sql"
);
const ROOT_RUNTIME: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/roots/runtime_compatibility.sql"
);
const VIEW: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/view.sql"
);
const V254_FENCES: &str = include_str!(
    "../../store_migrations/compute_external_pool_provider_activation_candidate/guards/fences.rs"
);

#[test]
fn task_protocol_conformance_boundary_preserves_all_v254_fences_and_opens_none() {
    assert_eq!(
        V254_FENCES.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        18
    );
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
    );

    let v272_migration = format!(
        "{MIGRATION_ROOT}{TABLES_INSTALLER}{GUARDS_INSTALLER}{ROOTS_INSTALLER}{VIEW_INSTALLER}{RUN_TABLE}{REVOCATION_TABLE}{INDEXES}{IMMUTABILITY}{NO_REPLACE}{PROJECTION}{LINEAGE}{ROOT_RELEASE_SECURITY}{ROOT_RUNTIME}{VIEW}"
    );
    for forbidden in ["DROP TRIGGER", "DROP TABLE", "ALTER TABLE"] {
        assert!(
            !v272_migration.contains(forbidden),
            "V272 migration gained destructive fence bypass {forbidden}"
        );
    }
    for protected_table in [
        "compute_providers",
        "compute_provider_versions",
        "compute_route_adapters",
        "compute_route_adapter_versions",
        "compute_service_actor_authorizations",
        "compute_route_credential_versions",
        "compute_route_authorization_receipts",
        "compute_route_authorization_capabilities",
        "compute_route_authorization_seals",
        "compute_capacity_pools",
        "compute_capacity_pool_versions",
        "compute_offers",
        "compute_offer_versions",
    ] {
        assert!(
            !v272_migration.contains(protected_table),
            "V272 migration touched protected execution/market table {protected_table}"
        );
    }
}
