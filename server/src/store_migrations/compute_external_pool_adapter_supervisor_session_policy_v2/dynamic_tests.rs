use std::collections::BTreeMap;

use rusqlite::Connection;
use uuid::Uuid;

use super::migration_v267;
use crate::{
    compute_federation::external_pool_adapter_supervisor_session_policy_companion::{
        historical_supervisor_session_policy_v1_catalog, server_supervisor_session_policy_catalog,
        validate_embedded_supervisor_session_policy_shape,
    },
    store::Store,
};

const CURRENT_POLICY_TRIGGER: &str =
    "external_pool_adapter_supervisor_session_policy_companion_policy_json_projection";
const CURRENT_POLICY_VIEW: &str =
    "compute_external_pool_adapter_supervisor_session_policy_companion_current";
const EXACT_ROOTS_TRIGGER: &str =
    "external_pool_adapter_supervisor_session_policy_companion_exact_roots";
const COMPANION_TABLE: &str = "compute_external_pool_adapter_supervisor_session_policy_companions";
const REVOCATION_TABLE: &str =
    "compute_external_pool_adapter_supervisor_session_policy_companion_revocations";
const SOURCE_CAPSULE_POLICY_DIGEST: &str =
    "710decef25b4d19b33f086239f55f809a513508eb5ba431967971ff89249604f";

#[test]
fn v267_fresh_and_repeat_migration_install_current_v2_without_rewriting_history() {
    let root = std::env::temp_dir().join(format!(
        "elon-supervisor-session-policy-v267-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary migration directory should exist");
    let database = root.join("state.sqlite");

    {
        let store = Store::open(&database).expect("fresh migration through V267 should succeed");
        let connection = store.conn().expect("V267 database should lock");
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM schema_migrations WHERE version=267",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("V267 migration row should read"),
            1
        );

        let initial = protected_schema(&connection);
        assert_current_v2_projection(&connection);
        assert_preserved_roots_and_denies(&connection);

        migration_v267(&connection).expect("first explicit V267 reinstall should succeed");
        assert_eq!(initial, protected_schema(&connection));
        assert_current_v2_projection(&connection);

        migration_v267(&connection).expect("second explicit V267 reinstall should succeed");
        assert_eq!(initial, protected_schema(&connection));
        assert_current_v2_projection(&connection);
    }

    {
        let store = Store::open(&database).expect("repeat Store open through V267 should succeed");
        let connection = store.conn().expect("reopened V267 database should lock");
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM schema_migrations WHERE version=267",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("repeated V267 migration row should read"),
            1
        );
        assert_current_v2_projection(&connection);
        assert_preserved_roots_and_denies(&connection);
    }

    remove_sqlite_artifacts(&root, &database);
}

#[test]
fn v267_keeps_frozen_v1_policy_valid_for_historical_receipt_readback() {
    let (historical, historical_digest) = historical_supervisor_session_policy_v1_catalog()
        .expect("frozen V1 catalog should remain readable");
    validate_embedded_supervisor_session_policy_shape(&historical)
        .expect("frozen V1 policy should remain valid as embedded historical evidence");
    let (current, current_digest) =
        server_supervisor_session_policy_catalog().expect("current V2 catalog should be valid");

    assert_eq!(historical.policy_revision, 1);
    assert_eq!(current.policy_revision, 2);
    assert_ne!(historical.policy_id, current.policy_id);
    assert_ne!(historical_digest, current_digest);
}

fn assert_current_v2_projection(connection: &Connection) {
    let (current, current_digest) =
        server_supervisor_session_policy_catalog().expect("current V2 catalog should be valid");
    let (historical, historical_digest) = historical_supervisor_session_policy_v1_catalog()
        .expect("frozen V1 catalog should remain readable");
    for (kind, name) in [
        ("trigger", CURRENT_POLICY_TRIGGER),
        ("view", CURRENT_POLICY_VIEW),
    ] {
        let sql = schema_sql(connection, kind, name);
        assert!(
            sql.contains(&current.policy_id),
            "{kind} {name} lacks V2 id"
        );
        assert!(
            sql.contains(&current_digest),
            "{kind} {name} lacks V2 digest"
        );
        assert!(
            !sql.contains(&historical.policy_id),
            "{kind} {name} still projects V1 id"
        );
        assert!(
            !sql.contains(&historical_digest),
            "{kind} {name} still projects V1 digest"
        );
    }
}

fn assert_preserved_roots_and_denies(connection: &Connection) {
    let exact_roots = schema_sql(connection, "trigger", EXACT_ROOTS_TRIGGER);
    assert!(exact_roots.contains(SOURCE_CAPSULE_POLICY_DIGEST));
    for name in V254_TRIGGER_NAMES {
        let count = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name=?1",
                [name],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or_else(|error| panic!("read V254 trigger {name}: {error:#}"));
        assert_eq!(count, 1, "V254 deny trigger {name} must remain installed");
    }
}

fn protected_schema(connection: &Connection) -> BTreeMap<String, String> {
    [
        ("trigger", CURRENT_POLICY_TRIGGER),
        ("view", CURRENT_POLICY_VIEW),
        ("trigger", EXACT_ROOTS_TRIGGER),
        ("table", COMPANION_TABLE),
        ("table", REVOCATION_TABLE),
    ]
    .into_iter()
    .map(|(kind, name)| (format!("{kind}:{name}"), schema_sql(connection, kind, name)))
    .collect()
}

fn schema_sql(connection: &Connection, kind: &str, name: &str) -> String {
    connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type=?1 AND name=?2",
            [kind, name],
            |row| row.get(0),
        )
        .unwrap_or_else(|error| panic!("read {kind} {name}: {error:#}"))
}

fn remove_sqlite_artifacts(root: &std::path::Path, database: &std::path::Path) {
    for path in [
        database.to_path_buf(),
        root.join("state.sqlite-wal"),
        root.join("state.sqlite-shm"),
    ] {
        if path.exists() {
            std::fs::remove_file(&path)
                .unwrap_or_else(|error| panic!("remove {}: {error:#}", path.display()));
        }
    }
    std::fs::remove_dir(root).expect("temporary migration directory should be empty");
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
