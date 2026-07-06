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


mod helpers;
use self::helpers::*;
