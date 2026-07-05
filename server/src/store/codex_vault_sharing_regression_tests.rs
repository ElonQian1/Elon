use super::{
    codex_vault_emergency::CodexVaultEmergencyLeaseCreate,
    token_usage::BILLING_SOURCE_SHARED_CODEX, SettleParams, Store, TokenUsageRecord,
};

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-codex-sharing-regression-test-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("store should open"), path)
}

#[test]
fn reciprocal_shared_codex_usage_keeps_audit_chain_separate() {
    let (store, path) = temp_store();
    let a = store
        .create_user("sharing-a@example.com", "secret1", Some("A"), None)
        .unwrap();
    let b = store
        .create_user("sharing-b@example.com", "secret1", Some("B"), None)
        .unwrap();
    let grant_ab = store
        .upsert_codex_vault_emergency_grant(
            &a.id,
            &b.id,
            Some("A shares to B"),
            Some("robot_codex_vault_shared_access"),
            Some(900),
            None,
            &a.id,
        )
        .unwrap();
    let grant_ba = store
        .upsert_codex_vault_emergency_grant(
            &b.id,
            &a.id,
            Some("B shares to A"),
            Some("robot_codex_vault_shared_access"),
            Some(900),
            None,
            &b.id,
        )
        .unwrap();

    let lease_ab = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant_ab.id,
            provider_user_id: &a.id,
            consumer_user_id: &b.id,
            consumer_node_id: "node-b",
            provider_slot_id: "slot-a",
            account_hint_hash: Some("hint-a"),
            purpose: Some("unit_test_ab"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    let lease_ba = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant_ba.id,
            provider_user_id: &b.id,
            consumer_user_id: &a.id,
            consumer_node_id: "node-a",
            provider_slot_id: "slot-b",
            account_hint_hash: Some("hint-b"),
            purpose: Some("unit_test_ba"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();

    assert!(store
        .attach_codex_vault_emergency_usage(
            &lease_ab.id,
            Some("tok_ab"),
            Some("bill_ab"),
            Some("txn_ab"),
            100,
            50,
            9,
            7,
            Some("billed"),
        )
        .unwrap());
    assert!(store
        .attach_codex_vault_emergency_usage(
            &lease_ba.id,
            Some("tok_ba"),
            Some("bill_ba"),
            Some("txn_ba"),
            200,
            80,
            17,
            13,
            Some("settled"),
        )
        .unwrap());

    let audited_ab = store
        .get_codex_vault_emergency_lease(&lease_ab.id)
        .unwrap()
        .unwrap();
    let audited_ba = store
        .get_codex_vault_emergency_lease(&lease_ba.id)
        .unwrap()
        .unwrap();
    assert_eq!(audited_ab.billing_source, "shared_codex");
    assert_eq!(audited_ab.provider_user_id, a.id);
    assert_eq!(audited_ab.consumer_user_id, b.id);
    assert_eq!(audited_ab.token_usage_event_id.as_deref(), Some("tok_ab"));
    assert_eq!(audited_ab.billing_event_id.as_deref(), Some("bill_ab"));
    assert_eq!(audited_ab.node_transaction_id.as_deref(), Some("txn_ab"));
    assert_eq!(audited_ab.total_tokens, 150);
    assert_eq!(audited_ab.provider_earned_fen, 7);

    assert_eq!(audited_ba.billing_source, "shared_codex");
    assert_eq!(audited_ba.provider_user_id, b.id);
    assert_eq!(audited_ba.consumer_user_id, a.id);
    assert_eq!(audited_ba.token_usage_event_id.as_deref(), Some("tok_ba"));
    assert_eq!(audited_ba.billing_event_id.as_deref(), Some("bill_ba"));
    assert_eq!(audited_ba.node_transaction_id.as_deref(), Some("txn_ba"));
    assert_eq!(audited_ba.total_tokens, 280);
    assert_eq!(audited_ba.provider_earned_fen, 13);

    store
        .clear_codex_vault_emergency_lease_for_node(&b.id, "node-b", Some(&lease_ab.id))
        .unwrap()
        .expect("cleared AB lease");
    assert!(!store
        .attach_codex_vault_emergency_usage(
            &lease_ab.id,
            Some("tok_ab_after_clear"),
            Some("bill_ab_after_clear"),
            Some("txn_ab_after_clear"),
            1,
            1,
            1,
            1,
            Some("billed"),
        )
        .unwrap());
    assert!(store
        .attach_codex_vault_emergency_usage(
            &lease_ba.id,
            Some("tok_ba_second"),
            Some("bill_ba_second"),
            Some("txn_ba_second"),
            10,
            5,
            2,
            1,
            Some("settled"),
        )
        .unwrap());

    let final_ab = store
        .get_codex_vault_emergency_lease(&lease_ab.id)
        .unwrap()
        .unwrap();
    let final_ba = store
        .get_codex_vault_emergency_lease(&lease_ba.id)
        .unwrap()
        .unwrap();
    assert_eq!(final_ab.total_tokens, 150);
    assert_eq!(final_ab.billing_event_id.as_deref(), Some("bill_ab"));
    assert_eq!(final_ba.total_tokens, 295);
    assert_eq!(final_ba.billing_event_id.as_deref(), Some("bill_ba_second"));

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn platform_shared_codex_task_billing_links_full_lease_audit_chain() {
    let (store, path) = temp_store();
    let consumer = store
        .create_user(
            "sharing-e2e-consumer@example.com",
            "secret1",
            Some("consumer"),
            None,
        )
        .unwrap();
    let provider = store
        .create_user(
            "sharing-e2e-provider@example.com",
            "secret1",
            Some("provider"),
            None,
        )
        .unwrap();
    store
        .billing_recharge(&consumer.id, 2_000, "test", "shared-codex-e2e", None)
        .unwrap();

    let grant = store
        .upsert_codex_vault_emergency_grant(
            &provider.id,
            &consumer.id,
            Some("provider shares to consumer"),
            Some("robot_codex_vault_shared_access"),
            Some(900),
            None,
            &provider.id,
        )
        .unwrap();
    let lease = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant.id,
            provider_user_id: &provider.id,
            consumer_user_id: &consumer.id,
            consumer_node_id: "node-consumer",
            provider_slot_id: "slot-provider",
            account_hint_hash: Some("hint-provider"),
            purpose: Some("platform_task_billing_e2e"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();

    let usage = TokenUsageRecord {
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
        idempotency_key: Some("pc_agent_cli:shared-codex-platform-e2e"),
    };
    let accounting =
        crate::billing::account_trusted_usage_with_charge_policy(&store, &usage, true).unwrap();
    assert_eq!(accounting.accounting_status, "billed");
    assert!(accounting.cost_rmb_fen > 0);
    assert!(accounting.billing_event_id.is_some());

    let node_tx = store
        .settle_node_inference(SettleParams {
            consumer_user_id: &consumer.id,
            provider_user_id: &provider.id,
            node_id: "node-consumer",
            model_id: "pc-cli/gpt-5-codex",
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            compute_call_id: accounting.idempotency_key.as_deref(),
            token_usage_event_id: Some(&accounting.token_usage_event_id),
            billing_event_id: accounting.billing_event_id.as_deref(),
            prompt_tokens: 3_000,
            completion_tokens: 2_000,
            price_per_1k_credits: 0.1,
            billed_cost_rmb_fen: accounting.cost_rmb_fen,
            accounting_status: Some(&accounting.accounting_status),
            provider_revenue_share_x1000: 800,
            platform_fee_rate: 0.2,
        })
        .unwrap();
    assert!(node_tx.provider_earned_fen > 0);

    assert!(store
        .attach_codex_vault_emergency_usage(
            &lease.id,
            Some(&accounting.token_usage_event_id),
            accounting.billing_event_id.as_deref(),
            Some(&node_tx.id),
            usage.input_tokens,
            usage.output_tokens,
            accounting.cost_rmb_fen,
            node_tx.provider_earned_fen,
            Some(&accounting.accounting_status),
        )
        .unwrap());

    let replay_accounting =
        crate::billing::account_trusted_usage_with_charge_policy(&store, &usage, true).unwrap();
    assert!(replay_accounting.deduplicated);
    assert_eq!(
        replay_accounting.token_usage_event_id,
        accounting.token_usage_event_id
    );
    let replay_node_tx = store
        .settle_node_inference(SettleParams {
            consumer_user_id: &consumer.id,
            provider_user_id: &provider.id,
            node_id: "node-consumer",
            model_id: "pc-cli/gpt-5-codex",
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            compute_call_id: replay_accounting.idempotency_key.as_deref(),
            token_usage_event_id: Some(&replay_accounting.token_usage_event_id),
            billing_event_id: replay_accounting.billing_event_id.as_deref(),
            prompt_tokens: 3_000,
            completion_tokens: 2_000,
            price_per_1k_credits: 0.1,
            billed_cost_rmb_fen: replay_accounting.cost_rmb_fen,
            accounting_status: Some(&replay_accounting.accounting_status),
            provider_revenue_share_x1000: 800,
            platform_fee_rate: 0.2,
        })
        .unwrap();
    assert_eq!(replay_node_tx.id, node_tx.id);
    assert!(!store
        .attach_codex_vault_emergency_usage(
            &lease.id,
            Some(&replay_accounting.token_usage_event_id),
            replay_accounting.billing_event_id.as_deref(),
            Some(&replay_node_tx.id),
            usage.input_tokens,
            usage.output_tokens,
            replay_accounting.cost_rmb_fen,
            replay_node_tx.provider_earned_fen,
            Some(&replay_accounting.accounting_status),
        )
        .unwrap());

    let audited = store
        .get_codex_vault_emergency_lease(&lease.id)
        .unwrap()
        .expect("lease should exist");
    assert_eq!(audited.billing_source, BILLING_SOURCE_SHARED_CODEX);
    assert_eq!(audited.provider_user_id, provider.id);
    assert_eq!(audited.consumer_user_id, consumer.id);
    assert_eq!(
        audited.token_usage_event_id.as_deref(),
        Some(accounting.token_usage_event_id.as_str())
    );
    assert_eq!(
        audited.billing_event_id.as_deref(),
        accounting.billing_event_id.as_deref()
    );
    assert_eq!(
        audited.node_transaction_id.as_deref(),
        Some(node_tx.id.as_str())
    );
    assert_eq!(
        audited.total_tokens,
        usage.input_tokens + usage.output_tokens
    );
    assert_eq!(audited.billed_cost_rmb_fen, accounting.cost_rmb_fen);
    assert_eq!(audited.provider_earned_fen, node_tx.provider_earned_fen);
    assert_eq!(audited.accounting_status.as_deref(), Some("billed"));

    let conn = store.conn().unwrap();
    let linked_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
               FROM codex_vault_emergency_leases l
               JOIN token_usage_events t
                 ON t.id = l.token_usage_event_id
                AND t.user_id = l.consumer_user_id
                AND t.resource_owner_user_id = l.provider_user_id
                AND t.billing_source = 'shared_codex'
               JOIN billing_events b
                 ON b.id = l.billing_event_id
                AND b.token_usage_event_id = t.id
                AND b.user_id = l.consumer_user_id
               JOIN node_transactions n
                 ON n.id = l.node_transaction_id
                AND n.token_usage_event_id = t.id
                AND n.billing_event_id = b.id
                AND n.consumer_user_id = l.consumer_user_id
                AND n.provider_user_id = l.provider_user_id
              WHERE l.id = ?1",
            rusqlite::params![lease.id],
            |row| row.get(0),
        )
        .unwrap();
    drop(conn);
    assert_eq!(linked_count, 1);

    let consumer_health = store.codex_vault_sharing_health(&consumer.id).unwrap();
    assert_eq!(consumer_health.status, "ok");
    assert_eq!(consumer_health.active_lease_count, 1);
    assert_eq!(consumer_health.accounting_anomaly_count, 0);

    store
        .clear_codex_vault_emergency_lease_for_node(&consumer.id, "node-consumer", Some(&lease.id))
        .unwrap()
        .expect("active lease should clear");
    assert!(!store
        .attach_codex_vault_emergency_usage(
            &lease.id,
            Some("tok-after-clear"),
            Some("bill-after-clear"),
            Some("txn-after-clear"),
            1,
            1,
            1,
            1,
            Some("billed"),
        )
        .unwrap());
    let final_health = store.codex_vault_sharing_health(&consumer.id).unwrap();
    assert_eq!(final_health.status, "ok");
    assert_eq!(final_health.active_lease_count, 0);

    drop(store);
    let _ = std::fs::remove_file(path);
}
