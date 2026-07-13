use super::CodexVaultEmergencyLeaseCreate;
use crate::store::{
    BillingReservationRequest, NodeComputeReplayBinding, NodeComputeRunFinish, NodeComputeRunStart,
    Store,
};

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-codex-sharing-test-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("store should open"), path)
}

struct PendingSharedDispatch {
    provider_id: String,
    consumer_id: String,
    grant_id: String,
    lease_id: String,
    node_id: String,
    compute_call_id: String,
    deadline: String,
    reservation_id: String,
    reserved_fen: i64,
}

impl PendingSharedDispatch {
    fn binding(&self) -> NodeComputeReplayBinding<'_> {
        NodeComputeReplayBinding {
            billing_source: "shared_codex",
            resource_owner_user_id: Some(&self.provider_id),
            lease_id: Some(&self.lease_id),
            offline_policy: "require_active_reservation",
            replay_deadline: Some(&self.deadline),
            max_cost_rmb_fen: self.reserved_fen,
            allowance_id: Some(&self.reservation_id),
        }
    }
}

fn pending_shared_dispatch(store: &Store, label: &str) -> PendingSharedDispatch {
    let provider = store
        .create_user(
            &format!("strict-provider-{label}@example.com"),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let consumer = store
        .create_user(
            &format!("strict-consumer-{label}@example.com"),
            "secret1",
            None,
            None,
        )
        .unwrap();
    store
        .billing_recharge(&consumer.id, 1_000, "test", "test", None)
        .unwrap();
    let grant = store
        .upsert_codex_vault_emergency_grant(
            &provider.id,
            &consumer.id,
            Some("strict dispatch"),
            Some("unit_test"),
            Some(900),
            None,
            &provider.id,
        )
        .unwrap();
    let node_id = format!("node-strict-{label}");
    let lease = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant.id,
            provider_user_id: &provider.id,
            consumer_user_id: &consumer.id,
            consumer_node_id: &node_id,
            provider_slot_id: "slot-strict",
            account_hint_hash: None,
            purpose: Some("strict_dispatch_test"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    let compute_call_id = format!("pc_agent_cli:strict-{label}");
    store
        .reserve_billing_call(&BillingReservationRequest {
            user_id: &consumer.id,
            compute_call_id: &compute_call_id,
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            model: Some("pc-cli/codex"),
            reserve_fen: 100,
            bill_missing_balance: true,
        })
        .unwrap();
    let held = store
        .hold_billing_reservation_for_dispatch(&consumer.id, &compute_call_id)
        .unwrap()
        .unwrap();
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: &compute_call_id,
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&consumer.id),
            node_id: &node_id,
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .unwrap();
    let reservation_deadline = held.expires_at.as_deref().unwrap();
    let lease_time = chrono::DateTime::parse_from_rfc3339(&lease.expires_at).unwrap();
    let reservation_time = chrono::DateTime::parse_from_rfc3339(reservation_deadline).unwrap();
    PendingSharedDispatch {
        provider_id: provider.id,
        consumer_id: consumer.id,
        grant_id: grant.id,
        lease_id: lease.id,
        node_id,
        compute_call_id,
        deadline: if lease_time <= reservation_time {
            lease.expires_at
        } else {
            reservation_deadline.to_string()
        },
        reservation_id: held.reservation_id,
        reserved_fen: held.reserved_fen,
    }
}

fn finish_verification_run(store: &Store, compute_call_id: &str, settlement_status: &str) {
    store
        .finish_node_compute_run(
            compute_call_id,
            NodeComputeRunFinish {
                provider_user_id: None,
                status: "verification_pending",
                prompt_tokens: 0,
                completion_tokens: 0,
                billed_cost_rmb_fen: 0,
                provider_earned_fen: 0,
                settlement_status: Some(settlement_status),
                error_message: Some("test verification state"),
            },
        )
        .unwrap()
        .expect("verification run should be recorded");
}

fn add_bound_usage_verification_run(store: &Store, fixture: &PendingSharedDispatch, req_id: &str) {
    let compute_call_id = format!("pc_agent_cli:{req_id}");
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: &compute_call_id,
            consumer_user_id: &fixture.consumer_id,
            provider_user_id: Some(&fixture.consumer_id),
            node_id: &fixture.node_id,
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .unwrap();
    store
        .bind_node_compute_run_replay_policy(&compute_call_id, fixture.binding())
        .unwrap()
        .expect("usage-verification run should share the same lease");
    finish_verification_run(store, &compute_call_id, "usage_verification_pending");
}

#[test]
fn active_shared_bind_and_each_retry_require_exact_live_authorization() {
    let (store, path) = temp_store();
    let fixture = pending_shared_dispatch(&store, "live");
    let bound = store
        .bind_node_compute_run_to_active_emergency_lease(
            &fixture.compute_call_id,
            fixture.binding(),
        )
        .unwrap()
        .expect("active exact lease should bind");
    assert_eq!(bound.lease_id.as_deref(), Some(fixture.lease_id.as_str()));
    store
        .require_node_compute_run_dispatch_authorization(
            &fixture.compute_call_id,
            &fixture.node_id,
            true,
            Some(&fixture.deadline),
            Some(&fixture.lease_id),
        )
        .unwrap();
    assert!(store
        .require_node_compute_run_dispatch_authorization(
            &fixture.compute_call_id,
            &fixture.node_id,
            true,
            Some(&fixture.deadline),
            Some("wrong-lease"),
        )
        .is_err());

    store
        .revoke_codex_vault_emergency_grant(&fixture.grant_id, &fixture.provider_id)
        .unwrap();
    assert!(store
        .require_node_compute_run_dispatch_authorization(
            &fixture.compute_call_id,
            &fixture.node_id,
            true,
            Some(&fixture.deadline),
            Some(&fixture.lease_id),
        )
        .is_err());
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn revoke_cancels_unknown_dispatch_but_not_usage_verification_run() {
    let (store, path) = temp_store();
    let fixture = pending_shared_dispatch(&store, "revoke-unknown");
    store
        .bind_node_compute_run_to_active_emergency_lease(
            &fixture.compute_call_id,
            fixture.binding(),
        )
        .unwrap()
        .expect("shared dispatch should bind before its outcome becomes unknown");
    finish_verification_run(&store, &fixture.compute_call_id, "dispatch_outcome_unknown");
    add_bound_usage_verification_run(&store, &fixture, "strict-revoke-usage");

    let cancel_targets = store
        .revoke_codex_vault_emergency_grant(&fixture.grant_id, &fixture.provider_id)
        .unwrap()
        .expect("active grant should revoke");
    assert_eq!(
        cancel_targets,
        vec![(fixture.node_id.clone(), "strict-revoke-unknown".to_string())]
    );

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn revoke_before_initial_shared_bind_rolls_back_run_cas() {
    let (store, path) = temp_store();
    let fixture = pending_shared_dispatch(&store, "revoked-before-bind");
    store
        .revoke_codex_vault_emergency_grant(&fixture.grant_id, &fixture.provider_id)
        .unwrap();
    assert!(store
        .bind_node_compute_run_to_active_emergency_lease(
            &fixture.compute_call_id,
            fixture.binding(),
        )
        .is_err());
    let run = store
        .get_node_compute_run_by_compute_call_id(&fixture.compute_call_id)
        .unwrap()
        .unwrap();
    assert_eq!(run.billing_source, "platform");
    assert_eq!(run.offline_policy, "online_only");
    assert!(run.lease_id.is_none());
    assert!(run.allowance_id.is_none());
    assert_eq!(run.consumer_user_id, fixture.consumer_id);
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn clearing_lease_cancels_unknown_but_not_usage_verification_and_fences_run() {
    let (store, path) = temp_store();
    let fixture = pending_shared_dispatch(&store, "clear-unknown");
    store
        .bind_node_compute_run_to_active_emergency_lease(
            &fixture.compute_call_id,
            fixture.binding(),
        )
        .unwrap()
        .expect("shared dispatch should bind before clear");
    finish_verification_run(&store, &fixture.compute_call_id, "dispatch_outcome_unknown");
    add_bound_usage_verification_run(&store, &fixture, "strict-clear-usage-verification");

    let issue = store
        .clear_codex_vault_emergency_lease_for_node_with_cancel_targets(
            &fixture.consumer_id,
            &fixture.node_id,
            Some(&fixture.lease_id),
        )
        .unwrap()
        .expect("active lease should clear once");
    assert_eq!(issue.lease.status, "cleared");
    assert_eq!(
        issue.cancel_targets,
        vec![(fixture.node_id.clone(), "strict-clear-unknown".to_string())]
    );
    let run = store
        .get_node_compute_run_by_compute_call_id(&fixture.compute_call_id)
        .unwrap()
        .unwrap();
    let fenced_at = run
        .replay_deadline
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .expect("clear should deadline-fence its shared run");
    assert!(fenced_at <= chrono::Utc::now());
    assert!(store
        .clear_codex_vault_emergency_lease_for_node_with_cancel_targets(
            &fixture.consumer_id,
            &fixture.node_id,
            Some(&fixture.lease_id),
        )
        .unwrap()
        .is_none());

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn superseding_lease_cancels_unknown_but_not_usage_verification() {
    let (store, path) = temp_store();
    let fixture = pending_shared_dispatch(&store, "supersede-unknown");
    store
        .bind_node_compute_run_to_active_emergency_lease(
            &fixture.compute_call_id,
            fixture.binding(),
        )
        .unwrap()
        .expect("shared dispatch should bind before supersede");
    finish_verification_run(&store, &fixture.compute_call_id, "dispatch_outcome_unknown");
    add_bound_usage_verification_run(&store, &fixture, "strict-supersede-usage-verification");

    let issue = store
        .create_codex_vault_emergency_lease_with_superseded_runs(CodexVaultEmergencyLeaseCreate {
            grant_id: &fixture.grant_id,
            provider_user_id: &fixture.provider_id,
            consumer_user_id: &fixture.consumer_id,
            consumer_node_id: &fixture.node_id,
            provider_slot_id: "slot-superseding",
            account_hint_hash: None,
            purpose: Some("supersede_unknown_dispatch_test"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    assert_eq!(
        issue.superseded_cancel_targets,
        vec![(
            fixture.node_id.clone(),
            "strict-supersede-unknown".to_string()
        )]
    );

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn lease_deadline_uses_transactional_grant_expiry_and_max_policy() {
    let (store, path) = temp_store();
    let provider = store
        .create_user("vault-policy-provider@example.com", "secret1", None, None)
        .unwrap();
    let consumer = store
        .create_user("vault-policy-consumer@example.com", "secret1", None, None)
        .unwrap();
    let grant_expires_at = (chrono::Utc::now() + chrono::Duration::seconds(60)).to_rfc3339();
    let grant = store
        .upsert_codex_vault_emergency_grant(
            &provider.id,
            &consumer.id,
            Some("transactional policy"),
            Some("unit_test"),
            Some(7_200),
            Some(&grant_expires_at),
            &provider.id,
        )
        .unwrap();
    let expiry_clamped = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant.id,
            provider_user_id: &provider.id,
            consumer_user_id: &consumer.id,
            consumer_node_id: "node-policy-expiry",
            provider_slot_id: "slot-expiry",
            account_hint_hash: None,
            purpose: Some("expiry_clamp"),
            failure_reason: None,
            max_lease_seconds: 7_200,
        })
        .unwrap();
    let actual_expiry = chrono::DateTime::parse_from_rfc3339(&expiry_clamped.expires_at).unwrap();
    let grant_expiry = chrono::DateTime::parse_from_rfc3339(&grant_expires_at).unwrap();
    assert!(actual_expiry <= grant_expiry);
    assert!(actual_expiry > chrono::Utc::now());

    {
        let conn = store.conn().unwrap();
        conn.execute(
            "UPDATE codex_vault_emergency_grants
                SET max_lease_seconds = 60,
                    expires_at = NULL
              WHERE id = ?1",
            [&grant.id],
        )
        .unwrap();
    }
    let stale_input = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant.id,
            provider_user_id: &provider.id,
            consumer_user_id: &consumer.id,
            consumer_node_id: "node-policy-max",
            provider_slot_id: "slot-max",
            account_hint_hash: None,
            purpose: Some("stale_max"),
            failure_reason: None,
            max_lease_seconds: 7_200,
        })
        .unwrap();
    let leased_at = chrono::DateTime::parse_from_rfc3339(&stale_input.leased_at).unwrap();
    let expires_at = chrono::DateTime::parse_from_rfc3339(&stale_input.expires_at).unwrap();
    assert!(expires_at - leased_at <= chrono::Duration::seconds(61));

    {
        let conn = store.conn().unwrap();
        conn.execute(
            "UPDATE codex_vault_emergency_grants
                SET expires_at = 'not-rfc3339'
              WHERE id = ?1",
            [&grant.id],
        )
        .unwrap();
    }
    let malformed = store.create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
        grant_id: &grant.id,
        provider_user_id: &provider.id,
        consumer_user_id: &consumer.id,
        consumer_node_id: "node-policy-malformed",
        provider_slot_id: "slot-malformed",
        account_hint_hash: None,
        purpose: Some("malformed_expiry"),
        failure_reason: None,
        max_lease_seconds: 7_200,
    });
    assert!(
        malformed.is_err(),
        "malformed grant expiry must fail closed"
    );

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn revoked_and_expired_grants_are_not_shareable() {
    let (store, path) = temp_store();
    let provider = store
        .create_user(
            "vault-provider@example.com",
            "secret1",
            Some("provider"),
            None,
        )
        .unwrap();
    let consumer = store
        .create_user(
            "vault-consumer@example.com",
            "secret1",
            Some("consumer"),
            None,
        )
        .unwrap();
    let grant = store
        .upsert_codex_vault_emergency_grant(
            &provider.id,
            &consumer.id,
            Some("provider to consumer"),
            Some("robot_codex_vault_shared_access"),
            Some(900),
            None,
            &provider.id,
        )
        .unwrap();
    assert!(store
        .find_active_codex_vault_emergency_grant(&provider.id, &consumer.id)
        .unwrap()
        .is_some());
    let issued = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant.id,
            provider_user_id: &provider.id,
            consumer_user_id: &consumer.id,
            consumer_node_id: "node-revoked",
            provider_slot_id: "slot-revoked",
            account_hint_hash: None,
            purpose: Some("revoke_test"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:revoked-lease",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&consumer.id),
            node_id: "node-revoked",
            model_id: Some("codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("unit_test"),
        })
        .unwrap();
    store
        .bind_node_compute_run_replay_policy(
            "pc_agent_cli:revoked-lease",
            NodeComputeReplayBinding {
                billing_source: "shared_codex",
                resource_owner_user_id: Some(&provider.id),
                lease_id: Some(&issued.id),
                offline_policy: "require_active_reservation",
                replay_deadline: Some(&issued.expires_at),
                max_cost_rmb_fen: 10,
                allowance_id: Some("reservation-revoked"),
            },
        )
        .unwrap();
    let cancel_targets = store
        .revoke_codex_vault_emergency_grant(&grant.id, &provider.id)
        .unwrap()
        .expect("active grant should be revoked");
    assert_eq!(
        cancel_targets,
        vec![("node-revoked".to_string(), "revoked-lease".to_string())]
    );
    assert!(store
        .find_active_codex_vault_emergency_grant(&provider.id, &consumer.id)
        .unwrap()
        .is_none());
    assert_eq!(
        store
            .get_codex_vault_emergency_lease(&issued.id)
            .unwrap()
            .unwrap()
            .status,
        "cleared"
    );
    assert!(store
        .get_active_codex_vault_emergency_lease_for_node(&consumer.id, "node-revoked")
        .unwrap()
        .is_none());
    let fenced_run = store
        .get_node_compute_run_by_compute_call_id("pc_agent_cli:revoked-lease")
        .unwrap()
        .unwrap();
    let fenced_at = fenced_run
        .replay_deadline
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .expect("revocation should deadline-fence the bound run");
    assert!(fenced_at <= chrono::Utc::now());
    assert_ne!(
        fenced_run.replay_deadline.as_deref(),
        Some(issued.expires_at.as_str())
    );

    let reverse = store
        .upsert_codex_vault_emergency_grant(
            &consumer.id,
            &provider.id,
            Some("expired reverse"),
            Some("robot_codex_vault_shared_access"),
            Some(900),
            Some("2000-01-01T00:00:00+00:00"),
            &consumer.id,
        )
        .unwrap();
    assert_eq!(reverse.status, "active");
    assert!(store
        .find_active_codex_vault_emergency_grant(&consumer.id, &provider.id)
        .unwrap()
        .is_none());

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn superseded_lease_never_reappears_after_current_lease_is_cleared() {
    let (store, path) = temp_store();
    let provider = store
        .create_user(
            "vault-singleton-provider@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let consumer = store
        .create_user(
            "vault-singleton-consumer@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let grant = store
        .upsert_codex_vault_emergency_grant(
            &provider.id,
            &consumer.id,
            Some("singleton test"),
            Some("unit_test"),
            Some(900),
            None,
            &provider.id,
        )
        .unwrap();
    let old = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant.id,
            provider_user_id: &provider.id,
            consumer_user_id: &consumer.id,
            consumer_node_id: "node-singleton",
            provider_slot_id: "slot-old",
            account_hint_hash: None,
            purpose: Some("unit_test"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    let current = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant.id,
            provider_user_id: &provider.id,
            consumer_user_id: &consumer.id,
            consumer_node_id: "node-singleton",
            provider_slot_id: "slot-current",
            account_hint_hash: None,
            purpose: Some("unit_test"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    assert_eq!(
        store
            .get_codex_vault_emergency_lease(&old.id)
            .unwrap()
            .unwrap()
            .status,
        "cleared"
    );
    assert_eq!(
        store
            .get_active_codex_vault_emergency_lease_for_node(&consumer.id, "node-singleton")
            .unwrap()
            .unwrap()
            .id,
        current.id
    );

    store
        .clear_codex_vault_emergency_lease_for_node(
            &consumer.id,
            "node-singleton",
            Some(&current.id),
        )
        .unwrap();
    assert!(store
        .get_active_codex_vault_emergency_lease_for_node(&consumer.id, "node-singleton")
        .unwrap()
        .is_none());
    assert_eq!(
        store
            .get_codex_vault_emergency_lease(&old.id)
            .unwrap()
            .unwrap()
            .status,
        "cleared"
    );

    drop(store);
    let _ = std::fs::remove_file(path);
}
