use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::{collections::BTreeSet, sync::Arc};

use crate::{
    agent_fallback::{is_retryable_agent_error, server_api_agents_in_fallback_order},
    agent_llm_call::friendly_ai_api_error,
    types::{AgentConfig, AppState},
};

use super::{
    ExternalAppRouteCChatRequest, RouteCAction, RouteCChatMessage, RouteCModelOutput, RuntimeRoute,
    MAX_HISTORY_CONTENT_CHARS, MAX_HISTORY_ITEMS, MAX_JSON_CONTEXT_CHARS, MAX_MESSAGE_CHARS,
    MAX_OUTPUT_TOKENS, MAX_TOOL_MANIFEST_CHARS, MAX_TOOL_RESULTS, MAX_TOOL_RESULT_CHARS,
};

pub(super) fn route_c_sdk_enabled() -> bool {
    env_flag("ELON_EXTERNAL_APP_ROUTE_C_SDK_ENABLED")
        || env_flag("ELON_EXTERNAL_APP_MVP_CHAT_ENABLED")
}

pub(super) fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub(super) fn request_danger_full_access(req: &ExternalAppRouteCChatRequest) -> bool {
    req.runtime_permission
        .as_deref()
        .is_some_and(|value| value.trim() == "danger_full_access")
        || value_declares_danger_full_access(&req.sdk)
        || value_declares_danger_full_access(&req.tool_manifest)
        || value_declares_danger_full_access(&req.local_context)
}

