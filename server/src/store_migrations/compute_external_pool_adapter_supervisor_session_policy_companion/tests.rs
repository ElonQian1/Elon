use sha2::{Digest, Sha256};

use super::*;

const TABLES: &str = include_str!("tables.sql");
const VIEW: &str = include_str!("view.sql");
const VIEW_INSTALLER: &str = include_str!("view.rs");
const IMMUTABILITY: &str = include_str!("guards/immutability.rs");
const LINEAGE: &str = include_str!("guards/lineage.rs");
const POLICY: &str = include_str!("guards/policy_projection.rs");
const PROJECTION: &str = include_str!("guards/receipt_projection.rs");
const RECEIPT_INTEGRITY: &str = include_str!("receipt_integrity.rs");
const ROOTS: &str = include_str!("guards/roots.rs");
const TIMESTAMP: &str = include_str!("guards/timestamp.rs");
const PERSISTENCE: &str = include_str!(
    "../../store/compute_external_pool_adapter_supervisor_session_policy_companion/persistence.rs"
);
const DOMAIN_POLICY: &str = include_str!(
    "../../compute_federation/external_pool_adapter_supervisor_session_policy_companion/policy.rs"
);
const DOMAIN_VALIDATION: &str = include_str!(
    "../../compute_federation/external_pool_adapter_supervisor_session_policy_companion/validation.rs"
);
const CARGO: &str = include_str!("../../../Cargo.toml");
const FACADE: &str =
    include_str!("../compute_external_pool_adapter_supervisor_session_policy_companion.rs");
const REGISTRY: &str = include_str!("../../store_migrations.rs");
const STORE_SCHEMA: &str = include_str!("../../store_schema.rs");
const V254_FENCES: &str =
    include_str!("../compute_external_pool_provider_activation_candidate/guards/fences.rs");

#[test]
fn v259_store_insert_columns_match_frozen_tables_exactly() {
    for (table, expected) in [
        (
            "compute_external_pool_adapter_supervisor_session_policy_companions",
            79,
        ),
        (
            "compute_external_pool_adapter_supervisor_session_policy_companion_revocations",
            41,
        ),
    ] {
        let columns = insert_columns(PERSISTENCE, table);
        assert_eq!(columns.len(), expected, "{table} persistence ABI drift");
        assert_eq!(columns, ddl_columns(TABLES, table));
    }
}

#[test]
fn v259_registration_receipts_and_policy_catalog_are_exact() {
    assert!(
        REGISTRY.contains("mod compute_external_pool_adapter_supervisor_session_policy_companion;")
    );
    assert!(REGISTRY.contains(
        "compute_external_pool_adapter_supervisor_session_policy_companion::migration_v259"
    ));
    assert_eq!(guards::receipt_projection_counts(), (78, 40));
    for required in [
        "server_supervisor_session_policy_catalog",
        "canonical_compute_plugin_ijson_and_sha256",
        "supervisor_session_policy_digest",
        "supervisor_session_policy_json",
        "NEW.supervisor_session_policy_json IS NOT",
    ] {
        assert!(POLICY.contains(required), "missing policy gate {required}");
    }
    for required in [
        "anonymous_child_socketpair_seqpacket_v1",
        "clone3_v1",
        "cgroup_v2_dedicated_leaf_v1",
        "kill_process",
        "pidfd_only_v1",
        "bootstrap_allowed_syscalls",
        "runtime_allowed_syscalls",
        "argument_rules",
        "unknown_syscall_action",
        "audit_arch_policy",
    ] {
        assert!(
            DOMAIN_POLICY.contains(required),
            "missing policy value {required}"
        );
    }
}

