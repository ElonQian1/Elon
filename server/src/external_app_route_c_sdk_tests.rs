use super::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[test]
fn parses_model_output_actions() {
    let output = parse_model_output(
        r#"{"reply":"先检测","done":false,"actions":[{"tool":"test-google","args":{"url":"https://google.com"},"reason":"验证链路"}]}"#,
    );
    assert_eq!(output.reply, "先检测");
    assert_eq!(output.actions[0].tool, "test_google");
    assert_eq!(output.actions[0].args["url"], "https://google.com");
}

#[test]
fn filters_actions_by_manifest() {
    let actions = vec![
        RouteCAction {
            id: String::new(),
            tool: "test_google".to_string(),
            args: json!({}),
            reason: None,
            dangerous: false,
        },
        RouteCAction {
            id: String::new(),
            tool: "force_close_proxy".to_string(),
            args: json!({}),
            reason: None,
            dangerous: true,
        },
    ];
    let allowed = allowed_tool_names(
        "bb64a",
        &json!({"tools": [{"name": "test_google"}]}),
        &Value::Null,
    );
    let sanitized = sanitize_actions(actions, &allowed, 3);
    assert_eq!(sanitized.len(), 1);
    assert_eq!(sanitized[0].tool, "test_google");
    assert_eq!(sanitized[0].id, "tool_1");
}

#[test]
fn supports_default_bb64a_tools_when_manifest_missing() {
    let allowed = allowed_tool_names("bb64a", &Value::Null, &Value::Null);
    assert!(allowed.contains("bb64a_doctor"));
    assert!(allowed.contains("test_google"));
    assert!(!allowed.contains("force_close_proxy"));
}

#[test]
fn empty_manifest_filters_all_actions_for_custom_apps() {
    let actions = vec![RouteCAction {
        id: String::new(),
        tool: "invented_tool".to_string(),
        args: json!({}),
        reason: None,
        dangerous: false,
    }];
    let sanitized = sanitize_actions(actions, &BTreeSet::new(), 3);
    assert!(sanitized.is_empty());
}

#[test]
fn collects_manifest_tools_from_common_shapes() {
    let allowed = allowed_tool_names(
        "custom",
        &json!({
            "tool_ids": ["alpha-tool"],
            "tools": [{"id": "beta_tool"}],
            "chat_auto_executable_tool_ids": ["gamma.tool"]
        }),
        &Value::Null,
    );
    assert!(allowed.contains("alpha_tool"));
    assert!(allowed.contains("beta_tool"));
    assert!(allowed.contains("gamma.tool"));
}

#[test]
fn detects_danger_full_access_from_sdk_request() {
    let req = ExternalAppRouteCChatRequest {
        conversation_id: None,
        message: "诊断网络".to_string(),
        history: Vec::new(),
        client: Value::Null,
        local_context: Value::Null,
        tool_manifest: json!({
            "tools": [
                {"name": "run_command", "permission": "danger_full_access", "dangerous": true}
            ]
        }),
        tool_results: Vec::new(),
        sdk: Value::Null,
        runtime_permission: None,
        runtime_route: None,
        agent: None,
        max_actions: None,
    };

    assert!(request_danger_full_access(&req));
}

#[test]
fn danger_full_access_prompt_explains_local_cli_shape() {
    let allowed = BTreeSet::from(["run_command".to_string(), "read_file".to_string()]);
    let prompt = system_prompt("bb64a", &allowed, 3, true, RuntimeRoute::RouteC);

    assert!(prompt.contains("runtime_permission=danger_full_access"));
    assert!(prompt.contains("\"shell\":\"cmd|powershell|pwsh|bash|sh\""));
    assert!(prompt.contains("\"program\":\"cmd\""));
}

#[test]
fn parses_project_ai_runtime_routes() {
    assert_eq!(parse_runtime_route("Route A"), Some(RuntimeRoute::RouteA));
    assert_eq!(parse_runtime_route("byok"), Some(RuntimeRoute::RouteB));
    assert_eq!(
        parse_runtime_route("server-model"),
        Some(RuntimeRoute::RouteC)
    );
}

#[test]
fn detects_runtime_route_from_sdk_request() {
    let req = ExternalAppRouteCChatRequest {
        conversation_id: None,
        message: "诊断网络".to_string(),
        history: Vec::new(),
        client: Value::Null,
        local_context: Value::Null,
        tool_manifest: Value::Null,
        tool_results: Vec::new(),
        sdk: json!({"runtimeRoute": "local-api-key"}),
        runtime_permission: None,
        runtime_route: None,
        agent: None,
        max_actions: None,
    };

    assert_eq!(request_runtime_route(&req), RuntimeRoute::RouteB);
}

#[test]
fn project_ai_prompt_explains_routes_remote_source_and_feedback() {
    let allowed = BTreeSet::from([
        "run_command".to_string(),
        "remote_source_ask".to_string(),
        "create_feedback_post".to_string(),
    ]);
    let prompt = system_prompt("bb64a", &allowed, 3, true, RuntimeRoute::RouteA);

    assert!(prompt.contains("Route A / Route B / Route C"));
    assert!(prompt.contains("remote_source_ask"));
    assert!(prompt.contains("create_feedback_post"));
    assert!(prompt.contains("需求频道"));
}

#[test]
fn collects_project_ai_remote_source_and_feedback_tool_groups() {
    let allowed = allowed_tool_names(
        "custom",
        &json!({
            "remote_source_tools": ["remote-source-ask"],
            "feedbackTools": [{"name": "create_feedback_post"}]
        }),
        &Value::Null,
    );

    assert!(allowed.contains("remote_source_ask"));
    assert!(allowed.contains("create_feedback_post"));
}
