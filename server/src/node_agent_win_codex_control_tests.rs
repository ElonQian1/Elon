use super::*;

#[test]
fn action_contract_rejects_arbitrary_urls_scripts_and_routes() {
    let hub = WinCodexControlHub::default();
    assert!(hub
        .enqueue_action("trace", "navigate", Some("/codex-control"), None, "test")
        .is_ok());
    assert!(hub
        .enqueue_action(
            "trace",
            "navigate",
            Some("https://example.com"),
            None,
            "test"
        )
        .is_err());
    assert!(hub
        .enqueue_action("trace", "navigate", Some("/unknown"), None, "test")
        .is_err());
    assert!(hub
        .enqueue_action("trace", "eval_javascript", None, None, "test")
        .is_err());
    assert!(hub
        .enqueue_action("trace", "focus_ai_window", None, Some("chatgpt"), "test",)
        .is_ok());
    assert!(hub
        .enqueue_action("trace", "focus_ai_window", None, None, "test")
        .is_err());
    assert!(hub
        .enqueue_action(
            "trace",
            "capture_ai_window_state",
            None,
            Some("local-ai-native-chatgpt-owner"),
            "test",
        )
        .is_err());
    assert!(hub
        .enqueue_action("trace", "list_ai_windows", None, Some("chatgpt"), "test",)
        .is_err());
}

#[test]
fn update_restart_requires_codex_and_an_exact_release_identity() {
    let hub = WinCodexControlHub::default();
    let target = format!("0.3.69+{}", "A".repeat(40));
    let normalized_target = format!("0.3.69+{}", "a".repeat(40));
    let action = hub
        .enqueue_action_with_target(
            "trace",
            "update_and_restart",
            None,
            None,
            Some(&target),
            "codex_mcp",
        )
        .unwrap();
    assert_eq!(
        action.target_release_identity.as_deref(),
        Some(normalized_target.as_str())
    );
    assert_eq!(action.requested_by, "codex_mcp");
    assert!(hub
        .enqueue_action_with_target(
            "trace",
            "update_and_restart",
            None,
            None,
            Some(&target),
            "pc_ui",
        )
        .is_err());
    assert!(hub
        .enqueue_action_with_target(
            "trace",
            "update_and_restart",
            None,
            None,
            Some("latest"),
            "codex_mcp",
        )
        .is_err());
    assert!(hub
        .enqueue_action_with_target("trace", "focus_window", None, None, Some(&target), "test",)
        .is_err());
}