#[test]
fn v259_receipt_jcs_and_domain_digests_are_connection_enforced() {
    assert!(CARGO.contains("features = [\"bundled\", \"functions\"]"));
    assert!(STORE_SCHEMA.contains("register_v259_receipt_integrity_functions(conn)?;"));
    let register = FACADE
        .find("register_receipt_integrity_functions(conn)?;")
        .expect("migration must register receipt integrity functions");
    let transaction = FACADE
        .find("Transaction::new_unchecked")
        .expect("migration transaction missing");
    assert!(
        register < transaction,
        "UDFs must exist before trigger installation"
    );
    for required in [
        "FunctionFlags::SQLITE_DETERMINISTIC",
        "FunctionFlags::SQLITE_INNOCUOUS",
        "validate_supervisor_session_companion_receipt",
        "validate_supervisor_session_companion_revocation_receipt",
        "canonical_supervisor_session_companion_json_and_digest",
        "canonical_supervisor_session_companion_revocation_json_and_digest",
        "canonical == json",
        "get_raw(0)",
        "json.len() > 1_048_576",
        "WHEN {COMPANION_RECEIPT_IS_EXACT}(NEW.companion_json) IS NOT 1",
        "WHEN {REVOCATION_RECEIPT_IS_EXACT}(NEW.revocation_json) IS NOT 1",
    ] {
        assert!(
            RECEIPT_INTEGRITY.contains(required),
            "missing receipt integrity gate {required}"
        );
    }
    for required in [
        "validate_embedded_supervisor_session_policy_shape",
        "SUPERVISOR_SESSION_POLICY_V1_ID",
        "SUPERVISOR_SESSION_POLICY_V1_REVISION",
        "policy_v1_for_validation",
        "supervisor_session_policy_digest(&c.supervisor_session_policy)",
        "supervisor_session_companion_material_digest(c)",
        "canonical_supervisor_session_companion_json_and_digest(receipt)?.1",
        "supervisor_session_companion_revocation_material_digest(r)",
        "canonical_supervisor_session_companion_revocation_json_and_digest(receipt)?.1",
    ] {
        assert!(
            DOMAIN_VALIDATION.contains(required),
            "missing historical-self receipt validation {required}"
        );
    }
}

#[test]
fn v259_projection_roots_lineage_and_time_are_fail_closed() {
    for required in [
        "COUNT(*) FROM json_each(NEW.{json_column}))!=7",
        "material_count",
        "supervisor_session_policy_json",
        "process_spawn_ready",
        "ipc_session_ready",
        "secret_delivery_ready",
        "activation_ready",
    ] {
        assert!(
            PROJECTION.contains(required),
            "missing projection {required}"
        );
    }
    for required in [
        "compute_external_pool_adapter_upstream_transport_target_current",
        "current_target.current_status='upstream_transport_target_current_inert'",
        "current_target.revocation_status='unrevoked'",
        "profile.launch_policy_json,'$.process_isolation_policy_id'",
        "profile.launch_policy_json,'$.resource_policy_digest'",
        "profile.launch_policy_json,'$.network_egress_policy_digest'",
        "profile.launch_policy_digest=target.launch_policy_digest",
        "profile.provider_binding_digest=target.provider_binding_digest",
        "profile.installation_content_digest=target.installation_content_digest",
        "entrypoint_capsule_policy_digest='710decef25b4d19b33f086239f55f809a513508eb5ba431967971ff89249604f'",
    ] {
        assert!(ROOTS.contains(required), "missing root gate {required}");
    }
    for required in [
        "NEW.sequence=1",
        "predecessor.sequence=NEW.sequence-1",
        "successor.predecessor_companion_id=predecessor.companion_id",
        "companion.recorded_at<=NEW.revoked_at",
        "prior.companion_id=companion.companion_id",
        "UNIQUE (provider_binding_id,sequence)",
        "UNIQUE (predecessor_companion_id)",
        "uq_external_pool_adapter_supervisor_session_policy_companion_genesis",
        "recorded_at=revoked_at",
    ] {
        assert!(
            LINEAGE.contains(required) || TABLES.contains(required),
            "missing lineage fence {required}"
        );
    }
    for required in [
        "%400=0",
        "%100<>0",
        "NOT BETWEEN 0 AND 23",
        "NOT BETWEEN 0 AND 59",
        "julianday(NEW.{column})<julianday('now','-5 minutes')",
        "julianday(NEW.{column})>julianday('now','+5 minutes')",
    ] {
        assert!(TIMESTAMP.contains(required), "missing time gate {required}");
    }
}

