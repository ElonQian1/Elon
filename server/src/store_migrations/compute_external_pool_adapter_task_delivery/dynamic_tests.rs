use std::collections::BTreeMap;

use rusqlite::Connection;
use uuid::Uuid;

use super::migration_v273;
use crate::store::Store;

const TABLES: [&str; 6] = [
    "compute_external_pool_adapter_task_exchange_attempts",
    "compute_external_pool_adapter_task_exchange_receipts",
    "compute_external_pool_adapter_task_reconcile_polls",
    "compute_external_pool_adapter_task_event_polls",
    "compute_external_pool_adapter_task_event_batches",
    "compute_external_pool_adapter_task_events",
];

#[test]
fn v273_task_protocol_production_fresh_repeat_and_reopen_preserve_dormant_schema() {
    let root = std::env::temp_dir().join(format!(
        "elon-task-protocol-production-v273-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary V273 directory should exist");
    let database = root.join("state.sqlite");

    let (expected_schema, expected_fences) = {
        let store = Store::open(&database).expect("fresh Store should migrate through V273");
        assert_eq!(
            store
                .recover_external_pool_adapter_task_delivery()
                .expect("fresh V273 recovery should stay dormant"),
            0
        );
        let connection = store.conn().expect("fresh V273 database should lock");
        assert_migration_and_empty_effects(&connection);
        let schema = v273_schema(&connection);
        let fences = v254_fences(&connection);

        migration_v273(&connection).expect("explicit V273 reinstall should succeed");
        assert_eq!(schema, v273_schema(&connection));
        assert_eq!(fences, v254_fences(&connection));
        migration_v273(&connection).expect("repeat V273 reinstall should succeed");
        assert_eq!(schema, v273_schema(&connection));
        assert_eq!(fences, v254_fences(&connection));
        (schema, fences)
    };

    {
        let store = Store::open(&database).expect("V273 database should reopen");
        assert_eq!(
            store
                .recover_external_pool_adapter_task_delivery()
                .expect("reopened V273 recovery should stay dormant"),
            0
        );
        let connection = store.conn().expect("reopened V273 database should lock");
        assert_migration_and_empty_effects(&connection);
        assert_eq!(expected_schema, v273_schema(&connection));
        assert_eq!(expected_fences, v254_fences(&connection));
    }

    remove_sqlite_artifacts(&root, &database);
}

#[test]
fn v273_task_protocol_production_integrity_functions_reject_malformed_envelopes() {
    let connection = Connection::open_in_memory().expect("in-memory database should open");
    super::register_receipt_integrity_functions(&connection)
        .expect("V273 integrity functions should register");

    for function in [
        "elon_v273_task_exchange_attempt_is_exact",
        "elon_v273_task_exchange_receipt_is_exact",
        "elon_v273_task_reconcile_poll_is_exact",
        "elon_v273_task_event_poll_is_exact",
        "elon_v273_task_event_batch_is_exact",
        "elon_v273_task_event_is_exact",
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
}

fn assert_migration_and_empty_effects(connection: &Connection) {
    let migration_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=273",
            [],
            |row| row.get(0),
        )
        .expect("V273 migration row should read");
    assert_eq!(migration_count, 1);

    for table in TABLES {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|error| panic!("count {table}: {error:#}"));
        assert_eq!(count, 0, "migration unexpectedly created rows in {table}");
    }

    let schema = v273_schema(connection);
    assert_eq!(
        schema
            .keys()
            .filter(|key| key.starts_with("table:compute_external_pool_adapter_task_"))
            .count(),
        TABLES.len()
    );
    assert!(!schema.keys().any(|key| key.starts_with("view:")));
    assert_eq!(v254_fences(connection).len(), 18);
}

fn v273_schema(connection: &Connection) -> BTreeMap<String, String> {
    schema_matching(
        connection,
        "name LIKE 'compute_external_pool_adapter_task_exchange_%'
         OR name IN (
           'compute_external_pool_adapter_task_reconcile_polls',
           'compute_external_pool_adapter_task_event_polls',
           'compute_external_pool_adapter_task_event_batches',
           'compute_external_pool_adapter_task_events'
         )
         OR name LIKE 'v273_task_%'",
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
    std::fs::remove_dir(root).expect("temporary V273 directory should be empty");
}
