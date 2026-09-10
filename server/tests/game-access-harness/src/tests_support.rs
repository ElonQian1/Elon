use super::*;
use rusqlite::{params, Connection};
pub const AT: i64 = 1_700_000_000_000;
pub const MASTER: &str = "synthetic-main-token-alice";
pub const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
pub fn service() -> String {
    format!("egs_{}", "42".repeat(32))
}
pub fn policy() -> policy::Policy {
    policy::Policy::from_input(policy::PolicyInput {
        schema: "esk.game.access.policy.v1".into(),
        main_issuer: "synthetic-main".into(),
        client_id: model::CLIENT.into(),
        redirect_uri: "https://game.example.test/api/account/callback".into(),
        service_secret_sha256: policy::hash(&service()),
        key_id: "synthetic-key".into(),
        signing_seed_hex: "44".repeat(32),
    })
    .unwrap()
}
pub fn initialize(c: &Connection) {
    c.execute_batch(
        "PRAGMA foreign_keys=ON;
        CREATE TABLE users(id TEXT PRIMARY KEY,status TEXT NOT NULL);
        CREATE TABLE sessions(id TEXT PRIMARY KEY,user_id TEXT NOT NULL REFERENCES users(id),
            token_hash TEXT NOT NULL UNIQUE,expires_at TEXT NOT NULL,revoked_at TEXT);
        INSERT INTO users VALUES('alice','active'),('bob','active'),('local-owner','active');",
    )
    .unwrap();
    let expiry = chrono::DateTime::from_timestamp_millis(AT + 7_200_000)
        .unwrap()
        .to_rfc3339();
    for (id, user, token) in [
        ("session-alice", "alice", MASTER),
        ("session-bob", "bob", "synthetic-bob"),
    ] {
        c.execute(
            "INSERT INTO sessions VALUES(?1,?2,?3,?4,NULL)",
            params![id, user, policy::hash(token), expiry],
        )
        .unwrap();
    }
    migration::migration_v292(c).unwrap();
    migration::migration_v292(c).unwrap();
}
pub fn db() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    initialize(&c);
    c
}
pub fn authorize_body() -> model::AuthorizeRequest {
    model::AuthorizeRequest {
        schema: "esk.game.access.authorize.v1".into(),
        client_id: model::CLIENT.into(),
        redirect_uri: policy().redirect.clone(),
        state: "s".repeat(43),
        code_challenge: policy::pkce(VERIFIER).unwrap(),
        code_challenge_method: "S256".into(),
        scopes: vec!["play".into(), "inventory_read".into(), "redeem".into()],
        expires_in_seconds: 900,
        explicit_consent: true,
        confirmation: model::CONSENT.into(),
    }
}
pub fn exchange_body(code: &model::AuthorizationCode) -> model::ExchangeRequest {
    model::ExchangeRequest {
        schema: "esk.game.access.exchange.v1".into(),
        grant_type: "authorization_code".into(),
        client_id: model::CLIENT.into(),
        redirect_uri: code.redirect_uri.clone(),
        state: code.state.clone(),
        code: code.code.clone(),
        code_verifier: VERIFIER.into(),
    }
}
pub fn login(c: &mut Connection) -> model::GameToken {
    let p = policy();
    let code = issue::authorize(c, &p, "alice", MASTER, &authorize_body(), &|| Ok(AT)).unwrap();
    issue::exchange(c, &p, &service(), &exchange_body(&code), &|| Ok(AT + 1)).unwrap()
}
pub fn challenge(token: &str, nonce: u64) -> protocol::Challenge {
    protocol::Challenge {
        domain: "esk.game.session.challenge.v1".into(),
        main_issuer: "synthetic-main".into(),
        audience: "esk-game".into(),
        stage: "platform_recorded".into(),
        nonce: format!("{nonce:064x}"),
        credential_digest: policy::hash(token),
        action: protocol::Action::Authenticate {},
    }
}
pub fn revoke_body() -> model::RevokeRequest {
    model::RevokeRequest {
        schema: "esk.game.access.revoke.v1".into(),
        expected_revision: "1".into(),
    }
}
pub fn count(c: &Connection, table: &str) -> i64 {
    c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}
