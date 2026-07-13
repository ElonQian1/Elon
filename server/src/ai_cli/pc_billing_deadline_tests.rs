use super::{
    bind_pc_cli_replay_policy_in_store, pc_cli_billing_context_from_store, PcCliBillingContext,
};
use crate::{
    ai_cli::pc_billing_policy::prepare_pc_cli_billing_for_dispatch,
    billing_lifecycle::TrustedBillingCall,
    homecli_agent::freeze_cloud_control_dispatch_window,
    store::{token_usage::BILLING_SOURCE_PLATFORM, NodeComputeRunStart, Store},
};

#[test]
fn platform_dispatch_without_lease_keeps_absolute_deadline_and_passes_pre_send_check() {
    let path = std::env::temp_dir().join(format!(
        "elon-pc-cli-platform-deadline-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    let store = Store::open(&path).expect("store should open");
    let user = store
        .create_user("platform-deadline@example.com", "secret1", None, None)
        .unwrap();
    store
        .billing_recharge(&user.id, 1_000, "test", "deadline", None)
        .unwrap();
    let key = "pc_agent_cli:platform-deadline";
    let mut call = TrustedBillingCall::reserve(
        &store,
        &user.id,
        key,
        "pc_agent_cli_chat",
        "pc_agent_cli",
        Some("pc-cli/codex"),
        100,
    )
    .unwrap();
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: key,
            consumer_user_id: &user.id,
            provider_user_id: None,
            node_id: "node-platform-deadline",
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            route_reason: Some("test"),
        })
        .unwrap();
    let context = PcCliBillingContext {
        billing_source: BILLING_SOURCE_PLATFORM.to_string(),
        resource_owner_user_id: None,
        lease_id: None,
        replay_deadline: None,
        charge_platform_balance: true,
        max_cost_rmb_fen: 0,
        allowance_id: None,
        frozen_reservation_required: false,
    };

    prepare_pc_cli_billing_for_dispatch(&mut call, &context).unwrap();
    let deadline = bind_pc_cli_replay_policy_in_store(&store, &user.id, key, &context)
        .unwrap()
        .expect("platform dispatch must inherit its frozen reservation deadline");
    let window = freeze_cloud_control_dispatch_window(&deadline)
        .expect("the pre-send cloud-control check must accept the frozen deadline");
    assert!(window.ttl_ms > 0);
    let run = store
        .get_node_compute_run_by_compute_call_id(key)
        .unwrap()
        .unwrap();
    assert_eq!(run.replay_deadline.as_deref(), Some(deadline.as_str()));
    assert_eq!(run.offline_policy, "require_active_reservation");

    call.release_dispatch_not_sent();
    drop(call);
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn billing_context_fails_closed_when_shared_lease_lookup_fails() {
    let path = std::env::temp_dir().join(format!(
        "elon-pc-cli-billing-test-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    let store = Store::open(&path).expect("store should open");
    let owner = store
        .create_user("lease-db-owner@example.com", "secret1", None, None)
        .unwrap();
    store
        .create_node_credential(
            "lease-db-node",
            "secret-hash",
            &owner.id,
            None,
            None,
            Some("lease-db-install"),
        )
        .unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch("DROP TABLE codex_vault_emergency_leases")
        .unwrap();

    let error = pc_cli_billing_context_from_store(&store, &owner.id, "lease-db-node", "codex")
        .expect_err("lease lookup failure must never fall back to own Codex");
    assert!(error.to_string().contains("codex_vault_emergency_leases"));

    drop(store);
    let _ = std::fs::remove_file(path);
}
