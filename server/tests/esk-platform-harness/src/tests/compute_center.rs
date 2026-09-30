use super::*;
use crate::esk_asset::platform::sellback::{load_configuration, SellbackConfiguration};

fn billing_tables(fixture: &Fixture) {
    fixture.store.conn().unwrap().execute_batch(
        "CREATE TABLE user_balance(user_id TEXT PRIMARY KEY,balance_fen INTEGER);
         CREATE TABLE token_usage_events(id TEXT PRIMARY KEY,user_id TEXT,feature TEXT,
          billing_source TEXT,total_tokens INTEGER,input_tokens INTEGER,cached_input_tokens INTEGER,
          output_tokens INTEGER,created_at TEXT,idempotency_key TEXT);
         CREATE TABLE billing_events(id TEXT PRIMARY KEY,user_id TEXT,token_usage_event_id TEXT,
          model TEXT,input_tokens INTEGER,cached_input_tokens INTEGER,output_tokens INTEGER,
          cost_rmb_fen INTEGER,price_rule_version INTEGER,price_source TEXT,created_at TEXT);
         CREATE TABLE billing_reservations(id TEXT PRIMARY KEY,user_id TEXT,compute_call_id TEXT,
          feature TEXT,model TEXT,reserved_fen INTEGER,status TEXT,expires_at TEXT,created_at TEXT);
         INSERT INTO user_balance VALUES('alice',1234),('bob',9876);
         INSERT INTO token_usage_events VALUES('tok1','alice','chat','platform',12000,10000,0,2000,datetime('now'),'task1');
         INSERT INTO token_usage_events VALUES('ref1','alice','chat','client_reported',900000,900000,0,0,datetime('now'),NULL);
         INSERT INTO token_usage_events VALUES('bobtok','bob','chat','platform',42,40,0,2,datetime('now'),'bobtask');
         INSERT INTO billing_events VALUES('bill1','alice','tok1','test-model',10000,0,2000,4,1,'rule',datetime('now'));
         INSERT INTO billing_events VALUES('bobbill','bob','bobtok','test-model',40,0,2,999,2,'rule',datetime('now'));
         INSERT INTO billing_reservations VALUES('hold1','alice','task2','project_chat','test-model',200,'dispatch_hold',NULL,datetime('now'));"
    ).unwrap();
}

fn config() -> SellbackConfiguration {
    load_configuration()
}

#[test]
fn formal_assets_and_legacy_billing_are_separate_and_user_bound() {
    let fixture = Fixture::new();
    billing_tables(&fixture);
    let p = policy(100_000_000);
    history::post(&fixture, &p, "alice", 1);
    let a = fixture
        .store
        .esk_compute_center("alice", &token("alice"), 1, &config())
        .unwrap();
    assert_eq!(a.asset.total_base_units, "10000000");
    assert_eq!(a.asset.entry_count, "1");
    assert_eq!(a.legacy_balance_fen.as_deref(), Some("1234"));
    assert_eq!(a.month_cost_fen, "4");
    assert_eq!(a.bills.len(), 1);
    assert_eq!(a.bills[0].task_reference.as_deref(), Some("task1"));
    assert_eq!(a.holds[0].reserved_fen, "200");
    let reported = a
        .usage_sources
        .iter()
        .find(|v| v.billing_source == "client_reported")
        .unwrap();
    assert_eq!(reported.total_tokens, "900000");
    assert_eq!(a.bills[0].cost_fen, "4");
    let b = fixture
        .store
        .esk_compute_center("bob", &token("bob"), 1, &config())
        .unwrap();
    assert_eq!(b.asset.total_base_units, "0");
    assert_eq!(b.month_cost_fen, "999");
    assert!(b.holds.is_empty());
    assert!(fixture
        .store
        .esk_compute_center("bob", &token("alice"), 1, &config())
        .is_err());
    assert!(fixture
        .store
        .esk_compute_center("local-owner", &token("local-owner"), 1, &config())
        .is_err());
    assert!(fixture
        .store
        .esk_compute_center("inactive-user", &token("inactive-user"), 1, &config())
        .is_err());
    assert_eq!(fixture.paper_total(), 123000000);
}

#[test]
fn missing_storage_is_an_error_and_not_a_zero_account() {
    let fixture = Fixture::new();
    assert!(fixture
        .store
        .esk_compute_center("alice", &token("alice"), 1, &config())
        .is_err());
    billing_tables(&fixture);
    for page in [0, 1001, usize::MAX] {
        assert!(fixture
            .store
            .esk_compute_center("alice", &token("alice"), page, &config())
            .is_err());
    }
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE sessions SET revoked_at='fixture' WHERE user_id='alice'",
            [],
        )
        .unwrap();
    assert!(fixture
        .store
        .esk_compute_center("alice", &token("alice"), 1, &config())
        .is_err());
}

#[test]
fn billing_pages_and_hold_truncation_are_explicit() {
    let fixture = Fixture::new();
    billing_tables(&fixture);
    let conn = fixture.store.conn().unwrap();
    for index in 0..23 {
        conn.execute("INSERT INTO billing_events VALUES(?1,'alice',NULL,'m',0,0,0,1,NULL,'legacy',datetime('now'))", [format!("b{index:02}")]).unwrap();
        conn.execute("INSERT INTO billing_reservations VALUES(?1,'alice',?1,'chat','m',1,'verification_hold',NULL,datetime('now'))", [format!("h{index:02}")]).unwrap();
    }
    let a = fixture
        .store
        .esk_compute_center("alice", &token("alice"), 1, &config())
        .unwrap();
    let b = fixture
        .store
        .esk_compute_center("alice", &token("alice"), 2, &config())
        .unwrap();
    assert_eq!(a.bills.len(), 20);
    assert!(a.bills_has_more && a.holds_has_more);
    assert_eq!(b.bills.len(), 4);
    assert!(!b.bills_has_more);
    assert!(a
        .bills
        .iter()
        .all(|v| !b.bills.iter().any(|w| w.id == v.id)));
}
