//! POST /api/llm/chat — 通用 LLM 代理（带记忆+会话）
//!
//! 把 messages 转发给服务器 LLM，同时：
//!   - 把用户长期记忆注入首条 system 消息（如无 system 则自动添加）
//!   - 把本轮消息写入 conversations 表，供聊天区查看历史
//!   - 对话结束后异步提取新记忆写入 user_memories 表
//!
//! 适用场景（悬浮球 agent 子系统）：
//!   - 闲聊对话（携带自定义 ASSISTANT_PERSONA system prompt）
//!   - 手机自动化脚本生成（携带严格 JSON 格式 system prompt）
//!   - 意图分析（携带意图分类 prompt）

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    agent_api_loop::resolve_agent,
    agent_llm_call::call_chat_llm,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
    user_memory_extract::extract_and_save_memories,
};

/// 悬浮球专用项目 ID（用于隔离会话记录和脚本历史，与主聊天区项目不混用）
const AGENT_BALLOON_PROJECT_ID: &str = "__agent_balloon__";

#[derive(Deserialize)]
pub struct LmChatRequest {
    /// OpenAI 格式的消息数组，如 [{role:"system",content:"..."},{role:"user",content:"..."}]
    pub messages: Vec<Value>,
    /// 可选：指定使用哪个 agent（model）
    pub agent: Option<String>,
    /// 可选：会话 ID；为空时服务器自动生成新会话
    pub conversation_id: Option<String>,
}

/// POST /api/llm/chat
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

    // ── 1. 确保会话存在 ───────────────────────────────────────────────────────
    let conversation_id = state
        .store
        .ensure_conversation(
            AGENT_BALLOON_PROJECT_ID,
            &user.id,
            req.conversation_id.as_deref(),
            Some("悬浮球语音会话"),
        )
        .unwrap_or_else(|_| req.conversation_id.clone().unwrap_or_else(|| "default".into()));

    // ── 2. 把用户长期记忆注入 system prompt ──────────────────────────────────
    let memories = state.store.get_user_memories(&user.id, 15).unwrap_or_default();
    let mut messages = req.messages.clone();
    if !memories.is_empty() {
        let memory_block = memories
            .iter()
            .map(|m| format!("- {}", m.content))
            .collect::<Vec<_>>()
            .join("\n");
        let memory_note = format!(
            "\n\n=== 关于这位用户的长期记忆（仅供参考，勿对用户提及）===\n{memory_block}"
        );
        // 注入到第一条 system 消息，没有则在最前面插入
        let has_system = messages.first().and_then(|m| m["role"].as_str()) == Some("system");
        if has_system {
            if let Some(sys) = messages.first_mut() {
                let orig = sys["content"].as_str().unwrap_or("").to_string();
                sys["content"] = json!(format!("{orig}{memory_note}"));
            }
        } else {
            messages.insert(0, json!({"role": "system", "content": memory_note.trim()}));
        }
    }

    // ── 3. 调用 LLM ──────────────────────────────────────────────────────────
    let agent = match resolve_agent(&state, std::path::Path::new(""), req.agent.as_deref()).await {
        Ok(a) => a,
        Err(e) => {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("无可用 AI 配置：{e}"),
            );
        }
    };

    let response = match call_chat_llm(&state, &agent, &messages, &user.id, "lm_chat").await {
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

    // ── 4. 保存消息到会话记录 ────────────────────────────────────────────────
    let user_msg = req
        .messages
        .iter()
        .filter(|m| m["role"].as_str() == Some("user"))
        .last()
        .and_then(|m| m["content"].as_str())
        .unwrap_or("")
        .to_string();

    let _ = state.store.add_message(
        AGENT_BALLOON_PROJECT_ID,
        Some(&conversation_id),
        None,
        Some(&user.id),
        "user",
        &user_msg,
    );
    let _ = state.store.add_message(
        AGENT_BALLOON_PROJECT_ID,
        Some(&conversation_id),
        None,
        None,
        "assistant",
        &reply,
    );

    // ── 5. 异步提取长期记忆 ──────────────────────────────────────────────────
    if !user_msg.is_empty() && !reply.is_empty() {
        let state2 = state.clone();
        let uid = user.id.clone();
        let umsg = user_msg.clone();
        let rep = reply.clone();
        tokio::spawn(async move {
            extract_and_save_memories(state2, uid, umsg, rep).await;
        });
    }

    Json(json!({
        "reply": reply,
        "conversation_id": conversation_id,
    }))
    .into_response()
}