#[test]
fn v259_current_view_is_structural_inert_and_append_only() {
    for required in ["_no_update", "_no_delete", "_no_replace"] {
        assert!(IMMUTABILITY.contains(required));
    }
    for required in [
        "compute_external_pool_adapter_supervisor_session_policy_companion_current",
        "SELECT companion.*",
        "revocation.revocation_id",
        "'head' AS head_status",
        "supervisor_session_policy_companion_current_inert",
        "historical_only",
        "upstream_transport_target_current_inert",
        "AS profile_status",
        "AS policy_status",
        "server_policy_current",
        "AS revocation_status",
        "head.supervisor_session_policy_digest=__POLICY_DIGEST_SQL__",
        "head.supervisor_session_policy_json=__POLICY_JSON_SQL__",
        "__POLICY_ID_SQL__",
        "__POLICY_REVISION__",
    ] {
        assert!(VIEW.contains(required), "missing view contract {required}");
    }
    for required in [
        "catalog_json_and_digest",
        "policy_json",
        "policy_digest",
        "policy_id",
        "policy_revision",
    ] {
        assert!(VIEW_INSTALLER.contains(required));
    }
}

#[test]
fn v259_preserves_v254_denies_and_claims_no_execution() {
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
    );
    for name in V254_TRIGGER_NAMES {
        assert!(V254_FENCES.contains(name), "missing V254 deny {name}");
    }
    let v259 = concat!(
        include_str!("../compute_external_pool_adapter_supervisor_session_policy_companion.rs"),
        include_str!("tables.sql"),
        include_str!("view.sql"),
        include_str!("guards/immutability.rs"),
        include_str!("guards/lineage.rs"),
        include_str!("guards/policy_projection.rs"),
        include_str!("guards/receipt_projection.rs"),
        include_str!("receipt_integrity.rs"),
        include_str!("guards/roots.rs"),
        include_str!("guards/timestamp.rs"),
    );
    for forbidden in [
        "DROP TRIGGER",
        "UPDATE compute_providers",
        "INSERT INTO compute_route_",
        "provider.status='active'",
        "process_spawn_ready=1",
        "ipc_session_ready=1",
        "secret_delivery_ready=1",
        "broker_connect_ready=1",
        "upstream_probe_observed=1",
        "runtime_launch_ready=1",
        "activation_ready=1",
    ] {
        assert!(!v259.contains(forbidden), "V259 crosses no-go {forbidden}");
    }
}

fn insert_columns(source: &str, table: &str) -> Vec<String> {
    let marker = format!("INSERT INTO {table}(");
    source
        .split_once(&marker)
        .unwrap_or_else(|| panic!("missing Store INSERT for {table}"))
        .1
        .split_once(") VALUES (")
        .unwrap_or_else(|| panic!("missing Store VALUES for {table}"))
        .0
        .split(',')
        .map(str::trim)
        .map(str::to_owned)
        .collect()
}

fn ddl_columns(source: &str, table: &str) -> Vec<String> {
    let marker = format!("CREATE TABLE IF NOT EXISTS {table} (");
    let body = source
        .split_once(&marker)
        .unwrap_or_else(|| panic!("missing DDL for {table}"))
        .1
        .split_once("\n);")
        .unwrap_or_else(|| panic!("unterminated DDL for {table}"))
        .0;
    body.lines()
        .filter_map(|line| {
            let column = line.trim().split_whitespace().next()?;
            (!column.starts_with("CHECK")
                && !column.starts_with("UNIQUE")
                && !column.starts_with("FOREIGN"))
            .then(|| column.to_owned())
        })
        .collect()
}

const V254_TRIGGER_NAMES: &[&str] = &[
    "v254_external_pool_provider_activation_fence",
    "v254_external_pool_provider_insert_active_fence",
    "v254_external_pool_provider_identity_update_fence",
    "v254_external_pool_provider_kind_update_fence",
    "v254_external_pool_provider_version_active_fence",
    "v254_external_pool_candidate_projection_adapter_fence",
    "v254_external_pool_candidate_projection_adapter_version_fence",
    "v254_external_pool_candidate_service_actor_fence",
    "v254_external_pool_route_credential_fence",
    "v254_external_pool_route_authorization_fence",
    "v254_external_pool_route_capability_fence",
    "v254_external_pool_route_seal_fence",
    "v254_external_pool_capacity_pool_insert_active_fence",
    "v254_external_pool_capacity_pool_update_active_fence",
    "v254_external_pool_capacity_pool_version_active_fence",
    "v254_external_pool_offer_insert_market_fence",
    "v254_external_pool_offer_update_market_fence",
    "v254_external_pool_offer_version_market_fence",
];
