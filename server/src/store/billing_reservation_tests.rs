use rusqlite::params;

use super::{
    BillingPriceSnapshot, BillingReservationRequest, PublicUser, Store, TokenUsageBillingCharge,
    TokenUsageRecord,
};

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-billing-reservation-test-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("store should open"), path)
}

fn create_funded_user(store: &Store, amount_fen: i64) -> PublicUser {
    let user = store
        .create_user(
            &format!("reservation-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .expect("user should be created");
    store
        .billing_recharge(&user.id, amount_fen, "test", "test", None)
        .expect("balance should be funded");
    user
}

#[test]
fn reservation_settles_with_refund_and_idempotency() {
    let (store, path) = temp_store();
    let user = create_funded_user(&store, 1_000);
    let key = "codex_cli:codex_cli_dev:test-reserve";

    let reservation = store
        .reserve_billing_call(&BillingReservationRequest {
            user_id: &user.id,
            compute_call_id: key,
            feature: "codex_cli_dev",
            usage_mode: "server_codex_cli",
            model: Some("gpt-4o-mini"),
            reserve_fen: 100,
            bill_missing_balance: true,
        })
        .expect("reservation should succeed");
    assert_eq!(reservation.reserved_fen, 100);
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(900));

    let record = TokenUsageRecord {
        user_id: &user.id,
        feature: "codex_cli_dev",
        usage_mode: "server_codex_cli",
        model: Some("gpt-4o-mini"),
        input_tokens: 100,
        cached_input_tokens: 0,
        output_tokens: 100,
        reasoning_tokens: 0,
        total_tokens: 200,
        billing_source: None,
        resource_owner_user_id: None,
        idempotency_key: Some(key),
    };
    let charge = TokenUsageBillingCharge {
        model: Some("gpt-4o-mini"),
        input_tokens: 100,
        cached_input_tokens: 0,
        output_tokens: 100,
        cost_rmb_fen: 60,
        exchange_rate_x10000: 73_000,
        markup_x1000: 1_200,
        price_snapshot: BillingPriceSnapshot::legacy(),
        bill_missing_balance: true,
        charge_platform_balance: true,
    };
    let first = store
        .record_token_usage_with_billing(&record, &charge)
        .expect("settlement should succeed");
    let second = store
        .record_token_usage_with_billing(&record, &charge)
        .expect("second settlement should dedupe");

    assert_eq!(first.accounting_status, "billed");
    assert!(!first.deduplicated);
    assert!(second.deduplicated);
    assert_eq!(first.token_usage_event_id, second.token_usage_event_id);
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(940));

    let (events, total) = store.billing_list_events(&user.id, 1, 10).unwrap();
    assert_eq!(total, 1);
    assert_eq!(events[0].cost_rmb_fen, 60);
    let summary = store.admin_billing_reconciliation_summary(30).unwrap();
    assert_eq!(summary.open_reservations, 0);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn release_billing_call_refunds_reserved_balance() {
    let (store, path) = temp_store();
    let user = create_funded_user(&store, 500);
    let key = "pc_agent_cli:test-release";

    store
        .reserve_billing_call(&BillingReservationRequest {
            user_id: &user.id,
            compute_call_id: key,
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            model: Some("copilot"),
            reserve_fen: 50,
            bill_missing_balance: true,
        })
        .unwrap();
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(450));

    let released = store
        .release_billing_call(&user.id, key, "released_no_usage")
        .unwrap()
        .expect("reservation should be released");
    assert_eq!(released.status, "released_no_usage");
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(500));
    assert!(store
        .release_billing_call(&user.id, key, "released_no_usage")
        .unwrap()
        .is_none());

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn expired_reservations_are_released_once() {
    let (store, path) = temp_store();
    let user = create_funded_user(&store, 300);
    let key = "node_llm:test-expired";

    store
        .reserve_billing_call(&BillingReservationRequest {
            user_id: &user.id,
            compute_call_id: key,
            feature: "node_llm",
            usage_mode: "server_node_llm",
            model: Some("gpt-4o-mini"),
            reserve_fen: 25,
            bill_missing_balance: true,
        })
        .unwrap();
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(275));
    {
        let conn = store.conn.lock().unwrap();
        conn.execute(
            "UPDATE billing_reservations SET expires_at = ?1 WHERE user_id = ?2 AND compute_call_id = ?3",
            params!["2000-01-01T00:00:00Z", user.id, key],
        )
        .unwrap();
    }

    assert_eq!(store.release_expired_billing_reservations().unwrap(), 1);
    assert_eq!(store.release_expired_billing_reservations().unwrap(), 0);
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(300));

    drop(store);
    let _ = std::fs::remove_file(path);
}
