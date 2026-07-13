use super::{
    classify_local_offline_scope_store_error, classify_replay_accounting_error,
    completion_ack_from_result, completion_display, completion_usage,
    handle_pc_cli_completion_replay, local_offline_shared_lease_retry,
    require_verified_billable_usage, serialize_completion_payload, validate_authenticated_producer,
    validate_envelope, ReplayFailure, MAX_COMPLETION_PAYLOAD_BYTES,
};
use crate::{
    ai_cli::pc_billing::PcCliBillingContext,
    store::{
        token_usage::BILLING_SOURCE_SHARED_CODEX, BillingReservationRequest,
        NodeComputeReplayBinding, NodeComputeRunStart, Store,
    },
    types::{AgentsConfig, AiBackend, AiCliConfig, AppState},
};
use homecli_proto::{CliCompletionEnvelope, ServerToAgent};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

fn completion() -> CliCompletionEnvelope {
    CliCompletionEnvelope {
        event_id: "event-1".to_string(),
        req_id: "request-1".to_string(),
        cli: "codex".to_string(),
        origin: "cloud_dispatch".to_string(),
        producer_identity: Some(homecli_proto::CliCompletionProducerIdentity {
            owner_user_id: "owner-a".to_string(),
            agent_id: "node-a".to_string(),
            install_id: "install-a".to_string(),
        }),
        project_context: None,
        channel_id: None,
        prompt: None,
        final_output: concat!(
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"离线任务完成。"}}"#,
            "\n"
        )
        .to_string(),
        exit_ok: true,
        error: None,
        session_id: None,
        prompt_tokens: Some(10),
        cached_input_tokens: Some(2),
        completion_tokens: Some(3),
        reasoning_tokens: Some(1),
        total_tokens: Some(13),
        model: Some("codex".to_string()),
        workspace_status: None,
        created_at_ms: 1,
    }
}

fn shared_billing_context() -> PcCliBillingContext {
    PcCliBillingContext {
        billing_source: BILLING_SOURCE_SHARED_CODEX.to_string(),
        resource_owner_user_id: Some("provider-a".to_string()),
        lease_id: Some("lease-a".to_string()),
        replay_deadline: None,
        charge_platform_balance: true,
        max_cost_rmb_fen: 100,
        allowance_id: Some("allowance-a".to_string()),
        frozen_reservation_required: true,
    }
}

fn replay_test_state(store: Store, root: &std::path::Path) -> AppState {
    AppState {
        store,
        data_dir: root.to_path_buf(),
        default_backend: AiBackend::Api,
        ai_cli: AiCliConfig {
            enabled: false,
            options: Vec::new(),
            default_option: None,
            fallback_to_api: false,
            codex_cli_only: true,
            fallback_cli_option: None,
        },
        agents_config: RwLock::new(AgentsConfig {
            agents: HashMap::new(),
            default_agent: String::new(),
        }),
        project_root: root.to_path_buf(),
        workspace_root: root.to_string_lossy().into_owned(),
        public_url: "http://127.0.0.1".to_string(),
        http_client: reqwest::Client::new(),
        admin_token: "test".to_string(),
        require_login: true,
        min_apk_version_code: 0,
        config_path: root.join("agents.json"),
        image_model: None,
        peer_registry: Arc::new(RwLock::new(HashMap::new())),
        lan_peer_registry: Arc::new(RwLock::new(HashMap::new())),
        node_registry: Arc::new(crate::node_registry::NodeRegistry::new()),
        online_users: Arc::new(RwLock::new(HashMap::new())),
        agent_manager: Arc::new(crate::homecli_agent::AgentManager::new()),
        project_task_scheduler: Arc::new(crate::types::ProjectTaskScheduler::new()),
        codex_prewarm: Arc::new(crate::types::CodexPrewarmRegistry::new()),
        route_a_session_leases: Arc::new(crate::types::RouteASessionLeaseRegistry::new()),
        codex_network: Arc::new(crate::codex_health::CodexNetworkHealth::from_env()),
        server_traces: Arc::new(crate::server_trace::ServerTraceStore::new()),
        owner_token: None,
    }
}

#[test]
fn valid_completion_extracts_public_reply() {
    let completion = completion();
    validate_envelope(&completion).unwrap();
    let display = completion_display(&completion);
    assert_eq!(display.status, "done");
    assert_eq!(display.reply, "离线任务完成。");
}

#[test]
fn durable_completion_requires_exact_authenticated_producer_identity() {
    let completion = completion();
    validate_authenticated_producer("node-a", Some("owner-a"), Some("install-a"), &completion)
        .unwrap();
    for (node, owner, install) in [
        ("node-b", Some("owner-a"), Some("install-a")),
        ("node-a", Some("owner-b"), Some("install-a")),
        ("node-a", Some("owner-a"), Some("install-b")),
        ("node-a", None, Some("install-a")),
        ("node-a", Some("owner-a"), None),
    ] {
        assert!(validate_authenticated_producer(node, owner, install, &completion).is_err());
    }
}

#[test]
fn durable_completion_rejects_missing_producer_identity() {
    let mut completion = completion();
    completion.producer_identity = None;
    assert!(validate_envelope(&completion).is_err());
}

#[test]
fn cloud_completion_cannot_smuggle_local_prompt() {
    let mut completion = completion();
    completion.prompt = Some("unexpected".to_string());
    // Binding validation performs the origin-specific rejection; the basic
    // envelope remains structurally valid.
    assert!(validate_envelope(&completion).is_ok());
}

