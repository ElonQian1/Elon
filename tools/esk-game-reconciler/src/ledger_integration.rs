use crate::{
    prepare,
    tests::{fixture, sign},
    verify::*,
};
use elon_game_rewards_harness::{
    ledger,
    model::{BudgetIntent, Settlement, Signed},
    policy::{Policy, PolicyInput},
};
use rusqlite::{params, Connection};

#[test]
fn approved_candidate_records_once_and_actual_ledger_prevents_profit_reuse() {
    let (mut config, mut request) = fixture();
    let address = config.beneficiary.clone();
    let policy = Policy::from_input(PolicyInput {
        schema: "esk.game.rewards.policy.v1".into(),
        main_issuer: "synthetic-main".into(),
        network: "testnet".into(),
        chain_identifier: "4c78adac".into(),
        genesis_checkpoint_digest: "69WiPg3DAQiwdxfncX6wYQ2siKwAe6L9BZthQea3JNMD".into(),
        rewards_package_id: address.clone(),
        registry_id: address.clone(),
        asset_type: config.asset_type.clone(),
        asset_decimals: config.asset_decimals,
        reconciler_public_key_hex: config.reconciler_public_key_hex.clone(),
        funding_observer_public_key_hex: config.source_public_key_hex.clone(),
    })
    .unwrap();
    config.policy_digest = policy.digest.clone();
    request.statement.payload.policy_digest = policy.digest.clone();
    request.statement = sign(request.statement.payload, SOURCE_DOMAIN, 31);
    let candidate = prepare(&config, &request, 1000).unwrap();
    let payload: Settlement =
        serde_json::from_value(serde_json::to_value(candidate.settlement).unwrap()).unwrap();
    let proof = sign(payload, SETTLEMENT_DOMAIN, 32);
    let proof = Signed {
        payload: proof.payload,
        signature_hex: proof.signature_hex,
    };
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE users(id TEXT PRIMARY KEY,status TEXT,role TEXT); CREATE TABLE sessions(user_id TEXT,token_hash TEXT,revoked_at TEXT,expires_at TEXT); INSERT INTO users VALUES('admin','active','admin'),('user-7','active','user');").unwrap();
    conn.execute(
        "INSERT INTO sessions VALUES('admin',?1,NULL,'2099-01-01T00:00:00Z')",
        [hash("synthetic-session")],
    )
    .unwrap();
    conn.execute_batch(include_str!(
        "../../../server/src/esk_platform/game_rewards/schema.sql"
    ))
    .unwrap();
    let clock = || Ok(1000);
    let digest = ledger::record_settlement(
        &mut conn,
        &policy,
        "admin",
        "synthetic-session",
        &proof,
        &clock,
    )
    .unwrap();
    assert_eq!(
        ledger::record_settlement(
            &mut conn,
            &policy,
            "admin",
            "synthetic-session",
            &proof,
            &clock
        )
        .unwrap(),
        digest
    );
    let mut intent = BudgetIntent {
        schema: "esk.game.rewards.intent.v1".into(),
        policy_digest: policy.digest.clone(),
        allocation_hash: "55".repeat(32),
        report_digest: digest,
        user_id: config.user_id.clone(),
        beneficiary: address.clone(),
        mint_authority: address.clone(),
        creator: address,
        content_hash: "66".repeat(32),
        license_hash: "77".repeat(32),
        allow_modification: true,
        allow_external_game: true,
        opens_at_ms: "1000".into(),
        direct_claim_after_ms: "2000".into(),
        denominations: [1, 5, 10, 50, 100, 500].map(|n| n.to_string()).to_vec(),
        tickets: [0, 0, 0, 0, 0, 4].map(|n| n.to_string()).to_vec(),
    };
    let budget = ledger::prepare(
        &mut conn,
        &policy,
        "admin",
        "synthetic-session",
        &intent,
        &clock,
    )
    .unwrap();
    assert_eq!(budget.amount_units, "2000");
    assert!(!budget.offchain_payment_authorized);
    intent.allocation_hash = "88".repeat(32);
    assert!(ledger::prepare(
        &mut conn,
        &policy,
        "admin",
        "synthetic-session",
        &intent,
        &clock
    )
    .is_err());
    assert_eq!(ledger::reserved(&conn, &config.user_id).unwrap(), 2000);
    let reports: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM game_reward_reports WHERE user_id=?1",
            params![config.user_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reports, 1);
}
