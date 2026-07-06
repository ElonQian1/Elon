//! Route C SDK MVP endpoint for external apps.
//!
//! The server calls the model. The child app SDK keeps tool execution local and
//! sends tool results back on the next request. This is intentionally separate
//! from the existing `mvp-chat` endpoint so early clients can migrate gradually.

use anyhow::{anyhow, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeSet, sync::Arc};

use crate::{
    agent_fallback::{is_retryable_agent_error, server_api_agents_in_fallback_order},
    agent_llm_call::friendly_ai_api_error,
    external_app_registry::{external_app_by_id, public_external_app_config},
    project_auth::json_error,
    types::{AgentConfig, AppState},
};

const SDK_SCHEMA: &str = "external_app.route_c_chat.v0";
const MAX_MESSAGE_CHARS: usize = 4_000;
const MAX_HISTORY_ITEMS: usize = 10;
const MAX_HISTORY_CONTENT_CHARS: usize = 2_000;
const MAX_JSON_CONTEXT_CHARS: usize = 18_000;
const MAX_TOOL_RESULTS: usize = 8;
const MAX_TOOL_RESULT_CHARS: usize = 12_000;
const MAX_TOOL_MANIFEST_CHARS: usize = 12_000;
const MAX_OUTPUT_TOKENS: usize = 1_000;
const DEFAULT_MAX_ACTIONS: usize = 3;
const HARD_MAX_ACTIONS: usize = 5;
const PROJECT_AI_SCHEMA: &str = "elon.project_ai_sdk.mvp.v0";
const PROJECT_AI_SUPPORTED_ROUTES: [&str; 3] = ["route_a", "route_b", "route_c"];
const PROJECT_AI_REMOTE_SOURCE_TOOLS: [&str; 3] = [
    "remote_source_search",
    "remote_source_read_file",
    "remote_source_ask",
];
const PROJECT_AI_FEEDBACK_TOOLS: [&str; 1] = ["create_feedback_post"];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExternalAppRouteCChatRequest {
    #[serde(default, alias = "conversation_id")]
    conversation_id: Option<String>,
    message: String,
    #[serde(default)]
    history: Vec<RouteCChatMessage>,
    #[serde(default)]
    client: Value,
    #[serde(default, alias = "local_context")]
    local_context: Value,
    #[serde(default, alias = "tool_manifest")]
    tool_manifest: Value,
    #[serde(default, alias = "tool_results")]
    tool_results: Vec<Value>,
    #[serde(default)]
    sdk: Value,
    #[serde(default, alias = "runtime_permission")]
    runtime_permission: Option<String>,
    #[serde(default, alias = "runtime_route")]
    runtime_route: Option<String>,
    #[serde(default)]
    agent: Option<String>,
    #[serde(default, alias = "max_actions")]
    max_actions: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct RouteCChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteCAction {
    id: String,
    tool: String,
    #[serde(default)]
    args: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(default)]
    dangerous: bool,
}

#[derive(Debug, Default)]
struct RouteCModelOutput {
    reply: String,
    actions: Vec<RouteCAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeRoute {
    RouteA,
    RouteB,
    RouteC,
}

impl RuntimeRoute {
    fn as_str(self) -> &'static str {
        match self {
            RuntimeRoute::RouteA => "route_a",
            RuntimeRoute::RouteB => "route_b",
            RuntimeRoute::RouteC => "route_c",
        }
    }

    fn prompt_rule(self) -> &'static str {
        match self {
            RuntimeRoute::RouteA => {
                "当前是 Route A：用户本机已有 Codex/Copilot/Claude 等 CLI 接管子项目，模型和工具主要在用户电脑侧；一龙 SDK 负责提供统一工具协议、远程源码查询和反馈帖子契约。"
            }
            RuntimeRoute::RouteB => {
                "当前是 Route B：用户在本机配置自己的 API key，模型调用由用户侧完成，本机工具仍走一龙 SDK；一龙 SDK 负责统一工具协议、远程源码查询和反馈帖子契约。"
            }
            RuntimeRoute::RouteC => {
                "当前是 Route C：模型调用走一龙服务器，本地文件、命令、诊断和业务工具由用户设备上的 SDK 执行。"
            }
        }
    }
}

