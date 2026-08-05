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
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::StreamExt;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::mpsc;

use crate::{
    agent_api_loop::resolve_agent,
    agent_fallback::{
        call_chat_llm_with_default_fallback_options, start_chat_llm_stream_with_default_fallback,
    },
    conversation_router::{resolve_system_conversation_route, ConversationEntryKind},
    home_ai_orchestrator, home_ai_tools,
    lm_chat_prompt::{append_system_prompt_note, CHAT_MEMORY_LOCAL_CLI_NOTE},
    lm_chat_request::LmChatRequest,
    lm_chat_stream_support::{send_stream_error, send_stream_event, stream_response},
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    project_auth::{auth_from_headers, json_error},
    types::{AppState, UserAgentConfig},
    user_memory_extract::extract_and_save_memories_scoped,
};

pub(crate) use crate::lm_chat_history::{
    list_ai_chat_conversation_messages, list_ai_chat_conversations,
};

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
        append_system_prompt_note(
            &mut messages,
            &home_ai_tools::runtime_note(home_ai_tools::now()),
        );
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

    // ── 3. 注入近期会话历史（短期记忆，最近 30 条）───────────────────────────
    // 只在最后一条用户消息前面插入历史，让 LLM 有上下文。
    // 意图分析 / 脚本生成的调用通常只有一条 user 消息，历史注入对它们无害。
    let history = state
        .store
        .list_recent_conversation_messages(&route.project_id, Some(&conversation_id), 30)
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

    // ── 4. 总 AI 基础能力与项目任务分流 ─────────────────────────────────────
    let user_msg = req
        .messages
        .iter()
        .filter(|m| m["role"].as_str() == Some("user"))
        .last()
        .and_then(|m| m["content"].as_str())
        .unwrap_or("")
        .to_string();
    let handoff = if entry_kind == ConversationEntryKind::ChatMemory
        && home_ai_tools::needs_project_handoff(&user_msg)
    {
        let candidates = state
            .store
            .list_projects_for_user(&user.id)
            .map(|projects| home_ai_tools::project_candidates(&user_msg, &projects))
            .unwrap_or_default();
        Some(json!({
            "request": user_msg,
            "reason": "这是需要读取或修改项目代码的任务，应交给具体项目 AI 执行。",
            "candidates": candidates,
        }))
    } else {
        None
    };

    let deterministic = if handoff.is_none() {
        home_ai_tools::deterministic_answer(&user_msg, home_ai_tools::now())
    } else {
        None
    };
    let mut orchestrated_sources = Vec::new();
    let (reply, used_agent_name, used_model, used_fallback, assistant_mode, tool_used) =
        if let Some(answer) = deterministic {
            (
                answer.reply,
                "总 AI".to_string(),
                "deterministic".to_string(),
                false,
                "deterministic".to_string(),
                Some(answer.tool.to_string()),
            )
        } else if let Some(handoff) = &handoff {
            let has_candidates = handoff["candidates"]
                .as_array()
                .map(|items| !items.is_empty())
                .unwrap_or(false);
            (
                if has_candidates {
                    "我识别到这是一个需要直接操作项目代码的任务。请选择下面的项目，我会把原始需求交给对应的项目 AI 继续执行。"
                } else {
                    "我识别到这是一个需要直接操作项目代码的任务，但当前没有找到可用项目。请先创建或加入项目，再从项目 AI 入口继续。"
                }
                .to_string(),
                "总 AI".to_string(),
                "handoff".to_string(),
                false,
                "handoff".to_string(),
                Some("project_handoff".to_string()),
            )
        } else {
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
            if entry_kind == ConversationEntryKind::ChatMemory {
                match home_ai_orchestrator::run(
                    &state,
                    &agent,
                    allow_agent_fallback,
                    &messages,
                    &user.id,
                    &history,
                )
                .await
                {
                    Ok(answer) => {
                        orchestrated_sources = answer.sources;
                        (
                            answer.reply,
                            answer.agent_name,
                            answer.model,
                            answer.used_fallback,
                            "model".to_string(),
                            answer.tool_used,
                        )
                    }
                    Err(error) => {
                        tracing::warn!(
                            user_id = %user.id,
                            error = %error,
                            "首页工具编排不可用，回退到普通聊天"
                        );
                        match regular_home_chat(
                            &state,
                            &agent,
                            allow_agent_fallback,
                            &messages,
                            &user.id,
                        )
                        .await
                        {
                            Ok(result) => result,
                            Err(error) => {
                                return json_error(
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    format!("LLM 调用失败：{error}"),
                                );
                            }
                        }
                    }
                }
            } else {
                match regular_home_chat(&state, &agent, allow_agent_fallback, &messages, &user.id)
                    .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        return json_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("LLM 调用失败：{error}"),
                        );
                    }
                }
            }
        };

    // ── 5. 保存消息到会话记录 ────────────────────────────────────────────────
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
        let rep = reply.to_string();
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
        "agent_used": used_agent_name,
        "model_used": used_model,
        "agent_fallback": used_fallback,
        "scope": route.entry_key,
        "assistant_mode": assistant_mode,
        "tool_used": tool_used,
        "sources": orchestrated_sources,
        "handoff": handoff,
    }))
    .into_response()
}

