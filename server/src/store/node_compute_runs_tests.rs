use super::*;
use crate::store::{
    LocalOfflineNodeComputeRunClaim, LocalOfflineNodeComputeRunClaimOutcome,
    NodeComputeReplayBinding, ProjectExecutionSessionStart,
};

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-node-runs-test-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("store should open"), path)
}

#[test]
fn start_is_idempotent_and_finish_records_settlement() {
    let (store, path) = temp_store();
    let consumer = store
        .create_user("node-run-consumer@example.com", "secret1", None, None)
        .unwrap();
    let initial_provider = store
        .create_user(
            "node-run-initial-provider@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let provider = store
        .create_user("node-run-provider@example.com", "secret1", None, None)
        .unwrap();

    let first = store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:req-1",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&initial_provider.id),
            node_id: "node-a",
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .unwrap();
    let second = store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:req-1",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&initial_provider.id),
            node_id: "node-a",
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(first.status, "started");
    let fetched = store
        .get_node_compute_run_by_compute_call_id("pc_agent_cli:req-1")
        .unwrap()
        .unwrap();
    assert_eq!(fetched.id, first.id);

    let bound = store
        .bind_node_compute_run_replay_policy(
            "pc_agent_cli:req-1",
            NodeComputeReplayBinding {
                billing_source: "own_codex",
                resource_owner_user_id: Some(&consumer.id),
                lease_id: None,
                offline_policy: "allow_offline",
                replay_deadline: Some("2099-01-01T00:00:00Z"),
                max_cost_rmb_fen: 0,
                allowance_id: None,
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(bound.billing_source, "own_codex");
    assert_eq!(bound.offline_policy, "allow_offline");
    assert_eq!(
        bound.resource_owner_user_id.as_deref(),
        Some(consumer.id.as_str())
    );
    assert!(store
        .can_replay_node_compute_run_offline("pc_agent_cli:req-1")
        .unwrap());

    let finished = store
        .finish_node_compute_run(
            "pc_agent_cli:req-1",
            NodeComputeRunFinish {
                provider_user_id: Some(&provider.id),
                status: "settled",
                prompt_tokens: 10,
                completion_tokens: 20,
                billed_cost_rmb_fen: 30,
                provider_earned_fen: 24,
                settlement_status: Some("billed"),
                error_message: None,
            },
        )
        .unwrap()
        .unwrap();

    assert_eq!(finished.status, "settled");
    assert_eq!(finished.prompt_tokens, 10);
    assert_eq!(finished.completion_tokens, 20);
    assert_eq!(finished.billed_cost_rmb_fen, 30);
    assert_eq!(finished.provider_earned_fen, 24);
    assert_eq!(
        finished.provider_user_id.as_deref(),
        Some(provider.id.as_str())
    );
    assert!(finished.finished_at.is_some());

    let rebound = store
        .bind_node_compute_run_replay_policy(
            "pc_agent_cli:req-1",
            NodeComputeReplayBinding {
                billing_source: "platform",
                resource_owner_user_id: None,
                lease_id: None,
                offline_policy: "online_only",
                replay_deadline: None,
                max_cost_rmb_fen: 100,
                allowance_id: None,
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(rebound.billing_source, "own_codex");
    assert_eq!(rebound.offline_policy, "allow_offline");

    let scores = store.node_quality_scores().unwrap();
    let score = scores.get("node-a").unwrap();
    assert_eq!(score.total_runs, 1);
    assert_eq!(score.successful_runs, 1);
    assert_eq!(score.success_rate_x1000, 1000);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn start_rejects_idempotency_key_rebinding() {
    let (store, path) = temp_store();
    let consumer = store
        .create_user("node-run-rebind@example.com", "secret1", None, None)
        .unwrap();
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "node_llm:stable-key",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&consumer.id),
            node_id: "node-a",
            model_id: Some("qwen"),
            feature: "node_llm",
            usage_mode: "server_node_llm",
            route_reason: None,
        })
        .unwrap();

    let error = store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "node_llm:stable-key",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&consumer.id),
            node_id: "node-b",
            model_id: Some("qwen"),
            feature: "node_llm",
            usage_mode: "server_node_llm",
            route_reason: None,
        })
        .unwrap_err();
    assert!(error.to_string().contains("不能绑定到不同"));

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn replay_binding_rejects_unsafe_source_policy_combinations() {
    let (store, path) = temp_store();
    let consumer = store
        .create_user("node-run-policy@example.com", "secret1", None, None)
        .unwrap();
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:policy-1",
            consumer_user_id: &consumer.id,
            provider_user_id: None,
            node_id: "node-a",
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .unwrap();

    let error = store
        .bind_node_compute_run_replay_policy(
            "pc_agent_cli:policy-1",
            NodeComputeReplayBinding {
                billing_source: "platform",
                resource_owner_user_id: None,
                lease_id: None,
                offline_policy: "allow_offline",
                replay_deadline: None,
                max_cost_rmb_fen: 100,
                allowance_id: None,
            },
        )
        .expect_err("platform tasks must not gain unreserved offline execution");
    assert!(error.to_string().contains("组合不允许"));

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn startup_interrupts_started_pc_agent_runs() {
    let (store, path) = temp_store();
    let consumer = store
        .create_user("node-run-restart@example.com", "secret1", None, None)
        .unwrap();
    let provider = store
        .create_user(
            "node-run-restart-provider@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();

    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:restart-1",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&provider.id),
            node_id: "node-a",
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .unwrap();
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "node_llm:still-running",
            consumer_user_id: &consumer.id,
            provider_user_id: Some(&provider.id),
            node_id: "node-a",
            model_id: Some("gpt"),
            feature: "node_llm",
            usage_mode: "node_llm",
            route_reason: None,
        })
        .unwrap();

    assert_eq!(store.mark_interrupted_started_pc_agent_runs().unwrap(), 1);
    let pc_run = store
        .get_node_compute_run_by_compute_call_id("pc_agent_cli:restart-1")
        .unwrap()
        .unwrap();
    assert_eq!(pc_run.status, "failed");
    assert!(pc_run.finished_at.is_some());
    assert_eq!(
        pc_run.error_message.as_deref(),
        Some("server restarted before PC CLI terminal event")
    );
    let other_run = store
        .get_node_compute_run_by_compute_call_id("node_llm:still-running")
        .unwrap()
        .unwrap();
    assert_eq!(other_run.status, "started");

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn local_offline_claim_is_atomic_strict_and_terminal_idempotent() {
    let (store, path) = temp_store();
    let owner = store
        .create_user("local-claim-owner@example.com", "secret1", None, None)
        .unwrap();
    let local_project = store
        .create_project(&owner.id, "本机离线认领项目", None, Some("android"))
        .unwrap()
        .project;
    let other_project = store
        .create_project(&owner.id, "本机离线冲突项目", None, Some("android"))
        .unwrap()
        .project;
    let claim = || LocalOfflineNodeComputeRunClaim {
        compute_call_id: "pc_agent_cli:local-claim",
        request_id: "local-claim",
        owner_user_id: &owner.id,
        node_id: "node-local",
        project_id: &local_project.id,
        conversation_id: "conversation-local",
        model_id: Some("codex"),
    };

    let first = store.claim_local_offline_node_compute_run(claim()).unwrap();
    let run = match first {
        LocalOfflineNodeComputeRunClaimOutcome::Claimed { run, created } => {
            assert!(created);
            run
        }
        LocalOfflineNodeComputeRunClaimOutcome::Conflict { reason } => panic!("{reason}"),
    };
    assert_eq!(run.billing_source, "own_codex");
    assert_eq!(run.offline_policy, "allow_offline");
    assert_eq!(
        run.resource_owner_user_id.as_deref(),
        Some(owner.id.as_str())
    );
    assert!(run.lease_id.is_none());
    assert!(run.allowance_id.is_none());
    let session = store
        .get_project_execution_session_by_request_id("local-claim")
        .unwrap()
        .expect("claim must atomically create its project identity");
    assert_eq!(session.project_id, local_project.id);
    assert_eq!(session.conversation_id, "conversation-local");
    assert_eq!(session.user_id, owner.id);
    assert_eq!(session.node_id, "node-local");

    let rebound = store
        .claim_local_offline_node_compute_run(LocalOfflineNodeComputeRunClaim {
            compute_call_id: "pc_agent_cli:local-claim",
            request_id: "local-claim",
            owner_user_id: &owner.id,
            node_id: "node-local",
            project_id: &other_project.id,
            conversation_id: "conversation-other",
            model_id: Some("codex"),
        })
        .unwrap();
    assert!(matches!(
        rebound,
        LocalOfflineNodeComputeRunClaimOutcome::Conflict { .. }
    ));

    store
        .finish_node_compute_run(
            "pc_agent_cli:local-claim",
            NodeComputeRunFinish {
                provider_user_id: Some(&owner.id),
                status: "settled_no_provider",
                prompt_tokens: 3,
                completion_tokens: 2,
                billed_cost_rmb_fen: 0,
                provider_earned_fen: 0,
                settlement_status: Some("unbilled_own_codex"),
                error_message: None,
            },
        )
        .unwrap();
    match store.claim_local_offline_node_compute_run(claim()).unwrap() {
        LocalOfflineNodeComputeRunClaimOutcome::Claimed { run, created } => {
            assert!(!created);
            assert_eq!(run.status, "settled_no_provider");
        }
        LocalOfflineNodeComputeRunClaimOutcome::Conflict { reason } => panic!("{reason}"),
    }

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn local_offline_claim_never_rebinds_shared_run_or_orphan_session() {
    let (store, path) = temp_store();
    let owner = store
        .create_user("local-collision-owner@example.com", "secret1", None, None)
        .unwrap();
    let provider = store
        .create_user(
            "local-collision-provider@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let cloud_project = store
        .create_project(&owner.id, "云端会话冲突项目", None, Some("android"))
        .unwrap()
        .project;
    let local_project = store
        .create_project(&owner.id, "本机认领冲突项目", None, Some("android"))
        .unwrap()
        .project;
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:shared-collision",
            consumer_user_id: &owner.id,
            provider_user_id: Some(&provider.id),
            node_id: "node-local",
            model_id: Some("codex"),
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .unwrap();
    store
        .bind_node_compute_run_replay_policy(
            "pc_agent_cli:shared-collision",
            NodeComputeReplayBinding {
                billing_source: "shared_codex",
                resource_owner_user_id: Some(&provider.id),
                lease_id: Some("lease-shared"),
                offline_policy: "require_active_reservation",
                replay_deadline: Some("2099-01-01T00:00:00Z"),
                max_cost_rmb_fen: 50,
                allowance_id: Some("reservation-shared"),
            },
        )
        .unwrap();
    let collision = store
        .claim_local_offline_node_compute_run(LocalOfflineNodeComputeRunClaim {
            compute_call_id: "pc_agent_cli:shared-collision",
            request_id: "shared-collision",
            owner_user_id: &owner.id,
            node_id: "node-local",
            project_id: &local_project.id,
            conversation_id: "conversation-local",
            model_id: Some("codex"),
        })
        .unwrap();
    assert!(matches!(
        collision,
        LocalOfflineNodeComputeRunClaimOutcome::Conflict { .. }
    ));
    let unchanged = store
        .get_node_compute_run_by_compute_call_id("pc_agent_cli:shared-collision")
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.billing_source, "shared_codex");
    assert_eq!(unchanged.lease_id.as_deref(), Some("lease-shared"));
    assert_eq!(
        unchanged.allowance_id.as_deref(),
        Some("reservation-shared")
    );

    assert!(store
        .record_project_execution_started(ProjectExecutionSessionStart {
            project_id: &cloud_project.id,
            conversation_id: "cloud-conversation",
            user_id: &owner.id,
            node_id: "node-local",
            request_id: "orphan-session",
            requested_workspace_path: None,
            model: Some("codex"),
        })
        .unwrap());
    let orphan = store
        .claim_local_offline_node_compute_run(LocalOfflineNodeComputeRunClaim {
            compute_call_id: "pc_agent_cli:orphan-session",
            request_id: "orphan-session",
            owner_user_id: &owner.id,
            node_id: "node-local",
            project_id: &local_project.id,
            conversation_id: "local-conversation",
            model_id: Some("codex"),
        })
        .unwrap();
    assert!(matches!(
        orphan,
        LocalOfflineNodeComputeRunClaimOutcome::Conflict { .. }
    ));
    assert!(store
        .get_node_compute_run_by_compute_call_id("pc_agent_cli:orphan-session")
        .unwrap()
        .is_none());

    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: "pc_agent_cli:orphan-run",
            consumer_user_id: &owner.id,
            provider_user_id: Some(&owner.id),
            node_id: "node-local",
            model_id: Some("codex"),
            feature: "pc_agent_cli_offline_dev",
            usage_mode: "pc_agent_cli",
            route_reason: Some("owner_local_offline"),
        })
        .unwrap();
    store
        .bind_node_compute_run_replay_policy(
            "pc_agent_cli:orphan-run",
            NodeComputeReplayBinding {
                billing_source: "own_codex",
                resource_owner_user_id: Some(&owner.id),
                lease_id: None,
                offline_policy: "allow_offline",
                replay_deadline: None,
                max_cost_rmb_fen: 0,
                allowance_id: None,
            },
        )
        .unwrap();
    let orphan_run = store
        .claim_local_offline_node_compute_run(LocalOfflineNodeComputeRunClaim {
            compute_call_id: "pc_agent_cli:orphan-run",
            request_id: "orphan-run",
            owner_user_id: &owner.id,
            node_id: "node-local",
            project_id: &local_project.id,
            conversation_id: "local-conversation",
            model_id: Some("codex"),
        })
        .unwrap();
    assert!(matches!(
        orphan_run,
        LocalOfflineNodeComputeRunClaimOutcome::Conflict { .. }
    ));

    drop(store);
    let _ = std::fs::remove_file(path);
}
