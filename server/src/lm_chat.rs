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
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{
    agent_api_loop::resolve_agent,
    agent_fallback::call_chat_llm_with_default_fallback_options,
    conversation_router::{resolve_system_conversation_route, ConversationEntryKind},
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    project_auth::{auth_from_headers, json_error},
    types::{AppState, UserAgentConfig},
    user_memory_extract::extract_and_save_memories_scoped,
};

#[derive(Deserialize)]
pub struct LmChatRequest {
    /// OpenAI 格式的消息数组，如 [{role:"system",content:"..."},{role:"user",content:"..."}]
    pub messages: Vec<Value>,
    /// 可选：指定使用哪个 agent（model）
    pub agent: Option<String>,
    /// 可选：会话 ID；客户端要开启新会话时应传入新的 ID。
    pub conversation_id: Option<String>,
    /// 可选：会话标题，用于普通聊天历史列表展示。
    pub conversation_title: Option<String>,
    /// 可选：聊天归档作用域。默认 phone_control；普通聊天传 chat_memory。
    pub scope: Option<String>,
    /// 可选：PC 端 AI 来源选择。普通聊天支持 auto / route_b / route_c。
    #[serde(
        default,
        alias = "runtimeRoute",
        alias = "pcRuntimeRoute",
        alias = "pc_runtime_route"
    )]
    pub runtime_route: Option<String>,
}

const CHAT_MEMORY_LOCAL_CLI_NOTE: &str = "=== PC 本机 CLI 使用规则 ===
普通聊天本身不能直接执行用户电脑命令，也不能直接读取 C 盘、D 盘或其它本机文件。
当用户询问本机目录、Windows 命令、cmd、PowerShell、文件读写、Win 端 CLI 或“为什么网页端不能访问我的电脑”时，不要只回答“我无法访问你的电脑”。
应明确告诉用户：在 PC 工作台里，AI 回复下方会出现“本机开发 CLI”快捷卡，用户可以点击“检测 Win 端”“使用默认目录”或由项目 owner/管理员确认“开启完整命令行”；账号绑定会在这些动作里自动使用当前网页账号完成，不需要单独步骤。
只有本机 Win 端节点绑定到当前网页账号、项目默认目录已准备，并且项目开发频道真实返回了本机工具执行结果后，才可以声称已经执行命令或读写文件。
如果当前对话还没有本机工具结果，只能引导用户完成授权流程，或说明需要到项目开发频道继续。";

