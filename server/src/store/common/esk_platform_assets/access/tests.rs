use super::issue::{authorize_on, exchange_at, exchange_on};
use super::*;
use rusqlite::Connection;

pub(super) const PUBLIC: &str = "https://main.example.test";
pub(super) const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
pub(super) const SESSION: &str = "synthetic-master-session-never-exported";

pub(super) fn initialize(conn: &Connection) {
    conn.execute_batch("PRAGMA foreign_keys=ON;
        CREATE TABLE users(id TEXT PRIMARY KEY,status TEXT NOT NULL,nickname TEXT);
        CREATE TABLE sessions(id TEXT PRIMARY KEY,user_id TEXT NOT NULL REFERENCES users(id),
            token_hash TEXT NOT NULL UNIQUE,expires_at TEXT NOT NULL,revoked_at TEXT);
        INSERT INTO users VALUES('alice','active','A'),('bob','active','B'),('local-owner','active','Virtual');").unwrap();
    conn.execute(
        "INSERT INTO sessions VALUES('session-a','alice',?1,?2,NULL)",
        params![
            hash_token(SESSION),
            timestamp(clock().unwrap() + 7200).unwrap()
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO sessions VALUES('session-b','bob',?1,?2,NULL)",
        params![
            hash_token("synthetic-bob"),
            timestamp(clock().unwrap() + 7200).unwrap()
        ],
    )
    .unwrap();
    migration::migration_v289(conn).unwrap();
    migration::migration_v289(conn).unwrap();
}

pub(super) fn fixture() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    initialize(&conn);
    conn
}

pub(super) fn body() -> AuthorizeBody {
    AuthorizeBody {
        schema: AUTHORIZE_SCHEMA.into(),
        client_id: "quant.android".into(),
        redirect_uri: "com.elon.quant:/asset-access/callback".into(),
        state: "s".repeat(32),
        code_challenge: challenge(VERIFIER).unwrap(),
        code_challenge_method: "S256".into(),
        scopes: vec![AccessScope::EskSummaryRead],
        expires_in: 3600,
        explicit_consent: true,
        confirmation: AUTHORIZE_CONFIRMATION.into(),
    }
}

pub(super) fn exchange_body(code: &AuthorizationCode) -> TokenBody {
    TokenBody {
        schema: TOKEN_SCHEMA.into(),
        grant_type: "authorization_code".into(),
        client_id: code.client_id.clone(),
        redirect_uri: code.redirect_uri.clone(),
        state: code.state.clone(),
        code: code.code.clone(),
        code_verifier: VERIFIER.into(),
    }
}

pub(super) fn issued(conn: &mut Connection) -> AccessToken {
    let code = authorize_on(conn, "alice", SESSION, &body(), PUBLIC).unwrap();
    exchange_on(conn, &exchange_body(&code), PUBLIC).unwrap()
}

#[test]
fn code_checks_all_bindings_before_consuming_and_cannot_replay() {
    let mut conn = fixture();
    let code = authorize_on(&mut conn, "alice", SESSION, &body(), PUBLIC).unwrap();
    for kind in 0..5 {
        let mut input = exchange_body(&code);
        match kind {
            0 => {
                input.code_verifier =
                    "wrong-but-well-formed-verifier-with-over-forty-three-chars".into()
            }
            1 => input.state = "t".repeat(32),
            2 => {
                input.client_id = "quant.web".into();
                input.redirect_uri = format!("{PUBLIC}/quant/asset-access/callback");
            }
            3 => input.redirect_uri.push_str("/wrong"),
            _ => input.schema = "yilong.asset_access.token_request.v0".into(),
        }
        assert!(exchange_on(&mut conn, &input, PUBLIC).is_err());
    }
    let value = exchange_on(&mut conn, &exchange_body(&code), PUBLIC).unwrap();
    assert!(valid_secret(&value.access_token, "aat_"));
    assert!(exchange_on(&mut conn, &exchange_body(&code), PUBLIC).is_err());
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM asset_access_tokens", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    assert!(!serde_json::to_string(&value).unwrap().contains(SESSION));
}

#[test]
fn identity_is_stable_per_user_and_client_and_credentials_never_cross_clients() {
    let mut conn = fixture();
    let first = issued(&mut conn);
    let second = issued(&mut conn);
    assert_eq!(first.subject, second.subject);
    assert_ne!(first.access_token, second.access_token);
    let mut web = body();
    web.client_id = "quant.web".into();
    web.redirect_uri = format!("{PUBLIC}/quant/asset-access/callback");
    let code = authorize_on(&mut conn, "alice", SESSION, &web, PUBLIC).unwrap();
    let other_client = exchange_on(&mut conn, &exchange_body(&code), PUBLIC).unwrap();
    assert_ne!(first.subject, other_client.subject);
    let code = authorize_on(&mut conn, "bob", "synthetic-bob", &body(), PUBLIC).unwrap();
    let other_user = exchange_on(&mut conn, &exchange_body(&code), PUBLIC).unwrap();
    assert_ne!(first.subject, other_user.subject);
    assert!(verify_read_on(&conn, &first.access_token, "quant.web", "esk.summary.read").is_err());
    let before: i64 = conn
        .query_row("SELECT total_changes()", [], |row| row.get(0))
        .unwrap();
    let read = verify_read_on(
        &conn,
        &first.access_token,
        "quant.android",
        "esk.summary.read",
    )
    .unwrap();
    assert_eq!(read.user_id(), "alice");
    assert!(verify_read_on(
        &conn,
        &first.access_token,
        "quant.android",
        "esk.progress.read"
    )
    .is_err());
    assert!(verify_read_on(&conn, SESSION, "quant.android", "esk.summary.read").is_err());
    let after: i64 = conn
        .query_row("SELECT total_changes()", [], |row| row.get(0))
        .unwrap();
    assert_eq!(after, before);
}

#[test]
fn consent_parent_owner_scope_and_lifetime_are_not_client_assertions() {
    let mut conn = fixture();
    assert!(authorize_on(&mut conn, "bob", SESSION, &body(), PUBLIC).is_err());
    assert!(authorize_on(&mut conn, "local-owner", SESSION, &body(), PUBLIC).is_err());
    let mut input = body();
    input.explicit_consent = false;
    assert!(authorize_on(&mut conn, "alice", SESSION, &input, PUBLIC).is_err());
    input = body();
    input.expires_in = 3601;
    assert!(authorize_on(&mut conn, "alice", SESSION, &input, PUBLIC).is_err());
    input = body();
    input.scopes = vec![AccessScope::ProfileRead];
    assert!(authorize_on(&mut conn, "alice", SESSION, &input, PUBLIC).is_err());
    let deadline = clock().unwrap() + 45;
    conn.execute(
        "UPDATE sessions SET expires_at=?1 WHERE id='session-a'",
        params![timestamp(deadline).unwrap()],
    )
    .unwrap();
    let code = authorize_on(&mut conn, "alice", SESSION, &body(), PUBLIC).unwrap();
    assert_eq!(code.expires_at, timestamp(deadline).unwrap());
    let token = exchange_on(&mut conn, &exchange_body(&code), PUBLIC).unwrap();
    assert_eq!(token.expires_at, timestamp(deadline).unwrap());
    assert!(token.expires_in <= 45);
}

#[test]
fn revoked_parent_reassigned_parent_and_disabled_account_invalidate_existing_token() {
    for sql in [
        "UPDATE sessions SET revoked_at='revoked' WHERE id='session-a'",
        "UPDATE sessions SET user_id='bob' WHERE id='session-a'",
        "UPDATE sessions SET expires_at='2000-01-01T00:00:00Z' WHERE id='session-a'",
        "UPDATE users SET status='disabled' WHERE id='alice'",
    ] {
        let mut conn = fixture();
        let token = issued(&mut conn);
        conn.execute_batch(sql).unwrap();
        assert!(verify_read_on(
            &conn,
            &token.access_token,
            "quant.android",
            "esk.summary.read"
        )
        .is_err());
    }
}

#[test]
fn expiry_and_revocation_are_persistent_and_never_extend_access() {
    let mut conn = fixture();
    let token = issued(&mut conn);
    let at = clock().unwrap();
    assert!(read::verify_token_on(
        &conn,
        &token.access_token,
        "quant.android",
        at + 3601,
        false
    )
    .is_err());
    revoke::revoke_on(&conn, &token.grant_id, at).unwrap();
    revoke::revoke_on(&conn, &token.grant_id, at).unwrap();
    assert!(verify_read_on(
        &conn,
        &token.access_token,
        "quant.android",
        "esk.summary.read"
    )
    .is_err());
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM asset_access_audit WHERE action='revoked'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    assert!(conn
        .execute(
            "UPDATE asset_access_grants SET revoked_at_unix=NULL WHERE grant_id=?1",
            params![token.grant_id]
        )
        .is_err());
    let code = authorize_on(&mut conn, "alice", SESSION, &body(), PUBLIC).unwrap();
    assert!(exchange_at(&mut conn, &exchange_body(&code), PUBLIC, at + 121).is_err());
}

#[test]
fn secret_material_is_absent_from_persisted_rows() {
    let mut conn = fixture();
    let code = authorize_on(&mut conn, "alice", SESSION, &body(), PUBLIC).unwrap();
    let token = exchange_on(&mut conn, &exchange_body(&code), PUBLIC).unwrap();
    let persisted:String=conn.query_row("SELECT c.code_hash||c.state_hash||c.code_challenge||g.parent_session_hash||t.token_hash
        FROM asset_access_codes c JOIN asset_access_grants g ON c.grant_id=g.grant_id
        JOIN asset_access_tokens t ON t.grant_id=g.grant_id LIMIT 1",[],|r|r.get(0)).unwrap();
    for secret in [
        &code.code,
        &token.access_token,
        &code.state,
        &VERIFIER.to_owned(),
        &SESSION.to_owned(),
    ] {
        assert!(!persisted.contains(secret));
    }
}

#[test]
fn profile_projection_requires_scope_and_bounds_unicode_without_controls_or_cross_user_data() {
    let mut conn = fixture();
    let token = issued(&mut conn);
    let read = verify_read_on(
        &conn,
        &token.access_token,
        "quant.android",
        "esk.summary.read",
    )
    .unwrap();
    assert!(read::profile_on(&conn, &read).unwrap().is_none());
    let mut input = body();
    input.scopes.push(AccessScope::ProfileRead);
    let code = authorize_on(&mut conn, "alice", SESSION, &input, PUBLIC).unwrap();
    let token = exchange_on(&mut conn, &exchange_body(&code), PUBLIC).unwrap();
    let read = verify_read_on(&conn, &token.access_token, "quant.android", "profile.read").unwrap();
    let unsafe_label = format!("\u{7f}\r\n\t{}TAIL-MUST-NOT-APPEAR", "🦀".repeat(140));
    conn.execute(
        "UPDATE users SET nickname=?1 WHERE id='alice'",
        params![unsafe_label],
    )
    .unwrap();
    conn.execute(
        "UPDATE users SET nickname='PRIVATE-OTHER-USER' WHERE id='bob'",
        [],
    )
    .unwrap();
    let before: i64 = conn
        .query_row("SELECT total_changes()", [], |row| row.get(0))
        .unwrap();
    let projected = read::profile_on(&conn, &read).unwrap().unwrap();
    assert_eq!(projected, "🦀".repeat(128));
    assert_eq!(projected.encode_utf16().count(), 256);
    assert!(!projected.chars().any(char::is_control));
    let after: i64 = conn
        .query_row("SELECT total_changes()", [], |row| row.get(0))
        .unwrap();
    assert_eq!(before, after);
}
