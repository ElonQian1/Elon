use super::*;
use crate::{platform_migration::migration_v287, sellback_migration::migration_v288};
use rusqlite::{params, Connection};

fn empty_database() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../src/store_migrations/migrations_v1_v16.rs"
    ));
    let start = source.find("CREATE TABLE IF NOT EXISTS users (").unwrap();
    let end = start + source[start..].find(");").unwrap() + 2;
    conn.execute_batch(&source[start..end]).unwrap();
    conn
}

fn schema(conn: &Connection) -> Vec<(String, String, Option<String>)> {
    conn.prepare("SELECT type,name,sql FROM sqlite_master ORDER BY type,name")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

#[test]
fn fresh_and_populated_v287_upgrade_are_reentrant_and_preserve_formal_and_paper() {
    let fresh = empty_database();
    migration_v287(&fresh).unwrap();
    migration_v288(&fresh).unwrap();
    let installed = schema(&fresh);
    migration_v288(&fresh).unwrap();
    assert_eq!(schema(&fresh), installed);

    let (fixture, _, config) = setup();
    let before = fixture
        .store
        .esk_platform_history("alice", &token("alice"), 20, None)
        .unwrap();
    let paper = fixture.paper_total();
    let conn = fixture.store.conn().unwrap();
    // Only the empty V288 tables in this owned synthetic fixture are removed to model V287.
    conn.execute_batch("DROP TABLE esk_platform_sellback_cancellations; DROP TABLE esk_platform_sellback_requests;").unwrap();
    migration_v288(&conn).unwrap();
    let installed = schema(&conn);
    migration_v288(&conn).unwrap();
    assert_eq!(schema(&conn), installed);
    drop(conn);
    let after = fixture
        .store
        .esk_platform_history("alice", &token("alice"), 20, None)
        .unwrap();
    assert_eq!(before.snapshot_digest, after.snapshot_digest);
    assert_eq!(before.total_base_units, after.total_base_units);
    assert_eq!(before.entry_count, after.entry_count);
    assert_eq!(fixture.paper_total(), paper);
    assert_eq!(
        page(&fixture, "alice", &config).summary.reserved_base_units,
        0
    );
}

#[test]
fn success_inside_outer_transaction_does_not_commit_the_outer_transaction() {
    let mut conn = empty_database();
    migration_v287(&conn).unwrap();
    let before = schema(&conn);
    let tx = conn.transaction().unwrap();
    migration_v288(&tx).unwrap();
    migration_v288(&tx).unwrap();
    assert!(!tx.is_autocommit());
    tx.rollback().unwrap();
    assert_eq!(schema(&conn), before);
    migration_v288(&conn).unwrap();
}

#[test]
fn schema_conflict_rolls_back_partial_ddl_and_leaves_outer_transaction_usable() {
    let mut conn = empty_database();
    migration_v287(&conn).unwrap();
    conn.execute_batch(
        "CREATE VIEW esk_platform_sellback_cancellations AS SELECT 'synthetic-conflict' AS marker;
        CREATE TABLE synthetic_outer_marker(value TEXT);",
    )
    .unwrap();
    let before = schema(&conn);
    assert!(migration_v288(&conn).is_err());
    assert!(conn.is_autocommit());
    assert_eq!(schema(&conn), before);
    let tx = conn.transaction().unwrap();
    tx.execute("INSERT INTO synthetic_outer_marker VALUES('before')", [])
        .unwrap();
    assert!(migration_v288(&tx).is_err());
    assert!(!tx.is_autocommit());
    assert_eq!(schema(&tx), before);
    tx.execute("INSERT INTO synthetic_outer_marker VALUES('after')", [])
        .unwrap();
    assert_eq!(
        tx.query_row("SELECT COUNT(*) FROM synthetic_outer_marker", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    tx.rollback().unwrap();
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM synthetic_outer_marker", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    conn.execute_batch("DROP VIEW esk_platform_sellback_cancellations")
        .unwrap();
    migration_v288(&conn).unwrap();
}

#[test]
fn both_real_event_tables_are_append_only_and_unique() {
    let (fixture, _, config) = setup();
    let accepted = submit(&fixture, "alice", "synthetic-unique", 1, &config);
    let id = &accepted.request.request_id;
    let canceled = fixture
        .store
        .cancel_esk_platform_sellback("alice", &token("alice"), id, &config)
        .unwrap();
    let before = page(&fixture, "alice", &config);
    let conn = fixture.store.conn().unwrap();
    for table in [
        "esk_platform_sellback_requests",
        "esk_platform_sellback_cancellations",
    ] {
        assert!(conn
            .execute(&format!("UPDATE {table} SET created_at=created_at"), [])
            .is_err());
        assert!(conn.execute(&format!("DELETE FROM {table}"), []).is_err());
    }
    for (copy_id, copy_key) in [
        (id.clone(), "different-key".to_owned()),
        (
            format!("eskpsr_{}", "1".repeat(32)),
            "synthetic-unique".to_owned(),
        ),
    ] {
        assert!(conn.execute("INSERT INTO esk_platform_sellback_requests SELECT ?1,user_id,?2,
            amount_base_units,request_digest,input_json,policy_json,platform_policy_digest,source_fingerprint,created_at
            FROM esk_platform_sellback_requests WHERE request_id=?3", params![copy_id,copy_key,id]).is_err());
    }
    for event_id in [
        canceled.request.cancel_event_id.unwrap(),
        format!("eskpsc_{}", "1".repeat(32)),
    ] {
        assert!(conn.execute("INSERT INTO esk_platform_sellback_cancellations SELECT ?1,request_id,
            request_digest,canceled_by,created_at FROM esk_platform_sellback_cancellations WHERE request_id=?2",
            params![event_id,id]).is_err());
    }
    migration_v288(&conn).unwrap();
    drop(conn);
    assert_eq!(page(&fixture, "alice", &config), before);
    assert_eq!(fixture.count("esk_platform_sellback_requests"), 1);
    assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 1);
}

#[test]
fn sql_bindings_reject_invalid_source_identity_amount_and_cancel_owner() {
    let (fixture, formal, config) = setup();
    let accepted = submit(&fixture, "alice", "synthetic-binding", 1, &config);
    let id = &accepted.request.request_id;
    let conn = fixture.store.conn().unwrap();
    let new_id = format!("eskpsr_{}", "2".repeat(32));
    for (user, units, source, policy) in [
        (
            "inactive-user",
            1,
            formal.source_fingerprint.clone(),
            formal.policy_digest.clone(),
        ),
        (
            "missing-user",
            1,
            formal.source_fingerprint.clone(),
            formal.policy_digest.clone(),
        ),
        (
            "local-owner",
            1,
            formal.source_fingerprint.clone(),
            formal.policy_digest.clone(),
        ),
        (
            "alice",
            0,
            formal.source_fingerprint.clone(),
            formal.policy_digest.clone(),
        ),
        (
            "alice",
            -1,
            formal.source_fingerprint.clone(),
            formal.policy_digest.clone(),
        ),
        ("alice", 1, "f".repeat(64), formal.policy_digest.clone()),
        (
            "alice",
            1,
            formal.source_fingerprint.clone(),
            "f".repeat(64),
        ),
    ] {
        assert!(conn
            .execute(
                "INSERT INTO esk_platform_sellback_requests SELECT ?1,?2,'new-key',?3,
            request_digest,input_json,policy_json,?4,?5,created_at
            FROM esk_platform_sellback_requests WHERE request_id=?6",
                params![new_id, user, units, policy, source, id]
            )
            .is_err());
    }
    let cancel_id = format!("eskpsc_{}", "3".repeat(32));
    for (request, owner, digest) in [
        (id.clone(), "bob", accepted.request.request_digest.clone()),
        (id.clone(), "alice", "f".repeat(64)),
        (
            format!("eskpsr_{}", "3".repeat(32)),
            "alice",
            accepted.request.request_digest.clone(),
        ),
    ] {
        assert!(conn
            .execute(
                "INSERT INTO esk_platform_sellback_cancellations VALUES(?1,?2,?3,?4,?5)",
                params![
                    cancel_id,
                    request,
                    digest,
                    owner,
                    accepted.request.created_at
                ]
            )
            .is_err());
    }
    drop(conn);
    assert_eq!(fixture.count("esk_platform_sellback_requests"), 1);
    assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 0);
    assert_eq!(
        page(&fixture, "alice", &config).summary.reserved_base_units,
        1
    );
}
