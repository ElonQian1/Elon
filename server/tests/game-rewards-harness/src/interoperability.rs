use crate::{
    funding, ledger, migration,
    model::*,
    policy::{self, Policy, PolicyInput},
};
use rusqlite::Connection;
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    policy: PolicyInput,
    policy_digest: String,
    settlement: Signed<Settlement>,
    intent: BudgetIntent,
    intent_digest: String,
    funding: Signed<FundingEvidence>,
}

#[test]
fn independent_game_observer_signature_is_accepted_once_by_the_production_ledger() {
    let f: Fixture =
        serde_json::from_str(include_str!("../fixtures/rewards-funding-v1.json")).unwrap();
    let p = Policy::from_input(f.policy).unwrap();
    assert_eq!(p.digest, f.policy_digest);
    assert_eq!(
        policy::hash(policy::canonical("intent", &f.intent).unwrap()),
        f.intent_digest
    );
    let mut c = Connection::open_in_memory().unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE users(id TEXT PRIMARY KEY,status TEXT,role TEXT); CREATE TABLE sessions(user_id TEXT,token_hash TEXT,revoked_at TEXT,expires_at TEXT); INSERT INTO users VALUES('admin','active','admin'),('alice','active','user');").unwrap();
    let now = 1_800_000_000_000;
    for user in ["admin", "alice"] {
        c.execute(
            "INSERT INTO sessions VALUES(?1,?2,NULL,?3)",
            rusqlite::params![
                user,
                policy::hash(user),
                chrono::DateTime::from_timestamp_millis(now + 10000)
                    .unwrap()
                    .to_rfc3339()
            ],
        )
        .unwrap();
    }
    migration::migration_v293(&c).unwrap();
    ledger::record_settlement(&mut c, &p, "admin", "admin", &f.settlement, &|| Ok(now)).unwrap();
    let reserved = ledger::prepare(&mut c, &p, "admin", "admin", &f.intent, &|| Ok(now)).unwrap();
    assert_eq!(reserved.amount_units, "601");
    assert_eq!(reserved.state, "reserved_awaiting_funding");
    let funded = funding::confirm(&mut c, &p, "admin", "admin", &f.funding, &|| Ok(now)).unwrap();
    assert_eq!(funded.state, "funding_attested");
    assert!(!funded.offchain_payment_authorized);
    assert!(
        funding::confirm(&mut c, &p, "admin", "admin", &f.funding, &|| Ok(now))
            .unwrap()
            .replayed
    );
    let account = funding::account(&mut c, &p, "alice", "alice", &|| Ok(now)).unwrap();
    assert_eq!(account.reserved_profit_units, "601");
    assert_eq!(account.available_profit_units, "399");
    let mut bad = f.funding;
    bad.payload.amount_units = "602".into();
    assert!(funding::confirm(&mut c, &p, "admin", "admin", &bad, &|| Ok(now)).is_err());
}
