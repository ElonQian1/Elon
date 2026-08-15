use std::collections::BTreeMap;

use rusqlite::Connection;
use uuid::Uuid;

use super::migration_v268;
use crate::store::Store;

const CHALLENGES: &str =
    "compute_external_pool_adapter_runtime_compatibility_verification_challenges";
const OBSERVATIONS: &str =
    "compute_external_pool_adapter_runtime_compatibility_verification_run_observations";
const RECEIPTS: &str = "compute_external_pool_adapter_runtime_compatibility_verification_receipts";
const REVOCATIONS: &str =
    "compute_external_pool_adapter_runtime_compatibility_verification_revocations";
const CURRENT: &str = "compute_external_pool_adapter_runtime_compatibility_verification_current";

#[test]
fn v268_fresh_repeat_and_reopen_preserve_the_dormant_schema() {
    let root = std::env::temp_dir().join(format!(
        "elon-runtime-compatibility-v268-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary V268 directory should exist");
    let database = root.join("state.sqlite");

    let (expected_schema, expected_fences) = {
        let store = Store::open(&database).expect("fresh Store should migrate through V268");
        assert_eq!(
            store
                .recover_external_pool_adapter_task_delivery()
                .expect("fresh production task recovery should remain dormant"),
            0
        );
        let connection = store.conn().expect("fresh V268 database should lock");
        assert_migration_and_dormant_effects(&connection);
        let schema = v268_schema(&connection);
        let fences = v254_fences(&connection);

        migration_v268(&connection).expect("explicit V268 reinstall should succeed");
        assert_migration_and_dormant_effects(&connection);
        assert_eq!(schema, v268_schema(&connection));
        assert_eq!(fences, v254_fences(&connection));

        migration_v268(&connection).expect("repeat V268 reinstall should succeed");
        assert_migration_and_dormant_effects(&connection);
        assert_eq!(schema, v268_schema(&connection));
        assert_eq!(fences, v254_fences(&connection));
        (schema, fences)
    };

    {
        let store = Store::open(&database).expect("V268 database should reopen");
        assert_eq!(
            store
                .recover_external_pool_adapter_task_delivery()
                .expect("reopened production task recovery should remain dormant"),
            0
        );
        let connection = store.conn().expect("reopened V268 database should lock");
        assert_migration_and_dormant_effects(&connection);
        assert_eq!(expected_schema, v268_schema(&connection));
        assert_eq!(expected_fences, v254_fences(&connection));
    }

    remove_sqlite_artifacts(&root, &database);
}

#[test]
fn v268_integrity_functions_reject_malformed_envelopes() {
    let connection = Connection::open_in_memory().expect("in-memory database should open");
    super::register_receipt_integrity_functions(&connection)
        .expect("V268 integrity functions should register");

    for function in [
        "elon_v268_runtime_compatibility_challenge_is_exact",
        "elon_v268_runtime_compatibility_revocation_is_exact",
    ] {
        for malformed in ["{}", "null", "[]", "not-json"] {
            assert_integrity_result(&connection, function, &[malformed], 0);
        }
    }

    for malformed in ["{}", "null", "[]", "not-json"] {
        assert_integrity_result(
            &connection,
            "elon_v268_runtime_compatibility_observation_is_exact",
            &[malformed, malformed],
            0,
        );
        assert_integrity_result(
            &connection,
            "elon_v268_runtime_compatibility_verification_is_exact",
            &[malformed, malformed, malformed, malformed],
            0,
        );
    }
}

fn assert_integrity_result(
    connection: &Connection,
    function: &str,
    values: &[&str],
    expected: i64,
) {
    let placeholders = (1..=values.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let actual: i64 = connection
        .query_row(
            &format!("SELECT {function}({placeholders})"),
            rusqlite::params_from_iter(values.iter().copied()),
            |row| row.get(0),
        )
        .unwrap_or_else(|error| panic!("invoke {function}: {error:#}"));
    assert_eq!(actual, expected, "unexpected {function} result");
}

fn assert_migration_and_dormant_effects(connection: &Connection) {
    let migration_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=268",
            [],
            |row| row.get(0),
        )
        .expect("V268 migration row should read");
    assert_eq!(migration_count, 1);

    for relation in [CHALLENGES, OBSERVATIONS, RECEIPTS, REVOCATIONS, CURRENT] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {relation}"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|error| panic!("count {relation}: {error:#}"));
        assert_eq!(count, 0, "V268 unexpectedly created rows in {relation}");
    }

    let mut statement = connection
        .prepare(
            "SELECT type,name FROM sqlite_master
             WHERE type IN ('table','view')
               AND name LIKE 'compute_external_pool_adapter_runtime_compatibility_verification_%'
             ORDER BY type,name",
        )
        .expect("V268 namespace query should prepare");
    let relations = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("V268 namespace query should run")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("V268 namespace should decode");
    assert_eq!(
        relations,
        vec![
            ("table".into(), CHALLENGES.into()),
            ("table".into(), RECEIPTS.into()),
            ("table".into(), REVOCATIONS.into()),
            ("table".into(), OBSERVATIONS.into()),
            ("view".into(), CURRENT.into()),
        ]
    );
    assert_eq!(v254_fences(connection).len(), 18);
}

fn v268_schema(connection: &Connection) -> BTreeMap<String, String> {
    schema_matching(
        connection,
        "name LIKE 'compute_external_pool_adapter_runtime_compatibility_verification_%'
         OR name LIKE 'v268_runtime_compatibility_%'
         OR name LIKE 'idx_v268_%'",
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
    std::fs::remove_dir(root).expect("temporary V268 directory should be empty");
}
