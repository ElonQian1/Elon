use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

/// Advances only the current V259 policy projection to the post-exec-hardened catalog.
///
/// Historical companion and revocation rows remain byte-for-byte unchanged and continue to be
/// validated by their embedded frozen policy revision.
pub(crate) fn migration_v267(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    super::compute_external_pool_adapter_supervisor_session_policy_companion::reinstall_current_policy(
        &transaction,
    )?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    const REGISTRY: &str = include_str!("../store_migrations.rs");
    const MIGRATION: &str =
        include_str!("compute_external_pool_adapter_supervisor_session_policy_v2.rs");
    const GUARDS: &str =
        include_str!("compute_external_pool_adapter_supervisor_session_policy_companion/guards.rs");
    const DOMAIN_POLICY: &str = include_str!(
        "../compute_federation/external_pool_adapter_supervisor_session_policy_companion/policy.rs"
    );
    const V254_FENCES: &str =
        include_str!("compute_external_pool_provider_activation_candidate/guards/fences.rs");

    #[test]
    fn v267_reinstalls_only_current_policy_gates_from_single_catalogs() {
        for required in [
            "migration_v267",
            "TransactionBehavior::Immediate",
            "reinstall_current_policy",
            "transaction.commit()?",
        ] {
            assert!(
                MIGRATION.contains(required),
                "missing V267 migration gate {required}"
            );
        }
        assert!(REGISTRY.contains(
            "compute_external_pool_adapter_supervisor_session_policy_v2::migration_v267"
        ));
        for required in [
            "DROP TRIGGER IF EXISTS external_pool_adapter_supervisor_session_policy_companion_policy_json_projection",
            "policy_projection::install(conn)?",
        ] {
            assert!(GUARDS.contains(required), "missing V267 guard reinstall {required}");
        }
        assert!(!GUARDS.contains(
            "DROP TRIGGER IF EXISTS external_pool_adapter_supervisor_session_policy_companion_exact_roots"
        ));
        for required in [
            "SUPERVISOR_SESSION_POLICY_V1_ID",
            "SUPERVISOR_SESSION_POLICY_V2_ID",
            "policy_v1_for_validation",
            "policy_v2_for_validation",
            "historical_supervisor_session_policy_v1_catalog",
            "yama_ptrace_scope_2_or_stricter_v2",
            "prctl_dumpable_set_zero_or_get_only",
            "single_execveat_derived_launch_capsule_fd_4_at_empty_path_v2",
        ] {
            assert!(
                DOMAIN_POLICY.contains(required),
                "missing V267 policy rule {required}"
            );
        }
    }

    #[test]
    fn v267_does_not_change_receipt_tables_or_execution_effects() {
        for forbidden in [
            "CREATE TABLE",
            "ALTER TABLE",
            "DROP TABLE",
            "INSERT INTO",
            "UPDATE compute_",
            "DELETE FROM",
            "process_spawn_ready=1",
            "secret_delivery_ready=1",
            "runtime_launch_ready=1",
            "activation_ready=1",
        ] {
            assert!(
                !MIGRATION.contains(forbidden),
                "V267 crossed no-effect fence {forbidden}"
            );
        }
    }

    #[test]
    fn v267_preserves_v254_absolute_denies_byte_exact() {
        assert_eq!(
            hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
            "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
        );
        assert_eq!(V254_TRIGGER_NAMES.len(), 18);
        for name in V254_TRIGGER_NAMES {
            assert!(V254_FENCES.contains(name), "missing V254 fence {name}");
        }
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
}
