#[cfg(test)]
mod tests {
    use super::{
        api_runtime_chat_payload, api_runtime_config_from_lookup,
        api_runtime_should_retry_without_json_mode, ensure_runtime_response_size,
        parse_agent_response, run_runtime_loop, runtime_http_error_message,
        runtime_response_too_large_message, system_prompt, ApiRuntimeConfig, RuntimeLoopOptions,
        MAX_RUNTIME_HTTP_BODY_BYTES,
    };
    use crate::{
        node_agent_task_journal::TaskJournal, node_agent_tool_approval::ToolApprovalState,
        node_agent_tool_guard::ToolGuard,
    };
    use anyhow::anyhow;
    use homecli_proto::AgentToServer;
    use serde_json::{json, Value};
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{SystemTime, UNIX_EPOCH},
    };
    use tokio::sync::{mpsc, watch};
    use tokio_tungstenite::tungstenite::Message;

    #[test]
    fn api_runtime_config_defaults_openai_base_but_requires_model() {
        assert!(api_runtime_config_from_lookup(|name| match name {
            "OPENAI_API_KEY" => Some("sk-test".to_string()),
            _ => None,
        })
        .is_none());

        let config = api_runtime_config_from_lookup(|name| match name {
            "OPENAI_API_KEY" => Some(" sk-test ".to_string()),
            "OPENAI_MODEL" => Some(" gpt-test ".to_string()),
            _ => None,
        })
        .expect("api key and model should create config");

        assert_eq!(config.api_base, "https://api.openai.com/v1");
        assert_eq!(config.api_key, "sk-test");
        assert_eq!(config.model, "gpt-test");
    }

    #[test]
    fn api_runtime_config_prefers_elon_specific_env() {
        let config = api_runtime_config_from_lookup(|name| match name {
            "ELON_AGENT_API_BASE" => Some("https://example.test/v1/".to_string()),
            "ELON_AGENT_API_KEY" => Some("elon-key".to_string()),
            "ELON_AGENT_MODEL" => Some("custom-model".to_string()),
            "OPENAI_API_KEY" => Some("openai-key".to_string()),
            _ => None,
        })
        .expect("elon env should create config");

        assert_eq!(config.api_base, "https://example.test/v1");
        assert_eq!(config.api_key, "elon-key");
        assert_eq!(config.model, "custom-model");
    }

    #[test]
    fn api_runtime_payload_uses_json_mode_and_tools_by_default() {
        let config = ApiRuntimeConfig {
            api_base: "https://example.test/v1".to_string(),
            api_key: "sk-test".to_string(),
            model: "gpt-test".to_string(),
        };
        let messages = vec![json!({"role": "user", "content": "Return JSON"})];

        let payload = api_runtime_chat_payload(&config, &messages, true, true);
        assert_eq!(payload["model"], "gpt-test");
        assert_eq!(payload["temperature"], 0.2);
        assert_eq!(payload["response_format"]["type"], "json_object");
        assert_eq!(payload["tool_choice"], "auto");
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "file_info"));
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "git_status"));
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "git_diff"));
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "git_log"));
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "git_show"));
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "run_command"));
        assert_eq!(payload["messages"][0]["content"], "Return JSON");

        let fallback = api_runtime_chat_payload(&config, &messages, false, false);
        assert!(fallback.get("response_format").is_none());
        assert!(fallback.get("tools").is_none());
    }

    #[test]
    fn api_runtime_json_mode_retry_is_limited_to_compatibility_errors() {
        assert!(api_runtime_should_retry_without_json_mode(
            reqwest::StatusCode::BAD_REQUEST,
            "Unrecognized request argument supplied: response_format"
        ));
        assert!(api_runtime_should_retry_without_json_mode(
            reqwest::StatusCode::UNPROCESSABLE_ENTITY,
            "json_object is not supported by this model"
        ));
        assert!(!api_runtime_should_retry_without_json_mode(
            reqwest::StatusCode::UNAUTHORIZED,
            "invalid api key"
        ));
        assert!(!api_runtime_should_retry_without_json_mode(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "rate limit"
        ));
    }

    #[test]
    fn system_prompt_matches_runtime_route_identity() {
        let route_b = system_prompt("api-runtime", false, false);
        let route_c = system_prompt("server-runtime", true, false);
        let danger = system_prompt("server-runtime", false, true);

        assert!(route_b.contains("Route B local API runtime"));
        assert!(route_b.contains("\"tool\": \"git_status\""));
        assert!(route_b.contains("\"tool\": \"git_diff\""));
        assert!(route_b.contains("\"tool\": \"git_log\""));
        assert!(route_b.contains("\"tool\": \"git_show\""));
        assert!(route_b.contains("Use git_status, git_diff, git_log, and git_show"));
        assert!(!route_b.contains("Route C server runtime for"));
        assert!(route_c.contains("Route C server runtime"));
        assert!(route_c.contains("read-only planning"));
        assert!(route_c.contains("Do not request write_file, apply_patch, or run_command"));
        assert!(route_c.contains("You may still use git_status, git_diff, git_log, and git_show"));
        assert!(danger.contains("danger_full_access"));
        assert!(danger.contains("arbitrary cmd/powershell/pwsh commands"));
        assert!(danger.contains("\"shell\": \"cmd\""));
    }

    #[test]
    fn runtime_http_error_message_redacts_provider_body() {
        let message = runtime_http_error_message(
            "本机 API runtime",
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "429 rate limit: sk-secret and prompt text",
        );

        assert!(message.contains("429"));
        assert!(message.contains("rate_limit"));
        assert!(message.contains("fingerprint="));
        assert!(!message.contains("sk-secret"));
        assert!(!message.contains("prompt text"));
    }

    #[test]
    fn runtime_response_size_limit_allows_boundary() {
        ensure_runtime_response_size("服务器 AI runtime", MAX_RUNTIME_HTTP_BODY_BYTES)
            .expect("exact limit should be accepted");
    }

    #[test]
    fn runtime_response_size_limit_rejects_oversized_body() {
        let error =
            ensure_runtime_response_size("服务器 AI runtime", MAX_RUNTIME_HTTP_BODY_BYTES + 1)
                .unwrap_err();
        let message = error.to_string();

        assert!(message.contains("服务器 AI runtime 响应过大"));
        assert!(message.contains("已中止读取"));
        assert!(message.contains(&(MAX_RUNTIME_HTTP_BODY_BYTES + 1).to_string()));
    }

    #[test]
    fn runtime_response_too_large_message_does_not_include_body() {
        let message =
            runtime_response_too_large_message("本机 API runtime", MAX_RUNTIME_HTTP_BODY_BYTES + 9);

        assert!(message.contains("本机 API runtime 响应过大"));
        assert!(message.contains("1048585"));
        assert!(!message.contains("sk-secret"));
        assert!(!message.contains("prompt text"));
    }

    #[test]
    fn parse_agent_response_accepts_markdown_fenced_json() {
        let parsed = parse_agent_response(
            r#"```json
{"message":"ok","done":true,"actions":[]}
```"#,
        )
        .expect("fenced json should parse");

        assert_eq!(parsed["message"], "ok");
        assert_eq!(parsed["done"], true);
    }

    #[test]
    fn parse_agent_response_skips_non_json_braces_before_payload() {
        let parsed = parse_agent_response(
            r#"先说明：{这不是 JSON}
{"message":"继续","done":false,"actions":[{"tool":"list_dir","path":"."}]}
后续文字"#,
        )
        .expect("first valid json object should parse");

        assert_eq!(parsed["message"], "继续");
        assert_eq!(parsed["actions"][0]["tool"], "list_dir");
    }

    #[test]
    fn parse_agent_response_ignores_braces_inside_json_strings() {
        let parsed = parse_agent_response(
            r#"prefix {"message":"literal { brace } text","done":true,"actions":[]} suffix"#,
        )
        .expect("json string braces should not break scanning");

        assert_eq!(parsed["message"], "literal { brace } text");
        assert_eq!(parsed["done"], true);
    }

    #[tokio::test]
    async fn runtime_loop_emits_structured_status_events() {
        let workspace = temp_test_dir("runtime_loop_emits_structured_status_events");
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: "req-status",
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "finish",
                    approval_state: Some(ToolApprovalState::default()),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("test-model".to_string()),
                },
                move |_| async move {
                    Ok(chat_response(json!({
                        "message": "done",
                        "done": true,
                        "actions": []
                    })))
                },
            )
            .await
            .unwrap()
        });

        let thinking = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(thinking["phase"], "thinking");
        assert_eq!(thinking["runtime"], "test-runtime");
        let completed = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(completed["phase"], "completed");
        assert_eq!(completed["status"], "ok");

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        let _ = cancel_tx.send(true);
    }

    #[tokio::test]
    async fn runtime_loop_executes_openai_tool_calls() {
        let workspace = temp_test_dir("runtime_loop_executes_openai_tool_calls");
        std::fs::write(workspace.join("README.md"), "hello\n").unwrap();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();

        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: "req-tool-calls",
                    label: "api-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "inspect files",
                    approval_state: Some(ToolApprovalState::default()),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("gpt-test".to_string()),
                },
                move |_| {
                    let call_index = calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                    async move {
                        if call_index == 0 {
                            Ok(chat_tool_call_response("list_dir", json!({ "path": "." })))
                        } else {
                            Ok(chat_response(json!({
                                "message": "done after list",
                                "done": true,
                                "actions": []
                            })))
                        }
                    }
                },
            )
            .await
            .unwrap()
        });

        let tool_call = next_tool_event(&mut out_rx, "tool_call").await;
        assert_eq!(tool_call["tool"], "list_dir");
        let tool_result = next_tool_event(&mut out_rx, "tool_result").await;
        assert_eq!(tool_result["status"], "ok");
        assert!(tool_result["result"]
            .as_str()
            .unwrap_or_default()
            .contains("README.md"));

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let _ = cancel_tx.send(true);
    }

    #[tokio::test]
    async fn runtime_loop_emits_canceled_summary_when_stopped_before_turn() {
        let workspace =
            temp_test_dir("runtime_loop_emits_canceled_summary_when_stopped_before_turn");
        let (_cancel_tx, cancel_rx) = watch::channel(true);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();

        let result = run_runtime_loop(
            RuntimeLoopOptions {
                req_id: "req-canceled",
                label: "test-runtime",
                guard: ToolGuard::new(workspace, Some("project_write")),
                prompt: "finish",
                approval_state: Some(ToolApprovalState::default()),
                cancel_rx,
                out_tx,
                task_journal: None,
                initial_model: Some("test-model".to_string()),
            },
            move |_| {
                calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                async move {
                    Ok(chat_response(json!({
                        "message": "should not run",
                        "done": true,
                        "actions": []
                    })))
                }
            },
        )
        .await
        .unwrap();

        let canceled = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(canceled["phase"], "canceled");
        assert_eq!(canceled["status"], "canceled");
        let summary = next_tool_event(&mut out_rx, "runtime_summary").await;
        assert_eq!(summary["status"], "canceled");
        assert_eq!(summary["total_tools"], 0);
        assert!(!result.exit_ok);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn runtime_loop_emits_failure_summary_on_model_error() {
        let workspace = temp_test_dir("runtime_loop_emits_failure_summary_on_model_error");
        let (_cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();

        let error = match run_runtime_loop(
            RuntimeLoopOptions {
                req_id: "req-model-error",
                label: "test-runtime",
                guard: ToolGuard::new(workspace, Some("project_write")),
                prompt: "finish",
                approval_state: Some(ToolApprovalState::default()),
                cancel_rx,
                out_tx,
                task_journal: None,
                initial_model: Some("test-model".to_string()),
            },
            move |_| async move { Err(anyhow!("provider unavailable")) },
        )
        .await
        {
            Ok(_) => panic!("runtime should fail when model call fails"),
            Err(error) => error,
        };

        let thinking = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(thinking["phase"], "thinking");
        let failed = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(failed["phase"], "failed");
        assert_eq!(failed["status"], "error");
        assert!(failed["message"]
            .as_str()
            .unwrap_or_default()
            .contains("调用 test-runtime 失败"));
        assert!(failed["message"]
            .as_str()
            .unwrap_or_default()
            .contains("provider unavailable"));
        let summary = next_tool_event(&mut out_rx, "runtime_summary").await;
        assert_eq!(summary["status"], "error");
        assert_eq!(summary["failed_tools"], 0);
        assert!(format!("{error:#}").contains("provider unavailable"));
    }

    #[tokio::test]
    async fn runtime_denies_write_without_executing_tool() {
        let workspace = temp_test_dir("runtime_denies_write_without_executing_tool");
        let target = workspace.join("blocked.txt");
        let approval_state = ToolApprovalState::default();
        let approval_decider = approval_state.clone();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();
        let req_id = "req-deny-write".to_string();
        let runtime_req_id = req_id.clone();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: &runtime_req_id,
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "write a file",
                    approval_state: Some(approval_state),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("test-model".to_string()),
                },
                move |_| {
                    let call_index = calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                    async move {
                        if call_index == 0 {
                            Ok(chat_response(json!({
                                "message": "need write",
                                "done": false,
                                "actions": [{
                                    "tool": "write_file",
                                    "path": "blocked.txt",
                                    "content": "should not be written"
                                }]
                            })))
                        } else {
                            Ok(chat_response(json!({
                                "message": "done after deny",
                                "done": true,
                                "actions": []
                            })))
                        }
                    }
                },
            )
            .await
            .unwrap()
        });

        let approval = next_tool_event(&mut out_rx, "tool_approval_required").await;
        assert_eq!(approval["approval_id"], "tap_1_1");
        assert_eq!(approval["tool"], "write_file");
        assert_eq!(approval["diff"]["source"], "write_file");
        assert_eq!(approval["diff"]["kind"], "create");
        assert_eq!(approval["diff"]["files"][0], "blocked.txt");
        assert!(approval["diff"]["preview"]
            .as_str()
            .unwrap_or_default()
            .contains("--- /dev/null"));
        assert!(
            approval["diff"]["new_sha256"]
                .as_str()
                .unwrap_or_default()
                .len()
                >= 64
        );
        assert!(approval["diff"]["old_sha256"].is_null());
        assert!(!target.exists(), "write_file must not run before approval");

        assert!(approval_decider.decide(&req_id, "tap_1_1", "deny").await);
        let denied = next_tool_event(&mut out_rx, "tool_result").await;
        assert_eq!(denied["status"], "error");
        assert!(denied["result"]
            .as_str()
            .unwrap_or_default()
            .contains("denied by user"));

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        assert!(
            !target.exists(),
            "denied write_file must not create the file"
        );
        let _ = cancel_tx.send(true);
    }

    #[tokio::test]
    async fn runtime_records_canceled_approval_decision_in_journal() {
        let workspace = temp_test_dir("runtime_records_canceled_approval_decision_in_journal");
        let target = workspace.join("blocked.txt");
        let task_journal = TaskJournal::new(workspace.join(".journal"));
        let journal_for_runtime = task_journal.clone();
        let approval_state = ToolApprovalState::default();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let req_id = "req-cancel-approval".to_string();
        let runtime_req_id = req_id.clone();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: &runtime_req_id,
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "write a file",
                    approval_state: Some(approval_state),
                    cancel_rx,
                    out_tx,
                    task_journal: Some(journal_for_runtime),
                    initial_model: Some("test-model".to_string()),
                },
                move |_| async move {
                    Ok(chat_response(json!({
                        "message": "need write",
                        "done": false,
                        "actions": [{
                            "tool": "write_file",
                            "path": "blocked.txt",
                            "content": "should not be written"
                        }]
                    })))
                },
            )
            .await
            .unwrap()
        });

        let approval = next_tool_event(&mut out_rx, "tool_approval_required").await;
        assert_eq!(approval["approval_id"], "tap_1_1");
        assert_eq!(approval["tool"], "write_file");
        assert_eq!(
            approval["approval_checkpoint"]["schema"],
            "elon.routebc.tool_approval_checkpoint.v1"
        );
        assert_eq!(
            approval["approval_checkpoint"]["restart_recovery"]["next_action"],
            "continue_from_snapshot"
        );
        assert_eq!(
            approval["approval_checkpoint"]["restart_recovery"]["supported"].as_bool(),
            Some(false)
        );
        assert!(
            approval["approval_checkpoint"]["action_sha256"]
                .as_str()
                .unwrap_or_default()
                .len()
                >= 64
        );
        assert!(
            !serde_json::to_string(&approval["approval_checkpoint"])
                .unwrap()
                .contains("should not be written"),
            "approval checkpoint must not store write_file content"
        );
        cancel_tx.send(true).expect("cancel should reach runtime");

        let decision = next_tool_event(&mut out_rx, "tool_approval_decision").await;
        assert_eq!(decision["approval_id"], "tap_1_1");
        assert_eq!(decision["decision"], "cancel");
        assert_eq!(decision["status"], "canceled");
        let canceled = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(canceled["phase"], "canceled");

        let result = runtime.await.unwrap();
        assert!(!result.exit_ok);
        assert!(
            !target.exists(),
            "canceled approval must not execute write_file"
        );

        let snapshot = task_journal
            .snapshot(&req_id, 0, 20)
            .expect("approval decision should be replayable from local journal");
        assert_eq!(snapshot.approvals.approvals.len(), 1);
        let checkpoint = snapshot.approvals.approvals[0]
            .checkpoint
            .as_ref()
            .expect("approval checkpoint should persist through task journal");
        assert_eq!(
            checkpoint["schema"],
            "elon.routebc.tool_approval_checkpoint.v1"
        );
        assert_eq!(
            checkpoint["restart_recovery"]["next_action"],
            "continue_from_snapshot"
        );
        assert!(snapshot.events.iter().any(|entry| {
            entry.event.get("type").and_then(Value::as_str) == Some("tool_event")
                && entry
                    .event
                    .get("event")
                    .and_then(|event| event.get("type"))
                    .and_then(Value::as_str)
                    == Some("tool_approval_decision")
                && entry
                    .event
                    .get("event")
                    .and_then(|event| event.get("status"))
                    .and_then(Value::as_str)
                    == Some("canceled")
        }));

    // Integration tests (continued from tests_integration.rs)
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node_agent_server_runtime/tests_integration.rs"));
    #[tokio::test]
    async fn runtime_rejects_stale_write_file_after_approval() {
        let workspace = temp_test_dir("runtime_rejects_stale_write_file_after_approval");
        let target = workspace.join("note.txt");
        tokio::fs::write(&target, "old\n").await.unwrap();
        let approval_state = ToolApprovalState::default();
        let approval_decider = approval_state.clone();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();
        let req_id = "req-stale-write".to_string();
        let runtime_req_id = req_id.clone();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: &runtime_req_id,
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "write a file",
                    approval_state: Some(approval_state),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("test-model".to_string()),
                },
                move |_| {
                    let call_index = calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                    async move {
                        if call_index == 0 {
                            Ok(chat_response(json!({
                                "message": "need write",
                                "done": false,
                                "actions": [{
                                    "tool": "write_file",
                                    "path": "note.txt",
                                    "content": "new\n"
                                }]
                            })))
                        } else {
                            Ok(chat_response(json!({
                                "message": "done after stale",
                                "done": true,
                                "actions": []
                            })))
                        }
                    }
                },
            )
            .await
            .unwrap()
        });

        let approval = next_tool_event(&mut out_rx, "tool_approval_required").await;
        assert!(approval["diff"]["preview"]
            .as_str()
            .unwrap_or_default()
            .contains("-old"));
        tokio::fs::write(&target, "changed elsewhere\n")
            .await
            .unwrap();
        assert!(approval_decider.decide(&req_id, "tap_1_1", "approve").await);

        let stale = next_tool_event(&mut out_rx, "tool_result").await;
        assert_eq!(stale["status"], "error");
        assert!(stale["result"]
            .as_str()
            .unwrap_or_default()
            .contains("approval preview is stale"));
        assert_eq!(
            tokio::fs::read_to_string(&target).await.unwrap(),
            "changed elsewhere\n"
        );

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        let _ = cancel_tx.send(true);
    }

    #[tokio::test]
    async fn runtime_writes_file_after_approval_when_preview_is_current() {
        let workspace = temp_test_dir("runtime_writes_file_after_approval_when_preview_is_current");
        let target = workspace.join("note.txt");
        let approval_state = ToolApprovalState::default();
        let approval_decider = approval_state.clone();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();
        let req_id = "req-approve-write".to_string();
        let runtime_req_id = req_id.clone();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: &runtime_req_id,
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "write a file",
                    approval_state: Some(approval_state),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("test-model".to_string()),
                },
                move |_| {
                    let call_index = calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                    async move {
                        if call_index == 0 {
                            Ok(chat_response(json!({
                                "message": "need write",
                                "done": false,
                                "actions": [{
                                    "tool": "write_file",
                                    "path": "note.txt",
                                    "content": "approved\n"
                                }]
                            })))
                        } else {
                            Ok(chat_response(json!({
                                "message": "done after approve",
                                "done": true,
                                "actions": []
                            })))
                        }
                    }
                },
            )
            .await
            .unwrap()
        });

        let approval = next_tool_event(&mut out_rx, "tool_approval_required").await;
        assert_eq!(approval["diff"]["kind"], "create");
        assert!(approval_decider.decide(&req_id, "tap_1_1", "approve").await);

        let tool_call = next_tool_event(&mut out_rx, "tool_call").await;
        assert_eq!(tool_call["tool"], "write_file");
        let tool_result = next_tool_event(&mut out_rx, "tool_result").await;
        assert_eq!(tool_result["status"], "ok");
        assert_eq!(
            tokio::fs::read_to_string(&target).await.unwrap(),
            "approved\n"
        );

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        let _ = cancel_tx.send(true);
    }

    #[tokio::test]
    async fn runtime_rejects_stale_apply_patch_after_approval() {
        let workspace = temp_test_dir("runtime_rejects_stale_apply_patch_after_approval");
        let target = workspace.join("note.txt");
        tokio::fs::write(&target, "old\n").await.unwrap();
        init_git_repo(&workspace);
        let approval_state = ToolApprovalState::default();
        let approval_decider = approval_state.clone();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();
        let req_id = "req-stale-apply-patch".to_string();
        let runtime_req_id = req_id.clone();
        let patch = "diff --git a/note.txt b/note.txt\n--- a/note.txt\n+++ b/note.txt\n@@ -1 +1 @@\n-old\n+new\n".to_string();
        let patch_for_runtime = patch.clone();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: &runtime_req_id,
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "patch a file",
                    approval_state: Some(approval_state),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("test-model".to_string()),
                },
                move |_| {
                    let call_index = calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                    let patch = patch_for_runtime.clone();
                    async move {
                        if call_index == 0 {
                            Ok(chat_response(json!({
                                "message": "need patch",
                                "done": false,
                                "actions": [{
                                    "tool": "apply_patch",
                                    "patch": patch
                                }]
                            })))
                        } else {
                            Ok(chat_response(json!({
                                "message": "done after stale",
                                "done": true,
                                "actions": []
                            })))
                        }
                    }
                },
            )
            .await
            .unwrap()
        });

        let approval = next_tool_event(&mut out_rx, "tool_approval_required").await;
        assert_eq!(approval["tool"], "apply_patch");
        assert_eq!(approval["diff"]["source"], "apply_patch");
        assert_eq!(approval["diff"]["files"][0], "note.txt");
        assert!(approval["diff"]["preview"]
            .as_str()
            .unwrap_or_default()
            .contains("-old"));
        assert!(
            approval["diff"]["patch_sha256"]
                .as_str()
                .unwrap_or_default()
                .len()
                >= 64
        );
        tokio::fs::write(&target, "changed elsewhere\n")
            .await
            .unwrap();
        assert!(approval_decider.decide(&req_id, "tap_1_1", "approve").await);

        let stale = next_tool_event(&mut out_rx, "tool_result").await;
        assert_eq!(stale["status"], "error");
        assert!(stale["result"]
            .as_str()
            .unwrap_or_default()
            .contains("approval preview is stale"));
        assert_eq!(
            tokio::fs::read_to_string(&target).await.unwrap(),
            "changed elsewhere\n"
        );

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        let _ = cancel_tx.send(true);
    }

    async fn next_tool_event(
        out_rx: &mut mpsc::UnboundedReceiver<Message>,
        event_type: &str,
    ) -> Value {
        let deadline = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while let Some(frame) = out_rx.recv().await {
                let Message::Text(text) = frame else {
                    continue;
                };
                let Ok(AgentToServer::CliChunk { text, .. }) =
                    serde_json::from_str::<AgentToServer>(&text)
                else {
                    continue;
                };
                let Ok(value) = serde_json::from_str::<Value>(text.trim()) else {
                    continue;
                };
                if value.get("type").and_then(Value::as_str) == Some(event_type) {
                    return value;
                }
            }
            panic!("event stream closed before {event_type}");
        })
        .await;
        deadline.unwrap_or_else(|_| panic!("timed out waiting for {event_type}"))
    }

    fn chat_response(agent: Value) -> Value {
        json!({
            "model": "test-model",
            "choices": [{
                "message": {
                    "content": serde_json::to_string(&agent).unwrap()
                }
            }]
        })
    }

    fn chat_tool_call_response(name: &str, arguments: Value) -> Value {
        json!({
            "model": "test-model",
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_test",
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": serde_json::to_string(&arguments).unwrap()
                        }
                    }]
                }
            }]
        })
    }

    fn init_git_repo(path: &std::path::Path) {
        let status = std::process::Command::new("git")
            .args(["init"])
            .current_dir(path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn temp_test_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!("elon-{name}-{nanos}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
