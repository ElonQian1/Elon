use super::CodexVaultEmergencyLeaseCreate;
use crate::store::{NodeComputeReplayBinding, NodeComputeRunStart, Store};

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-codex-sharing-test-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("store should open"), path)
}

#[test]
fn grant_reciprocal_and_active_lease_are_queryable() {
    let (store, path) = temp_store();
    let a = store
        .create_user("vault-a@example.com", "secret1", Some("A"), None)
        .unwrap();
    let b = store
        .create_user("vault-b@example.com", "secret1", Some("B"), None)
        .unwrap();
    let ab = store
        .upsert_codex_vault_emergency_grant(
            &a.id,
            &b.id,
            Some("A to B"),
            Some("test"),
            Some(900),
            None,
            &a.id,
        )
        .unwrap();
    store
        .upsert_codex_vault_emergency_grant(
            &b.id,
            &a.id,
            Some("B to A"),
            Some("test"),
            Some(900),
            None,
            &b.id,
        )
        .unwrap();

    let grants = store.list_codex_vault_emergency_grants(&a.id).unwrap();
    assert_eq!(grants.len(), 2);
    assert!(grants.iter().any(|grant| grant.reciprocal_active));

    let lease = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &ab.id,
            provider_user_id: &a.id,
            consumer_user_id: &b.id,
            consumer_node_id: "node-b",
            provider_slot_id: "slot-a",
            account_hint_hash: Some("hint-a"),
            purpose: Some("unit_test"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    assert_eq!(lease.billing_source, "shared_codex");
    let active = store
        .get_active_codex_vault_emergency_lease_for_node(&b.id, "node-b")
        .unwrap()
        .expect("active lease");
    assert_eq!(active.provider_user_id, a.id);

    assert!(store
        .attach_codex_vault_emergency_usage(
            &lease.id,
            Some("tok_1"),
            Some("bill_1"),
            Some("ntx_1"),
            100,
            50,
            7,
            5,
            Some("billed"),
        )
        .unwrap());
    let billed = store
        .get_codex_vault_emergency_lease(&lease.id)
        .unwrap()
        .unwrap();
    assert_eq!(billed.total_tokens, 150);
    assert_eq!(billed.billed_cost_rmb_fen, 7);

    let cleared = store
        .clear_codex_vault_emergency_lease_for_node(&b.id, "node-b", Some(&lease.id))
        .unwrap()
        .expect("cleared lease");
    assert_eq!(cleared.status, "cleared");
    assert!(cleared.cleared_at.is_some());
    assert!(store
        .get_active_codex_vault_emergency_lease_for_node(&b.id, "node-b")
        .unwrap()
        .is_none());

    assert!(!store
        .attach_codex_vault_emergency_usage(
            &lease.id,
            Some("tok_after_clear"),
            Some("bill_after_clear"),
            Some("ntx_after_clear"),
            100,
            50,
            7,
            5,
            Some("billed"),
        )
        .unwrap());
    let updated = store
        .get_codex_vault_emergency_lease(&lease.id)
        .unwrap()
        .unwrap();
    assert_eq!(updated.total_tokens, 150);
    assert_eq!(updated.billed_cost_rmb_fen, 7);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn superseding_lease_fences_and_returns_cancel_for_old_started_run() {
    let (store, path) = temp_store();
    let provider = store
        .create_user("vault-fence-provider@example.com", "secret1", None, None)
        .unwrap();
    let consumer = store
        .create_user("vault-fence-consumer@example.com", "secret1", None, None)
        .unwrap();
    let grant = store
        .upsert_codex_vault_emergency_grant(
            &provider.id,
            &consumer.id,
            Some("supersede fence"),
            Some("unit_test"),
            Some(900),
            None,
            &provider.id,
        )
        .unwrap();
    let old_lease = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant.id,
            provider_user_id: &provider.id,
            consumer_user_id: &consumer.id,
            consumer_node_id: "node-supersede",
            provider_slot_id: "slot-old",
            account_hint_hash: None,
            purpose: Some("old_run"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:run-a",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&consumer.id),
            node_id: "node-supersede",
            model_id: Some("codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("unit_test"),
        })
        .unwrap();
    store
        .bind_node_compute_run_replay_policy(
            "pc_agent_cli:run-a",
            NodeComputeReplayBinding {
                billing_source: "shared_codex",
                resource_owner_user_id: Some(&provider.id),
                lease_id: Some(&old_lease.id),
                offline_policy: "require_active_reservation",
                replay_deadline: Some(&old_lease.expires_at),
                max_cost_rmb_fen: 10,
                allowance_id: Some("reservation-run-a"),
            },
        )
        .unwrap();

    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:run-b",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&consumer.id),
            node_id: "node-supersede",
            model_id: Some("codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("unit_test"),
        })
        .unwrap();
    let run_b = store
        .bind_node_compute_run_replay_policy(
            "pc_agent_cli:run-b",
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
    store
        .billing_recharge(&consumer.id, 1_000, "unit_test", "supersede fence", None)
        .unwrap();
    crate::billing::reserve_trusted_call(
        &store,
        &consumer.id,
        &run_b.compute_call_id,
        &run_b.feature,
        &run_b.usage_mode,
        run_b.model_id.as_deref(),
        100,
    )
    .unwrap();
    let reservation = store
        .get_active_billing_reservation(&consumer.id, &run_b.compute_call_id)
        .unwrap()
        .unwrap();
    let issue = store
        .create_codex_vault_emergency_lease_for_run(
            CodexVaultEmergencyLeaseCreate {
                grant_id: &grant.id,
                provider_user_id: &provider.id,
                consumer_user_id: &consumer.id,
                consumer_node_id: "node-supersede",
                provider_slot_id: "slot-new",
                account_hint_hash: None,
                purpose: Some("new_run"),
                failure_reason: None,
                max_lease_seconds: 900,
            },
            &run_b,
            &reservation.reservation_id,
        )
        .unwrap()
        .expect("run B should atomically replace lease A");

    assert_eq!(
        issue.superseded_cancel_targets,
        vec![("node-supersede".to_string(), "run-a".to_string())]
    );
    assert_eq!(issue.run.lease_id.as_deref(), Some(issue.lease.id.as_str()));
    let run_a = store
        .get_node_compute_run_by_compute_call_id("pc_agent_cli:run-a")
        .unwrap()
        .unwrap();
    let fenced_at = run_a
        .replay_deadline
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .expect("superseded run A should be deadline-fenced");
    assert!(fenced_at <= chrono::Utc::now());
    assert_eq!(
        store
            .get_codex_vault_emergency_lease(&old_lease.id)
            .unwrap()
            .unwrap()
            .status,
        "cleared"
    );

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn racing_shared_lease_claims_bind_exactly_one_provider_and_lease() {
    use std::sync::{Arc, Barrier};

    let (store, path) = temp_store();
    let consumer = store
        .create_user("vault-race-consumer@example.com", "secret1", None, None)
        .unwrap();
    let provider_a = store
        .create_user("vault-race-provider-a@example.com", "secret1", None, None)
        .unwrap();
    let provider_b = store
        .create_user("vault-race-provider-b@example.com", "secret1", None, None)
        .unwrap();
    let grant_a = store
        .upsert_codex_vault_emergency_grant(
            &provider_a.id,
            &consumer.id,
            Some("race A"),
            Some("unit_test"),
            Some(900),
            None,
            &provider_a.id,
        )
        .unwrap();
    let grant_b = store
        .upsert_codex_vault_emergency_grant(
            &provider_b.id,
            &consumer.id,
            Some("race B"),
            Some("unit_test"),
            Some(900),
            None,
            &provider_b.id,
        )
        .unwrap();
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:lease-race",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&consumer.id),
            node_id: "node-race",
            model_id: Some("codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("unit_test"),
        })
        .unwrap();
    let expected = store
        .bind_node_compute_run_replay_policy(
            "pc_agent_cli:lease-race",
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
    store
        .billing_recharge(&consumer.id, 1_000, "unit_test", "lease race", None)
        .unwrap();
    crate::billing::reserve_trusted_call(
        &store,
        &consumer.id,
        &expected.compute_call_id,
        &expected.feature,
        &expected.usage_mode,
        expected.model_id.as_deref(),
        100,
    )
    .unwrap();
    let reservation = store
        .get_active_billing_reservation(&consumer.id, &expected.compute_call_id)
        .unwrap()
        .unwrap();

    let store = Arc::new(store);
    let barrier = Arc::new(Barrier::new(3));
    let spawn_claim = |provider_id: String, grant_id: String, slot_id: &'static str| {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        let consumer_id = consumer.id.clone();
        let reservation_id = reservation.reservation_id.clone();
        let expected = expected.clone();
        std::thread::spawn(move || {
            barrier.wait();
            store.create_codex_vault_emergency_lease_for_run(
                CodexVaultEmergencyLeaseCreate {
                    grant_id: &grant_id,
                    provider_user_id: &provider_id,
                    consumer_user_id: &consumer_id,
                    consumer_node_id: "node-race",
                    provider_slot_id: slot_id,
                    account_hint_hash: None,
                    purpose: Some("race_test"),
                    failure_reason: None,
                    max_lease_seconds: 900,
                },
                &expected,
                &reservation_id,
            )
        })
    };
    let claim_a = spawn_claim(provider_a.id.clone(), grant_a.id, "slot-a");
    let claim_b = spawn_claim(provider_b.id.clone(), grant_b.id, "slot-b");
    barrier.wait();
    let result_a = claim_a.join().unwrap().unwrap();
    let result_b = claim_b.join().unwrap().unwrap();
    let winners = [result_a, result_b]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    assert_eq!(winners.len(), 1, "one stale run snapshot must lose its CAS");
    let winner = &winners[0];

    let rebound = store
        .get_node_compute_run_by_compute_call_id(&expected.compute_call_id)
        .unwrap()
        .unwrap();
    assert_eq!(rebound.lease_id.as_deref(), Some(winner.lease.id.as_str()));
    assert_eq!(
        rebound.resource_owner_user_id.as_deref(),
        Some(winner.lease.provider_user_id.as_str())
    );
    let active = store
        .get_active_codex_vault_emergency_lease_for_node(&consumer.id, "node-race")
        .unwrap()
        .unwrap();
    assert_eq!(active.id, winner.lease.id);
    let active_count: i64 = store
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM codex_vault_emergency_leases
              WHERE consumer_user_id = ?1 AND consumer_node_id = ?2
                AND status = 'active' AND cleared_at IS NULL",
            rusqlite::params![consumer.id, "node-race"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(active_count, 1);

    drop(store);
    let _ = std::fs::remove_file(path);
}
