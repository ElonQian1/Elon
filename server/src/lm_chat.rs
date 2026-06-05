//! POST /api/llm/chat — 轻量 LLM 代理
//!
//! 把 messages 数组直接转发给服务器配置的 LLM（不做意图分类、不启动 Codex、
//! 不覆盖 system prompt），并返回模型的原始回复文本。
//!
//! 适用场景（悬浮球 agent 子系统）：
//!   - 闲聊对话（携带自定义 ASSISTANT_PERSONA system prompt）
//!   - 手机自动化脚本生成（携带严格 JSON 格式 system prompt）
//!   - 意图分析（携带意图分类 prompt）
//!
//! 与 /api/projects/{id}/chat 的区别：
//!   - 不需要 project_id，不需要项目权限
//!   - 不触发 Codex 开发工作流
//!   - system prompt 由客户端完整控制
//!   - 同步返回，适合短请求（< 10s）

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};

use crate::{
    agent_api_loop::resolve_agent,
    agent_llm_call::call_chat_llm,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

#[derive(Deserialize)]
pub struct LmChatRequest {
    /// OpenAI 格式的消息数组，如 [{role:"system",content:"..."},{role:"user",content:"..."}]
    pub messages: Vec<Value>,
    /// 可选：指定使用哪个 agent（model），不传则用全局默认
    pub agent: Option<String>,
}

/// POST /api/llm/chat
///
/// 调用方自己控制 messages（含 system prompt），服务器只做鉴权 + LLM 转发。
pub async fn lm_chat_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<LmChatRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    if req.messages.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "messages 不能为空");
    }

    // 用伪 workspace 路径；resolve_agent 找不到用户配置会回退到全局默认 agent
    let dummy = Path::new("/tmp");
    let agent = match resolve_agent(&state, dummy, req.agent.as_deref()).await {
        Ok(a) => a,
        Err(e) => {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("无可用 AI 配置：{e}，请在服务器配置 AGENT_* 环境变量"),
            );
        }
    };

    let response =
        match call_chat_llm(&state, &agent, &req.messages, &user.id, "lm_chat").await {
            Ok(r) => r,
            Err(e) => {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("LLM 调用失败：{e}"),
                );
            }
        };

    let reply = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    Json(json!({ "reply": reply })).into_response()
}
