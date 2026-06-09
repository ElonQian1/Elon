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
    store::{
        CHAT_MEMORY_PROJECT_NAME, MEMORY_SCOPE_CHAT, MEMORY_SCOPE_PHONE_CONTROL,
        PHONE_CONTROL_PROJECT_NAME,
    },
    types::AppState,
    user_memory_extract::extract_and_save_memories_scoped,
};

#[derive(Deserialize)]
pub struct LmChatRequest {
    /// OpenAI 格式的消息数组，如 [{role:"system",content:"..."},{role:"user",content:"..."}]
    pub messages: Vec<Value>,
    /// 可选：指定使用哪个 agent（model）
    pub agent: Option<String>,
    /// 可选：会话 ID；为空时服务器自动生成新会话
    pub conversation_id: Option<String>,
    /// 可选：聊天归档作用域。默认 phone_control；普通聊天传 chat_memory。
    pub scope: Option<String>,
}

#[derive(Clone, Copy)]
enum LmChatScope {
    PhoneControl,
    ChatMemory,
}

impl LmChatScope {
    fn from_request(value: Option<&str>) -> Self {
        match value.unwrap_or("").trim().to_ascii_lowercase().as_str() {
            "chat" | "chat_memory" | "normal_chat" | "ordinary_chat" => Self::ChatMemory,
            _ => Self::PhoneControl,
        }
    }

    fn wire_name(self) -> &'static str {
        match self {
            Self::PhoneControl => "phone_control",
            Self::ChatMemory => "chat_memory",
        }
    }

    fn project_name(self) -> &'static str {
        match self {
            Self::PhoneControl => PHONE_CONTROL_PROJECT_NAME,
            Self::ChatMemory => CHAT_MEMORY_PROJECT_NAME,
        }
    }

    fn memory_scope(self) -> &'static str {
        match self {
            Self::PhoneControl => MEMORY_SCOPE_PHONE_CONTROL,
            Self::ChatMemory => MEMORY_SCOPE_CHAT,
        }
    }

    fn conversation_title(self) -> &'static str {
        match self {
            Self::PhoneControl => "悬浮球语音会话",
            Self::ChatMemory => "普通聊天会话",
        }
    }
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

    let chat_scope = LmChatScope::from_request(req.scope.as_deref());
    let (project_id, _created) = match chat_scope {
        LmChatScope::PhoneControl => state.store.ensure_balloon_project_for_user(&user.id),
        LmChatScope::ChatMemory => state.store.ensure_chat_memory_project_for_user(&user.id),
    }
    .unwrap_or_else(|e| {
        tracing::warn!(
            "确保 LLM 聊天系统项目失败 user={} scope={}: {e}",
            user.id,
            chat_scope.wire_name()
        );
        (String::new(), false)
    });
    if project_id.is_empty() {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "创建聊天归档项目失败");
    }

    // ── 1. 确保会话存在 ───────────────────────────────────────────────────────
    let conversation_id = state
        .store
        .ensure_conversation(
            &project_id,
            &user.id,
            req.conversation_id.as_deref(),
            Some(chat_scope.conversation_title()),
        )
        .unwrap_or_else(|_| {
            req.conversation_id
                .clone()
                .unwrap_or_else(|| "default".into())
        });

    // ── 2. 把用户长期记忆注入 system prompt ──────────────────────────────────
    let memories = state
        .store
        .get_user_memories_for_scope(&user.id, chat_scope.memory_scope(), Some(&project_id), 15)
        .unwrap_or_default();
    let mut messages = req.messages.clone();
    if !memories.is_empty() {
        let memory_block = memories
            .iter()
            .map(|m| format!("- {}", m.content))
            .collect::<Vec<_>>()
            .join("\n");
        let memory_note =
            format!("\n\n=== 关于这位用户的长期记忆（仅供参考，勿对用户提及）===\n{memory_block}");
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

    // ── 3. 注入近期会话历史（短期记忆，最近 6 条）────────────────────────────
    // 只在最后一条用户消息前面插入历史，让 LLM 有上下文。
    // 意图分析 / 脚本生成的调用通常只有一条 user 消息，历史注入对它们无害。
    let history = state
        .store
        .list_recent_conversation_messages(&project_id, Some(&conversation_id), 6)
        .unwrap_or_default();
    if !history.is_empty() {
        // 找到第一条 user/assistant 消息的插入位置（system prompt 之后）
        let insert_at = messages
            .iter()
            .position(|m| m["role"].as_str() != Some("system"))
            .unwrap_or(messages.len());
        for (i, h) in history.iter().enumerate() {
            messages.insert(insert_at + i, json!({"role": h.role, "content": h.content}));
        }
    }

    // ── 4. 调用 LLM ──────────────────────────────────────────────────────────
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

    // ── 5. 保存消息到会话记录 ────────────────────────────────────────────────
    let user_msg = req
        .messages
        .iter()
        .filter(|m| m["role"].as_str() == Some("user"))
        .last()
        .and_then(|m| m["content"].as_str())
        .unwrap_or("")
        .to_string();

    let _ = state.store.add_message(
        &project_id,
        Some(&conversation_id),
        None,
        Some(&user.id),
        "user",
        &user_msg,
    );
    let _ = state.store.add_message(
        &project_id,
        Some(&conversation_id),
        None,
        None,
        "assistant",
        &reply,
    );

    // ── 6. 异步提取长期记忆 ──────────────────────────────────────────────────
    if !user_msg.is_empty() && !reply.is_empty() {
        let state2 = state.clone();
        let uid = user.id.clone();
        let umsg = user_msg.clone();
        let rep = reply.clone();
        let scope_type = chat_scope.memory_scope().to_string();
        let scope_id = Some(project_id.clone());
        let source_conv_id = Some(conversation_id.clone());
        tokio::spawn(async move {
            extract_and_save_memories_scoped(
                state2,
                uid,
                umsg,
                rep,
                scope_type,
                scope_id,
                source_conv_id,
            )
            .await;
        });
    }

    Json(json!({
        "reply": reply,
        "conversation_id": conversation_id,
        "project_id": project_id,
        "project_name": chat_scope.project_name(),
        "scope": chat_scope.wire_name(),
    }))
    .into_response()
}
