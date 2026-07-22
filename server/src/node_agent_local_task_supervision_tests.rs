use super::*;

#[test]
fn review_request_accepts_legacy_actor_hints_without_trusting_them() {
    let _: SupervisionReviewRequest = serde_json::from_value(json!({
        "verdict": "accepted",
        "summary": "兼容旧 helper",
        "improvements": [],
        "reviewed_by": "legacy-caller",
        "review_source": "legacy-helper"
    }))
    .unwrap();
}

#[test]
fn supervised_prompt_marks_executor_and_prevents_redispatch() {
    let contract = normalize_contract(SupervisionContractInput {
        protocol: None,
        supervisor: None,
        task_role: None,
        parent_task_id: None,
        root_task_id: None,
        acceptance_criteria: vec!["测试通过".to_string()],
        improvement_policy: None,
    })
    .unwrap();
    let prompt = executor_prompt("修复登录", Some(&contract));
    assert!(prompt.contains("<elon-pc-executor"));
    assert!(prompt.contains("不得再次把写任务派发给 PC 节点"));
    assert!(prompt.contains("测试通过"));
    assert!(prompt.contains("修复登录"));
}

#[test]
fn state_keeps_latest_review_and_counts_evidence() {
    let contract = normalize_contract(SupervisionContractInput {
        protocol: None,
        supervisor: None,
        task_role: None,
        parent_task_id: None,
        root_task_id: None,
        acceptance_criteria: Vec::new(),
        improvement_policy: None,
    })
    .unwrap();
    let events = vec![
        TaskJournalEventView {
            seq: 1,
            event: json!({
                "type":"supervision_contract", "payload": contract_payload(&contract)
            }),
        },
        TaskJournalEventView {
            seq: 2,
            event: json!({
                "type":"tool_event", "event": {"type":"tool_call", "tool":"file_change", "args":{"changes":[{"path":"src/a.rs", "kind":"modify"}]}}
            }),
        },
        TaskJournalEventView {
            seq: 3,
            event: json!({
                "type":"tool_event", "event": {"type":"tool_result", "tool":"shell", "result":"exit=1 tests failed"}
            }),
        },
        TaskJournalEventView {
            seq: 4,
            event: json!({
                "type":"supervision_review", "payload": {
                    "protocol":SUPERVISION_PROTOCOL, "verdict":"needs_follow_up", "summary":"补测试",
                    "improvements":["增加回归"], "reviewed_by":"codex_desktop", "reviewed_at_ms":1
                }
            }),
        },
    ];
    let state = supervision_state(&events);
    assert!(state.enabled);
    assert_eq!(state.evidence.tool_calls, 1);
    assert_eq!(state.evidence.failed_tools, 1);
    assert_eq!(state.evidence.changed_files, vec!["src/a.rs"]);
    assert_eq!(state.review.unwrap().verdict, "needs_follow_up");
}

#[test]
fn codex_json_items_produce_precise_nonzero_evidence() {
    let events = vec![
        TaskJournalEventView {
            seq: 1,
            event: json!({
                "type":"codex_item", "lifecycle":"started",
                "item":{"id":"cmd-1","type":"command_execution","command":"cargo test"}
            }),
        },
        TaskJournalEventView {
            seq: 2,
            event: json!({
                "type":"codex_item", "lifecycle":"completed",
                "item":{"id":"cmd-1","type":"command_execution","command":"cargo test","status":"completed","exit_code":0}
            }),
        },
        TaskJournalEventView {
            seq: 3,
            event: json!({
                "type":"codex_item", "lifecycle":"started",
                "item":{"id":"cmd-2","type":"command_execution","command":"cargo test bad"}
            }),
        },
        TaskJournalEventView {
            seq: 4,
            event: json!({
                "type":"codex_item", "lifecycle":"completed",
                "item":{"id":"cmd-2","type":"command_execution","command":"cargo test bad","status":"failed","exit_code":2,
                    "output":{"tail":["assertion failed"]}}
            }),
        },
        TaskJournalEventView {
            seq: 5,
            event: json!({
                "type":"codex_item", "lifecycle":"started",
                "item":{"id":"file-1","type":"file_change","changes":[{"path":"src/main.rs","kind":"update"}]}
            }),
        },
        TaskJournalEventView {
            seq: 6,
            event: json!({
                "type":"codex_item", "lifecycle":"completed",
                "item":{"id":"file-1","type":"file_change","status":"completed","changes":[{"path":"src/main.rs","kind":"update"}]}
            }),
        },
        TaskJournalEventView {
            seq: 7,
            event: json!({
                "type":"codex_item", "lifecycle":"completed",
                "item":{"id":"msg-1","type":"agent_message","text":"done"}
            }),
        },
    ];
    let state = supervision_state(&events);
    assert_eq!(state.evidence.tool_calls, 3);
    assert_eq!(state.evidence.tool_results, 3);
    assert_eq!(state.evidence.failed_tools, 1);
    assert_eq!(state.evidence.file_change_events, 1);
    assert_eq!(state.evidence.changed_files, vec!["src/main.rs"]);
    assert_eq!(state.evidence.command_exit_codes.len(), 2);
    assert_eq!(state.evidence.failure_summaries.len(), 1);
    assert_eq!(state.evidence.agent_messages, 1);
}

