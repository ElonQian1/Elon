use super::*;

use axum::{body::to_bytes, body::Body, http::Request};
use tower::ServiceExt;

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
    assert!(local_identity_is_valid("project-1"));
    assert!(local_identity_is_valid(&"x".repeat(200)));
    assert!(!local_identity_is_valid(""));
    assert!(!local_identity_is_valid(&"x".repeat(201)));
    assert!(!local_identity_is_valid("project\nother"));
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
            )
            .unwrap(),
        crate::node_agent_local_task_store::idempotency::IdempotencyClaim::Claimed { .. }
    ));
    let cached = json!({"ok": true, "task_id": task_id, "status": "running"});
    runtime
        .local_tasks
        .complete_local_post(owner, "handler-key", task_id, 202, &cached)
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
