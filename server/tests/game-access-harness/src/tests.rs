use super::*;
use rusqlite::{params, Connection};
#[path = "tests_support.rs"]
mod support;
use support::*;

#[test]
fn main_sdk_fixed_signature_verifies_with_production_rust_protocol() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../sdk/game-access/test/fixtures/session-v1.json"
    ))
    .unwrap();
    let observation = serde_json::from_value(f["observation"].clone()).unwrap();
    let challenge = serde_json::from_value(f["expected_challenge"].clone()).unwrap();
    let authority = protocol::Authority {
        main_issuer: f["main_issuer"].as_str().unwrap().into(),
        key_id: f["key_id"].as_str().unwrap().into(),
        public_key: hex::decode(f["public_key_hex"].as_str().unwrap())
            .unwrap()
            .try_into()
            .unwrap(),
    };
    let verified =
        protocol::verify_observation(&observation, &challenge, &authority, 1_700_000_001_500)
            .unwrap();
    assert_eq!(verified.authorization_digest, f["authorization_digest"]);
}

#[test]
fn authorize_exchange_observe_uses_original_main_identity_and_real_signature() {
    let mut c = db();
    let p = policy();
    let t = login(&mut c);
    let ch = challenge(&t.access_token, 1);
    let obs =
        observe::observe(&mut c, &p, &service(), &t.access_token, &ch, &|| Ok(AT + 2)).unwrap();
    let authority = protocol::Authority {
        main_issuer: p.issuer.clone(),
        key_id: p.key_id.clone(),
        public_key: p.public_key(),
    };
    let verified = protocol::verify_observation(&obs, &ch, &authority, (AT + 3) as u64).unwrap();
    assert_eq!(verified.main_user_id, "alice");
    assert_eq!(verified.main_session_id, "session-alice");
    assert!(!verified.wallet_bound && !verified.funds_moved);
    assert_eq!(count(&c, "game_access_audit"), 2);
    let (parent_hash, token_hash): (String, String) = c
        .query_row(
            "SELECT parent_hash,token_hash FROM game_access_grants",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(parent_hash, policy::hash(MASTER));
    assert_eq!(token_hash, policy::hash(&t.access_token));
    assert!(!serde_json::to_string(&obs).unwrap().contains(MASTER));
}

#[test]
fn nonce_replay_and_switched_action_token_or_issuer_fail() {
    let mut c = db();
    let p = policy();
    let t = login(&mut c);
    let ch = challenge(&t.access_token, 1);
    observe::observe(&mut c, &p, &service(), &t.access_token, &ch, &|| Ok(AT + 2)).unwrap();
    let mut other = ch.clone();
    other.action = protocol::Action::Inventory {};
    assert!(
        observe::observe(&mut c, &p, &service(), &t.access_token, &other, &|| Ok(
            AT + 3
        ))
        .is_err()
    );
    for (field, value) in [
        ("main_issuer", "other-main"),
        ("credential_digest", &"a".repeat(64)),
        ("stage", "paper"),
    ] {
        let mut wire = serde_json::to_value(challenge(&t.access_token, 2)).unwrap();
        wire[field] = value.into();
        let changed = serde_json::from_value(wire).unwrap();
        assert!(
            observe::observe(&mut c, &p, &service(), &t.access_token, &changed, &|| Ok(
                AT + 3
            ))
            .is_err()
        );
    }
    assert_eq!(count(&c, "game_access_nonces"), 1);
}

#[test]
fn exchange_binds_verifier_state_redirect_client_and_service() {
    let mut c = db();
    let p = policy();
    let code =
        issue::authorize(&mut c, &p, "alice", MASTER, &authorize_body(), &|| Ok(AT)).unwrap();
    for field in ["code_verifier", "state", "redirect_uri", "client_id"] {
        let mut b = exchange_body(&code);
        match field {
            "code_verifier" => b.code_verifier = "z".repeat(43),
            "state" => b.state = "t".repeat(43),
            "redirect_uri" => b.redirect_uri.push_str("/other"),
            _ => b.client_id = "quant.web".into(),
        }
        assert!(issue::exchange(&mut c, &p, &service(), &b, &|| Ok(AT + 1)).is_err());
    }
    assert!(issue::exchange(
        &mut c,
        &p,
        &format!("egs_{}", "11".repeat(32)),
        &exchange_body(&code),
        &|| Ok(AT + 1)
    )
    .is_err());
    let t = issue::exchange(
        &mut c,
        &p,
        &service(),
        &exchange_body(&code),
        &|| Ok(AT + 1),
    )
    .unwrap();
    assert!(issue::exchange(
        &mut c,
        &p,
        &service(),
        &exchange_body(&code),
        &|| Ok(AT + 1)
    )
    .is_err());
    assert_eq!(count(&c, "game_access_audit"), 2);
    assert!(t.access_token.starts_with("egt_"));
}

#[test]
fn parent_revocation_disable_expiry_and_session_substitution_close_access() {
    for change in [
        "UPDATE sessions SET revoked_at='revoked' WHERE user_id='alice'",
        "UPDATE users SET status='disabled' WHERE id='alice'",
        "UPDATE sessions SET expires_at='2020-01-01T00:00:00Z' WHERE user_id='alice'",
        "UPDATE sessions SET id='replaced-session' WHERE user_id='alice'",
    ] {
        let mut c = db();
        let p = policy();
        let t = login(&mut c);
        // A legitimate FK may already prevent substituting the session id.
        if c.execute_batch(change).is_ok() {
            assert!(observe::observe(
                &mut c,
                &p,
                &service(),
                &t.access_token,
                &challenge(&t.access_token, 1),
                &|| Ok(AT + 2)
            )
            .is_err());
        }
    }
    let mut c = db();
    let p = policy();
    let code =
        issue::authorize(&mut c, &p, "alice", MASTER, &authorize_body(), &|| Ok(AT)).unwrap();
    c.execute(
        "UPDATE sessions SET revoked_at='revoked' WHERE user_id='alice'",
        [],
    )
    .unwrap();
    assert!(issue::exchange(
        &mut c,
        &p,
        &service(),
        &exchange_body(&code),
        &|| Ok(AT + 1)
    )
    .is_err());
}

#[test]
fn principal_permission_is_separate_and_revocation_is_owner_bound_and_idempotent() {
    let mut c = db();
    let p = policy();
    let t = login(&mut c);
    let mut ch = challenge(&t.access_token, 1);
    ch.action = protocol::Action::PrincipalWithdraw {
        position_id: "position-1".into(),
        idempotency_key: "request-1".into(),
    };
    assert!(
        observe::observe(&mut c, &p, &service(), &t.access_token, &ch, &|| Ok(AT + 2)).is_err()
    );
    assert!(revoke::revoke_owner(
        &mut c,
        "bob",
        "synthetic-bob",
        &t.grant_id,
        &revoke_body(),
        &|| Ok(AT + 2)
    )
    .is_err());
    revoke::revoke_owner(
        &mut c,
        "alice",
        MASTER,
        &t.grant_id,
        &revoke_body(),
        &|| Ok(AT + 2),
    )
    .unwrap();
    revoke::revoke_owner(
        &mut c,
        "alice",
        MASTER,
        &t.grant_id,
        &revoke_body(),
        &|| Ok(AT + 3),
    )
    .unwrap();
    revoke::revoke_self(
        &mut c,
        &p,
        &service(),
        &t.access_token,
        &revoke_body(),
        &|| Ok(AT + 3),
    )
    .unwrap();
    assert!(observe::observe(
        &mut c,
        &p,
        &service(),
        &t.access_token,
        &challenge(&t.access_token, 2),
        &|| Ok(AT + 4)
    )
    .is_err());
    assert_eq!(count(&c, "game_access_audit"), 3);
}

#[test]
fn code_and_grant_expire_exactly_and_clock_is_read_under_the_transaction() {
    let mut c = db();
    let p = policy();
    let code =
        issue::authorize(&mut c, &p, "alice", MASTER, &authorize_body(), &|| Ok(AT)).unwrap();
    assert!(
        issue::exchange(&mut c, &p, &service(), &exchange_body(&code), &|| Ok(
            AT + 120_000
        ))
        .is_err()
    );
    let t = login(&mut c);
    assert!(observe::observe(
        &mut c,
        &p,
        &service(),
        &t.access_token,
        &challenge(&t.access_token, 1),
        &|| Ok(AT + 900_000)
    )
    .is_err());
    let calls = std::cell::Cell::new(0);
    let slow = || {
        let n = calls.get();
        calls.set(n + 1);
        Ok(AT + 2 + 5000 * n)
    };
    assert!(observe::observe(
        &mut c,
        &p,
        &service(),
        &t.access_token,
        &challenge(&t.access_token, 2),
        &slow
    )
    .is_err());
    assert_eq!(count(&c, "game_access_nonces"), 0);
}

#[test]
fn failed_exchange_commit_rolls_back_code_consumption_and_audit() {
    let mut c = db();
    let p = policy();
    let code =
        issue::authorize(&mut c, &p, "alice", MASTER, &authorize_body(), &|| Ok(AT)).unwrap();
    c.execute_batch("CREATE TRIGGER fault BEFORE INSERT ON game_access_audit WHEN NEW.action='exchanged' BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
    assert!(issue::exchange(
        &mut c,
        &p,
        &service(),
        &exchange_body(&code),
        &|| Ok(AT + 1)
    )
    .is_err());
    let consumed: Option<i64> = c
        .query_row("SELECT consumed_ms FROM game_access_grants", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(consumed, None);
    c.execute_batch("DROP TRIGGER fault").unwrap();
    assert!(issue::exchange(
        &mut c,
        &p,
        &service(),
        &exchange_body(&code),
        &|| Ok(AT + 1)
    )
    .is_ok());
}

#[test]
fn concurrent_exchange_has_exactly_one_winner_and_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.sqlite");
    let mut c = Connection::open(&path).unwrap();
    initialize(&c);
    let code = issue::authorize(
        &mut c,
        &policy(),
        "alice",
        MASTER,
        &authorize_body(),
        &|| Ok(AT),
    )
    .unwrap();
    let request = serde_json::to_string(&exchange_body(&code)).unwrap();
    drop(c);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let path = path.clone();
            let request = request.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut c = Connection::open(path).unwrap();
                c.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
                c.pragma_update(None, "foreign_keys", true).unwrap();
                barrier.wait();
                issue::exchange(
                    &mut c,
                    &policy(),
                    &service(),
                    &serde_json::from_str(&request).unwrap(),
                    &|| Ok(AT + 1),
                )
                .is_ok()
            })
        })
        .collect();
    assert_eq!(
        threads
            .into_iter()
            .filter_map(|t| t.join().ok())
            .filter(|v| *v)
            .count(),
        1
    );
    let mut c = Connection::open(path).unwrap();
    assert_eq!(count(&c, "game_access_audit"), 2);
    assert!(issue::exchange(
        &mut c,
        &policy(),
        &service(),
        &serde_json::from_str(&request).unwrap(),
        &|| Ok(AT + 2)
    )
    .is_err());
}