async fn regular_home_chat(
    state: &Arc<AppState>,
    agent: &crate::types::AgentConfig,
    allow_fallback: bool,
    messages: &[Value],
    user_id: &str,
) -> anyhow::Result<(String, String, String, bool, String, Option<String>)> {
    let (response, used_agent, used_fallback) = call_chat_llm_with_default_fallback_options(
        state,
        agent,
        allow_fallback,
        messages,
        user_id,
        "lm_chat",
        0.8,
        900,
    )
    .await?;
    if used_fallback {
        tracing::warn!(
            user_id = %user_id,
            preferred_agent = %agent.name,
            used_agent = %used_agent.name,
            model = %used_agent.model,
            "默认聊天模型失败后已自动切换备用代理"
        );
    }
    Ok((
        response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string(),
        used_agent.name,
        used_agent.model,
        used_fallback,
        "model".to_string(),
        None,
    ))
}

/// POST /api/llm/chat/stream — 首页 AI 的流式兼容入口。
pub async fn lm_chat_stream_handler(
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
            "普通聊天暂不通过流式接口执行本机或远程 PC 节点；请使用 PC 前端节点通道或切换到我的Key/平台AI",
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

    let (tx, rx) = mpsc::channel(32);
    let user_id = user.id.clone();
    tokio::spawn(async move {
        run_lm_chat_stream(
            state,
            user_id,
            req,
            pc_runtime_route,
            user_agent_workspace,
            tx,
        )
        .await;
    });
    stream_response(rx)
}