#[test]
fn capabilities_expose_the_current_release_and_pinned_restart_policy() {
    let capabilities = WinCodexControlHub::default().capabilities();
    assert!(capabilities["actions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|action| action.as_str() == Some("update_and_restart")));
    assert_eq!(
        capabilities["security"]["update_restart_requires_exact_release"],
        true
    );
    assert!(capabilities["release_identity"].as_str().is_some());
}

#[test]
fn event_fields_and_sensitive_summaries_are_redacted() {
    let hub = WinCodexControlHub::default();
    let event = hub.record(
        "trace-1",
        "network",
        "info",
        "request.done",
        "Authorization: Bearer private",
        json!({"path":"/api/me", "cookie":"private", "nested":{"access_token":"private"}}),
    );
    assert!(event.summary.contains("已脱敏"));
    assert_eq!(event.fields["cookie"], "[REDACTED]");
    assert_eq!(event.fields["nested"]["access_token"], "[REDACTED]");
}

#[test]
fn receipts_are_idempotent_but_conflicting_terminal_states_fail() {
    let hub = WinCodexControlHub::default();
    let action = hub
        .enqueue_action("trace", "focus_window", None, None, "test")
        .unwrap();
    let receipt = WinControlReceipt {
        status: "succeeded".to_string(),
        message: Some("focused".to_string()),
        route: None,
        window_state: None,
        at_ms: None,
    };
    assert_eq!(
        hub.claim_action(&action.action_id).unwrap().status,
        "executing"
    );
    assert!(hub
        .record_receipt(&action.action_id, receipt.clone())
        .is_ok());
    assert!(hub.record_receipt(&action.action_id, receipt).is_ok());
    assert!(hub
        .record_receipt(
            &action.action_id,
            WinControlReceipt {
                status: "failed".to_string(),
                message: None,
                route: None,
                window_state: None,
                at_ms: None,
            },
        )
        .is_err());
}

#[test]
fn claiming_an_action_removes_it_from_the_pending_queue() {
    let hub = WinCodexControlHub::default();
    let action = hub
        .enqueue_action("trace", "reload_page", None, None, "test")
        .unwrap();
    assert_eq!(hub.pending_actions(10).len(), 1);
    hub.claim_action(&action.action_id).unwrap();
    assert!(hub.pending_actions(10).is_empty());
    assert_eq!(
        hub.claim_action(&action.action_id).unwrap().status,
        "executing"
    );
}

#[test]
fn action_lookup_returns_only_sanitized_ai_window_receipts() {
    let hub = WinCodexControlHub::default();
    let action = hub
        .enqueue_action("trace", "list_ai_windows", None, None, "test")
        .unwrap();
    hub.claim_action(&action.action_id).unwrap();
    hub.record_receipt(
        &action.action_id,
        WinControlReceipt {
            status: "succeeded".to_string(),
            message: Some("listed".to_string()),
            route: None,
            window_state: Some(json!({
                "schema":"elon.tauri_ai_window_list.v1",
                "windows":[{
                    "provider_id":"chatgpt",
                    "phase":"ready",
                    "open":true,
                    "focused":false,
                    "page_ready":true,
                    "root_exists":true,
                    "root_child_count":1,
                    "last_error_code":null,
                    "retryable":false,
                    "updated_at_ms":42,
                    "window_label":"local-ai-native-chatgpt-owner-secret",
                    "url":"https://chatgpt.com/private",
                    "official_session":{
                        "present":true,
                        "window_status":"ready",
                        "adapter_connected":true,
                        "semantic_snapshot_ready":true,
                        "composer_ready":true,
                        "context_ready":true,
                        "page_kind":"conversation",
                        "cache_status":"live",
                        "semantic_cache_status":"live",
                        "navigation_cache_status":"live",
                        "navigation_snapshot_ready":true,
                        "navigation_live":true,
                        "directory_complete":false,
                        "directory_observed_count":3,
                        "directory_available_count":9,
                        "conversation_count":9,
                        "project_count":2,
                        "pinned_count":1,
                        "local_conversation_count":4,
                        "active_conversation":true,
                        "last_event_kind":"conversation_snapshot",
                        "last_command_action":"list_conversations",
                        "last_command_ok":true,
                        "message_count":6,
                        "assistant_message_count":3,
                        "streaming":false,
                        "updated_at_ms":41,
                        "draft":"private prompt",
                        "owner":"owner-secret",
                        "cookie":"cookie-secret",
                        "token":"token-secret",
                        "exception_detail":"private exception detail"
                    }
                },{
                    "provider_id":"google-ai-mode",
                    "phase":"not_created",
                    "open":false,
                    "focused":false,
                    "page_ready":false,
                    "root_exists":false,
                    "root_child_count":0,
                    "last_error_code":null,
                    "retryable":true,
                    "updated_at_ms":0
                }],
                "privacy":{"cookies":true}
            })),
            at_ms: None,
        },
    )
    .unwrap();

    let completed = hub.action(&action.action_id).unwrap();
    let state = completed.receipt.unwrap().window_state.unwrap();
    let serialized = serde_json::to_string(&state).unwrap();
    assert_eq!(state["windows"][0]["provider_id"], "chatgpt");
    assert_eq!(
        state["windows"][0]["official_session"]["directory_available_count"],
        9
    );
    assert_eq!(
        state["windows"][0]["official_session"]["last_command_action"],
        "list_conversations"
    );
    assert_eq!(state["privacy"]["cookies"], false);
    assert!(!serialized.contains("\"window_label\":"));
    assert!(!serialized.contains("chatgpt.com"));
    assert!(!serialized.contains("owner-secret"));
    assert!(!serialized.contains("private prompt"));
    assert!(!serialized.contains("cookie-secret"));
    assert!(!serialized.contains("token-secret"));
    assert!(!serialized.contains("private exception detail"));
}

#[test]
fn ai_window_receipts_are_bound_to_action_kind_and_provider() {
    let hub = WinCodexControlHub::default();
    let action = hub
        .enqueue_action(
            "trace",
            "capture_ai_window_state",
            None,
            Some("google-ai-mode"),
            "test",
        )
        .unwrap();
    hub.claim_action(&action.action_id).unwrap();
    let mismatched = hub.record_receipt(
        &action.action_id,
        WinControlReceipt {
            status: "succeeded".to_string(),
            message: None,
            route: None,
            window_state: Some(json!({
                "schema":"elon.tauri_ai_window_capture.v1",
                "window":{
                    "provider_id":"chatgpt",
                    "phase":"ready",
                    "open":true,
                    "focused":true,
                    "page_ready":true,
                    "root_exists":true,
                    "root_child_count":1,
                    "retryable":false,
                    "updated_at_ms":1
                }
            })),
            at_ms: None,
        },
    );
    assert!(mismatched.is_err());

    let missing = hub.record_receipt(
        &action.action_id,
        WinControlReceipt {
            status: "succeeded".to_string(),
            message: None,
            route: None,
            window_state: None,
            at_ms: None,
        },
    );
    assert!(missing.is_err());
}

#[test]
fn source_filter_and_cursor_are_bounded() {
    let hub = WinCodexControlHub::default();
    let first = hub.record("t", "frontend", "info", "ready", "ready", json!({}));
    let second = hub.record("t", "network", "info", "request", "request", json!({}));
    let sources = HashSet::from(["network".to_string()]);
    let events = hub.events(first.seq, 50, &sources);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, second.event_id);
}

#[test]
fn recent_cli_projection_keeps_the_newest_events() {
    let dir = std::env::temp_dir().join(format!(
        "elon_win_control_recent_cli_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let journal = crate::node_agent_task_journal::TaskJournal::new(&dir);
    journal
        .record_started(crate::node_agent_task_journal::TaskJournalStart {
            req_id: "task-recent",
            cli_name: "codex",
            route: Some("route_a"),
            run_handle_id: None,
            cwd: None,
            runtime_permission: None,
        })
        .unwrap();
    for index in 0..12 {
        journal
            .record_cli_chunk("task-recent", "stdout", &format!("chunk-{index}"))
            .unwrap();
    }
    let events = journal
        .recent_events_for_tasks(&HashSet::from(["task-recent".to_string()]), 3)
        .unwrap();
    let task_events = events.get("task-recent").unwrap();
    assert_eq!(task_events.len(), 3);
    assert!(task_events
        .last()
        .and_then(|event| event.event.get("text"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .contains("chunk-11"));
    let _ = std::fs::remove_dir_all(dir);
}
