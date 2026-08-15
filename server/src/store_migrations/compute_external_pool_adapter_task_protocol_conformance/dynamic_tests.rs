use std::collections::BTreeMap;

use rusqlite::{params, Connection};
use uuid::Uuid;

use super::migration_v272;
use crate::{
    compute_federation::external_pool_adapter_task_protocol_conformance::task_protocol_conformance_receipt_integrity_digest,
    store::Store,
};

const RUN_TABLE: &str = "compute_external_pool_adapter_task_protocol_conformance_run_receipts";
const REVOCATION_TABLE: &str =
    "compute_external_pool_adapter_task_protocol_conformance_revocations";
const CURRENT_VIEW: &str = "compute_external_pool_adapter_task_protocol_conformance_current";

#[test]
fn v272_fresh_repeat_and_reopen_preserve_schema_and_v254_fences() {
    let root = std::env::temp_dir().join(format!(
        "elon-task-protocol-conformance-v272-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary V272 directory should exist");
    let database = root.join("state.sqlite");

    let (expected_schema, expected_fences) = {
        let store = Store::open(&database).expect("fresh Store should migrate through V272");
        let connection = store.conn().expect("fresh V272 database should lock");
        assert_migration_and_empty_effects(&connection);
        let schema = v272_schema(&connection);
        let fences = v254_fences(&connection);

        migration_v272(&connection).expect("explicit V272 reinstall should succeed");
        assert_eq!(schema, v272_schema(&connection));
        assert_eq!(fences, v254_fences(&connection));
        migration_v272(&connection).expect("repeat V272 reinstall should succeed");
        assert_eq!(schema, v272_schema(&connection));
        assert_eq!(fences, v254_fences(&connection));
        (schema, fences)
    };

    {
        let store = Store::open(&database).expect("V272 database should reopen");
        let connection = store.conn().expect("reopened V272 database should lock");
        assert_migration_and_empty_effects(&connection);
        assert_eq!(expected_schema, v272_schema(&connection));
        assert_eq!(expected_fences, v254_fences(&connection));
    }

    remove_sqlite_artifacts(&root, &database);
}

#[test]
fn v272_registered_integrity_functions_fail_closed_and_accept_exact_integrity() {
    let connection = Connection::open_in_memory().expect("in-memory database should open");
    super::register_receipt_integrity_functions(&connection)
        .expect("V272 integrity functions should register");

    for function in [
        "elon_v272_task_protocol_conformance_run_receipt_is_exact",
        "elon_v272_task_protocol_conformance_revocation_receipt_is_exact",
    ] {
        for malformed in ["{}", "null", "[]", "not-json"] {
            let accepted: i64 = connection
                .query_row(&format!("SELECT {function}(?1)"), [malformed], |row| {
                    row.get(0)
                })
                .unwrap_or_else(|error| panic!("invoke {function}: {error:#}"));
            assert_eq!(accepted, 0, "{function} accepted {malformed}");
        }
    }

    let run_digest = "11".repeat(32);
    let custody_epoch = "22".repeat(32);
    let process_seal = "33".repeat(32);
    let integrity = task_protocol_conformance_receipt_integrity_digest(
        &run_digest,
        &custody_epoch,
        &process_seal,
    )
    .expect("exact V272 integrity digest should derive");
    let exact: i64 = connection
        .query_row(
            "SELECT elon_v272_task_protocol_conformance_receipt_integrity_is_exact(?1,?2,?3,?4)",
            params![run_digest, custody_epoch, process_seal, integrity],
            |row| row.get(0),
        )
        .expect("exact V272 integrity should evaluate");
    assert_eq!(exact, 1);

    let drifted: i64 = connection
        .query_row(
            "SELECT elon_v272_task_protocol_conformance_receipt_integrity_is_exact(?1,?2,?3,?4)",
            params![run_digest, custody_epoch, process_seal, "44".repeat(32)],
            |row| row.get(0),
        )
        .expect("drifted V272 integrity should evaluate");
    assert_eq!(drifted, 0);
}

fn assert_migration_and_empty_effects(connection: &Connection) {
    let migration_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=272",
            [],
            |row| row.get(0),
        )
        .expect("V272 migration row should read");
    assert_eq!(migration_count, 1);

    for table in [RUN_TABLE, REVOCATION_TABLE] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|error| panic!("count {table}: {error:#}"));
        assert_eq!(count, 0, "migration unexpectedly created rows in {table}");
    }

    let schema = v272_schema(connection);
    assert!(schema.contains_key(&format!("table:{RUN_TABLE}")));
    assert!(schema.contains_key(&format!("table:{REVOCATION_TABLE}")));
    let view = schema
        .get(&format!("view:{CURRENT_VIEW}"))
        .expect("V272 currentness view should exist");
    assert!(view.contains("relationally_current_requires_process_custody_and_prepared_reproof"));
    assert_eq!(v254_fences(connection).len(), 18);
}

fn v272_schema(connection: &Connection) -> BTreeMap<String, String> {
    schema_matching(
        connection,
        "name LIKE 'compute_external_pool_adapter_task_protocol_conformance_%'\
         OR name LIKE 'v272_task_protocol_conformance_%'",
    )
}

fn v254_fences(connection: &Connection) -> BTreeMap<String, String> {
    schema_matching(
        connection,
        "type='trigger' AND name LIKE 'v254_external_pool_%_fence'",
    )
}

fn schema_matching(connection: &Connection, predicate: &str) -> BTreeMap<String, String> {
    let sql = format!(
        "SELECT type,name,sql FROM sqlite_master
         WHERE sql IS NOT NULL AND ({predicate}) ORDER BY type,name"
    );
    let mut statement = connection
        .prepare(&sql)
        .unwrap_or_else(|error| panic!("prepare schema query: {error:#}"));
    statement
        .query_map([], |row| {
            let kind: String = row.get(0)?;
            let name: String = row.get(1)?;
            let sql: String = row.get(2)?;
            Ok((format!("{kind}:{name}"), sql))
        })
        .expect("schema query should run")
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()
        .expect("schema rows should decode")
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
    std::fs::remove_dir(root).expect("temporary V272 directory should be empty");
}
