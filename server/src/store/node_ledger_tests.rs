use super::*;

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-node-ledger-test-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("store should open"), path)
}

#[test]
fn install_id_renewal_reuses_existing_node_credential() {
    let (store, path) = temp_store();
    let owner = store
        .create_user("node-install-owner@example.com", "secret1", None, None)
        .unwrap();
    store
        .create_node_credential(
            "node-old",
            "old-hash",
            &owner.id,
            Some("一龙4060"),
            Some("ELONQIAN"),
            Some("ins_same"),
        )
        .unwrap();

    let reused = store
        .renew_node_credential_by_install_id(
            &owner.id,
            "ins_same",
            "new-hash",
            Some("ELONQIAN"),
            Some("ELONQIAN"),
        )
        .unwrap();

    assert_eq!(reused.as_deref(), Some("node-old"));
    assert_eq!(
        store
            .get_node_credential_hash("node-old")
            .unwrap()
            .as_deref(),
        Some("new-hash")
    );
    let credential = store
        .get_node_credential("node-old")
        .unwrap()
        .expect("credential");
    assert_eq!(credential.label, "一龙4060");
    assert_eq!(credential.device_name.as_deref(), Some("ELONQIAN"));
    assert_eq!(credential.install_id.as_deref(), Some("ins_same"));

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn settlement_uses_real_billed_cost_and_is_idempotent() {
    let (store, path) = temp_store();
    let consumer = store
        .create_user("node-ledger-consumer@example.com", "secret1", None, None)
        .unwrap();
    let provider = store
        .create_user("node-ledger-provider@example.com", "secret1", None, None)
        .unwrap();

    let params = SettleParams {
        consumer_user_id: &consumer.id,
        provider_user_id: &provider.id,
        node_id: "node-a",
        model_id: "pc-cli/codex",
        feature: "pc_agent_cli_dev",
        usage_mode: "pc_agent_cli",
        compute_call_id: Some("pc_agent_cli:req-1"),
        token_usage_event_id: Some("tok-real-1"),
        billing_event_id: Some("bev-real-1"),
        prompt_tokens: 400,
        completion_tokens: 600,
        price_per_1k_credits: 99.0,
        billed_cost_rmb_fen: 123,
        accounting_status: Some("billed"),
        provider_revenue_share_x1000: 800,
        platform_fee_rate: 0.2,
    };

    let first = store.settle_node_inference(params).unwrap();
    let second = store
        .settle_node_inference(SettleParams {
            consumer_user_id: &consumer.id,
            provider_user_id: &provider.id,
            node_id: "node-a",
            model_id: "pc-cli/codex",
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            compute_call_id: Some("pc_agent_cli:req-1"),
            token_usage_event_id: Some("tok-real-1"),
            billing_event_id: Some("bev-real-1"),
            prompt_tokens: 400,
            completion_tokens: 600,
            price_per_1k_credits: 99.0,
            billed_cost_rmb_fen: 123,
            accounting_status: Some("billed"),
            provider_revenue_share_x1000: 800,
            platform_fee_rate: 0.2,
        })
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(first.charged_credits, 1.23);
    assert_eq!(first.provider_earned_fen, 98);
    assert_eq!(first.settled_credits, 0.98);
    assert_eq!(first.billing_event_id.as_deref(), Some("bev-real-1"));
    assert_eq!(first.token_usage_event_id.as_deref(), Some("tok-real-1"));
    assert_eq!(store.get_node_balance_fen(&provider.id).unwrap(), 98);
    assert_eq!(store.get_node_balance(&provider.id).unwrap(), 0.98);
    assert_eq!(store.get_lifetime_earned_fen(&provider.id).unwrap(), 98);
    assert_eq!(store.get_lifetime_earned(&provider.id).unwrap(), 0.98);

    let txs = store.list_node_transactions(&provider.id, 10).unwrap();
    assert_eq!(txs.len(), 1);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn unbilled_usage_does_not_increase_provider_balance() {
    let (store, path) = temp_store();
    let consumer = store
        .create_user(
            "node-ledger-unbilled-consumer@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let provider = store
        .create_user(
            "node-ledger-unbilled-provider@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();

    let tx = store
        .settle_node_inference(SettleParams {
            consumer_user_id: &consumer.id,
            provider_user_id: &provider.id,
            node_id: "node-b",
            model_id: "local/qwen",
            feature: "node_llm",
            usage_mode: "server_node_llm",
            compute_call_id: Some("node_llm:req-2"),
            token_usage_event_id: Some("tok-unbilled-1"),
            billing_event_id: None,
            prompt_tokens: 500,
            completion_tokens: 500,
            price_per_1k_credits: 99.0,
            billed_cost_rmb_fen: 0,
            accounting_status: Some("unbilled_no_balance"),
            provider_revenue_share_x1000: 800,
            platform_fee_rate: 0.2,
        })
        .unwrap();

    assert_eq!(tx.settlement_status, "unbilled_no_balance");
    assert_eq!(tx.provider_earned_fen, 0);
    assert_eq!(tx.settled_credits, 0.0);
    assert_eq!(store.get_node_balance(&provider.id).unwrap(), 0.0);

    drop(store);
    let _ = std::fs::remove_file(path);
}
