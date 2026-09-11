use super::{
    funding, ledger, migration,
    model::*,
    policy::{self, Policy, PolicyInput},
};
use ed25519_dalek::{Signer, SigningKey};
use rusqlite::{params, Connection};
use std::cell::RefCell;
const NOW: i64 = 1_800_000_000_000;
struct TestDir(std::path::PathBuf);
impl TestDir {
    fn new() -> Self {
        static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let suffix = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("esk_rewards_{}_{at}_{suffix}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        let path = self.0.canonicalize().unwrap();
        assert_eq!(
            path.parent(),
            Some(std::env::temp_dir().canonicalize().unwrap().as_path())
        );
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("esk_rewards_"));
        std::fs::remove_dir_all(path).unwrap();
    }
}
thread_local! { static OVERRIDE: RefCell<Option<PolicyInput>> = const { RefCell::new(None) }; }
pub(crate) fn override_policy() -> Option<PolicyInput> {
    OVERRIDE.with(|v| v.borrow().clone())
}
pub(crate) struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        OVERRIDE.with(|v| v.replace(None));
    }
}
pub(crate) fn enable() -> Guard {
    OVERRIDE.with(|v| v.replace(Some(config())));
    Guard
}
fn key(n: u8) -> SigningKey {
    SigningKey::from_bytes(&[n; 32])
}
fn address(n: u8) -> String {
    format!("0x{}", hex::encode([n; 32]))
}
pub(crate) fn config() -> PolicyInput {
    PolicyInput {
        schema: "esk.game.rewards.policy.v1".into(),
        main_issuer: "synthetic-main".into(),
        network: "testnet".into(),
        chain_identifier: "4c78adac".into(),
        genesis_checkpoint_digest: "69WiPg3DAQiwdxfncX6wYQ2siKwAe6L9BZthQea3JNMD".into(),
        rewards_package_id: address(3),
        registry_id: address(4),
        asset_type: format!("{}::test::TEST", address(5)),
        asset_decimals: 6,
        reconciler_public_key_hex: hex::encode(key(11).verifying_key().as_bytes()),
        funding_observer_public_key_hex: hex::encode(key(22).verifying_key().as_bytes()),
    }
}
pub(crate) fn http_intent(report: &str, user: &str) -> BudgetIntent {
    intent(&Policy::from_input(config()).unwrap(), report, user, 1)
}
pub(crate) fn http_funding(intent: &BudgetIntent, at: i64) -> Signed<FundingEvidence> {
    let policy = Policy::from_input(config()).unwrap();
    let record = BudgetRecord {
        intent: intent.clone(),
        intent_digest: policy::hash(policy::canonical("intent", intent).unwrap()),
        amount_units: ledger::amount(intent, &policy).unwrap().to_string(),
        state: "reserved_awaiting_funding",
        funding: None,
        replayed: false,
        offchain_payment_authorized: false,
    };
    evidence(&policy, &record, at)
}
pub(crate) fn sign<T: serde::Serialize>(purpose: &str, payload: T, n: u8) -> Signed<T> {
    let signature_hex = hex::encode(
        key(n)
            .sign(&policy::canonical(purpose, &payload).unwrap())
            .to_bytes(),
    );
    Signed {
        payload,
        signature_hex,
    }
}
pub(crate) fn report(user: &str, at: i64) -> Signed<Settlement> {
    let p = Policy::from_input(config()).unwrap();
    sign(
        "settlement",
        Settlement {
            schema: "esk.game.rewards.settlement.v1".into(),
            policy_digest: p.digest,
            user_id: user.into(),
            beneficiary: address(6),
            wallet_binding_digest: "55".repeat(32),
            sequence: "1".into(),
            previous_report_digest: "00".repeat(32),
            period_end_ms: (at - 1000).to_string(),
            cumulative_net_profit_units: "1000".into(),
            reconciliation_digest: "66".repeat(32),
        },
        11,
    )
}
fn fixture(path: Option<&std::path::Path>) -> (Connection, Policy) {
    let c = path.map_or_else(
        || Connection::open_in_memory().unwrap(),
        |p| Connection::open(p).unwrap(),
    );
    c.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE users(id TEXT PRIMARY KEY,status TEXT,role TEXT); CREATE TABLE sessions(user_id TEXT,token_hash TEXT,revoked_at TEXT,expires_at TEXT); INSERT INTO users VALUES('admin','active','admin'),('alice','active','user'),('bob','active','user');").unwrap();
    let expiry = chrono::DateTime::from_timestamp_millis(NOW + 10000)
        .unwrap()
        .to_rfc3339();
    for user in ["admin", "alice", "bob"] {
        c.execute(
            "INSERT INTO sessions VALUES(?1,?2,NULL,?3)",
            params![user, policy::hash(user), expiry],
        )
        .unwrap();
    }
    migration::migration_v293(&c).unwrap();
    (c, Policy::from_input(config()).unwrap())
}
pub(crate) fn intent(p: &Policy, digest: &str, user: &str, n: u8) -> BudgetIntent {
    BudgetIntent {
        schema: "esk.game.rewards.intent.v1".into(),
        policy_digest: p.digest.clone(),
        allocation_hash: hex::encode([n; 32]),
        report_digest: digest.into(),
        user_id: user.into(),
        beneficiary: address(6),
        mint_authority: address(7),
        creator: address(8),
        content_hash: "88".repeat(32),
        license_hash: "99".repeat(32),
        allow_modification: true,
        allow_external_game: true,
        opens_at_ms: "0".into(),
        direct_claim_after_ms: "0".into(),
        denominations: [1, 5, 10, 50, 100, 500].map(|n| n.to_string()).to_vec(),
        tickets: [0, 0, 0, 0, 0, 1].map(|n| n.to_string()).to_vec(),
    }
}
pub(crate) fn evidence(p: &Policy, record: &BudgetRecord, at: i64) -> Signed<FundingEvidence> {
    sign(
        "funding",
        FundingEvidence {
            schema: "esk.game.rewards.funding.v1".into(),
            policy_digest: p.digest.clone(),
            intent_digest: record.intent_digest.clone(),
            allocation_hash: record.intent.allocation_hash.clone(),
            budget_id: address(9),
            transaction_digest: "1".repeat(32),
            checkpoint: "100".into(),
            checkpoint_digest: "2".repeat(43),
            checkpoint_time_ms: (at - 100).to_string(),
            observed_at_ms: (at - 50).to_string(),
            amount_units: record.amount_units.clone(),
            beneficiary: record.intent.beneficiary.clone(),
            initial_budget_verified: true,
        },
        22,
    )
}
fn ready(c: &mut Connection, p: &Policy) -> BudgetIntent {
    let digest =
        ledger::record_settlement(c, p, "admin", "admin", &report("alice", NOW), &|| Ok(NOW))
            .unwrap();
    intent(p, &digest, "alice", 1)
}
fn count(c: &Connection, table: &str) -> i64 {
    c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

#[test]
fn game_rewards_zero_profit_and_client_created_money_never_authorize_rewards() {
    let (mut c, p) = fixture(None);
    let i = intent(&p, &"11".repeat(32), "alice", 1);
    assert!(ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| Ok(NOW)).is_err());
    let mut forged = report("alice", NOW);
    forged.payload.cumulative_net_profit_units = "2000000000".into();
    assert!(ledger::record_settlement(&mut c, &p, "admin", "admin", &forged, &|| Ok(NOW)).is_err());
    let authentic = report("alice", NOW);
    assert!(
        ledger::record_settlement(&mut c, &p, "alice", "alice", &authentic, &|| Ok(NOW)).is_err()
    );
    assert_eq!(count(&c, "game_reward_reports"), 0);
    let view = funding::account(&mut c, &p, "alice", "alice", &|| Ok(NOW)).unwrap();
    assert_eq!(view.available_profit_units, "0");
    assert!(!view.offchain_payment_authorized);
}
#[test]
fn game_rewards_profit_budget_funding_and_restart_are_idempotent_and_isolated() {
    let dir = TestDir::new();
    let path = dir.path().join("journal.sqlite");
    let (mut c, p) = fixture(Some(&path));
    let i = ready(&mut c, &p);
    let a = ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| Ok(NOW)).unwrap();
    assert_eq!(a.state, "reserved_awaiting_funding");
    assert!(a.funding.is_none());
    assert!(
        ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| Ok(NOW))
            .unwrap()
            .replayed
    );
    let proof = evidence(&p, &a, NOW);
    let confirmed = funding::confirm(&mut c, &p, "admin", "admin", &proof, &|| Ok(NOW)).unwrap();
    assert_eq!(confirmed.state, "funding_attested");
    assert!(!confirmed.offchain_payment_authorized);
    drop(c);
    let mut c = Connection::open(&path).unwrap();
    assert!(
        funding::confirm(&mut c, &p, "admin", "admin", &proof, &|| Ok(NOW))
            .unwrap()
            .replayed
    );
    let v = funding::account(&mut c, &p, "alice", "alice", &|| Ok(NOW)).unwrap();
    assert_eq!(v.reserved_profit_units, "500");
    assert_eq!(v.available_profit_units, "500");
    assert_eq!(v.budgets.len(), 1);
    assert!(funding::account(&mut c, &p, "bob", "alice", &|| Ok(NOW)).is_err());
    assert!(funding::account(&mut c, &p, "bob", "bob", &|| Ok(NOW))
        .unwrap()
        .budgets
        .is_empty());
    assert_eq!(count(&c, "game_reward_funding"), 1);
}
#[test]
fn game_rewards_loss_recovery_does_not_redistribute_the_same_profit() {
    let (mut c, p) = fixture(None);
    let i = ready(&mut c, &p);
    ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| Ok(NOW)).unwrap();
    for (n, net, expected) in [(2, -100, 0), (3, 300, 0), (4, 700, 200)] {
        let (previous, last) = ledger::latest(&c, &p, "alice").unwrap().unwrap();
        let s = Settlement {
            sequence: n.to_string(),
            previous_report_digest: previous,
            period_end_ms: (NOW - 1000 + n).to_string(),
            cumulative_net_profit_units: net.to_string(),
            ..last
        };
        ledger::record_settlement(
            &mut c,
            &p,
            "admin",
            "admin",
            &sign("settlement", s, 11),
            &|| Ok(NOW),
        )
        .unwrap();
        let v = funding::account(&mut c, &p, "alice", "alice", &|| Ok(NOW)).unwrap();
        assert_eq!(v.available_profit_units, expected.to_string());
        assert_eq!(v.reserved_profit_units, "500");
    }
    let (d, _) = ledger::latest(&c, &p, "alice").unwrap().unwrap();
    let mut next = intent(&p, &d, "alice", 2);
    next.tickets = vec![
        "0".into(),
        "0".into(),
        "0".into(),
        "0".into(),
        "2".into(),
        "0".into(),
    ];
    ledger::prepare(&mut c, &p, "admin", "admin", &next, &|| Ok(NOW)).unwrap();
    assert_eq!(ledger::reserved(&c, "alice").unwrap(), 700);
    next.allocation_hash = "03".repeat(32);
    assert!(ledger::prepare(&mut c, &p, "admin", "admin", &next, &|| Ok(NOW)).is_err());
}
#[test]
fn game_rewards_report_sequence_identity_and_historical_replay_are_bound() {
    let (mut c, p) = fixture(None);
    let original = report("alice", NOW);
    let d =
        ledger::record_settlement(&mut c, &p, "admin", "admin", &original, &|| Ok(NOW)).unwrap();
    for changed in [
        Settlement {
            sequence: "3".into(),
            previous_report_digest: d.clone(),
            ..original.payload.clone()
        },
        Settlement {
            sequence: "2".into(),
            previous_report_digest: d.clone(),
            beneficiary: address(33),
            ..original.payload.clone()
        },
        Settlement {
            cumulative_net_profit_units: "1001".into(),
            ..original.payload.clone()
        },
    ] {
        assert!(ledger::record_settlement(
            &mut c,
            &p,
            "admin",
            "admin",
            &sign("settlement", changed, 11),
            &|| Ok(NOW)
        )
        .is_err());
    }
    let next = Settlement {
        sequence: "2".into(),
        previous_report_digest: d.clone(),
        period_end_ms: (NOW - 500).to_string(),
        ..original.payload.clone()
    };
    ledger::record_settlement(
        &mut c,
        &p,
        "admin",
        "admin",
        &sign("settlement", next, 11),
        &|| Ok(NOW),
    )
    .unwrap();
    assert_eq!(
        ledger::record_settlement(&mut c, &p, "admin", "admin", &original, &|| Ok(NOW)).unwrap(),
        d
    );
    assert_eq!(count(&c, "game_reward_reports"), 2);
    let stale = intent(&p, &d, "alice", 1);
    assert!(ledger::prepare(&mut c, &p, "admin", "admin", &stale, &|| Ok(NOW)).is_err());
}
#[test]
fn game_rewards_budget_cannot_be_repriced_redirected_or_overflowed() {
    let (mut c, p) = fixture(None);
    let i = ready(&mut c, &p);
    ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| Ok(NOW)).unwrap();
    let mut repriced = i.clone();
    repriced.tickets[5] = "2".into();
    assert!(ledger::prepare(&mut c, &p, "admin", "admin", &repriced, &|| Ok(NOW)).is_err());
    let mut moved = i.clone();
    moved.allocation_hash = "02".repeat(32);
    moved.beneficiary = address(10);
    assert!(ledger::prepare(&mut c, &p, "admin", "admin", &moved, &|| Ok(NOW)).is_err());
    for value in ["-1", "01", "1.0", "9223372036854775808"] {
        let mut invalid = i.clone();
        invalid.tickets[5] = value.into();
        assert!(ledger::amount(&invalid, &p).is_err());
    }
    let mut overflow = i.clone();
    overflow.denominations[5] = i64::MAX.to_string();
    overflow.tickets[5] = "2".into();
    assert!(ledger::amount(&overflow, &p).is_err());
    let mut zero = i.clone();
    zero.tickets = vec!["0".into(); 6];
    assert!(ledger::amount(&zero, &p).is_err());
    assert_eq!(ledger::reserved(&c, "alice").unwrap(), 500);
}
#[test]
fn game_rewards_observer_role_and_exact_funding_terms_are_required() {
    let (mut c, p) = fixture(None);
    let i = ready(&mut c, &p);
    let a = ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| Ok(NOW)).unwrap();
    let good = evidence(&p, &a, NOW);
    assert!(funding::confirm(
        &mut c,
        &p,
        "admin",
        "admin",
        &sign("funding", good.payload.clone(), 11),
        &|| Ok(NOW)
    )
    .is_err());
    for wrong in [
        FundingEvidence {
            amount_units: "499".into(),
            ..good.payload.clone()
        },
        FundingEvidence {
            initial_budget_verified: false,
            ..good.payload.clone()
        },
        FundingEvidence {
            beneficiary: address(55),
            ..good.payload.clone()
        },
        FundingEvidence {
            intent_digest: "10".repeat(32),
            ..good.payload.clone()
        },
        FundingEvidence {
            observed_at_ms: (NOW + 1).to_string(),
            ..good.payload.clone()
        },
    ] {
        assert!(funding::confirm(
            &mut c,
            &p,
            "admin",
            "admin",
            &sign("funding", wrong, 22),
            &|| Ok(NOW)
        )
        .is_err());
    }
    assert_eq!(count(&c, "game_reward_funding"), 0);
    assert_eq!(ledger::reserved(&c, "alice").unwrap(), 500);
}
#[test]
fn game_rewards_one_chain_budget_cannot_back_two_allocations() {
    let (mut c, p) = fixture(None);
    let i = ready(&mut c, &p);
    let a = ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| Ok(NOW)).unwrap();
    funding::confirm(
        &mut c,
        &p,
        "admin",
        "admin",
        &evidence(&p, &a, NOW),
        &|| Ok(NOW),
    )
    .unwrap();
    let b = ledger::prepare(
        &mut c,
        &p,
        "admin",
        "admin",
        &BudgetIntent {
            allocation_hash: "02".repeat(32),
            ..i
        },
        &|| Ok(NOW),
    )
    .unwrap();
    assert!(funding::confirm(
        &mut c,
        &p,
        "admin",
        "admin",
        &evidence(&p, &b, NOW),
        &|| Ok(NOW)
    )
    .is_err());
    assert_eq!(count(&c, "game_reward_funding"), 1);
    assert_eq!(ledger::reserved(&c, "alice").unwrap(), 1000);
}
#[test]
fn game_rewards_expired_or_revoked_session_rolls_back_and_blocks_replay() {
    let (mut c, p) = fixture(None);
    let i = ready(&mut c, &p);
    let clock = std::cell::Cell::new(0);
    assert!(ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| {
        let n = clock.get();
        clock.set(n + 1);
        Ok(if n == 0 { NOW } else { NOW + 10000 })
    })
    .is_err());
    assert_eq!(count(&c, "game_reward_intents"), 0);
    ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| Ok(NOW)).unwrap();
    c.execute(
        "UPDATE sessions SET revoked_at='revoked' WHERE user_id='admin'",
        [],
    )
    .unwrap();
    assert!(ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| Ok(NOW)).is_err());
}
#[test]
fn game_rewards_database_failure_and_replace_cannot_remove_reservations() {
    let (mut c, p) = fixture(None);
    let i = ready(&mut c, &p);
    c.execute_batch("CREATE TRIGGER fail_intent BEFORE INSERT ON game_reward_intents BEGIN SELECT RAISE(ABORT,'synthetic IO failure'); END;").unwrap();
    assert!(ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| Ok(NOW)).is_err());
    assert_eq!(ledger::reserved(&c, "alice").unwrap(), 0);
    c.execute_batch("DROP TRIGGER fail_intent").unwrap();
    ledger::prepare(&mut c, &p, "admin", "admin", &i, &|| Ok(NOW)).unwrap();
    for sql in [
        "UPDATE game_reward_intents SET amount_units=1",
        "DELETE FROM game_reward_intents",
        "INSERT OR REPLACE INTO game_reward_intents SELECT * FROM game_reward_intents",
        "DELETE FROM game_reward_reports",
        "DELETE FROM game_reward_policy",
    ] {
        assert!(c.execute_batch(sql).is_err(), "{sql}");
    }
    migration::migration_v293(&c).unwrap();
    assert_eq!(ledger::reserved(&c, "alice").unwrap(), 500);
}
#[test]
fn game_rewards_two_writers_cannot_overallocate_profit() {
    let dir = TestDir::new();
    let path = dir.path().join("race.sqlite");
    let (mut c, p) = fixture(Some(&path));
    let i = ready(&mut c, &p);
    drop(c);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (1..=2)
        .map(|n| {
            let path = path.clone();
            let barrier = barrier.clone();
            let mut i = i.clone();
            i.allocation_hash = hex::encode([n; 32]);
            i.tickets[0] = "1".into();
            std::thread::spawn(move || {
                let mut c = Connection::open(path).unwrap();
                c.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
                barrier.wait();
                ledger::prepare(
                    &mut c,
                    &Policy::from_input(config()).unwrap(),
                    "admin",
                    "admin",
                    &i,
                    &|| Ok(NOW),
                )
                .is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .filter(|h| h.thread().id() != std::thread::current().id())
            .map(|h| h.join().unwrap() as usize)
            .sum::<usize>(),
        1
    );
    assert_eq!(
        ledger::reserved(&Connection::open(path).unwrap(), "alice").unwrap(),
        501
    );
}
#[test]
fn game_rewards_policy_is_pinned_and_mainnet_or_shared_keys_are_rejected() {
    let (mut c, p) = fixture(None);
    ready(&mut c, &p);
    let mut changed = config();
    changed.registry_id = address(10);
    assert!(funding::account(
        &mut c,
        &Policy::from_input(changed).unwrap(),
        "alice",
        "alice",
        &|| Ok(NOW)
    )
    .is_err());
    let mut invalid = config();
    invalid.network = "mainnet".into();
    assert!(Policy::from_input(invalid).is_err());
    let mut invalid = config();
    invalid.funding_observer_public_key_hex = invalid.reconciler_public_key_hex.clone();
    assert!(Policy::from_input(invalid).is_err());
    let mut value = serde_json::to_value(report("alice", NOW)).unwrap();
    value["payload"]["unexpected"] = true.into();
    assert!(serde_json::from_value::<Signed<Settlement>>(value).is_err());
}
