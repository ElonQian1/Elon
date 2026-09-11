use super::{
    funding,
    funding_source::{self, *},
    ledger,
    model::*,
    policy, tests,
};
use rusqlite::Connection;
use std::cell::Cell;
use tests::NOW;

fn total_changes(c: &Connection) -> i64 {
    c.query_row("SELECT total_changes()", [], |r| r.get(0))
        .unwrap()
}

fn prepared() -> (Connection, policy::Policy, BudgetIntent) {
    let (mut c, p) = tests::fixture(None);
    let digest = ledger::record_settlement(
        &mut c,
        &p,
        "admin",
        "admin",
        &tests::report("alice", NOW),
        &|| Ok(NOW),
    )
    .unwrap();
    let intent = tests::intent(&p, &digest, "alice", 1);
    ledger::prepare(&mut c, &p, "admin", "admin", &intent, &|| Ok(NOW)).unwrap();
    (c, p, intent)
}
fn query(p: &policy::Policy, intent: &BudgetIntent) -> InspectRequest {
    InspectRequest {
        schema: "esk.game.rewards.funding-source.request.v1".into(),
        policy_digest: p.digest.clone(),
        allocation_hash: intent.allocation_hash.clone(),
    }
}
fn page(p: &policy::Policy) -> PendingRequest {
    PendingRequest {
        schema: "esk.game.rewards.pending-funding.request.v1".into(),
        policy_digest: p.digest.clone(),
        after_allocation_hash: None,
        limit: 1,
    }
}
fn inspect(c: &mut Connection, p: &policy::Policy, i: &BudgetIntent) -> SourceResponse {
    funding_source::inspect(c, p, "admin", "admin", &query(p, i), &|| Ok(NOW)).unwrap()
}
#[test]
fn funding_source_preserves_exact_signed_source_and_never_authorizes_payment() {
    let (mut c, p, i) = prepared();
    let before = total_changes(&c);
    let response = inspect(&mut c, &p, &i);
    assert!(!response.offchain_payment_authorized);
    assert!(!response.source.budget.offchain_payment_authorized);
    assert_eq!(response.source.source_status, SourceStatus::Reserved);
    assert_eq!(
        response.source.original_settlement,
        tests::report("alice", NOW)
    );
    assert_eq!(
        response.source.latest_settlement,
        response.source.original_settlement
    );
    assert_eq!(response.source.budget.intent, i);
    assert_eq!(response.source.reserved_profit_units, "500");
    assert_eq!(response.source.cumulative_net_profit_units, "1000");
    assert_eq!(response.observed_at_ms, NOW.to_string());
    assert_eq!(total_changes(&c), before);
}
#[test]
fn funding_source_pages_stably_and_excludes_only_attested_budgets() {
    let (mut c, p, i) = prepared();
    let second = tests::intent(&p, &i.report_digest, "alice", 2);
    ledger::prepare(&mut c, &p, "admin", "admin", &second, &|| Ok(NOW)).unwrap();
    let first = pending(&mut c, &p, "admin", "admin", &page(&p), &|| Ok(NOW)).unwrap();
    assert_eq!(first.sources.len(), 1);
    assert_eq!(first.sources[0].budget.intent, i);
    assert_eq!(
        first.next_after_allocation_hash,
        Some(i.allocation_hash.clone())
    );
    let mut request = page(&p);
    request.after_allocation_hash = first.next_after_allocation_hash;
    let next = pending(&mut c, &p, "admin", "admin", &request, &|| Ok(NOW)).unwrap();
    assert_eq!(next.sources[0].budget.intent, second);
    assert!(next.next_after_allocation_hash.is_none());
    let first_record = inspect(&mut c, &p, &i).source.budget;
    funding::confirm(
        &mut c,
        &p,
        "admin",
        "admin",
        &tests::evidence(&p, &first_record, NOW),
        &|| Ok(NOW),
    )
    .unwrap();
    let page = pending(&mut c, &p, "admin", "admin", &page(&p), &|| Ok(NOW)).unwrap();
    assert_eq!(page.sources[0].budget.intent, second);
    assert!(page.next_after_allocation_hash.is_none());
    assert_eq!(
        inspect(&mut c, &p, &i).source.source_status,
        SourceStatus::FundingAttested
    );
}
#[test]
fn funding_source_reports_later_losses_without_releasing_existing_reservations() {
    let (mut c, p, i) = prepared();
    let mut updated = tests::report("alice", NOW).payload;
    updated.sequence = "2".into();
    updated.previous_report_digest = i.report_digest.clone();
    updated.period_end_ms = (NOW - 1).to_string();
    updated.cumulative_net_profit_units = "-50".into();
    let proof = tests::sign("settlement", updated, 11);
    ledger::record_settlement(&mut c, &p, "admin", "admin", &proof, &|| Ok(NOW)).unwrap();
    let response = inspect(&mut c, &p, &i);
    assert_eq!(response.source.source_status, SourceStatus::ProfitDeficit);
    assert_eq!(response.source.latest_settlement, proof);
    assert_eq!(response.source.cumulative_net_profit_units, "-50");
    assert_eq!(response.source.reserved_profit_units, "500");
    assert_eq!(ledger::reserved(&c, "alice").unwrap(), 500);
    c.execute("UPDATE users SET status='disabled' WHERE id='alice'", [])
        .unwrap();
    let response = pending(&mut c, &p, "admin", "admin", &page(&p), &|| Ok(NOW)).unwrap();
    assert_eq!(
        response.sources[0].source_status,
        SourceStatus::UserInactive
    );
}
#[test]
fn funding_source_rejects_non_admin_revoked_expired_and_stale_observation() {
    let (mut c, p, i) = prepared();
    for (actor, token) in [
        ("alice", "alice"),
        ("local-owner", "admin"),
        ("admin", "fake"),
    ] {
        assert!(
            funding_source::inspect(&mut c, &p, actor, token, &query(&p, &i), &|| Ok(NOW)).is_err()
        );
        assert!(pending(&mut c, &p, actor, token, &page(&p), &|| Ok(NOW)).is_err());
    }
    for end in [NOW - 1, NOW + 5001, NOW + 10001] {
        let reads = Cell::new(0);
        let clock = || {
            let n = reads.get();
            reads.set(n + 1);
            Ok(if n == 0 { NOW } else { end })
        };
        assert!(
            funding_source::inspect(&mut c, &p, "admin", "admin", &query(&p, &i), &clock).is_err()
        );
    }
    let reads = Cell::new(0);
    let clock = || {
        let n = reads.get();
        reads.set(n + 1);
        Ok(if n == 0 { NOW + 9000 } else { NOW + 10001 })
    };
    assert!(funding_source::inspect(&mut c, &p, "admin", "admin", &query(&p, &i), &clock).is_err());
    c.execute(
        "UPDATE sessions SET revoked_at='revoked' WHERE user_id='admin'",
        [],
    )
    .unwrap();
    assert!(
        funding_source::inspect(&mut c, &p, "admin", "admin", &query(&p, &i), &|| Ok(NOW)).is_err()
    );
    assert!(pending(&mut c, &p, "admin", "admin", &page(&p), &|| Ok(NOW)).is_err());
}
#[test]
fn funding_source_rejects_policy_drift_unknown_allocations_and_unbounded_inputs() {
    let (mut c, p, i) = prepared();
    let mut request = query(&p, &i);
    request.policy_digest = "aa".repeat(32);
    assert!(funding_source::inspect(&mut c, &p, "admin", "admin", &request, &|| Ok(NOW)).is_err());
    request = query(&p, &i);
    request.allocation_hash = "ff".repeat(32);
    assert!(funding_source::inspect(&mut c, &p, "admin", "admin", &request, &|| Ok(NOW)).is_err());
    for n in [0, 21, 255] {
        let mut request = page(&p);
        request.limit = n;
        assert!(pending(&mut c, &p, "admin", "admin", &request, &|| Ok(NOW)).is_err());
    }
    for cursor in ["", "00", "A", &"A".repeat(64)] {
        let mut request = page(&p);
        request.after_allocation_hash = Some(cursor.into());
        assert!(pending(&mut c, &p, "admin", "admin", &request, &|| Ok(NOW)).is_err());
    }
    assert!(serde_json::from_value::<InspectRequest>(serde_json::json!({"schema":"x","policy_digest":p.digest,"allocation_hash":i.allocation_hash,"verified":true})).is_err());
    assert!(serde_json::from_value::<PendingRequest>(
        serde_json::json!({"schema":"x","policy_digest":p.digest,"limit":1,"token":"x"})
    )
    .is_err());
    let mut config = tests::config();
    config.registry_id = format!("0x{}", "ab".repeat(32));
    let other = policy::Policy::from_input(config).unwrap();
    assert!(funding_source::inspect(
        &mut c,
        &other,
        "admin",
        "admin",
        &query(&other, &i),
        &|| Ok(NOW)
    )
    .is_err());
}
#[test]
fn funding_source_revalidates_signed_database_records_and_intent_relationship() {
    for table in [
        "game_reward_reports",
        "game_reward_intents",
        "game_reward_funding",
    ] {
        let (mut c, p, i) = prepared();
        if table == "game_reward_funding" {
            let record = inspect(&mut c, &p, &i).source.budget;
            funding::confirm(
                &mut c,
                &p,
                "admin",
                "admin",
                &tests::evidence(&p, &record, NOW),
                &|| Ok(NOW),
            )
            .unwrap();
        }
        // Simulate restored/corrupt storage, not an API or a permitted production mutation.
        c.execute_batch(&format!("DROP TRIGGER {table}_no_update"))
            .unwrap();
        let sql = match table {
            "game_reward_reports" => "UPDATE game_reward_reports SET proof_json=json_set(proof_json,'$.payload.cumulative_net_profit_units','999999')",
            "game_reward_intents" => "UPDATE game_reward_intents SET user_id='bob'",
            _ => "UPDATE game_reward_funding SET proof_json=json_set(proof_json,'$.payload.amount_units','999999')",
        };
        c.execute_batch(sql).unwrap();
        assert!(
            funding_source::inspect(&mut c, &p, "admin", "admin", &query(&p, &i), &|| Ok(NOW))
                .is_err()
        );
    }
}
#[test]
fn funding_source_empty_page_does_not_create_a_policy_or_reservation() {
    let (mut c, p) = tests::fixture(None);
    let before = total_changes(&c);
    let result = pending(&mut c, &p, "admin", "admin", &page(&p), &|| Ok(NOW)).unwrap();
    assert!(result.sources.is_empty());
    assert!(result.next_after_allocation_hash.is_none());
    assert!(!result.offchain_payment_authorized);
    assert_eq!(total_changes(&c), before);
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM game_reward_policy", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
