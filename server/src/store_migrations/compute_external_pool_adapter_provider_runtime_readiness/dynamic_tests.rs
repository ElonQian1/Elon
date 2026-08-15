use std::collections::BTreeMap;

use rusqlite::Connection;
use uuid::Uuid;

use super::migration_v270;
use crate::store::Store;

const RECEIPT_TABLE: &str = "compute_external_pool_adapter_provider_runtime_readiness_receipts";
const REVOCATION_TABLE: &str =
    "compute_external_pool_adapter_provider_runtime_readiness_revocations";
const CURRENT_VIEW: &str = "compute_external_pool_adapter_provider_runtime_readiness_current";

#[test]
fn v270_fresh_repeat_and_reopen_preserve_the_exact_schema() {
    let root = std::env::temp_dir().join(format!(
        "elon-provider-runtime-readiness-v270-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary V270 directory should exist");
    let database = root.join("state.sqlite");

    let expected_schema = {
        let store = Store::open(&database).expect("fresh Store should migrate through V271");
        let connection = store.conn().expect("fresh V270 database should lock");
        assert_migration_and_empty_effects(&connection);
        let schema = v270_schema(&connection);

        migration_v270(&connection).expect("explicit V270 reinstall should succeed");
        assert_eq!(schema, v270_schema(&connection));
        migration_v270(&connection).expect("repeat V270 reinstall should succeed");
        assert_eq!(schema, v270_schema(&connection));
        schema
    };

    {
        let store = Store::open(&database).expect("V270 database should reopen");
        let connection = store.conn().expect("reopened V270 database should lock");
        assert_migration_and_empty_effects(&connection);
        assert_eq!(expected_schema, v270_schema(&connection));
    }

    remove_sqlite_artifacts(&root, &database);
}

#[test]
fn v270_registered_integrity_functions_reject_noncanonical_receipts() {
    let connection = Connection::open_in_memory().expect("in-memory database should open");
    super::register_receipt_integrity_functions(&connection)
        .expect("V270 integrity functions should register");

    for function in [
        "elon_v270_provider_runtime_readiness_receipt_is_exact",
        "elon_v270_provider_runtime_readiness_revocation_is_exact",
    ] {
        let malformed: i64 = connection
            .query_row(&format!("SELECT {function}('{{}}')"), [], |row| row.get(0))
            .unwrap_or_else(|error| panic!("invoke {function}: {error:#}"));
        let non_object: i64 = connection
            .query_row(&format!("SELECT {function}('null')"), [], |row| row.get(0))
            .unwrap_or_else(|error| panic!("invoke {function}: {error:#}"));
        assert_eq!(malformed, 0, "{function} accepted an incomplete receipt");
        assert_eq!(non_object, 0, "{function} accepted a non-object receipt");
    }
}

fn assert_migration_and_empty_effects(connection: &Connection) {
    let migration_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=270",
            [],
            |row| row.get(0),
        )
        .expect("V270 migration row should read");
    assert_eq!(migration_count, 1);

    for table in [RECEIPT_TABLE, REVOCATION_TABLE] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|error| panic!("count {table}: {error:#}"));
        assert_eq!(count, 0, "migration unexpectedly created rows in {table}");
    }
    assert!(v270_schema(connection).contains_key(&format!("view:{CURRENT_VIEW}")));
}

fn v270_schema(connection: &Connection) -> BTreeMap<String, String> {
    let mut statement = connection
        .prepare(
            "SELECT type,name,sql FROM sqlite_master
             WHERE sql IS NOT NULL AND (
               name LIKE 'compute_external_pool_adapter_provider_runtime_readiness_%'
               OR name LIKE 'v270_provider_runtime_readiness_%'
             ) ORDER BY type,name",
        )
        .expect("V270 schema query should prepare");
    statement
        .query_map([], |row| {
            let kind: String = row.get(0)?;
            let name: String = row.get(1)?;
            let sql: String = row.get(2)?;
            Ok((format!("{kind}:{name}"), sql))
        })
        .expect("V270 schema query should run")
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()
        .expect("V270 schema rows should decode")
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
    std::fs::remove_dir(root).expect("temporary V270 directory should be empty");
}