#[test]
fn journal_state_reads_review_after_paginated_event_window() {
    let task_id = "local-supervision-long";
    let directory = std::env::temp_dir().join(format!(
        "elon-supervision-test-{}-{}",
        std::process::id(),
        now_ms()
    ));
    let journal = TaskJournal::new(&directory);
    let contract = normalize_contract(SupervisionContractInput {
        protocol: None,
        supervisor: None,
        task_role: None,
        parent_task_id: None,
        root_task_id: None,
        acceptance_criteria: Vec::new(),
        improvement_policy: None,
    })
    .unwrap();
    record_supervision_event(
        &journal,
        task_id,
        "supervision_contract",
        contract_payload(&contract),
    )
    .unwrap();
    for index in 0..250 {
        journal
            .append_event(json!({"type":"tool_event", "req_id":task_id, "event":{"type":"tool_call", "tool":format!("tool-{index}")}}))
            .unwrap();
    }
    record_supervision_event(
        &journal,
        task_id,
        "supervision_review",
        json!({
            "protocol":SUPERVISION_PROTOCOL, "verdict":"accepted", "summary":"通过",
            "improvements":[], "reviewed_by":"codex_desktop", "reviewed_at_ms":2
        }),
    )
    .unwrap();

    let state = load_supervision_state(&journal, task_id).unwrap();
    assert_eq!(state.evidence.event_count, 252);
    assert_eq!(state.evidence.tool_calls, 250);
    assert_eq!(state.review.unwrap().verdict, "accepted");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn restart_drain_transition_is_a_supported_supervision_event() {
    let directory = std::env::temp_dir().join(format!(
        "elon-supervision-drain-test-{}-{}",
        std::process::id(),
        now_ms()
    ));
    let journal = TaskJournal::new(&directory);

    record_supervision_event(
        &journal,
        "local-stale-runtime",
        "supervision_stale_runtime_resume_required",
        json!({"state":"resume_required", "journal_preserved":true}),
    )
    .unwrap();

    assert_eq!(
        journal
            .snapshot("local-stale-runtime", 0, 10)
            .unwrap()
            .events
            .len(),
        1
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn resume_requires_explicit_current_protocol() {
    let missing = normalize_contract(SupervisionContractInput {
        protocol: None,
        supervisor: None,
        task_role: Some("resume_original".to_string()),
        parent_task_id: Some("local-parent".to_string()),
        root_task_id: Some("local-parent".to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: None,
    })
    .expect_err("resume must not infer its security protocol");
    assert!(missing.contains("必须显式携带 supervision.protocol"));

    let wrong = normalize_contract(SupervisionContractInput {
        protocol: Some("elon.desktop_pc_supervision.v0".to_string()),
        supervisor: None,
        task_role: Some("resume_original".to_string()),
        parent_task_id: Some("local-parent".to_string()),
        root_task_id: Some("local-parent".to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: None,
    })
    .expect_err("unknown protocol must fail closed");
    assert!(wrong.contains("supervision.protocol 必须是"));

    let current = normalize_contract(SupervisionContractInput {
        protocol: Some(SUPERVISION_PROTOCOL.to_string()),
        supervisor: None,
        task_role: Some("resume_original".to_string()),
        parent_task_id: Some("local-parent".to_string()),
        root_task_id: Some("local-parent".to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: None,
    })
    .expect("current protocol should admit resume");
    assert_eq!(current.protocol, SUPERVISION_PROTOCOL);
}

#[test]
fn post_task_improvement_requires_explicit_protocol_parent_and_root() {
    let missing_protocol = normalize_contract(SupervisionContractInput {
        protocol: None,
        supervisor: None,
        task_role: Some("post_task_improvement".to_string()),
        parent_task_id: Some("local-parent".to_string()),
        root_task_id: Some("local-root".to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: Some("after_task_only".to_string()),
    })
    .expect_err("self evolution must not infer its security protocol");
    assert!(missing_protocol.contains("必须显式携带 supervision.protocol"));

    let missing_root = normalize_contract(SupervisionContractInput {
        protocol: Some(SUPERVISION_PROTOCOL.to_string()),
        supervisor: None,
        task_role: Some("post_task_improvement".to_string()),
        parent_task_id: Some("local-parent".to_string()),
        root_task_id: None,
        acceptance_criteria: Vec::new(),
        improvement_policy: Some("after_task_only".to_string()),
    })
    .expect_err("self evolution must carry its exact root identity");
    assert!(missing_root.contains("parent_task_id 和 root_task_id"));
}

#[test]
fn review_identity_prevents_pc_operator_from_impersonating_desktop() {
    let compatible = serde_json::from_value::<SupervisionReviewRequest>(serde_json::json!({
        "verdict": "accepted", "summary": "operator review",
        "reviewed_by": "codex_desktop", "review_source": "codex_desktop_helper"
    }))
    .expect("legacy helper actor hints should remain wire-compatible");
    let normalized = normalize_review(compatible, "pc_operator", "local_pc_api").unwrap();
    assert_eq!(normalized.reviewed_by, "pc_operator");
    assert_eq!(normalized.review_source, "local_pc_api");

    let desktop = normalize_review(
        SupervisionReviewRequest {
            verdict: "accepted".to_string(),
            summary: "desktop review".to_string(),
            improvements: Vec::new(),
        },
        "codex_desktop",
        "codex_desktop_helper",
    )
    .expect("trusted desktop helper provenance should be accepted");
    assert_eq!(desktop.review_source, "codex_desktop_helper");

    let operator = normalize_review(
        SupervisionReviewRequest {
            verdict: "needs_follow_up".to_string(),
            summary: "local review".to_string(),
            improvements: Vec::new(),
        },
        "pc_operator",
        "local_pc_api",
    )
    .expect("local operator defaults should remain compatible");
    assert_eq!(operator.reviewed_by, "pc_operator");
    assert_eq!(operator.review_source, "local_pc_api");
}

#[test]
fn review_routes_enforce_desktop_ticket_and_server_owned_actor() {
    use crate::node_agent_desktop_review_auth::{
        DesktopReviewAuth, DesktopReviewAuthError, DESKTOP_REVIEW_TICKET_HEADER,
    };

    let auth = DesktopReviewAuth::for_test("desktop-route-credential-at-least-32-bytes");
    let owner = "owner-route-a";
    let task = "local-route-a";
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 60;
    let ticket = auth.mint_for_test(owner, task, expires, "route-nonce-12345678");

    let operator = resolve_review_identity(ReviewChannel::PcOperator, &auth, owner, task).unwrap();
    assert_eq!(operator.0, "pc_operator:owner-route-a");
    assert_ne!(operator.0, "codex_desktop:owner-route-a");
    assert_eq!(
        auth.verify_headers(&HeaderMap::new(), owner, task),
        Err(DesktopReviewAuthError::Missing)
    );

    let mut wrong = HeaderMap::new();
    wrong.insert(
        DESKTOP_REVIEW_TICKET_HEADER,
        format!("v1.{expires}.route-nonce-12345678.{}", "00".repeat(32))
            .parse()
            .unwrap(),
    );
    assert_eq!(
        auth.verify_headers(&wrong, owner, task),
        Err(DesktopReviewAuthError::Invalid)
    );

    let mut correct = HeaderMap::new();
    correct.insert(DESKTOP_REVIEW_TICKET_HEADER, ticket.parse().unwrap());
    auth.verify_headers(&correct, owner, task).unwrap();
    let desktop =
        resolve_review_identity(ReviewChannel::VerifiedDesktop, &auth, owner, task).unwrap();
    assert_eq!(desktop.0, "codex_desktop:owner-route-a");
    assert_eq!(desktop.1, "codex_desktop_helper");
}

#[tokio::test]
async fn desktop_review_v3_is_enforced_by_the_production_post_route() {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use rsa::{
        pkcs1v15::SigningKey,
        rand_core::OsRng,
        signature::{SignatureEncoding, Signer},
        RsaPrivateKey,
    };
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;

    fn ticket(
        key: &RsaPrivateKey,
        owner: &str,
        task: &str,
        path: &str,
        body: &[u8],
        nonce: &str,
    ) -> String {
        fn field(value: &str) -> String {
            format!("{}:{value}", value.as_bytes().len())
        }
        let expires = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + 120;
        let body_hash = hex::encode(Sha256::digest(body));
        let message = [
            "v3".to_string(),
            field(owner),
            field(task),
            field("POST"),
            field(path),
            field(&body_hash),
            expires.to_string(),
            field(nonce),
            field("0000000000000000"),
        ]
        .join("\n");
        let signature = SigningKey::<Sha256>::new(key.clone()).sign(message.as_bytes());
        format!(
            "v3.0000000000000000.{expires}.{nonce}.{}",
            BASE64.encode(signature.to_bytes())
        )
    }

    async fn post(app: Router, path: &str, body: &[u8], ticket: &str) -> (StatusCode, Value) {
        let response = app
            .oneshot(
                Request::post(path)
                    .header("content-type", "application/json")
                    .header(
                        crate::node_agent_desktop_review_auth::DESKTOP_REVIEW_TICKET_HEADER,
                        ticket,
                    )
                    .body(Body::from(body.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    let root = std::env::temp_dir().join(format!(
        "elon-desktop-review-route-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let owner = "owner-route-v3";
    let task = "local-route-v3";
    let path = "/api/local-tasks/local-route-v3/supervision/desktop-review";
    let body =
        "{\"verdict\":\"observing\",\"summary\":\"UTF-8 路由验收\",\"improvements\":[]}".as_bytes();
    let key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let mut runtime = crate::NodeRuntime::new(
        crate::node_agent_config::NodeConfig {
            cloud_url: "ws://127.0.0.1".to_string(),
            cloud_http_url: "http://127.0.0.1".to_string(),
            ollama_url: "http://127.0.0.1".to_string(),
            lm_studio_url: None,
            custom_url: None,
            price_per_1k: 0.0,
        },
        Some(crate::node_agent_config::Credentials {
            agent_id: "agent-route-v3".to_string(),
            agent_secret: "unused".to_string(),
            owner_user_id: owner.to_string(),
            user_token: None,
        }),
        crate::pc_storage_repo::StorageSettings::default(),
        crate::node_agent_data_root::resolve(None, None, None),
        "install-route-v3".to_string(),
    );
    runtime.task_journal = TaskJournal::new(root.join("journal"));
    runtime.local_tasks =
        crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.sqlite3"));
    runtime.desktop_review_auth =
        crate::node_agent_desktop_review_auth::DesktopReviewAuth::for_v3_route_test(
            key.to_public_key(),
            root.join("nonces.json"),
        );
    runtime
        .local_tasks
        .create(crate::node_agent_local_task_store::LocalTaskStart {
            task_id: task,
            owner_user_id: owner,
            agent_id: "agent-route-v3",
            install_id: "install-route-v3",
            project_id: "elon-self",
            channel_id: None,
            conversation_id: "conversation-route-v3",
            workspace_path: "C:\\isolated",
            prompt: "route test",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .unwrap();
    let contract = normalize_contract(SupervisionContractInput {
        protocol: Some(SUPERVISION_PROTOCOL.to_string()),
        supervisor: None,
        task_role: None,
        parent_task_id: None,
        root_task_id: None,
        acceptance_criteria: vec!["route".to_string()],
        improvement_policy: None,
    })
    .unwrap();
    record_supervision_event(
        &runtime.task_journal,
        task,
        "supervision_contract",
        contract_payload(&contract),
    )
    .unwrap();
    let app = routes().with_state(Arc::new(runtime));

    let valid = ticket(&key, owner, task, path, body, "route-success-nonce-1");
    let (status, response) = post(app.clone(), path, body, &valid).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response["supervision"]["review"]["reviewed_by"],
        "codex_desktop:owner-route-v3"
    );
    assert_eq!(
        response["supervision"]["review"]["review_source"],
        "codex_desktop_helper"
    );

    let (status, response) = post(app.clone(), path, body, &valid).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(response["error"], "desktop_review_ticket_replayed");

    let body_ticket = ticket(&key, owner, task, path, body, "route-body-nonce-123");
    let (status, _) = post(app.clone(), path, b"{}", &body_ticket).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let wrong_task_ticket = ticket(
        &key,
        owner,
        "local-other",
        path,
        body,
        "route-task-nonce-123",
    );
    let (status, _) = post(app.clone(), path, body, &wrong_task_ticket).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let wrong_path_ticket = ticket(&key, owner, task, "/wrong", body, "route-path-nonce-123");
    let (status, _) = post(app, path, body, &wrong_path_ticket).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    std::fs::remove_dir_all(root).unwrap();
}
