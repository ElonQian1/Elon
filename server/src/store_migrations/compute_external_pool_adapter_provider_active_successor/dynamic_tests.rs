use std::collections::BTreeMap;

use rusqlite::{params, Connection};
use uuid::Uuid;

use super::migration_v274;
use crate::{
    compute_federation::external_pool_adapter_provider_active_successor::{
        provider_active_successor_private_integrity_digest,
        ExternalPoolAdapterProviderActiveSuccessorProcessCustody,
        PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
    },
    store::Store,
};

const RECEIPTS: &str = "compute_external_pool_adapter_provider_active_successor_receipts";
const REVOCATIONS: &str = "compute_external_pool_adapter_provider_active_successor_revocations";
const CURRENT: &str = "compute_external_pool_adapter_provider_active_successor_current";

#[test]
fn v274_fresh_repeat_and_reopen_preserve_the_dormant_schema() {
    let root = std::env::temp_dir().join(format!(
        "elon-provider-active-successor-v274-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary V274 directory should exist");
    let database = root.join("state.sqlite");

    let (expected_schema, expected_fences) = {
        let store = Store::open(&database).expect("fresh Store should migrate through V274");
        assert_eq!(
            store
                .recover_external_pool_adapter_task_delivery()
                .expect("fresh V273 recovery should remain dormant"),
            0
        );
        let connection = store.conn().expect("fresh V274 database should lock");
        assert_migration_and_dormant_effects(&connection);
        let schema = v274_schema(&connection);
        let fences = v254_fences(&connection);

        migration_v274(&connection).expect("explicit V274 reinstall should succeed");
        assert_migration_and_dormant_effects(&connection);
        assert_eq!(schema, v274_schema(&connection));
        assert_eq!(fences, v254_fences(&connection));

        migration_v274(&connection).expect("repeat V274 reinstall should succeed");
        assert_migration_and_dormant_effects(&connection);
        assert_eq!(schema, v274_schema(&connection));
        assert_eq!(fences, v254_fences(&connection));
        (schema, fences)
    };

    {
        let store = Store::open(&database).expect("V274 database should reopen");
        assert_eq!(
            store
                .recover_external_pool_adapter_task_delivery()
                .expect("reopened V273 recovery should remain dormant"),
            0
        );
        let connection = store.conn().expect("reopened V274 database should lock");
        assert_migration_and_dormant_effects(&connection);
        assert_eq!(expected_schema, v274_schema(&connection));
        assert_eq!(expected_fences, v254_fences(&connection));
    }

    remove_sqlite_artifacts(&root, &database);
}

#[test]
fn v274_integrity_functions_reject_malformed_and_unsealed_values() {
    let connection = Connection::open_in_memory().expect("in-memory database should open");
    super::register_receipt_integrity_functions(&connection)
        .expect("V274 integrity functions should register");

    for function in [
        "elon_v274_provider_active_successor_receipt_is_exact",
        "elon_v274_provider_active_successor_revocation_is_exact",
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

    let custody = ExternalPoolAdapterProviderActiveSuccessorProcessCustody {
        process_custody_epoch_digest: "a".repeat(64),
        process_custody_nonce_digest: "b".repeat(64),
        process_custody_seal_digest: "c".repeat(64),
    };
    let entity_digest = "d".repeat(64);
    let integrity_digest = provider_active_successor_private_integrity_digest(
        PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
        &entity_digest,
        &custody,
    )
    .expect("exact V274 private integrity digest should derive");
    let accepted: i64 = connection
        .query_row(
            "SELECT elon_v274_provider_active_successor_receipt_integrity_is_exact(\
             ?1,?2,?3,?4,?5,?6)",
            params![
                PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
                entity_digest,
                custody.process_custody_epoch_digest,
                custody.process_custody_nonce_digest,
                custody.process_custody_seal_digest,
                integrity_digest,
            ],
            |row| row.get(0),
        )
        .expect("exact V274 integrity tuple should evaluate");
    assert_eq!(accepted, 1);

    let pending: i64 = connection
        .query_row(
            "SELECT elon_v274_provider_active_successor_pending_process_seal_is_exact(\
             ?1,?2,?3,?4,?5,?6,?7)",
            params![
                PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
                "unregistered-receipt",
                "d".repeat(64),
                "a".repeat(64),
                "b".repeat(64),
                "c".repeat(64),
                integrity_digest,
            ],
            |row| row.get(0),
        )
        .expect("unregistered pending seal should evaluate");
    assert_eq!(pending, 0, "migration must not create process authority");
}

fn assert_migration_and_dormant_effects(connection: &Connection) {
    let migration_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=274",
            [],
            |row| row.get(0),
        )
        .expect("V274 migration row should read");
    assert_eq!(migration_count, 1);

    for relation in [RECEIPTS, REVOCATIONS, CURRENT] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {relation}"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|error| panic!("count {relation}: {error:#}"));
        assert_eq!(count, 0, "V274 unexpectedly created rows in {relation}");
    }

    let mut statement = connection
        .prepare(
            "SELECT type,name FROM sqlite_master
             WHERE type IN ('table','view')
               AND name LIKE 'compute_external_pool_adapter_provider_active_successor_%'
             ORDER BY type,name",
        )
        .expect("V274 namespace query should prepare");
    let relations = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("V274 namespace query should run")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("V274 namespace should decode");
    assert_eq!(
        relations,
        vec![
            ("table".into(), RECEIPTS.into()),
            ("table".into(), REVOCATIONS.into()),
            ("view".into(), CURRENT.into()),
        ]
    );
    assert_eq!(v254_fences(connection).len(), 18);
}

fn v274_schema(connection: &Connection) -> BTreeMap<String, String> {
    schema_matching(
        connection,
        "name LIKE 'compute_external_pool_adapter_provider_active_successor_%'
         OR name LIKE 'v274_provider_active_successor_%'
         OR name LIKE 'idx_v274_active_successor_%'
         OR name IN (
           'compute_external_pool_adapter_credential_reattestation_current',
           'external_pool_adapter_credential_reattestation_challenge_exact_roots',
           'external_pool_adapter_credential_reattestation_receipt_current_roots'
         )",
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
    std::fs::remove_dir(root).expect("temporary V274 directory should be empty");
}
