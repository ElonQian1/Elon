use super::{reserve_midrun_shared_call, validate_midrun_switch};
use crate::store::{
    codex_vault_emergency::CodexVaultEmergencyLeaseCreate,
    CodexVaultEmergencyCredentialDeliveryClaim, NodeComputeReplayBinding, NodeComputeRunStart,
    Store,
};

fn test_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-midrun-shared-switch-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("test store should open"), path)
}

#[tokio::test]
async fn midrun_shared_switch_is_bound_to_exact_owner_node_and_live_hold() {
    let (store, path) = test_store();
    let consumer = store
        .create_user("midrun-consumer@example.com", "secret1", None, None)
        .unwrap();
    let provider = store
        .create_user("midrun-provider@example.com", "secret1", None, None)
        .unwrap();
    store
        .upsert_user_codex_credential(
            &provider.id,
            "chatgpt",
            Some("midrun-account"),
            Some("test"),
            "ciphertext",
            "nonce",
        )
        .unwrap();
    let slot = store
        .select_user_codex_credential_slot(&provider.id, None)
        .unwrap()
        .unwrap();
    let grant = store
        .upsert_codex_vault_emergency_grant(
            &provider.id,
            &consumer.id,
            Some("midrun test"),
            Some("unit_test"),
            Some(900),
            None,
            &provider.id,
        )
        .unwrap();
    store
        .create_node_credential(
            "node-midrun",
            "secret-hash",
            &consumer.id,
            Some("midrun node"),
            None,
            Some("install-midrun"),
        )
        .unwrap();
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:req-midrun",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&consumer.id),
            node_id: "node-midrun",
            model_id: Some("codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("test"),
        })
        .unwrap();
    let run = store
        .bind_node_compute_run_replay_policy(
            "pc_agent_cli:req-midrun",
            NodeComputeReplayBinding {
                billing_source: "own_codex",
                resource_owner_user_id: Some(&consumer.id),
                lease_id: None,
                offline_policy: "allow_offline",
                replay_deadline: None,
                max_cost_rmb_fen: 0,
                allowance_id: None,
            },
        )
        .unwrap()
        .unwrap();

    let wrong_node = validate_midrun_switch(
        &store,
        &consumer.id,
        "node-other",
        Some("pc_agent_cli:req-midrun"),
    )
    .await
    .expect_err("another node must not claim this run");
    assert!(wrong_node.contains("不匹配"));

    let validated = validate_midrun_switch(
        &store,
        &consumer.id,
        "node-midrun",
        Some("pc_agent_cli:req-midrun"),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(validated.compute_call_id, run.compute_call_id);

    store
        .billing_recharge(&consumer.id, 1_000, "test", "test", None)
        .unwrap();
    let reservation = reserve_midrun_shared_call(&store, &run).unwrap();
    let issue = store
        .create_codex_vault_emergency_lease_for_run(
            CodexVaultEmergencyLeaseCreate {
                grant_id: &grant.id,
                provider_user_id: &provider.id,
                consumer_user_id: &consumer.id,
                consumer_node_id: "node-midrun",
                provider_slot_id: &slot.slot_id,
                account_hint_hash: None,
                purpose: Some("unit_test"),
                failure_reason: None,
                max_lease_seconds: 900,
            },
            &run,
            &reservation.reservation_id,
        )
        .unwrap()
        .expect("exact run snapshot should bind the lease");
    let lease = issue.lease;

    let rebound = store
        .get_node_compute_run_by_compute_call_id("pc_agent_cli:req-midrun")
        .unwrap()
        .unwrap();
    assert_eq!(rebound.billing_source, "shared_codex");
    assert_eq!(
        rebound.resource_owner_user_id.as_deref(),
        Some(provider.id.as_str())
    );
    assert_eq!(rebound.lease_id.as_deref(), Some(lease.id.as_str()));
    assert_eq!(
        rebound.allowance_id.as_deref(),
        Some(reservation.reservation_id.as_str())
    );
    assert!(store
        .claim_codex_vault_emergency_credential_delivery(
            CodexVaultEmergencyCredentialDeliveryClaim {
                lease_id: &lease.id,
                expected_lease_updated_at: &lease.updated_at,
                grant_id: &grant.id,
                provider_user_id: &provider.id,
                consumer_user_id: &consumer.id,
                consumer_node_id: "node-midrun",
                provider_slot_id: &slot.slot_id,
                credential_version: slot.credential_version,
                compute_call_id: Some("pc_agent_cli:req-midrun"),
                cloud_control_deadline: &lease.expires_at,
            },
        )
        .unwrap());
    assert!(store
        .can_replay_node_compute_run_offline("pc_agent_cli:req-midrun")
        .unwrap());
    assert_eq!(
        store
            .admin_billing_reservations(Some("dispatch_hold"), 10)
            .unwrap()[0]
            .status,
        "dispatch_hold"
    );
    assert_eq!(store.release_expired_billing_reservations().unwrap(), 0);

    store
        .release_billing_hold_after_manual_verification(&consumer.id, "pc_agent_cli:req-midrun")
        .unwrap()
        .expect("verified operator release should refund the dispatch hold");
    assert!(!store
        .can_replay_node_compute_run_offline("pc_agent_cli:req-midrun")
        .unwrap());

    drop(store);
    let _ = std::fs::remove_file(path);
}
