use rusqlite::{params, Connection};

use super::migration_v160;
use crate::open_commerce_data_request_migration::migration_v128;

#[test]
fn migration_is_idempotent_enforces_receipt_shape_and_cascades() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE users(id TEXT PRIMARY KEY);
         CREATE TABLE projects(id TEXT PRIMARY KEY);
         CREATE TABLE open_commerce_merchants(id TEXT PRIMARY KEY);
         CREATE TABLE open_commerce_consumer_relationships(id TEXT PRIMARY KEY);",
    )
    .unwrap();
    migration_v128(&conn).unwrap();
    migration_v160(&conn).unwrap();
    migration_v160(&conn).unwrap();

    conn.execute(
        "INSERT INTO users(id) VALUES ('consumer'), ('merchant-user')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO projects(id) VALUES ('consumer-project'), ('merchant-project')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO open_commerce_merchants(id) VALUES ('merchant')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO open_commerce_consumer_relationships(id) VALUES ('relationship')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO open_commerce_consumer_data_requests (
           id, consumer_project_id, consumer_user_id, merchant_project_id,
           merchant_id, relationship_id, request_type, status, subject_alias,
           requested_at, updated_at
         ) VALUES (
           'request', 'consumer-project', 'consumer', 'merchant-project',
           'merchant', 'relationship', 'erase_linked_data', 'completed', 'subject',
           '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'
         )",
        [],
    )
    .unwrap();

    let invalid_kind = insert_evidence(&conn, "invalid-kind", "invalid", &"a".repeat(64));
    assert!(invalid_kind.is_err());
    let invalid_digest = insert_evidence(&conn, "invalid-digest", "external_system_receipt", "ABC");
    assert!(invalid_digest.is_err());
    insert_evidence(
        &conn,
        "evidence-one",
        "external_system_receipt",
        &"a".repeat(64),
    )
    .unwrap();
    let duplicate = insert_evidence(
        &conn,
        "evidence-two",
        "external_system_receipt",
        &"a".repeat(64),
    );
    assert!(duplicate.is_err());

    conn.execute(
        "DELETE FROM open_commerce_consumer_data_requests WHERE id='request'",
        [],
    )
    .unwrap();
    let remaining: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM open_commerce_data_erasure_evidence",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 0);
}

fn insert_evidence(
    conn: &Connection,
    evidence_id: &str,
    evidence_kind: &str,
    receipt_sha256: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO open_commerce_data_erasure_evidence (
           id, data_request_id, merchant_project_id, merchant_id, evidence_kind,
           external_system, reference_id, receipt_sha256, summary,
           submitted_by_user_id, created_at
         ) VALUES (
           ?1, 'request', 'merchant-project', 'merchant', ?2,
           'erp', 'receipt', ?3, 'summary', 'merchant-user',
           '2026-08-01T00:00:00Z'
         )",
        params![evidence_id, evidence_kind, receipt_sha256],
    )
}
