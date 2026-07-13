use rusqlite::params;

use super::{
    token_usage::{BillingReservationConstraint, BillingReservationConstraintViolation},
    BillingPriceSnapshot, BillingReservationRequest, NodeComputeReplayBinding, NodeComputeRunStart,
    PublicUser, Store, TokenUsageBillingCharge, TokenUsageRecord,
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
        reservation_constraint: None,
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
    let active = store
        .get_active_billing_reservation(&user.id, key)
        .unwrap()
        .expect("reservation should be replayable while reserved");
    assert_eq!(active.compute_call_id, key);
    assert_eq!(active.reserved_fen, 50);
    assert!(store
        .billing_reservation_is_still_reserved(&user.id, key)
        .unwrap());
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: key,
            consumer_user_id: &user.id,
            provider_user_id: None,
            node_id: "node-reservation",
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .unwrap();
    store
        .bind_node_compute_run_replay_policy(
            key,
            NodeComputeReplayBinding {
                billing_source: "platform",
                resource_owner_user_id: None,
                lease_id: None,
                offline_policy: "require_active_reservation",
                replay_deadline: Some("2099-01-01T00:00:00Z"),
                max_cost_rmb_fen: 50,
                allowance_id: None,
            },
        )
        .unwrap();
    assert!(store.can_replay_node_compute_run_offline(key).unwrap());

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
    assert!(!store
        .billing_reservation_is_still_reserved(&user.id, key)
        .unwrap());
    assert!(!store.can_replay_node_compute_run_offline(key).unwrap());

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

    assert!(!store
        .billing_reservation_is_still_reserved(&user.id, key)
        .unwrap());

    assert_eq!(store.release_expired_billing_reservations().unwrap(), 1);
    assert_eq!(store.release_expired_billing_reservations().unwrap(), 0);
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(300));

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn verification_hold_survives_expiry_janitor_without_refund() {
    let (store, path) = temp_store();
    let user = create_funded_user(&store, 300);
    let key = "pc_agent_cli:test-verification-hold";
    store
        .reserve_billing_call(&BillingReservationRequest {
            user_id: &user.id,
            compute_call_id: key,
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            model: Some("pc-cli/codex"),
            reserve_fen: 25,
            bill_missing_balance: true,
        })
        .unwrap();
    store
        .hold_billing_reservation_for_dispatch(&user.id, key)
        .unwrap()
        .expect("accepted work should first become a dispatch hold");
    {
        let conn = store.conn.lock().unwrap();
        conn.execute(
            "UPDATE billing_reservations SET expires_at = ?1 WHERE user_id = ?2 AND compute_call_id = ?3",
            params!["2000-01-01T00:00:00Z", user.id, key],
        )
        .unwrap();
    }

    let held = store
        .hold_billing_reservation_for_verification(&user.id, key)
        .unwrap()
        .expect("reserved funds should become a verification hold");
    assert_eq!(held.reserved_fen, 25);
    assert_eq!(held.expires_at, None);
    assert_eq!(store.release_expired_billing_reservations().unwrap(), 0);
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(275));
    assert!(store
        .release_billing_call(&user.id, key, "released_no_usage")
        .unwrap()
        .is_none());
    assert_eq!(
        store
            .admin_billing_reservations(Some("verification_hold"), 10)
            .unwrap()[0]
            .status,
        "verification_hold"
    );

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn late_durable_completion_after_execution_deadline_settles_dispatch_hold() {
    let (store, path) = temp_store();
    let user = create_funded_user(&store, 1_000);
    let key = "pc_agent_cli:test-dispatch-hold-settlement";
    store
        .reserve_billing_call(&BillingReservationRequest {
            user_id: &user.id,
            compute_call_id: key,
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            model: Some("pc-cli/codex"),
            reserve_fen: 100,
            bill_missing_balance: true,
        })
        .unwrap();
    let reserved_deadline = store
        .get_active_billing_reservation(&user.id, key)
        .unwrap()
        .unwrap()
        .expires_at
        .expect("pre-dispatch reservation should have an execution deadline");
    let held = store
        .hold_billing_reservation_for_dispatch(&user.id, key)
        .unwrap()
        .expect("pre-send reservation should become durable");
    assert_eq!(held.reserved_fen, 100);
    assert_eq!(held.expires_at.as_deref(), Some(reserved_deadline.as_str()));
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: key,
            consumer_user_id: &user.id,
            provider_user_id: None,
            node_id: "node-dispatch-hold",
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            route_reason: Some("test"),
        })
        .unwrap();
    store
        .bind_node_compute_run_replay_policy(
            key,
            NodeComputeReplayBinding {
                billing_source: "platform",
                resource_owner_user_id: None,
                lease_id: None,
                offline_policy: "require_active_reservation",
                replay_deadline: Some("2000-01-01T00:00:00Z"),
                max_cost_rmb_fen: 100,
                allowance_id: Some(&held.reservation_id),
            },
        )
        .unwrap();
    {
        let conn = store.conn.lock().unwrap();
        conn.execute(
            "UPDATE billing_reservations SET expires_at = ?1, updated_at = ?1
              WHERE user_id = ?2 AND compute_call_id = ?3",
            params!["2000-01-01T00:00:00Z", user.id, key],
        )
        .unwrap();
    }

    assert_eq!(store.release_expired_billing_reservations().unwrap(), 0);
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(900));
    assert!(
        store.can_replay_node_compute_run_offline(key).unwrap(),
        "a terminal outbox receipt may arrive after execution authorization expired"
    );

    let record = TokenUsageRecord {
        user_id: &user.id,
        feature: "pc_agent_cli_chat",
        usage_mode: "pc_agent_cli",
        model: Some("pc-cli/codex"),
        input_tokens: 100,
        cached_input_tokens: 0,
        output_tokens: 100,
        reasoning_tokens: 0,
        total_tokens: 200,
        billing_source: Some("platform"),
        resource_owner_user_id: None,
        idempotency_key: Some(key),
    };
    let oversized_charge = TokenUsageBillingCharge {
        model: Some("pc-cli/codex"),
        input_tokens: 100,
        cached_input_tokens: 0,
        output_tokens: 100,
        cost_rmb_fen: 101,
        exchange_rate_x10000: 73_000,
        markup_x1000: 1_200,
        price_snapshot: BillingPriceSnapshot::legacy(),
        bill_missing_balance: true,
        charge_platform_balance: true,
        reservation_constraint: Some(BillingReservationConstraint {
            expected_reservation_id: &held.reservation_id,
            max_cost_rmb_fen: held.reserved_fen,
        }),
    };
    let error = store
        .record_token_usage_with_billing(&record, &oversized_charge)
        .expect_err("late completion must not expand the frozen allowance");
    assert!(matches!(
        error.downcast_ref::<BillingReservationConstraintViolation>(),
        Some(BillingReservationConstraintViolation::CostExceedsFrozenMaximum)
    ));
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(900));

    let charge = TokenUsageBillingCharge {
        model: Some("pc-cli/codex"),
        input_tokens: 100,
        cached_input_tokens: 0,
        output_tokens: 100,
        cost_rmb_fen: 60,
        exchange_rate_x10000: 73_000,
        markup_x1000: 1_200,
        price_snapshot: BillingPriceSnapshot::legacy(),
        bill_missing_balance: true,
        charge_platform_balance: true,
        reservation_constraint: Some(BillingReservationConstraint {
            expected_reservation_id: &held.reservation_id,
            max_cost_rmb_fen: held.reserved_fen,
        }),
    };
    let settled = store
        .record_token_usage_with_billing(&record, &charge)
        .expect("late trusted completion usage should settle the bounded dispatch hold");
    assert_eq!(settled.accounting_status, "billed");
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(940));
    assert_eq!(
        store
            .admin_billing_reservations(Some("settled"), 10)
            .unwrap()[0]
            .status,
        "settled"
    );

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn dispatch_hold_requires_verified_pre_send_release() {
    let (store, path) = temp_store();
    let user = create_funded_user(&store, 300);
    let key = "pc_agent_cli:test-dispatch-not-sent";
    store
        .reserve_billing_call(&BillingReservationRequest {
            user_id: &user.id,
            compute_call_id: key,
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            model: Some("pc-cli/codex"),
            reserve_fen: 25,
            bill_missing_balance: true,
        })
        .unwrap();
    store
        .hold_billing_reservation_for_dispatch(&user.id, key)
        .unwrap()
        .expect("dispatch hold should exist before send");

    assert!(store
        .release_billing_call(&user.id, key, "released_error")
        .unwrap()
        .is_none());
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(275));
    let released = store
        .release_dispatch_billing_hold_before_send(&user.id, key)
        .unwrap()
        .expect("positive never-sent evidence may refund the dispatch hold");
    assert_eq!(released.status, "released_dispatch_not_sent");
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(300));

    drop(store);
    let _ = std::fs::remove_file(path);
}
