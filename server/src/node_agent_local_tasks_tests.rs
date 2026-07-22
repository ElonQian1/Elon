use super::*;

use axum::{body::to_bytes, body::Body, http::Request};
use tower::ServiceExt;

#[derive(Clone, Debug)]
pub(super) struct CapturedDispatch {
    pub executor_prompt: String,
    pub execution_workspace_path: String,
    pub supervision: Option<crate::node_agent_local_task_supervision::SupervisionContract>,
    pub inherited_authorization_task_id: Option<String>,
}

fn dispatch_captures(
) -> &'static std::sync::Mutex<std::collections::HashMap<String, Vec<CapturedDispatch>>> {
    static CAPTURES: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, Vec<CapturedDispatch>>>,
    > = std::sync::OnceLock::new();
    CAPTURES.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

pub(super) fn should_capture_dispatch(prompt: &str) -> bool {
    dispatch_captures().lock().unwrap().contains_key(prompt)
}

pub(super) fn record_captured_dispatch(
    prompt: &str,
    executor_prompt: &str,
    execution_workspace_path: &str,
    supervision: Option<&crate::node_agent_local_task_supervision::SupervisionContract>,
    inherited_authorization_record: Option<&crate::node_agent_local_task_store::LocalTaskRecord>,
) {
    if let Some(captures) = dispatch_captures().lock().unwrap().get_mut(prompt) {
        captures.push(CapturedDispatch {
            executor_prompt: executor_prompt.to_string(),
            execution_workspace_path: execution_workspace_path.to_string(),
            supervision: supervision.cloned(),
            inherited_authorization_task_id: inherited_authorization_record
                .map(|record| record.task_id.clone()),
        });
    }
}

fn install_dispatch_capture(prompt: &str) {
    dispatch_captures()
        .lock()
        .unwrap()
        .insert(prompt.to_string(), Vec::new());
}

fn take_dispatch_captures(prompt: &str) -> Vec<CapturedDispatch> {
    dispatch_captures()
        .lock()
        .unwrap()
        .remove(prompt)
        .unwrap_or_default()
}

