use std::collections::BTreeMap;

use rusqlite::Connection;
use uuid::Uuid;

use super::migration_v271;
use crate::store::Store;

#[path = "dynamic_tests/source_bridge_fixture.rs"]
mod source_bridge_fixture;
#[path = "dynamic_tests/source_bridge_tests.rs"]
mod source_bridge_tests;

const SOURCE_TRIGGER: &str = "trg_compute_route_authorization_exact_source";

#[test]
fn v271_fresh_repeat_and_reopen_replace_only_the_source_trigger() {
    let root = std::env::temp_dir().join(format!(
        "elon-route-source-projection-v271-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary V271 directory should exist");
    let database = root.join("state.sqlite");

    let expected = {
        let store = Store::open(&database).expect("fresh Store should migrate through V271");
        let connection = store.conn().expect("fresh V271 database should lock");
        assert_migration_state(&connection);
        let protected = protected_schema(&connection);
        migration_v271(&connection).expect("explicit V271 reinstall should succeed");
        assert_eq!(protected, protected_schema(&connection));
        migration_v271(&connection).expect("repeat V271 reinstall should succeed");
        assert_eq!(protected, protected_schema(&connection));
        protected
    };

    {
        let store = Store::open(&database).expect("V271 database should reopen");
        let connection = store.conn().expect("reopened V271 database should lock");
        assert_migration_state(&connection);
        assert_eq!(expected, protected_schema(&connection));
    }

    remove_sqlite_artifacts(&root, &database);
}

#[test]
fn v271_missing_fence_fails_before_replacing_the_source_trigger() {
    let root = std::env::temp_dir().join(format!(
        "elon-route-source-projection-v271-fence-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary V271 directory should exist");
    let database = root.join("state.sqlite");
    {
        let store = Store::open(&database).expect("fresh Store should migrate through V271");
        let connection = store.conn().expect("V271 database should lock");
        let source_before = schema_sql(&connection, "trigger", SOURCE_TRIGGER);
        connection
            .execute_batch("DROP TRIGGER v254_external_pool_route_authorization_fence")
            .expect("isolated test should remove one fence");

        let error = migration_v271(&connection).expect_err("V271 must reject a missing fence");
        assert!(
            format!("{error:#}").contains("requires all 18 V254 deny fences"),
            "unexpected V271 error: {error:#}"
        );
        assert_eq!(
            source_before,
            schema_sql(&connection, "trigger", SOURCE_TRIGGER)
        );
    }
    remove_sqlite_artifacts(&root, &database);
}

#[test]
fn v271_existing_external_pool_route_fails_before_any_schema_change() {
    let connection = Connection::open_in_memory().expect("in-memory database should open");
    connection
        .execute_batch(
            "CREATE TABLE compute_route_authorization_receipts(
                 source_kind TEXT NOT NULL,
                 provider_kind TEXT NOT NULL
             );
             CREATE TRIGGER trg_compute_route_authorization_exact_source
             BEFORE INSERT ON compute_route_authorization_receipts
             BEGIN SELECT 1; END;
             INSERT INTO compute_route_authorization_receipts
             VALUES('external_pool_onboarding','external_pool');",
        )
        .expect("legacy route fixture should install");
    let source_before = schema_sql(&connection, "trigger", SOURCE_TRIGGER);

    let error = migration_v271(&connection).expect_err("V271 must reject historical routes");
    assert!(
        format!("{error:#}").contains("refuses existing external_pool route authorization rows"),
        "unexpected V271 error: {error:#}"
    );
    assert_eq!(
        source_before,
        schema_sql(&connection, "trigger", SOURCE_TRIGGER)
    );
}

fn assert_migration_state(connection: &Connection) {
    let migration_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=271",
            [],
            |row| row.get(0),
        )
        .expect("V271 migration row should read");
    assert_eq!(migration_count, 1);
    assert_eq!(
        present_v254_fences(connection).len(),
        V254_TRIGGER_NAMES.len()
    );
    let source = schema_sql(connection, "trigger", SOURCE_TRIGGER);
    assert!(source.contains("binding.route_adapter_projection_id=NEW.adapter_id"));
    assert!(source.contains("binding.route_adapter_projection_id<>source.adapter_id"));
}

fn protected_schema(connection: &Connection) -> BTreeMap<String, String> {
    let mut protected = present_v254_fences(connection);
    protected.insert(
        format!("trigger:{SOURCE_TRIGGER}"),
        schema_sql(connection, "trigger", SOURCE_TRIGGER),
    );
    protected
}

fn present_v254_fences(connection: &Connection) -> BTreeMap<String, String> {
    V254_TRIGGER_NAMES
        .iter()
        .map(|name| {
            (
                format!("trigger:{name}"),
                schema_sql(connection, "trigger", name),
            )
        })
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
    std::fs::remove_dir(root).expect("temporary V271 directory should be empty");
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
