use super::issue::{authorize_on, exchange_on};
use super::tests::{body, exchange_body, initialize, issued, PUBLIC, SESSION};
use super::*;
use std::sync::{Arc, Barrier};
use std::time::Duration;

struct Database(std::path::PathBuf);
impl Database {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "asset-access-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        )))
    }
    fn conn(&self) -> Connection {
        let conn = Connection::open(&self.0).unwrap();
        conn.busy_timeout(Duration::from_secs(10)).unwrap();
        conn
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn concurrent_code_redemption_has_exactly_one_winner_across_connections() {
    let db = Database::new();
    let mut conn = db.conn();
    initialize(&conn);
    let code = authorize_on(&mut conn, "alice", SESSION, &body(), PUBLIC).unwrap();
    drop(conn);
    let barrier = Arc::new(Barrier::new(4));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let path = db.0.clone();
        let barrier = barrier.clone();
        let input = exchange_body(&code);
        handles.push(std::thread::spawn(move || {
            let mut conn = Connection::open(path).unwrap();
            conn.busy_timeout(Duration::from_secs(10)).unwrap();
            barrier.wait();
            exchange_on(&mut conn, &input, PUBLIC).is_ok()
        }));
    }
    assert_eq!(
        handles
            .into_iter()
            .filter_map(|h| h.join().ok())
            .filter(|won| *won)
            .count(),
        1
    );
    let conn = db.conn();
    let tokens: i64 = conn
        .query_row("SELECT COUNT(*) FROM asset_access_tokens", [], |r| r.get(0))
        .unwrap();
    let consumed: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM asset_access_codes WHERE consumed_at_unix IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!((tokens, consumed), (1, 1));
}

#[test]
fn failed_token_persistence_rolls_back_code_consumption_for_safe_retry() {
    let mut conn = tests::fixture();
    let code = authorize_on(&mut conn, "alice", SESSION, &body(), PUBLIC).unwrap();
    conn.execute_batch(
        "CREATE TRIGGER synthetic_token_failure BEFORE INSERT ON asset_access_tokens
        BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
    )
    .unwrap();
    assert!(exchange_on(&mut conn, &exchange_body(&code), PUBLIC).is_err());
    let consumed: Option<i64> = conn
        .query_row("SELECT consumed_at_unix FROM asset_access_codes", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(consumed.is_none());
    conn.execute_batch("DROP TRIGGER synthetic_token_failure")
        .unwrap();
    assert!(exchange_on(&mut conn, &exchange_body(&code), PUBLIC).is_ok());
}

#[test]
fn restart_preserves_credentials_revocation_and_migration_history() {
    let db = Database::new();
    let mut conn = db.conn();
    initialize(&conn);
    let token = issued(&mut conn);
    drop(conn);
    let conn = db.conn();
    migration::migration_v289(&conn).unwrap();
    assert!(verify_read_on(
        &conn,
        &token.access_token,
        "quant.android",
        "esk.summary.read"
    )
    .is_ok());
    revoke::revoke_on(&conn, &token.grant_id, clock().unwrap()).unwrap();
    drop(conn);
    let conn = db.conn();
    assert!(verify_read_on(
        &conn,
        &token.access_token,
        "quant.android",
        "esk.summary.read"
    )
    .is_err());
    assert!(conn
        .execute(
            "DELETE FROM asset_access_grants WHERE grant_id=?1",
            params![token.grant_id]
        )
        .is_err());
}