pub(crate) async fn route_c_chat_handler(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<ExternalAppRouteCChatRequest>,
) -> Response {
    if !route_c_sdk_enabled() {
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "外部应用 Route C SDK MVP 未启用，请设置 ELON_EXTERNAL_APP_ROUTE_C_SDK_ENABLED=true",
        );
    }

    let app_id = app_id.trim().to_ascii_lowercase();
    let Some(app) = external_app_by_id(&app_id) else {
        return json_error(StatusCode::NOT_FOUND, format!("未知外部应用：{app_id}"));
    };

    let message = normalize_text(&req.message, MAX_MESSAGE_CHARS);
    if message.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "message 不能为空");
    }

    let allowed_tools = allowed_tool_names(app.id, &req.tool_manifest, &req.local_context);
    let danger_full_access = request_danger_full_access(&req);
    let runtime_route = request_runtime_route(&req);
    let max_actions = req
        .max_actions
        .unwrap_or(DEFAULT_MAX_ACTIONS)
        .clamp(1, HARD_MAX_ACTIONS);
    let messages = build_messages(
        app.id,
        &message,
        &req,
        &allowed_tools,
        max_actions,
        danger_full_access,
        runtime_route,
    );

    let (raw_model_output, agent_used, model_used, agent_fallback) =
        match call_route_c_model(&state, req.agent.as_deref(), &messages).await {
            Ok(result) => result,
            Err(error) => {
                tracing::warn!(
                    target: "external_app_route_c_sdk",
                    app_id = app.id,
                    error = %error,
                    "Route C SDK chat failed"
                );
                return json_error(StatusCode::BAD_GATEWAY, format!("AI 回复失败：{error}"));
            }
        };

    let mut model_output = parse_model_output(&raw_model_output);
    model_output.actions = sanitize_actions(model_output.actions, &allowed_tools, max_actions);
    if model_output.reply.is_empty() {
        model_output.reply = if model_output.actions.is_empty() {
            "我已完成判断。".to_string()
        } else {
            "我需要先调用本地工具继续判断。".to_string()
        };
    }
    let done = model_output.actions.is_empty();

    tracing::info!(
        target: "external_app_route_c_sdk",
        app_id = app.id,
        message_chars = message.chars().count(),
        tool_count = allowed_tools.len(),
        action_count = model_output.actions.len(),
        danger_full_access = danger_full_access,
        runtime_route = runtime_route.as_str(),
        agent = %agent_used,
        model = %model_used,
        "external app Route C SDK chat completed"
    );

    Json(json!({
        "ok": true,
        "schema": SDK_SCHEMA,
        "app": public_external_app_config(app),
        "conversation_id": req.conversation_id,
        "reply": model_output.reply,
        "done": done,
        "actions": model_output.actions,
        "agent_used": agent_used,
        "model_used": model_used,
        "agent_fallback": agent_fallback,
        "project_ai": {
            "schema": PROJECT_AI_SCHEMA,
            "runtime_route": runtime_route.as_str(),
            "supported_routes": PROJECT_AI_SUPPORTED_ROUTES,
            "local_execution": "external_app_sdk",
            "remote_source_tools": PROJECT_AI_REMOTE_SOURCE_TOOLS,
            "feedback_tools": PROJECT_AI_FEEDBACK_TOOLS,
            "note": "MVP 阶段只定义通用工具协议；工具实际执行仍由子项目 SDK/provider 在用户本机或项目节点完成。"
        },
        "route_c": {
            "mode": "server_model_local_tools",
            "runtime_route": runtime_route.as_str(),
            "local_execution": "external_app_sdk",
            "approval": "client_managed_mvp_disabled",
            "tool_filter": "manifest_allowlist",
            "runtime_permission": if danger_full_access { "danger_full_access" } else { "client_managed_tools" }
        },
        "sdk": {
            "version": "0.1.0",
            "asset": "/assets/elon_route_c_sdk.js"
        },
        "billing": {
            "mode": "mvp_unmetered",
            "note": "临时 Route C SDK MVP 暂不绑定外部账号计费；正式版应改为 accounts/session token。"
        }
    }))
    .into_response()
}

fn route_c_sdk_enabled() -> bool {
    env_flag("ELON_EXTERNAL_APP_ROUTE_C_SDK_ENABLED")
        || env_flag("ELON_EXTERNAL_APP_MVP_CHAT_ENABLED")
}