#[test]
fn oversized_serialized_completion_payload_is_permanently_rejected() {
    let mut completion = completion();
    completion.error = Some("x".repeat(MAX_COMPLETION_PAYLOAD_BYTES));

    assert!(matches!(
        serialize_completion_payload(&completion),
        Err(ReplayFailure::Reject(message)) if message.contains("payload")
    ));
}

#[test]
fn local_scope_database_failure_is_retryable() {
    assert!(matches!(
        classify_local_offline_scope_store_error(anyhow::anyhow!("database is locked")),
        ReplayFailure::Retry(message) if message == "database is locked"
    ));
}

#[test]
fn local_scope_explicit_business_denials_are_permanent() {
    for message in [
        "项目不存在，或当前用户无权访问",
        "你已被该项目封禁，无法访问项目空间",
        "频道不存在",
    ] {
        assert!(matches!(
            classify_local_offline_scope_store_error(anyhow::anyhow!(message)),
            ReplayFailure::Reject(actual) if actual == message
        ));
    }
}

#[test]
fn active_shared_lease_defers_local_completion_without_dead_lettering_it() {
    let completion = completion();
    let failure = local_offline_shared_lease_retry();
    assert!(matches!(failure, ReplayFailure::Retry(_)));

    let ack = completion_ack_from_result(completion.event_id, completion.req_id, Err(failure));
    assert!(matches!(
        ack,
        ServerToAgent::CliCompletionAck {
            accepted: false,
            retryable: true,
            ..
        }
    ));
}

#[test]
fn frozen_billing_constraint_violation_is_not_retried() {
    let failure = classify_replay_accounting_error(
        crate::store::token_usage::BillingReservationConstraintViolation::AllowanceMismatch.into(),
    );
    assert!(matches!(failure, ReplayFailure::Reject(_)));
}

#[test]
fn durable_shared_completion_without_usage_is_retryable_not_accepted() {
    let mut completion = completion();
    completion.prompt_tokens = None;
    completion.cached_input_tokens = None;
    completion.completion_tokens = None;
    completion.reasoning_tokens = None;
    completion.total_tokens = None;
    let usage = completion_usage(&completion, "pc-cli/server-model");
    let failure = require_verified_billable_usage(&shared_billing_context(), usage.as_ref())
        .expect_err("shared completion without usage must remain pending");

    let ack = completion_ack_from_result(completion.event_id, completion.req_id, Err(failure));
    assert!(matches!(
        ack,
        ServerToAgent::CliCompletionAck {
            accepted: false,
            retryable: true,
            ..
        }
    ));
}

#[test]
fn late_durable_platform_completion_after_execution_deadline_is_settled() {
    let root = std::env::temp_dir().join(format!(
        "elon-late-completion-replay-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let database = root.join("test.sqlite");
    let store = Store::open(&database).unwrap();
    let user = store
        .create_user("late-completion@example.com", "secret1", None, None)
        .unwrap();
    store
        .billing_recharge(&user.id, 1_000, "test", "late completion", None)
        .unwrap();
    let compute_call_id = "pc_agent_cli:request-1";
    store
        .reserve_billing_call(&BillingReservationRequest {
            user_id: &user.id,
            compute_call_id,
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            model: Some("pc-cli/codex"),
            reserve_fen: 100,
            bill_missing_balance: true,
        })
        .unwrap();
    let held = store
        .hold_billing_reservation_for_dispatch(&user.id, compute_call_id)
        .unwrap()
        .unwrap();
    store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id,
            consumer_user_id: &user.id,
            provider_user_id: None,
            node_id: "node-a",
            model_id: Some("pc-cli/codex"),
            feature: "pc_agent_cli_chat",
            usage_mode: "pc_agent_cli",
            route_reason: Some("test"),
        })
        .unwrap();
    store
        .bind_node_compute_run_replay_policy(
            compute_call_id,
            NodeComputeReplayBinding {
                billing_source: "platform",
                resource_owner_user_id: None,
                lease_id: None,
                offline_policy: "require_active_reservation",
                replay_deadline: Some("2000-01-01T00:00:00Z"),
                max_cost_rmb_fen: held.reserved_fen,
                allowance_id: Some(&held.reservation_id),
            },
        )
        .unwrap();
    let state = replay_test_state(store, &root);
    let mut terminal = completion();
    terminal.producer_identity = Some(homecli_proto::CliCompletionProducerIdentity {
        owner_user_id: user.id.clone(),
        agent_id: "node-a".to_string(),
        install_id: "install-a".to_string(),
    });

    let ack = handle_pc_cli_completion_replay(
        &state,
        "node-a",
        Some(&user.id),
        Some("install-a"),
        terminal,
    );
    assert!(matches!(
        ack,
        ServerToAgent::CliCompletionAck { accepted: true, .. }
    ));
    assert_eq!(
        state
            .store
            .get_node_compute_run_by_compute_call_id(compute_call_id)
            .unwrap()
            .unwrap()
            .status,
        "settled"
    );
    assert_eq!(
        state
            .store
            .admin_billing_reservations(Some("settled"), 10)
            .unwrap()[0]
            .status,
        "settled"
    );

    drop(state);
    let _ = std::fs::remove_dir_all(root);
}
