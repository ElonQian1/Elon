use super::{
    token_usage::{BILLING_SOURCE_OWN_CODEX, BILLING_SOURCE_SHARED_CODEX},
    SettleParams, Store, TokenUsageRecord,
};

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-token-usage-test-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("store should open"), path)
}

#[test]
fn trusted_usage_records_token_event_and_billing_event_atomically() {
    let (store, path) = temp_store();
    let user = store
        .create_user(
            &format!("billing-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .unwrap();
    store
        .billing_recharge(&user.id, 1_000, "test", "test", None)
        .unwrap();

    let record = TokenUsageRecord {
        user_id: &user.id,
        feature: "test_feature",
        usage_mode: "server_codex_cli",
        model: Some("gpt-4o-mini"),
        input_tokens: 1_000,
        cached_input_tokens: 100,
        output_tokens: 1_000,
        reasoning_tokens: 0,
        total_tokens: 2_000,
        billing_source: None,
        resource_owner_user_id: None,
        idempotency_key: None,
    };
    let result = crate::billing::account_trusted_usage(&store, &record).unwrap();

    assert_eq!(result.accounting_status, "billed");
    assert!(result.cost_rmb_fen > 0);
    assert!(result.billing_event_id.is_some());
    assert_eq!(
        store.billing_get_balance(&user.id).unwrap(),
        Some(1_000 - result.cost_rmb_fen)
    );

    let (events, total) = store.billing_list_events(&user.id, 1, 10).unwrap();
    assert_eq!(total, 1);
    assert_eq!(
        events[0].token_usage_event_id.as_deref(),
        Some(result.token_usage_event_id.as_str())
    );

    let stats = store.get_usage_stats(&user.id, 30).unwrap();
    assert_eq!(stats.total.total_tokens, 2_000);
    assert_eq!(stats.total.billable_tokens, 2_000);
    assert_eq!(stats.total.billed_cost_rmb_fen, result.cost_rmb_fen);
    let audit = store.admin_accounting_audit(30, 10).unwrap();
    assert!(audit.iter().any(|row| {
        row.user_id == user.id
            && row.accounting_status == "billed"
            && row.billed_cost_rmb_fen == result.cost_rmb_fen
    }));

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn trusted_usage_without_balance_row_is_recorded_but_not_billed() {
    let (store, path) = temp_store();
    store
        .billing_set_config("billing_required_for_all_users", "false")
        .unwrap();
    let user = store
        .create_user(
            &format!("unbilled-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .unwrap();

    let record = TokenUsageRecord {
        user_id: &user.id,
        feature: "test_feature",
        usage_mode: "server_codex_cli",
        model: Some("gpt-4o-mini"),
        input_tokens: 100,
        cached_input_tokens: 0,
        output_tokens: 100,
        reasoning_tokens: 0,
        total_tokens: 200,
        billing_source: None,
        resource_owner_user_id: None,
        idempotency_key: None,
    };
    let result = crate::billing::account_trusted_usage(&store, &record).unwrap();

    assert_eq!(result.accounting_status, "unbilled_no_balance");
    assert_eq!(result.cost_rmb_fen, 0);
    assert!(result.billing_event_id.is_none());
    let (_events, total) = store.billing_list_events(&user.id, 1, 10).unwrap();
    assert_eq!(total, 0);

    let stats = store.get_usage_stats(&user.id, 30).unwrap();
    assert_eq!(stats.total.total_tokens, 200);
    assert_eq!(stats.total.billable_tokens, 200);
    assert_eq!(stats.total.billed_cost_rmb_fen, 0);
    let audit = store.admin_accounting_audit(30, 10).unwrap();
    assert!(audit.iter().any(|row| {
        row.user_id == user.id
            && row.accounting_status == "unbilled_no_balance"
            && row.total_tokens == 200
    }));

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn strict_billing_auto_opens_missing_balance_row_and_bills_negative() {
    let (store, path) = temp_store();
    let user = store
        .create_user(
            &format!("strict-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .unwrap();

    let record = TokenUsageRecord {
        user_id: &user.id,
        feature: "test_feature",
        usage_mode: "server_codex_cli",
        model: Some("gpt-4o-mini"),
        input_tokens: 100,
        cached_input_tokens: 0,
        output_tokens: 100,
        reasoning_tokens: 0,
        total_tokens: 200,
        billing_source: None,
        resource_owner_user_id: None,
        idempotency_key: None,
    };
    let result = crate::billing::account_trusted_usage(&store, &record).unwrap();

    assert_eq!(result.accounting_status, "billed");
    assert!(result.cost_rmb_fen > 0);
    assert_eq!(
        store.billing_get_balance(&user.id).unwrap(),
        Some(-result.cost_rmb_fen)
    );
    let (events, total) = store.billing_list_events(&user.id, 1, 10).unwrap();
    assert_eq!(total, 1);
    assert_eq!(
        events[0].token_usage_event_id.as_deref(),
        Some(result.token_usage_event_id.as_str())
    );

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn trusted_usage_idempotency_key_prevents_double_billing() {
    let (store, path) = temp_store();
    let user = store
        .create_user(
            &format!("idempotent-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .unwrap();
    store
        .billing_recharge(&user.id, 1_000, "test", "test", None)
        .unwrap();

    let record = TokenUsageRecord {
        user_id: &user.id,
        feature: "test_feature",
        usage_mode: "server_codex_cli",
        model: Some("gpt-4o-mini"),
        input_tokens: 1_000,
        cached_input_tokens: 0,
        output_tokens: 1_000,
        reasoning_tokens: 0,
        total_tokens: 2_000,
        billing_source: None,
        resource_owner_user_id: None,
        idempotency_key: Some("trace:test-123"),
    };
    let first = crate::billing::account_trusted_usage(&store, &record).unwrap();
    let second = crate::billing::account_trusted_usage(&store, &record).unwrap();

    assert!(!first.deduplicated);
    assert!(second.deduplicated);
    assert_eq!(first.token_usage_event_id, second.token_usage_event_id);
    assert_eq!(first.billing_event_id, second.billing_event_id);
    assert_eq!(first.cost_rmb_fen, second.cost_rmb_fen);
    assert_eq!(
        store.billing_get_balance(&user.id).unwrap(),
        Some(1_000 - first.cost_rmb_fen)
    );

    let (events, total) = store.billing_list_events(&user.id, 1, 10).unwrap();
    assert_eq!(total, 1);
    assert_eq!(
        events[0].token_usage_event_id.as_deref(),
        Some(first.token_usage_event_id.as_str())
    );
    let stats = store.get_usage_stats(&user.id, 30).unwrap();
    assert_eq!(stats.total.total_tokens, 2_000);
    assert_eq!(stats.total.billed_cost_rmb_fen, first.cost_rmb_fen);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn own_codex_usage_records_tokens_without_billing_or_quota_cost() {
    let (store, path) = temp_store();
    let user = store
        .create_user(
            &format!("own-codex-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .unwrap();

    let record = TokenUsageRecord {
        user_id: &user.id,
        feature: "pc_agent_cli_chat",
        usage_mode: "pc_agent_cli",
        model: Some("gpt-5-codex"),
        input_tokens: 3_000,
        cached_input_tokens: 1_000,
        output_tokens: 2_000,
        reasoning_tokens: 500,
        total_tokens: 5_000,
        billing_source: Some(BILLING_SOURCE_OWN_CODEX),
        resource_owner_user_id: Some(&user.id),
        idempotency_key: Some("pc_agent_cli:own-codex-1"),
    };
    let result =
        crate::billing::account_trusted_usage_with_charge_policy(&store, &record, false).unwrap();

    assert_eq!(result.accounting_status, "unbilled_own_codex");
    assert_eq!(result.cost_rmb_fen, 0);
    assert!(result.billing_event_id.is_none());
    assert_eq!(store.billing_get_balance(&user.id).unwrap(), None);

    let stats = store.get_usage_stats(&user.id, 30).unwrap();
    assert_eq!(stats.total.total_tokens, 5_000);
    assert_eq!(stats.total.billable_tokens, 0);
    assert!(stats.by_billing_source.iter().any(|row| {
        row.billing_source == BILLING_SOURCE_OWN_CODEX && row.total_tokens == 5_000
    }));
    let ledger = store.user_progression_ledger(&user.id).unwrap();
    assert_eq!(ledger.consumed_tokens, 5_000);
    assert_eq!(ledger.own_codex_tokens, 5_000);
    assert_eq!(ledger.shared_codex_tokens, 0);
    assert_eq!(ledger.provided_tokens, 0);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn shared_codex_usage_bills_consumer_and_counts_provider_share() {
    let (store, path) = temp_store();
    let consumer = store
        .create_user(
            &format!("shared-consumer-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let provider = store
        .create_user(
            &format!("shared-provider-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .unwrap();
    store
        .billing_recharge(&consumer.id, 2_000, "test", "test", None)
        .unwrap();

    let record = TokenUsageRecord {
        user_id: &consumer.id,
        feature: "pc_agent_cli_chat",
        usage_mode: "pc_agent_cli",
        model: Some("gpt-5-codex"),
        input_tokens: 3_000,
        cached_input_tokens: 0,
        output_tokens: 2_000,
        reasoning_tokens: 0,
        total_tokens: 5_000,
        billing_source: Some(BILLING_SOURCE_SHARED_CODEX),
        resource_owner_user_id: Some(&provider.id),
        idempotency_key: Some("pc_agent_cli:shared-codex-1"),
    };
    let result =
        crate::billing::account_trusted_usage_with_charge_policy(&store, &record, true).unwrap();

    assert_eq!(result.accounting_status, "billed");
    assert!(result.cost_rmb_fen > 0);
    assert!(result.billing_event_id.is_some());
    assert_eq!(
        store.billing_get_balance(&consumer.id).unwrap(),
        Some(2_000 - result.cost_rmb_fen)
    );
    let stats = store.get_usage_stats(&consumer.id, 30).unwrap();
    assert_eq!(stats.total.billable_tokens, 5_000);
    assert!(stats.by_billing_source.iter().any(|row| {
        row.billing_source == BILLING_SOURCE_SHARED_CODEX && row.total_tokens == 5_000
    }));

    store
        .settle_node_inference(SettleParams {
            consumer_user_id: &consumer.id,
            provider_user_id: &provider.id,
            node_id: "node-shared-codex",
            model_id: "pc-cli/gpt-5-codex",
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            compute_call_id: result.idempotency_key.as_deref(),
            token_usage_event_id: Some(&result.token_usage_event_id),
            billing_event_id: result.billing_event_id.as_deref(),
            prompt_tokens: 3_000,
            completion_tokens: 2_000,
            price_per_1k_credits: 0.1,
            billed_cost_rmb_fen: result.cost_rmb_fen,
            accounting_status: Some(&result.accounting_status),
            provider_revenue_share_x1000: 800,
            platform_fee_rate: 0.2,
        })
        .unwrap();

    let consumer_ledger = store.user_progression_ledger(&consumer.id).unwrap();
    assert_eq!(consumer_ledger.consumed_tokens, 5_000);
    assert_eq!(consumer_ledger.shared_codex_tokens, 5_000);
    assert_eq!(consumer_ledger.own_codex_tokens, 0);
    assert_eq!(consumer_ledger.platform_tokens, 0);

    let provider_ledger = store.user_progression_ledger(&provider.id).unwrap();
    assert_eq!(provider_ledger.consumed_tokens, 0);
    assert_eq!(provider_ledger.provided_tokens, 5_000);
    assert_eq!(provider_ledger.provided_run_count, 1);
    assert!(provider_ledger.provider_earned_fen > 0);

    drop(store);
    let _ = std::fs::remove_file(path);
}