fn env_flag(name: &str) -> bool {
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

fn request_danger_full_access(req: &ExternalAppRouteCChatRequest) -> bool {
    req.runtime_permission
        .as_deref()
        .is_some_and(|value| value.trim() == "danger_full_access")
        || value_declares_danger_full_access(&req.sdk)
        || value_declares_danger_full_access(&req.tool_manifest)
        || value_declares_danger_full_access(&req.local_context)
}

fn value_declares_danger_full_access(value: &Value) -> bool {
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

fn request_runtime_route(req: &ExternalAppRouteCChatRequest) -> RuntimeRoute {
    req.runtime_route
        .as_deref()
        .and_then(parse_runtime_route)
        .or_else(|| value_runtime_route(&req.sdk))
        .or_else(|| value_runtime_route(&req.client))
        .or_else(|| value_runtime_route(&req.local_context))
        .unwrap_or(RuntimeRoute::RouteC)
}

fn value_runtime_route(value: &Value) -> Option<RuntimeRoute> {
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

fn parse_runtime_route(raw: &str) -> Option<RuntimeRoute> {
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

fn build_messages(
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

fn system_prompt(
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

fn build_user_prompt(
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

fn history_message(message: &RouteCChatMessage) -> Option<Value> {
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

async fn call_route_c_model(
    state: &Arc<AppState>,
    requested_agent: Option<&str>,
    messages: &[Value],
) -> Result<(String, String, String, bool)> {
    let agents = candidate_agents(state, requested_agent).await?;
    let mut last_retryable_error = None;
    for (index, agent) in agents.iter().enumerate() {
        match send_chat_completion(state, agent, messages).await {
            Ok(response) => {
                let content = extract_content(&response).unwrap_or_else(|| {
                    "{\"reply\":\"我已收到问题，但模型没有返回可展示的文本。\",\"done\":true,\"actions\":[]}"
                        .to_string()
                });
                return Ok((content, agent.name.clone(), agent.model.clone(), index > 0));
            }
            Err(error) => {
                let message = error.to_string();
                let has_next = index + 1 < agents.len();
                if !is_retryable_agent_error(&message) || !has_next {
                    return Err(anyhow!(message));
                }
                last_retryable_error = Some(message);
            }
        }
    }
    Err(anyhow!(
        "{}",
        last_retryable_error.unwrap_or_else(|| "未配置可用 server_api_key AI 代理".to_string())
    ))
}

async fn candidate_agents(
    state: &Arc<AppState>,
    requested_agent: Option<&str>,
) -> Result<Vec<AgentConfig>> {
    let agents = server_api_agents_in_fallback_order(state).await;
    if agents.is_empty() {
        return Err(anyhow!("未配置可用 server_api_key AI 代理"));
    }
    let Some(requested) = requested_agent
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(agents);
    };
    let Some(index) = agents
        .iter()
        .position(|agent| agent.name.eq_ignore_ascii_case(requested))
    else {
        return Err(anyhow!("请求的 AI 代理未开放给 Route C SDK MVP"));
    };
    let mut ordered = vec![agents[index].clone()];
    ordered.extend(agents.into_iter().enumerate().filter_map(|(i, agent)| {
        if i == index {
            None
        } else {
            Some(agent)
        }
    }));
    Ok(ordered)
}

async fn send_chat_completion(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    messages: &[Value],
) -> Result<Value> {
    let url = format!("{}/chat/completions", agent.api_base.trim_end_matches('/'));
    let body = json!({
        "model": agent.model,
        "messages": messages,
        "stream": false,
        "temperature": 0.2,
        "max_tokens": MAX_OUTPUT_TOKENS,
    });
    let response = state
        .http_client
        .post(url)
        .bearer_auth(&agent.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!("AI 请求超时，请稍后重试")
            } else {
                anyhow!("AI 请求失败: {error}")
            }
        })?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("{}", friendly_ai_api_error(status, &text)));
    }
    Ok(response.json::<Value>().await?)
}

fn extract_content(value: &Value) -> Option<String> {
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|text| normalize_text(text, 16_000))
        .filter(|text| !text.is_empty())
}

fn parse_model_output(raw: &str) -> RouteCModelOutput {
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

fn parse_json_object(raw: &str) -> Option<Value> {
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

fn action_from_value(value: &Value) -> Option<RouteCAction> {
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

fn sanitize_actions(
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

fn allowed_tool_names(app_id: &str, manifest: &Value, local_context: &Value) -> BTreeSet<String> {
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

fn collect_tool_names(value: &Value, names: &mut BTreeSet<String>) {
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

fn normalize_tool_name(raw: &str) -> Option<String> {
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

fn truncate_tool_result(value: &Value, max_chars: usize) -> Value {
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

fn compact_json(value: &Value, max_chars: usize) -> String {
    serde_json::to_string_pretty(value)
        .unwrap_or_else(|_| "null".to_string())
        .chars()
        .take(max_chars)
        .collect()
}

fn normalize_text(value: &str, max_chars: usize) -> String {
    value
        .trim()
        .chars()
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}


#[cfg(test)]
#[path = "external_app_route_c_sdk_tests.rs"]
mod tests;