pub(super) fn value_declares_danger_full_access(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(value_declares_danger_full_access),
        Value::Object(object) => {
            for (key, child) in object {
                let key = key.as_str();
                if matches!(
                    key,
                    "runtime_permission" | "runtimePermission" | "permission"
                ) && child
                    .as_str()
                    .is_some_and(|value| value == "danger_full_access")
                {
                    return true;
                }
                if key == "dangerous" && child.as_bool() == Some(true) {
                    return true;
                }
                if value_declares_danger_full_access(child) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

pub(super) fn request_runtime_route(req: &ExternalAppRouteCChatRequest) -> RuntimeRoute {
    req.runtime_route
        .as_deref()
        .and_then(parse_runtime_route)
        .or_else(|| value_runtime_route(&req.sdk))
        .or_else(|| value_runtime_route(&req.client))
        .or_else(|| value_runtime_route(&req.local_context))
        .unwrap_or(RuntimeRoute::RouteC)
}

pub(super) fn value_runtime_route(value: &Value) -> Option<RuntimeRoute> {
    match value {
        Value::String(text) => parse_runtime_route(text),
        Value::Array(items) => items.iter().find_map(value_runtime_route),
        Value::Object(object) => {
            for key in [
                "runtime_route",
                "runtimeRoute",
                "route",
                "route_id",
                "routeId",
                "project_ai_route",
                "projectAiRoute",
            ] {
                if let Some(route) = object
                    .get(key)
                    .and_then(Value::as_str)
                    .and_then(parse_runtime_route)
                {
                    return Some(route);
                }
            }
            object.values().find_map(value_runtime_route)
        }
        _ => None,
    }
}

pub(super) fn parse_runtime_route(raw: &str) -> Option<RuntimeRoute> {
    let normalized = raw
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_")
        .replace(' ', "_");
    match normalized.as_str() {
        "a" | "route_a" | "local_cli" | "local_ai_cli" | "codex_cli" | "copilot_cli"
        | "claude_cli" => Some(RuntimeRoute::RouteA),
        "b" | "route_b" | "byok" | "bring_your_own_key" | "local_api_key" | "user_api_key" => {
            Some(RuntimeRoute::RouteB)
        }
        "c" | "route_c" | "elon_server" | "server_model" | "server_model_local_tools" => {
            Some(RuntimeRoute::RouteC)
        }
        _ => None,
    }
}

pub(super) fn build_messages(
    app_id: &str,
    user_message: &str,
    req: &ExternalAppRouteCChatRequest,
    allowed_tools: &BTreeSet<String>,
    max_actions: usize,
    danger_full_access: bool,
    runtime_route: RuntimeRoute,
) -> Vec<Value> {
    let mut messages = vec![json!({
        "role": "system",
        "content": system_prompt(
            app_id,
            allowed_tools,
            max_actions,
            danger_full_access,
            runtime_route
        )
    })];
    messages.extend(
        req.history
            .iter()
            .filter_map(history_message)
            .take(MAX_HISTORY_ITEMS),
    );
    messages.push(json!({
        "role": "user",
        "content": build_user_prompt(app_id, user_message, req, runtime_route)
    }));
    messages
}

pub(super) fn system_prompt(
    app_id: &str,
    allowed_tools: &BTreeSet<String>,
    max_actions: usize,
    danger_full_access: bool,
    runtime_route: RuntimeRoute,
) -> String {
    let app_rule = if app_id == "bb64a" {
        "这是 ElonSpeed / BB64A 代理软件的用户支持场景。优先判断本机配置、节点/订阅、Windows 代理/TUN/路由和产品 bug 的区别。不要输出订阅 URL、token、节点密码或无关本地文件内容。"
    } else {
        "这是外部子项目接入一龙 Route C SDK 的用户支持场景。根据子项目上下文和本地工具结果回答，不要编造上下文没有的事实。"
    };
    let tool_list = if allowed_tools.is_empty() {
        "当前没有可调用本地工具。".to_string()
    } else {
        format!(
            "只能调用这些本地工具：{}。",
            allowed_tools.iter().cloned().collect::<Vec<_>>().join(", ")
        )
    };
    let permission_rule = if danger_full_access {
        "当前 SDK 声明 runtime_permission=danger_full_access。若 manifest 提供 run_command/read_file/write_file/list_dir，你可以请求用户本机完整命令行与文件读写：run_command 可使用 {\"command\":\"...\",\"shell\":\"cmd|powershell|pwsh|bash|sh\",\"cwd\":\"...\"}，也可使用 {\"program\":\"cmd\",\"args\":[\"/C\",\"...\"]}；路径可以是绝对路径。"
    } else {
        "当前 SDK 未声明 danger_full_access。只能按 manifest 暴露的业务工具做本地诊断，不要假设可以执行任意命令或读写任意文件。"
    };
    format!(
        "你是一龙 Project AI SDK 协调模型，兼容 Route A / Route B / Route C 三种接入。\n\
         {route_rule}\n\
         {app_rule}\n\
         {tool_list}\n\
         {permission_rule}\n\
         远程源码节点协作：如果本机诊断结果指向产品源码、功能设计或适配缺口，并且 manifest 暴露 remote_source_search、remote_source_read_file 或 remote_source_ask，应先调用这些远程源码工具补充项目源码细节，再判断是不是源码问题。\n\
         需求频道反馈：如果源码结果确认或高度疑似是产品 bug/功能缺口，并且 manifest 暴露 create_feedback_post，请请求该工具创建需求频道帖子；args 至少包含 title、user_problem、local_evidence、source_findings、issue_type、suggested_next_step。\n\
         你不能声称已经执行尚未收到结果的工具。收到 tool_results 后，必须先基于结果继续判断。\n\
         如果需要本地工具，最多返回 {max_actions} 个 actions；如果不需要工具，actions 必须为空。\n\
         只输出一个 JSON 对象，不要 Markdown，不要代码块。格式：\n\
         {{\"reply\":\"给用户看的中文回复\",\"done\":true,\"actions\":[{{\"tool\":\"工具名\",\"args\":{{}},\"reason\":\"为什么需要这个工具\"}}]}}\n\
         done=false 表示 SDK 应先执行 actions 后再次请求；done=true 表示本轮可以直接展示最终回复。",
        route_rule = runtime_route.prompt_rule()
    )
}

pub(super) fn build_user_prompt(
    app_id: &str,
    user_message: &str,
    req: &ExternalAppRouteCChatRequest,
    runtime_route: RuntimeRoute,
) -> String {
    let client_json = compact_json(&req.client, MAX_JSON_CONTEXT_CHARS / 5);
    let context_json = compact_json(&req.local_context, MAX_JSON_CONTEXT_CHARS);
    let manifest_json = compact_json(&req.tool_manifest, MAX_TOOL_MANIFEST_CHARS);
    let tool_results_json = compact_json(
        &Value::Array(
            req.tool_results
                .iter()
                .take(MAX_TOOL_RESULTS)
                .map(|value| truncate_tool_result(value, MAX_TOOL_RESULT_CHARS))
                .collect(),
        ),
        MAX_JSON_CONTEXT_CHARS,
    );
    let sdk_json = compact_json(&req.sdk, MAX_JSON_CONTEXT_CHARS / 8);
    let runtime_permission = req
        .runtime_permission
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("client_managed_tools");
    format!(
        "app_id: {app_id}\n\n\
         runtime_route：\n{runtime_route}\n\n\
         用户问题：\n{user_message}\n\n\
         runtime_permission：\n{runtime_permission}\n\n\
         客户端信息：\n{client_json}\n\n\
         SDK 信息：\n{sdk_json}\n\n\
         本地上下文：\n{context_json}\n\n\
         本地工具 manifest：\n{manifest_json}\n\n\
         已执行工具结果 tool_results：\n{tool_results_json}\n\n\
         请按 Project AI SDK JSON 格式返回下一步。",
        runtime_route = runtime_route.as_str()
    )
}

pub(super) fn history_message(message: &RouteCChatMessage) -> Option<Value> {
    let role = match message.role.trim() {
        "user" => "user",
        "assistant" => "assistant",
        _ => return None,
    };
    let content = normalize_text(&message.content, MAX_HISTORY_CONTENT_CHARS);
    if content.is_empty() {
        return None;
    }
    Some(json!({ "role": role, "content": content }))
}

pub(super) fn extract_content(value: &Value) -> Option<String> {
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|text| normalize_text(text, 16_000))
        .filter(|text| !text.is_empty())
}

pub(super) fn parse_model_output(raw: &str) -> RouteCModelOutput {
    let Some(value) = parse_json_object(raw) else {
        return RouteCModelOutput {
            reply: normalize_text(raw, 12_000),
            actions: Vec::new(),
        };
    };
    let reply = value
        .get("reply")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .map(|text| normalize_text(text, 12_000))
        .unwrap_or_default();
    let actions = value
        .get("actions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(action_from_value)
                .collect::<Vec<RouteCAction>>()
        })
        .unwrap_or_default();
    RouteCModelOutput { reply, actions }
}

pub(super) fn parse_json_object(raw: &str) -> Option<Value> {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if value.is_object() {
            return Some(value);
        }
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end <= start {
        return None;
    }
    let value = serde_json::from_str::<Value>(&trimmed[start..=end]).ok()?;
    value.is_object().then_some(value)
}

pub(super) fn action_from_value(value: &Value) -> Option<RouteCAction> {
    let tool = value
        .get("tool")
        .or_else(|| value.get("name"))
        .and_then(Value::as_str)
        .and_then(normalize_tool_name)?;
    let id = value
        .get("id")
        .or_else(|| value.get("tool_call_id"))
        .and_then(Value::as_str)
        .map(|value| normalize_text(value, 80))
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    let args = value
        .get("args")
        .or_else(|| value.get("arguments"))
        .cloned()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    let reason = value
        .get("reason")
        .and_then(Value::as_str)
        .map(|value| normalize_text(value, 240))
        .filter(|value| !value.is_empty());
    let dangerous = value
        .get("dangerous")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some(RouteCAction {
        id,
        tool,
        args,
        reason,
        dangerous,
    })
}

pub(super) fn sanitize_actions(
    actions: Vec<RouteCAction>,
    allowed_tools: &BTreeSet<String>,
    max_actions: usize,
) -> Vec<RouteCAction> {
    if allowed_tools.is_empty() {
        return Vec::new();
    }
    let mut seen = BTreeSet::new();
    let mut sanitized = Vec::new();
    for (index, mut action) in actions.into_iter().enumerate() {
        if !allowed_tools.contains(&action.tool) {
            continue;
        }
        if !seen.insert(action.tool.clone()) {
            continue;
        }
        if action.id.is_empty() {
            action.id = format!("tool_{}", index + 1);
        }
        if !action.args.is_object() {
            action.args = json!({});
        }
        sanitized.push(action);
        if sanitized.len() >= max_actions {
            break;
        }
    }
    sanitized
}

pub(super) fn allowed_tool_names(
    app_id: &str,
    manifest: &Value,
    local_context: &Value,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    collect_tool_names(manifest, &mut names);
    if names.is_empty() {
        collect_tool_names(local_context, &mut names);
    }
    if names.is_empty() && app_id == "bb64a" {
        names.extend(
            [
                "bb64a_doctor",
                "get_status",
                "test_google",
                "detect_conflicts",
                "get_logs_tail",
                "get_system_proxy_status",
                "get_process_tcp",
            ]
            .into_iter()
            .map(str::to_string),
        );
    }
    names
}

pub(super) fn collect_tool_names(value: &Value, names: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => {
            if let Some(name) = normalize_tool_name(text) {
                names.insert(name);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_tool_names(item, names);
            }
        }
        Value::Object(object) => {
            for key in [
                "name",
                "tool",
                "id",
                "tool_id",
                "toolId",
                "tool_name",
                "toolName",
            ] {
                if let Some(name) = object
                    .get(key)
                    .and_then(Value::as_str)
                    .and_then(normalize_tool_name)
                {
                    names.insert(name);
                }
            }
            for key in [
                "tools",
                "tool_ids",
                "toolIds",
                "tools_available",
                "toolsAvailable",
                "chat_auto_executable_tool_ids",
                "chatAutoExecutableToolIds",
                "project_ai_tools",
                "projectAiTools",
                "remote_source_tools",
                "remoteSourceTools",
                "feedback_tools",
                "feedbackTools",
            ] {
                if let Some(child) = object.get(key) {
                    collect_tool_names(child, names);
                }
            }
        }
        _ => {}
    }
}

pub(super) fn normalize_tool_name(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
    if normalized.is_empty()
        || normalized.len() > 80
        || !normalized
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
    {
        return None;
    }
    Some(normalized)
}

pub(super) fn truncate_tool_result(value: &Value, max_chars: usize) -> Value {
    match value {
        Value::String(text) => Value::String(normalize_text(text, max_chars)),
        Value::Object(object) => {
            let mut cloned = object.clone();
            for key in ["data", "result", "stdout", "stderr", "summary", "error"] {
                if let Some(item) = cloned.get_mut(key) {
                    *item = truncate_tool_result(item, max_chars / 2);
                }
            }
            Value::Object(cloned)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .take(20)
                .map(|item| truncate_tool_result(item, max_chars / 2))
                .collect(),
        ),
        _ => value.clone(),
    }
}

pub(super) fn compact_json(value: &Value, max_chars: usize) -> String {
    serde_json::to_string_pretty(value)
        .unwrap_or_else(|_| "null".to_string())
        .chars()
        .take(max_chars)
        .collect()
}

pub(super) fn normalize_text(value: &str, max_chars: usize) -> String {
    value
        .trim()
        .chars()
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}
