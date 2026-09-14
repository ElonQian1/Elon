use super::super::model::Scope;
use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

const NOW: i64 = 1700000000;
const SESSION: &str = "synthetic-session-only";

fn request() -> IssueRequest {
    serde_json::from_value(
        serde_json::json!({"schema":"yilong.quant.native_grid_issue.v1",
        "environment":"native_paper","scopes":["native_grid.read","native_grid.create"],
        "explicit_consent":true,"confirmation":"授权使用原生模拟网格"}),
    )
    .unwrap()
}
fn signer() -> Signer {
    Signer::from_lookup(|key| {
        Ok(Some(if key.ends_with("KEY_ID") {
            "test-key".into()
        } else {
            URL_SAFE_NO_PAD.encode([7; 32])
        }))
    })
    .unwrap()
}
fn database() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE users(id TEXT PRIMARY KEY,status TEXT);
        CREATE TABLE sessions(user_id TEXT,token_hash TEXT UNIQUE,expires_at TEXT,revoked_at TEXT);
        INSERT INTO users VALUES('synthetic-alice','active'),('synthetic-bob','active'),('local-owner','active');").unwrap();
    conn.execute(
        "INSERT INTO sessions VALUES('synthetic-alice',?1,?2,NULL)",
        params![
            hex::encode(Sha256::digest(SESSION.as_bytes())),
            chrono::DateTime::from_timestamp(NOW + 3600, 0)
                .unwrap()
                .to_rfc3339()
        ],
    )
    .unwrap();
    conn
}

#[test]
fn valid_session_issues_scoped_grant_without_writing_database() {
    let mut conn = database();
    let changes: i64 = conn
        .query_row("SELECT total_changes()", [], |r| r.get(0))
        .unwrap();
    let response = issue_on(&mut conn, SESSION, &request(), &signer(), || NOW).unwrap();
    assert_eq!(response.expires_in, 300);
    assert_eq!(response.scopes, vec![Scope::Read, Scope::Create]);
    let after: i64 = conn
        .query_row("SELECT total_changes()", [], |r| r.get(0))
        .unwrap();
    assert_eq!(after, changes);
    assert!(conn.is_autocommit());
}

#[test]
fn parent_expiry_caps_grant_and_expiration_is_strict() {
    let mut conn = database();
    conn.execute(
        "UPDATE sessions SET expires_at=?1",
        [chrono::DateTime::from_timestamp(NOW + 20, 0)
            .unwrap()
            .to_rfc3339()],
    )
    .unwrap();
    let response = issue_on(&mut conn, SESSION, &request(), &signer(), || NOW).unwrap();
    assert_eq!(response.expires_in, 20);
    assert!(matches!(
        issue_on(&mut conn, SESSION, &request(), &signer(), || NOW + 20),
        Err(Error::Unauthorized)
    ));
}

#[test]
fn revoked_inactive_unknown_and_virtual_accounts_cannot_issue() {
    for sql in [
        "UPDATE sessions SET revoked_at='revoked'",
        "UPDATE users SET status='disabled'",
        "DELETE FROM users",
        "UPDATE sessions SET user_id='local-owner'",
        "UPDATE sessions SET expires_at='broken'",
    ] {
        let mut conn = database();
        conn.execute_batch(sql).unwrap();
        assert!(matches!(
            issue_on(&mut conn, SESSION, &request(), &signer(), || NOW),
            Err(Error::Unauthorized)
        ));
    }
    let mut conn = database();
    for token in ["", "unknown", " synthetic-session-only "] {
        assert!(matches!(
            issue_on(&mut conn, token, &request(), &signer(), || NOW),
            Err(Error::Unauthorized)
        ));
    }
}

#[test]
fn database_identity_is_authority_and_new_sessions_get_same_subject() {
    let mut conn = database();
    let a = issue_on(&mut conn, SESSION, &request(), &signer(), || NOW).unwrap();
    conn.execute(
        "UPDATE sessions SET token_hash=?1",
        [hex::encode(Sha256::digest(b"synthetic-new-session"))],
    )
    .unwrap();
    let b = issue_on(
        &mut conn,
        "synthetic-new-session",
        &request(),
        &signer(),
        || NOW,
    )
    .unwrap();
    assert_eq!(a.subject_ref, b.subject_ref);
    conn.execute("UPDATE sessions SET user_id='synthetic-bob'", [])
        .unwrap();
    let c = issue_on(
        &mut conn,
        "synthetic-new-session",
        &request(),
        &signer(),
        || NOW,
    )
    .unwrap();
    assert_ne!(a.subject_ref, c.subject_ref);
}

#[test]
fn malformed_or_injected_request_is_rejected() {
    let base = serde_json::json!({"schema":"yilong.quant.native_grid_issue.v1","environment":"native_paper",
        "scopes":["native_grid.read"],"explicit_consent":true,"confirmation":"授权使用原生模拟网格"});
    for (key, value) in [
        ("owner", serde_json::json!("synthetic-bob")),
        ("expires_in", serde_json::json!(999)),
        ("schema", serde_json::json!("wrong")),
        ("environment", serde_json::json!("live")),
        ("explicit_consent", serde_json::json!(false)),
        ("confirmation", serde_json::json!("")),
        ("scopes", serde_json::json!([])),
        (
            "scopes",
            serde_json::json!(["native_grid.read", "native_grid.read"]),
        ),
        ("scopes", serde_json::json!(["native_grid.simulate"])),
        ("scopes", serde_json::json!(["paper.position.read"])),
    ] {
        let mut value_base = base.clone();
        value_base[key] = value;
        assert!(serde_json::from_value::<IssueRequest>(value_base)
            .map(|v| v.validate().is_err())
            .unwrap_or(true));
    }
    let duplicate =
        serde_json::to_string(&base)
            .unwrap()
            .replacen('{', "{\"explicit_consent\":false,", 1);
    assert!(serde_json::from_str::<IssueRequest>(&duplicate).is_err());
}

#[test]
fn corrupt_database_fails_closed_and_transaction_is_released() {
    let mut conn = Connection::open_in_memory().unwrap();
    assert!(matches!(
        issue_on(&mut conn, SESSION, &request(), &signer(), || NOW),
        Err(Error::Unavailable)
    ));
    assert!(conn.is_autocommit());
}
