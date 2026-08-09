use rusqlite::Connection;

use super::migration_v216;

const ENDPOINT_AUTHORITY_TABLES: [&str; 5] = [
    "node_endpoint_credentials",
    "node_endpoint_credential_versions",
    "node_endpoint_credential_revocations",
    "node_endpoint_session_authentication_receipts",
    "node_endpoint_session_heads",
];

fn prerequisite_schema() -> Connection {
    let connection = Connection::open_in_memory().expect("in-memory database should open");
    connection
        .execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE node_credentials(
                agent_id TEXT PRIMARY KEY,
                owner_user_id TEXT NOT NULL,
                install_id TEXT NOT NULL,
                secret_hash TEXT NOT NULL,
                FOREIGN KEY(owner_user_id) REFERENCES users(id)
             );",
        )
        .expect("endpoint authority prerequisites should install");
    connection
}

fn assert_endpoint_authority_is_empty(connection: &Connection) {
    for table in ENDPOINT_AUTHORITY_TABLES {
        let row_count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("endpoint authority table should be queryable");
        assert_eq!(row_count, 0, "migration must leave {table} empty");
    }
}

#[test]
fn migration_v216_creates_five_empty_without_rowid_tables() {
    let connection = prerequisite_schema();

    migration_v216(&connection).expect("endpoint authority migration should install");
    migration_v216(&connection).expect("endpoint authority migration should be idempotent");

    for table in ENDPOINT_AUTHORITY_TABLES {
        let table_sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .expect("endpoint authority table should exist");
        assert!(
            table_sql.contains("WITHOUT ROWID"),
            "{table} must be WITHOUT ROWID"
        );
    }
    assert_endpoint_authority_is_empty(&connection);
}

#[test]
fn migration_v216_does_not_backfill_or_guard_legacy_credentials() {
    let connection = prerequisite_schema();
    connection
        .execute("INSERT INTO users(id) VALUES ('owner-1')", [])
        .expect("legacy owner should insert");
    connection
        .execute(
            "INSERT INTO node_credentials(agent_id,owner_user_id,install_id,secret_hash)
             VALUES ('agent-1','owner-1','install-1','legacy-secret')",
            [],
        )
        .expect("legacy credential should insert");

    migration_v216(&connection).expect("endpoint authority migration should install");
    assert_endpoint_authority_is_empty(&connection);

    let reverse_trigger_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
              WHERE type='trigger' AND tbl_name='node_credentials'
                AND name LIKE 'trg_node_endpoint_%'",
            [],
            |row| row.get(0),
        )
        .expect("legacy reverse trigger count should be queryable");
    assert_eq!(reverse_trigger_count, 0);

    connection
        .execute(
            "UPDATE node_credentials SET secret_hash='rotated-legacy-secret'
              WHERE agent_id='agent-1'",
            [],
        )
        .expect("v216 must not block the unchanged legacy rotation path");
    connection
        .execute("DELETE FROM node_credentials WHERE agent_id='agent-1'", [])
        .expect("v216 must not block the unchanged legacy deletion path");
    assert_endpoint_authority_is_empty(&connection);
}
