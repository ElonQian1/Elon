use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::{
    open_commerce_adapter_claim_service,
    open_commerce_adapter_claim_tests::{claim_enabled_credential, fixture_at_path},
    store_migrations::MIGRATIONS,
};

#[test]
fn v137_disk_database_upgrades_and_claims_a_terminal_order() {
    let root = std::env::temp_dir().join(format!(
        "elon-open-commerce-claim-v137-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary migration directory should exist");
    let database = root.join("state.sqlite");
    let connection = Connection::open(&database).expect("legacy database should open");
    connection
        .execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_migrations (
               version INTEGER PRIMARY KEY,
               applied_at TEXT NOT NULL
             );",
        )
        .expect("legacy migration ledger should initialize");
    for (version, _, apply) in MIGRATIONS.iter().filter(|(version, _, _)| *version <= 137) {
        apply(&connection).unwrap_or_else(|error| panic!("migration v{version} failed: {error:#}"));
        connection
            .execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
                params![version, "2026-08-01T00:00:00Z"],
            )
            .unwrap_or_else(|error| panic!("migration v{version} ledger failed: {error:#}"));
    }
    assert_eq!(latest_migration(&connection), 137);
    assert_eq!(
        table_count(&connection, "open_commerce_business_handoff_claims"),
        0
    );
    drop(connection);

    let fixture = fixture_at_path(database.clone());
    let credential = claim_enabled_credential(&fixture);
    let poll = open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 300)
        .expect("upgraded database should claim a terminal order");
    let issue = poll
        .issue
        .expect("upgraded database should issue one lease");
    assert_eq!(issue.claim.invocation_id, fixture.invocation_id);
    assert_eq!(issue.claim.attempt_no, 1);

    let connection = fixture.store.conn().unwrap();
    assert_eq!(
        latest_migration(&connection),
        i64::from(
            MIGRATIONS
                .last()
                .expect("migration list should not be empty")
                .0
        )
    );
    assert_eq!(
        table_count(&connection, "open_commerce_business_handoff_claims"),
        1
    );
    assert_eq!(claim_column_count(&connection), 9);
    drop(connection);
    drop(fixture);
    cleanup_database(&root, &database);
}

fn latest_migration(connection: &Connection) -> i64 {
    connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("latest migration should be readable")
}

fn table_count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |row| row.get(0),
        )
        .expect("table count should be readable")
}

fn claim_column_count(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('open_commerce_business_handoff_claims')
             WHERE name IN (
               'lease_deadline_at', 'release_reason_code', 'released_at',
               'completion_status', 'retry_not_before', 'retry_suspended_at',
               'retry_suspension_reason', 'retry_resumed_at',
               'retry_resumed_by_user_id'
             )",
            [],
            |row| row.get(0),
        )
        .expect("claim columns should be readable")
}

fn cleanup_database(root: &std::path::Path, database: &std::path::Path) {
    for path in [
        database.to_path_buf(),
        root.join("state.sqlite-wal"),
        root.join("state.sqlite-shm"),
    ] {
        if path.exists() {
            std::fs::remove_file(path).expect("temporary database artifact should be removable");
        }
    }
    std::fs::remove_dir(root).expect("temporary migration directory should be empty");
}