async fn persist_stream_chat_turn(
    state: &Arc<AppState>,
    project_id: &str,
    conversation_id: &str,
    user_id: &str,
    user_msg: &str,
    reply: &str,
    memory_scope_type: &str,
    memory_scope_id: Option<&str>,
) {
    let _ = state.store.add_message(
        project_id,
        Some(conversation_id),
        None,
        Some(user_id),
        "user",
        user_msg,
    );
    let _ = state.store.add_message(
        project_id,
        Some(conversation_id),
        None,
        None,
        "assistant",
        reply,
    );
    if user_msg.is_empty() || reply.is_empty() {
        return;
    }
    let state2 = state.clone();
    let uid = user_id.to_string();
    let umsg = user_msg.to_string();
    let rep = reply.to_string();
    let scope_type = memory_scope_type.to_string();
    let scope_id = memory_scope_id.map(str::to_string);
    let source_conv_id = Some(conversation_id.to_string());
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

async fn run_lm_chat_stream(
    state: Arc<AppState>,
    user_id: String,
    req: LmChatRequest,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    user_agent_workspace: PathBuf,
    tx: mpsc::Sender<String>,
) {
    let started_at = std::time::Instant::now();
    if !send_stream_event(
        &tx,
        json!({
            "type": "status",
            "phase": "preparing",
            "message": "正在准备回答…",
            "elapsed_ms": 0,
        }),
    )
    .await
    {
        return;
    }

    let entry_kind = ConversationEntryKind::from_scope(req.scope.as_deref());
    let route = match resolve_system_conversation_route(&state.store, &user_id, entry_kind) {
        Ok(route) => route,
        Err(e) => {
            send_stream_error(&tx, format!("创建聊天归档项目失败：{e}")).await;
            return;
        }
    };
    let conversation_id = state
        .store
        .ensure_conversation(
            &route.project_id,
            &user_id,
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
    let user_msg = req
        .messages
        .iter()
        .filter(|m| m["role"].as_str() == Some("user"))
        .last()
        .and_then(|m| m["content"].as_str())
        .unwrap_or("")
        .to_string();
    // 快速问题不读取历史、不调用模型，避免简单请求被慢链路拖住。
    let deterministic = if !home_ai_tools::needs_project_handoff(&user_msg) {
        home_ai_tools::deterministic_answer(&user_msg, home_ai_tools::now())
    } else {
        None
    };
    if let Some(answer) = deterministic {
        let reply = answer.reply;
        let tool = answer.tool;
        let _ = send_stream_event(
            &tx,
            json!({
                "type": "delta",
                "content": reply,
            }),
        )
        .await;
        persist_stream_chat_turn(
            &state,
            &route.project_id,
            &conversation_id,
            &user_id,
            &user_msg,
            &reply,
            &route.memory_scope_type,
            route.memory_scope_id.as_deref(),
        )
        .await;
        tracing::info!(
            feature = "lm_chat_stream",
            user_id = %user_id,
            mode = "deterministic",
            tool = %tool,
            elapsed_ms = started_at.elapsed().as_millis() as u64,
            "首页 AI 快速回答完成"
        );
        let _ = send_stream_event(
            &tx,
            json!({
                "type": "done",
                "conversation_id": conversation_id,
                "project_id": route.project_id,
                "project_name": route.project_name,
                "assistant_mode": "deterministic",
                "tool_used": tool,
                "sources": [],
                "handoff": Value::Null,
                "elapsed_ms": started_at.elapsed().as_millis(),
            }),
        )
        .await;
        return;
    }

    let handoff = if entry_kind == ConversationEntryKind::ChatMemory
        && home_ai_tools::needs_project_handoff(&user_msg)
    {
        let candidates = state
            .store
            .list_projects_for_user(&user_id)
            .map(|projects| home_ai_tools::project_candidates(&user_msg, &projects))
            .unwrap_or_default();
        Some(json!({
            "request": user_msg,
            "reason": "这是需要读取或修改项目代码的任务，应交给具体项目 AI 执行。",
            "candidates": candidates,
        }))
    } else {
        None
    };
    if let Some(handoff) = handoff {
        let has_candidates = handoff["candidates"]
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false);
        let reply = if has_candidates {
            "我识别到这是一个需要直接操作项目代码的任务。请选择下面的项目，我会把原始需求交给对应的项目 AI 继续执行。"
        } else {
            "我识别到这是一个需要直接操作项目代码的任务，但当前没有找到可用项目。请先创建或加入项目，再从项目 AI 入口继续。"
        };
        let _ = send_stream_event(&tx, json!({ "type": "delta", "content": reply })).await;
        let _ = send_stream_event(
            &tx,
            json!({ "type": "handoff", "handoff": handoff.clone() }),
        )
        .await;
        persist_stream_chat_turn(
            &state,
            &route.project_id,
            &conversation_id,
            &user_id,
            &user_msg,
            reply,
            &route.memory_scope_type,
            route.memory_scope_id.as_deref(),
        )
        .await;
        tracing::info!(
            feature = "lm_chat_stream",
            user_id = %user_id,
            mode = "handoff",
            elapsed_ms = started_at.elapsed().as_millis() as u64,
            "首页 AI 项目交接完成"
        );
        let _ = send_stream_event(
            &tx,
            json!({
                "type": "done",
                "conversation_id": conversation_id,
                "project_id": route.project_id,
                "project_name": route.project_name,
                "assistant_mode": "handoff",
                "tool_used": "project_handoff",
                "sources": [],
                "handoff": handoff,
                "elapsed_ms": started_at.elapsed().as_millis(),
            }),
        )
        .await;
        return;
    }

    let memories = state
        .store
        .get_user_memories_for_scope(
            &user_id,
            &route.memory_scope_type,
            route.memory_scope_id.as_deref(),
            15,
        )
        .unwrap_or_default();
    let mut messages = req.messages.clone();
    if entry_kind == ConversationEntryKind::ChatMemory {
        append_system_prompt_note(&mut messages, CHAT_MEMORY_LOCAL_CLI_NOTE);
        append_system_prompt_note(
            &mut messages,
            &home_ai_tools::runtime_note(home_ai_tools::now()),
        );
    }
    if !memories.is_empty() {
        let memory_block = memories
            .iter()
            .map(|m| format!("- {}", m.content))
            .collect::<Vec<_>>()
            .join("\n");
        append_system_prompt_note(
            &mut messages,
            &format!("=== 关于这位用户的长期记忆（仅供参考，勿对用户提及）===\n{memory_block}"),
        );
    }
    let history = state
        .store
        .list_recent_conversation_messages(&route.project_id, Some(&conversation_id), 30)
        .unwrap_or_default();
    if !history.is_empty() {
        let insert_at = messages
            .iter()
            .position(|m| m["role"].as_str() != Some("system"))
            .unwrap_or(messages.len());
        for (i, h) in history.iter().enumerate() {
            messages.insert(
                insert_at + i,
                json!({ "role": h.role, "content": h.content }),
            );
        }
    }
    if !send_stream_event(
        &tx,
        json!({
            "type": "status",
            "phase": "model",
            "message": "正在理解问题并准备回答…",
            "elapsed_ms": started_at.elapsed().as_millis(),
        }),
    )
    .await
    {
        return;
    }
    let platform_workspace = PathBuf::new();
    let agent_workspace = match pc_runtime_route {
        Some(PcRuntimeRoutePreference::RouteC) => platform_workspace.as_path(),
        _ => user_agent_workspace.as_path(),
    };
    let agent = match resolve_agent(&state, agent_workspace, req.agent.as_deref()).await {
        Ok(agent) => agent,
        Err(e) => {
            send_stream_error(&tx, format!("无可用 AI 配置：{e}")).await;
            return;
        }
    };
    let allow_agent_fallback = req.agent.as_deref().map(str::trim).unwrap_or("").is_empty();
    if entry_kind == ConversationEntryKind::ChatMemory {
        let _ = send_stream_event(
            &tx,
            json!({
                "type": "status",
                "phase": "capability",
                "message": "正在判断是否需要使用信息工具…",
                "elapsed_ms": started_at.elapsed().as_millis(),
            }),
        )
        .await;
        match home_ai_orchestrator::run(
            &state,
            &agent,
            allow_agent_fallback,
            &messages,
            &user_id,
            &history,
        )
        .await
        {
            Ok(answer) => {
                let reply = answer.reply;
                if !answer.sources.is_empty() {
                    let _ = send_stream_event(
                        &tx,
                        json!({ "type": "sources", "sources": answer.sources.clone() }),
                    )
                    .await;
                }
                let _ = send_stream_event(&tx, json!({ "type": "delta", "content": reply })).await;
                persist_stream_chat_turn(
                    &state,
                    &route.project_id,
                    &conversation_id,
                    &user_id,
                    &user_msg,
                    &reply,
                    &route.memory_scope_type,
                    route.memory_scope_id.as_deref(),
                )
                .await;
                let _ = send_stream_event(
                    &tx,
                    json!({
                        "type": "done",
                        "reply": reply,
                        "conversation_id": conversation_id,
                        "project_id": route.project_id,
                        "project_name": route.project_name,
                        "assistant_mode": "model",
                        "agent_used": answer.agent_name,
                        "model_used": answer.model,
                        "agent_fallback": answer.used_fallback,
                        "tool_used": answer.tool_used,
                        "sources": answer.sources,
                        "handoff": Value::Null,
                        "elapsed_ms": started_at.elapsed().as_millis(),
                    }),
                )
                .await;
                return;
            }
            Err(error) => {
                tracing::warn!(
                    user_id = %user_id,
                    error = %error,
                    "首页流式工具编排不可用，回退到普通流式聊天"
                );
            }
        }
    }
    let (response, used_agent, used_fallback) = match start_chat_llm_stream_with_default_fallback(
        &state,
        &agent,
        allow_agent_fallback,
        &messages,
        &user_id,
        0.8,
        900,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            send_stream_error(&tx, format!("LLM 调用失败：{e}")).await;
            return;
        }
    };

    let upstream_content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if !upstream_content_type.contains("text/event-stream") {
        let payload = match response.json::<Value>().await {
            Ok(payload) => payload,
            Err(e) => {
                send_stream_error(&tx, format!("AI 返回格式无法解析：{e}")).await;
                return;
            }
        };
        let reply = payload["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();
        if reply.is_empty() {
            send_stream_error(&tx, "AI 没有返回可显示的内容").await;
            return;
        }
        let _ = send_stream_event(&tx, json!({ "type": "delta", "content": reply })).await;
        persist_stream_chat_turn(
            &state,
            &route.project_id,
            &conversation_id,
            &user_id,
            &user_msg,
            &reply,
            &route.memory_scope_type,
            route.memory_scope_id.as_deref(),
        )
        .await;
        tracing::info!(
            feature = "lm_chat_stream",
            user_id = %user_id,
            mode = "model_non_stream_fallback",
            agent = %used_agent.name,
            model = %used_agent.model,
            fallback = used_fallback,
            elapsed_ms = started_at.elapsed().as_millis() as u64,
            "首页 AI 使用非流式兼容降级"
        );
        let _ = send_stream_event(
            &tx,
            json!({
                "type": "done",
                "reply": reply,
                "conversation_id": conversation_id,
                "project_id": route.project_id,
                "project_name": route.project_name,
                "agent_used": used_agent.name,
                "model_used": used_agent.model,
                "agent_fallback": used_fallback,
                "assistant_mode": "model",
                "tool_used": "",
                "sources": [],
                "handoff": Value::Null,
                "elapsed_ms": started_at.elapsed().as_millis(),
            }),
        )
        .await;
        return;
    }

    let mut upstream = response.bytes_stream();
    let mut buffer = String::new();
    let mut reply = String::new();
    let mut client_connected = true;
    while let Some(chunk) = upstream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                send_stream_error(&tx, format!("AI 流式响应中断：{e}")).await;
                return;
            }
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(position) = buffer.find('\n') {
            let line = buffer[..position].trim_end_matches('\r').to_string();
            buffer.drain(..=position);
            let Some(data) = line.strip_prefix("data:").map(str::trim) else {
                continue;
            };
            if data == "[DONE]" {
                continue;
            }
            let Ok(payload) = serde_json::from_str::<Value>(data) else {
                continue;
            };
            let delta = payload["choices"][0]["delta"]["content"]
                .as_str()
                .or_else(|| payload["choices"][0]["message"]["content"].as_str())
                .unwrap_or("");
            if delta.is_empty() {
                continue;
            }
            reply.push_str(delta);
            if !send_stream_event(
                &tx,
                json!({
                    "type": "delta",
                    "content": delta,
                }),
            )
            .await
            {
                client_connected = false;
                break;
            }
        }
        if !client_connected {
            break;
        }
    }
    if reply.trim().is_empty() {
        send_stream_error(&tx, "AI 没有返回可显示的内容").await;
        return;
    }
    persist_stream_chat_turn(
        &state,
        &route.project_id,
        &conversation_id,
        &user_id,
        &user_msg,
        &reply,
        &route.memory_scope_type,
        route.memory_scope_id.as_deref(),
    )
    .await;
    tracing::info!(
        feature = "lm_chat_stream",
        user_id = %user_id,
        mode = "model",
        agent = %used_agent.name,
        model = %used_agent.model,
        fallback = used_fallback,
        reply_chars = reply.chars().count(),
        elapsed_ms = started_at.elapsed().as_millis() as u64,
        "首页 AI 流式回答完成"
    );
    if client_connected {
        let _ = send_stream_event(
            &tx,
            json!({
                "type": "done",
                "reply": reply,
                "conversation_id": conversation_id,
                "project_id": route.project_id,
                "project_name": route.project_name,
                "agent_used": used_agent.name,
                "model_used": used_agent.model,
                "agent_fallback": used_fallback,
                "assistant_mode": "model",
                "tool_used": "",
                "sources": [],
                "handoff": Value::Null,
                "elapsed_ms": started_at.elapsed().as_millis(),
            }),
        )
        .await;
    }
}
