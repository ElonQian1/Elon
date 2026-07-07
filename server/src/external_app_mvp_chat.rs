//! Temporary external-app AI chat endpoint for early MVP integrations.
//!
//! This is intentionally thinner than the formal external app account/session
//! contract. It lets a child app send a user question plus local diagnostic
//! context to the main project's server-side model before the full SDK,
//! account bridge, and billing flow are wired in.

use anyhow::{anyhow, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    agent_fallback::{is_retryable_agent_error, server_api_agents_in_fallback_order},
    agent_llm_call::friendly_ai_api_error,
    external_app_registry::{external_app_by_id, public_external_app_config},
    project_auth::json_error,
    types::{AgentConfig, AppState},
};

const MAX_MESSAGE_CHARS: usize = 4_000;
const MAX_HISTORY_ITEMS: usize = 10;
const MAX_HISTORY_CONTENT_CHARS: usize = 2_000;
const MAX_JSON_CONTEXT_CHARS: usize = 16_000;
const MVP_MAX_OUTPUT_TOKENS: usize = 900;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExternalAppMvpChatRequest {
    #[serde(alias = "conversation_id")]
    conversation_id: Option<String>,
    message: String,
    #[serde(default)]
    history: Vec<ExternalAppMvpChatMessage>,
    #[serde(default)]
    client: Value,
    #[serde(default, alias = "local_context")]
    local_context: Value,
    agent: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ExternalAppMvpChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct SuggestedTool {
    tool: &'static str,
    reason: &'static str,
    dangerous: bool,
}

pub(crate) async fn mvp_chat_handler(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<ExternalAppMvpChatRequest>,
) -> Response {
    if !mvp_chat_enabled() {
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "外部应用 MVP AI 对话未启用，请设置 ELON_EXTERNAL_APP_MVP_CHAT_ENABLED=true",
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

    let messages = build_messages(app.id, &message, &req);
    let (reply, agent_used, model_used, agent_fallback) =
        match call_mvp_chat_model(&state, req.agent.as_deref(), &messages).await {
            Ok(result) => result,
            Err(error) => {
                tracing::warn!(
                    target: "external_app_mvp_chat",
                    app_id = app.id,
                    error = %error,
                    "MVP external app chat failed"
                );
                return json_error(StatusCode::BAD_GATEWAY, format!("AI 回复失败：{error}"));
            }
        };

    let suggested_tools = suggest_tools(app.id, &message, &req.local_context);
    tracing::info!(
        target: "external_app_mvp_chat",
        app_id = app.id,
        message_chars = message.chars().count(),
        context_chars = json_chars(&req.local_context),
        suggested_tool_count = suggested_tools.len(),
        agent = %agent_used,
        model = %model_used,
        "external app MVP chat completed"
    );

    Json(json!({
        "ok": true,
        "schema": "external_app.mvp_chat.v1",
        "app": public_external_app_config(app),
        "conversation_id": req.conversation_id,
        "reply": reply,
        "suggested_tools": suggested_tools,
        "agent_used": agent_used,
        "model_used": model_used,
        "agent_fallback": agent_fallback,
        "billing": {
            "mode": "mvp_unmetered",
            "note": "临时 MVP 接口暂不绑定外部账号计费；正式版应改为 accounts/session token。"
        }
    }))
    .into_response()
}


#[path = "external_app_mvp_chat_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