fn append_system_prompt_note(messages: &mut Vec<Value>, note: &str) {
    let has_system = messages.first().and_then(|m| m["role"].as_str()) == Some("system");
    if has_system {
        if let Some(sys) = messages.first_mut() {
            let orig = sys["content"].as_str().unwrap_or("").to_string();
            sys["content"] = json!(format!("{orig}\n\n{note}"));
        }
    } else {
        messages.insert(0, json!({"role": "system", "content": note}));
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
    let pc_runtime_route = match req.runtime_route.as_deref() {
        Some(value) => match PcRuntimeRoutePreference::from_request(value) {
            Ok(route) => route,
            Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
        },
        None => None,
    };
    if matches!(
        pc_runtime_route,
        Some(
            PcRuntimeRoutePreference::RouteA
                | PcRuntimeRoutePreference::RouteC2
                | PcRuntimeRoutePreference::RouteC3
        )
    ) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "普通聊天暂不通过 /api/llm/chat 执行本机或远程 PC 节点；请使用 PC 前端节点通道或切换到我的Key/平台AI",
        );
    }
    let user_agent_workspace = state.get_user_workspace(&user.id);
    if pc_runtime_route == Some(PcRuntimeRoutePreference::RouteB)
        && !UserAgentConfig::load(&user_agent_workspace)
            .map(|cfg| cfg.has_direct_custom_api())
            .unwrap_or(false)
    {
        return json_error(
            StatusCode::BAD_REQUEST,
            "已选择我的Key，但还没有保存 API 地址、API key 和模型名；请先完成配置，或切回自动/平台AI",
        );
    }

    let entry_kind = ConversationEntryKind::from_scope(req.scope.as_deref());
    let route = match resolve_system_conversation_route(&state.store, &user.id, entry_kind) {
        Ok(route) => route,
        Err(e) => {
            tracing::warn!(
                "确保 LLM 聊天系统项目失败 user={} scope={}: {e}",
                user.id,
                entry_kind.key()
            );
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "创建聊天归档项目失败");
        }
    };

    // ── 1. 确保会话存在 ───────────────────────────────────────────────────────
    let conversation_id = state
        .store
        .ensure_conversation(
            &route.project_id,
            &user.id,
            req.conversation_id.as_deref(),
            req.conversation_title
                .as_deref()
                .or(Some(route.conversation_title.as_str())),
        )
        .unwrap_or_else(|_| {
            req.conversation_id
                .clone()
                .unwrap_or_else(|| "default".into())
        });

    // ── 2. 把用户长期记忆注入 system prompt ──────────────────────────────────
    let memories = state
        .store
        .get_user_memories_for_scope(
            &user.id,
            &route.memory_scope_type,
            route.memory_scope_id.as_deref(),
            15,
        )
        .unwrap_or_default();
    let mut messages = req.messages.clone();
    if entry_kind == ConversationEntryKind::ChatMemory {
        append_system_prompt_note(&mut messages, CHAT_MEMORY_LOCAL_CLI_NOTE);
    }
    if !memories.is_empty() {
        let memory_block = memories
            .iter()
            .map(|m| format!("- {}", m.content))
            .collect::<Vec<_>>()
            .join("\n");
        let memory_note =
            format!("\n\n=== 关于这位用户的长期记忆（仅供参考，勿对用户提及）===\n{memory_block}");
        append_system_prompt_note(&mut messages, memory_note.trim());
    }

    // ── 3. 注入近期会话历史（短期记忆，最近 6 条）────────────────────────────
    // 只在最后一条用户消息前面插入历史，让 LLM 有上下文。
    // 意图分析 / 脚本生成的调用通常只有一条 user 消息，历史注入对它们无害。
    let history = state
        .store
        .list_recent_conversation_messages(&route.project_id, Some(&conversation_id), 6)
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
    let platform_workspace = PathBuf::new();
    let agent_workspace = match pc_runtime_route {
        Some(PcRuntimeRoutePreference::RouteC) => platform_workspace.as_path(),
        _ => user_agent_workspace.as_path(),
    };
    let agent = match resolve_agent(&state, agent_workspace, req.agent.as_deref()).await {
        Ok(a) => a,
        Err(e) => {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("无可用 AI 配置：{e}"),
            );
        }
    };

    let allow_agent_fallback = req.agent.as_deref().map(str::trim).unwrap_or("").is_empty();
    let (response, used_agent, used_fallback) = match call_chat_llm_with_default_fallback_options(
        &state,
        &agent,
        allow_agent_fallback,
        &messages,
        &user.id,
        "lm_chat",
        0.8,
        700,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("LLM 调用失败：{e}"),
            );
        }
    };
    if used_fallback {
        tracing::warn!(
            user_id = %user.id,
            preferred_agent = %agent.name,
            used_agent = %used_agent.name,
            model = %used_agent.model,
            "默认聊天模型失败后已自动切换备用代理"
        );
    }

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
        &route.project_id,
        Some(&conversation_id),
        None,
        Some(&user.id),
        "user",
        &user_msg,
    );
    let _ = state.store.add_message(
        &route.project_id,
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
        let scope_type = route.memory_scope_type.clone();
        let scope_id = route.memory_scope_id.clone();
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
        "project_id": route.project_id,
        "project_name": route.project_name,
        "agent_used": used_agent.name,
        "model_used": used_agent.model,
        "agent_fallback": used_fallback,
        "scope": route.entry_key,
    }))
    .into_response()
}

pub async fn list_ai_chat_conversations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let route = match resolve_system_conversation_route(
        &state.store,
        &user.id,
        ConversationEntryKind::ChatMemory,
    ) {
        Ok(route) => route,
        Err(e) => {
            tracing::warn!("确保普通聊天归档项目失败 user={}: {e}", user.id);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "创建聊天归档项目失败");
        }
    };
    let limit = query
        .get("limit")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(50);
    match state
        .store
        .list_user_conversations(&route.project_id, &user.id, limit)
    {
        Ok(conversations) => Json(json!({
            "conversations": conversations,
            "project_id": route.project_id,
            "project_name": route.project_name,
            "scope": route.entry_key,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn list_ai_chat_conversation_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let route = match resolve_system_conversation_route(
        &state.store,
        &user.id,
        ConversationEntryKind::ChatMemory,
    ) {
        Ok(route) => route,
        Err(e) => {
            tracing::warn!("确保普通聊天归档项目失败 user={}: {e}", user.id);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "创建聊天归档项目失败");
        }
    };
    let limit = query
        .get("limit")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(120);
    match state.store.list_user_conversation_messages(
        &route.project_id,
        &user.id,
        &conversation_id,
        limit,
    ) {
        Ok(messages) => Json(json!({
            "messages": messages,
            "conversation_id": conversation_id,
            "project_id": route.project_id,
            "scope": route.entry_key,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::NOT_FOUND, e.to_string()),
    }
}