#[test]
fn missing_durable_completion_never_synthesizes_local_terminal_event() {
    let path = std::env::temp_dir().join(format!(
        "elon-local-missing-outbox-{}.sqlite3",
        uuid::Uuid::new_v4().simple()
    ));
    let outbox = crate::node_agent_completion_outbox::CliCompletionOutbox::new(path.clone());
    assert!(
        durable_completion_for_local_display(&outbox, "local-missing")
            .unwrap()
            .is_none()
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn local_identity_matches_server_replay_bounds() {
    assert!(support::local_identity_is_valid(
        "project-1",
        MAX_LOCAL_ID_CHARS
    ));
    assert!(support::local_identity_is_valid(
        &"x".repeat(200),
        MAX_LOCAL_ID_CHARS
    ));
    assert!(!support::local_identity_is_valid("", MAX_LOCAL_ID_CHARS));
    assert!(!support::local_identity_is_valid(
        &"x".repeat(201),
        MAX_LOCAL_ID_CHARS
    ));
    assert!(!support::local_identity_is_valid(
        "project\nother",
        MAX_LOCAL_ID_CHARS
    ));
}

#[tokio::test]
async fn get_task_empty_tail_page_keeps_same_epoch_cursor() {
    let root = std::env::temp_dir().join(format!(
        "elon-local-get-cursor-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let owner = "owner-get-cursor";
    let mut runtime = crate::NodeRuntime::new(
        crate::node_agent_config::NodeConfig {
            cloud_url: "ws://127.0.0.1".into(),
            cloud_http_url: "http://127.0.0.1".into(),
            ollama_url: "http://127.0.0.1".into(),
            lm_studio_url: None,
            custom_url: None,
            price_per_1k: 0.0,
        },
        Some(crate::node_agent_config::Credentials {
            agent_id: "agent-get-cursor".into(),
            agent_secret: "unused".into(),
            owner_user_id: owner.into(),
            user_token: None,
        }),
        crate::pc_storage_repo::StorageSettings::default(),
        crate::node_agent_data_root::resolve(None, None, None),
        "install-get-cursor".into(),
    );
    runtime.task_journal = crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
    runtime.local_tasks =
        crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.sqlite3"));
    runtime
        .local_tasks
        .create(LocalTaskStart {
            task_id: "local-get-cursor",
            owner_user_id: owner,
            agent_id: "agent-get-cursor",
            install_id: "install-get-cursor",
            project_id: "elon-self",
            channel_id: None,
            conversation_id: "conversation-get-cursor",
            workspace_path: root.to_string_lossy().as_ref(),
            prompt: "inspect cursor",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .unwrap();
    runtime
        .task_journal
        .record_started(crate::node_agent_task_journal::TaskJournalStart {
            req_id: "local-get-cursor",
            cli_name: "codex",
            route: Some("managed_pipe_json_sidecar"),
            run_handle_id: Some("local-get-cursor"),
            cwd: Some(root.to_string_lossy().as_ref()),
            runtime_permission: Some("full_access"),
        })
        .unwrap();
    let first = runtime
        .task_journal
        .snapshot("local-get-cursor", 0, 20)
        .unwrap();
    let path = format!(
        "/api/local-tasks/local-get-cursor?since={}&limit=20&expected_cursor_epoch={}",
        first.last_event_seq, first.cursor_epoch
    );
    let response = routes()
        .with_state(Arc::new(runtime))
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["events"].as_array().unwrap().len(), 0);
    assert_eq!(body["cursor_reset"], false);
    assert_eq!(body["last_event_seq"], first.last_event_seq);
    assert_eq!(body["new_cursor"], first.last_event_seq);
    assert_eq!(body["resume_cursor"], first.last_event_seq);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn production_create_handler_replays_bound_result_and_rejects_changed_body() {
    let root = std::env::temp_dir().join(format!(
        "elon-local-post-handler-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let owner = "owner-idempotency-handler";
    let mut runtime = crate::NodeRuntime::new(
        crate::node_agent_config::NodeConfig {
            cloud_url: "ws://127.0.0.1".into(),
            cloud_http_url: "http://127.0.0.1".into(),
            ollama_url: "http://127.0.0.1".into(),
            lm_studio_url: None,
            custom_url: None,
            price_per_1k: 0.0,
        },
        Some(crate::node_agent_config::Credentials {
            agent_id: "agent-idempotency-handler".into(),
            agent_secret: "unused".into(),
            owner_user_id: owner.into(),
            user_token: None,
        }),
        crate::pc_storage_repo::StorageSettings::default(),
        crate::node_agent_data_root::resolve(None, None, None),
        "install-idempotency-handler".into(),
    );
    runtime.local_tasks =
        crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.sqlite3"));
    let request = CreateLocalTaskRequest {
        project_id: "elon-self".into(),
        channel_id: None,
        conversation_id: Some("conversation-idempotency".into()),
        workspace_path: root.to_string_lossy().into_owned(),
        prompt: "fake idempotency handler request".into(),
        runtime_permission: Some("full_access".into()),
        supervision: None,
    };
    let digest = idempotency::canonical_digest(&request).unwrap();
    let task_id = "local-bound-handler";
    assert!(matches!(
        runtime
            .local_tasks
            .claim_local_post(
                owner,
                "handler-key",
                "POST",
                "/api/local-tasks",
                &digest,
                task_id,
                "handler-test-process",
            )
            .unwrap(),
        crate::node_agent_local_task_store::idempotency::IdempotencyClaim::Claimed { .. }
    ));
    let cached = json!({"ok": true, "task_id": task_id, "status": "running"});
    runtime
        .local_tasks
        .complete_local_post(
            owner,
            "handler-key",
            task_id,
            "handler-test-process",
            202,
            &cached,
        )
        .unwrap();
    let app = routes().with_state(Arc::new(runtime));
    let body = serde_json::to_vec(&request).unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::post("/api/local-tasks")
                .header("content-type", "application/json")
                .header(idempotency::IDEMPOTENCY_HEADER, "handler-key")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let replayed: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(replayed, cached);

    let mut changed = serde_json::to_value(&request).unwrap();
    changed["prompt"] = serde_json::Value::String("different body".into());
    let response = app
        .oneshot(
            Request::post("/api/local-tasks")
                .header("content-type", "application/json")
                .header(idempotency::IDEMPOTENCY_HEADER, "handler-key")
                .body(Body::from(serde_json::to_vec(&changed).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn production_handler_releases_claim_after_every_admission_error() {
    let root = std::env::temp_dir().join(format!(
        "elon-local-post-error-compensation-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let owner = "owner-error-compensation";
    let mut runtime = test_runtime(root.clone(), owner, "agent-error", "install-error");
    runtime.local_tasks =
        crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.sqlite3"));
    let request = CreateLocalTaskRequest {
        project_id: "elon-self".into(),
        channel_id: None,
        conversation_id: Some("error-compensation".into()),
        workspace_path: root.to_string_lossy().into_owned(),
        prompt: "claim error must compensate immediately".into(),
        runtime_permission: Some("full_access".into()),
        supervision: Some(
            crate::node_agent_local_task_supervision::SupervisionContractInput {
                protocol: Some(
                    crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL.into(),
                ),
                supervisor: Some("codex_desktop".into()),
                task_role: Some("post_task_improvement".into()),
                parent_task_id: Some("missing-parent".into()),
                root_task_id: Some("missing-root".into()),
                acceptance_criteria: vec!["never dispatch".into()],
                improvement_policy: Some("after_task_only".into()),
            },
        ),
    };
    let app = routes().with_state(Arc::new(runtime));
    for attempt in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/local-tasks")
                    .header("content-type", "application/json")
                    .header(idempotency::IDEMPOTENCY_HEADER, "error-compensation-key")
                    .body(Body::from(serde_json::to_vec(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT, "attempt {attempt}");
        let body = String::from_utf8(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(!body.contains("IDEMPOTENCY_REQUEST_IN_FLIGHT"));
    }
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn production_handler_restart_takeover_replays_exact_plan_without_redispatch() {
    let root = std::env::temp_dir().join(format!(
        "elon-local-post-restart-handler-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let owner = "owner-restart-handler";
    let mut runtime = test_runtime(root.clone(), owner, "agent-restart", "install-restart");
    runtime.local_tasks =
        crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.sqlite3"));
    let prompt = "EXACT_EXECUTOR_PROMPT_RESTART_BOUNDARY";
    install_dispatch_capture(prompt);
    let request = CreateLocalTaskRequest {
        project_id: "elon-self".into(),
        channel_id: Some("channel-restart".into()),
        conversation_id: Some("conversation-restart".into()),
        workspace_path: root.to_string_lossy().into_owned(),
        prompt: prompt.into(),
        runtime_permission: Some("full_access".into()),
        supervision: None,
    };
    let digest = idempotency::canonical_digest(&request).unwrap();
    runtime
        .local_tasks
        .claim_local_post(
            owner,
            "restart-handler-key",
            "POST",
            "/api/local-tasks",
            &digest,
            "local-restart-stable",
            "dead-process-instance",
        )
        .unwrap();
    let app = routes().with_state(Arc::new(runtime));
    let first = app
        .clone()
        .oneshot(
            Request::post("/api/local-tasks")
                .header("content-type", "application/json")
                .header(idempotency::IDEMPOTENCY_HEADER, "restart-handler-key")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::ACCEPTED);
    let first_body: serde_json::Value =
        serde_json::from_slice(&to_bytes(first.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(first_body["task_id"], "local-restart-stable");

    let second = app
        .oneshot(
            Request::post("/api/local-tasks")
                .header("content-type", "application/json")
                .header(idempotency::IDEMPOTENCY_HEADER, "restart-handler-key")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::ACCEPTED);
    let second_body: serde_json::Value =
        serde_json::from_slice(&to_bytes(second.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(
        second_body, first_body,
        "cached response must be byte-semantic exact"
    );
    let captures = take_dispatch_captures(prompt);
    assert_eq!(
        captures.len(),
        1,
        "restart and replay must dispatch only once"
    );
    assert_eq!(captures[0].executor_prompt.matches(prompt).count(), 1);
    assert_eq!(captures[0].execution_workspace_path, root.to_string_lossy());
    assert!(captures[0].supervision.is_none());
    assert!(captures[0].inherited_authorization_task_id.is_none());
    let _ = std::fs::remove_dir_all(root);
}

fn test_runtime(
    root: std::path::PathBuf,
    owner: &str,
    agent: &str,
    install: &str,
) -> crate::NodeRuntime {
    let mut runtime = crate::NodeRuntime::new(
        crate::node_agent_config::NodeConfig {
            cloud_url: "ws://127.0.0.1".into(),
            cloud_http_url: "http://127.0.0.1".into(),
            ollama_url: "http://127.0.0.1".into(),
            lm_studio_url: None,
            custom_url: None,
            price_per_1k: 0.0,
        },
        Some(crate::node_agent_config::Credentials {
            agent_id: agent.into(),
            agent_secret: "unused".into(),
            owner_user_id: owner.into(),
            user_token: None,
        }),
        crate::pc_storage_repo::StorageSettings::default(),
        crate::node_agent_data_root::resolve(None, None, None),
        install.into(),
    );
    runtime.task_journal = crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
    runtime
}
