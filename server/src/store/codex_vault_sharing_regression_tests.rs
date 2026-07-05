use super::{codex_vault_emergency::CodexVaultEmergencyLeaseCreate, Store};

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