#[test]
fn policy_rotation_and_corrupt_or_mutable_records_never_restore_authority() {
    let mut c = db();
    let t = login(&mut c);
    let mut p = policy();
    p.digest = "a".repeat(64);
    assert!(observe::observe(
        &mut c,
        &p,
        &service(),
        &t.access_token,
        &challenge(&t.access_token, 1),
        &|| Ok(AT + 2)
    )
    .is_err());
    for sql in [
        "UPDATE game_access_grants SET user_id='bob'",
        "UPDATE game_access_grants SET scopes_json='[\"play\",\"principal_withdraw\"]'",
        "UPDATE game_access_grants SET consumed_ms=NULL,token_hash=NULL",
        "DELETE FROM game_access_grants",
        "DELETE FROM game_access_audit",
    ] {
        assert!(c.execute_batch(sql).is_err());
    }
}

#[test]
fn strict_requests_scope_order_consent_and_pkce_encoding() {
    let mut c = db();
    let p = policy();
    for mode in 0..5 {
        let mut b = authorize_body();
        match mode {
            0 => b.explicit_consent = false,
            1 => b.scopes.push("admin".into()),
            2 => b.scopes.reverse(),
            3 => b.code_challenge_method = "plain".into(),
            _ => b.redirect_uri.push_str("?next=evil"),
        }
        assert!(issue::authorize(&mut c, &p, "alice", MASTER, &b, &|| Ok(AT)).is_err());
    }
    assert_eq!(count(&c, "game_access_grants"), 0);
    for kind in ["authenticate", "inventory"] {
        assert!(serde_json::from_value::<protocol::Action>(
            serde_json::json!({"kind":kind,"admin":true})
        )
        .is_err());
    }
    assert!(!policy::identifier("alice\n"));
    assert!(!policy::lower_hex(&format!("{}\n", "a".repeat(63)), 32));
    assert_eq!(
        policy::pkce(VERIFIER).unwrap(),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}
