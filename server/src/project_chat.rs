// server/src/project_chat.rs

use axum::{
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::stream;
use std::convert::Infallible;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    agent, agent_intent,
    agent_routing::is_local_cli_option,
    ai_cli, intent_router,
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    project_attachment_notes::{
        append_project_attachment_notes, append_project_cli_attachment_artifacts,
    },
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    project_chat_executor::run_project_agent_in_execution_workspace,
    project_chat_pc_node::{
        acquire_pc_node_cli_permit, chat_billing_block, pc_node_cli_execution_progress_message,
        pc_node_fast_path_route, record_pc_node_cli_execution_granted, run_bill,
        should_auto_bind_local_node,
    },
    project_chat_reply::{append_nonempty_ws_text, chat_reply_after_intent_gate},
    project_conversation_workspace::{
        prepare_project_conversation_workspace, project_conversation_execution_key,
        project_shared_execution_key, ProjectConversationWorkspace,
    },
    project_execution_mode::ProjectExecutionMode,
    project_keys::clean_trace_id,
    project_keys::codex_prewarm_key,
    project_trace_events::record_server_message,
    project_workspace_recovery,
    project_ws_protocol::{ProjectAttachmentRef, ProjectChatRequest},
    store::{ProjectAccess, MEMORY_SCOPE_PROJECT},
    tools,
    types::{AppState, WsMessage},
    ui_design_tasks::{append_ui_design_task_context, resolve_ui_route_task},
};

pub async fn chat_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(project_id): AxumPath<String>,
    Json(req): Json<ProjectChatRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let mut project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    if !can_edit(&project.role) {
        return json_error(StatusCode::FORBIDDEN, "当前用户没有修改项目的权限");
    }
    let display_message = crate::project_attachment_notes::project_message_with_attachment_fallback(
        req.message.trim().to_string(),
        req.attachments.as_deref(),
    );
    if display_message.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "message 不能为空");
    }
    if looks_like_replaced_unicode_mojibake(&display_message) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "请求正文疑似发生字符编码损坏，中文已变成大量问号。请使用 UTF-8 发送 JSON；Windows 脚本建议使用 PowerShell 7，或先把 JSON 写成 UTF-8 文件后再用 curl.exe --data-binary @file 发送。",
        );
    }
    let pc_runtime_route = match req.pc_runtime_route() {
        Ok(route) => route,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };
    if should_auto_bind_local_node(pc_runtime_route) && !req.chat_only.unwrap_or(false) {
        if let (Some(node_id), Some(workspace_path)) = (
            req.local_node_id
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
            req.local_workspace_path
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
        ) {
            if project.node_id.as_deref() != Some(node_id)
                || project.workspace_path.as_deref() != Some(workspace_path)
            {
                match project_workspace_recovery::bind_existing_pc_workspace(
                    &state,
                    &user.id,
                    &project.id,
                    &project.role,
                    node_id,
                    workspace_path,
                )
                .await
                {
                    Ok(_) => {
                        if let Ok(updated) = project_access(&state, &user.id, &project_id) {
                            project = updated;
                        }
                    }
                    Err((status, message)) => return json_error(status, message),
                }
            }
        }
    }
    if let Some(msg) = chat_billing_block(&state, &user.id, &project, &req, pc_runtime_route) {
        return json_error(StatusCode::PAYMENT_REQUIRED, msg);
    }
    let conversation_id = match state.store.ensure_conversation(
        &project.id,
        &user.id,
        req.conversation_id.as_deref(),
        req.conversation_title.as_deref(),
    ) {
        Ok(id) => id,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let attachments = req.attachments.clone();
    let message = append_project_attachment_notes(
        &state,
        &project,
        &conversation_id,
        display_message.clone(),
        req.attachments.as_deref(),
    );
    let resolved_ui_route = resolve_ui_route_task(
        &state.store,
        &project.id,
        &display_message,
        req.ui_design_task.as_ref(),
        req.attachments.as_deref(),
    );
    let message = match append_ui_design_task_context(
        message,
        resolved_ui_route.task.as_ref(),
        req.attachments.as_deref(),
        !resolved_ui_route.suppress_inference,
    ) {
        Ok(message) => message,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };
    let trace_id = clean_trace_id(req.trace_id.as_deref());
    state.server_traces.record(
        &trace_id,
        "http_project_message_received",
        serde_json::json!({
            "project_id": &project.id,
            "user_id": &user.id,
            "conversation_id": &conversation_id,
            "message_chars": message.chars().count(),
            "ui_route_source": resolved_ui_route.source,
            "agent": req.agent.as_deref(),
            "pc_runtime_route": pc_runtime_route.map(|route| route.as_request_value()),
            "execution_mode": req.execution_mode.as_deref(),
            "plan_mode": req.plan_mode,
        }),
    );
    let project_id_for_history = project.id.clone();
    let original_user_message = display_message.clone();
    let task_id = match state.store.create_task_with_display_message(
        &project.id,
        &user.id,
        Some(&conversation_id),
        &message,
        &display_message,
    ) {
        Ok(id) => id,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let download_base = format!("{}/api/projects/{}/download", state.public_url, project.id);
    if req.chat_only.unwrap_or(false) {
        // 轻量对话：agent 子系统（悬浮球语音）借用服务器 AI 的对话能力，
        // 强制走 casual chat，绝不触发项目 Codex 工作流（避免误判开发任务而超时）。
        agent::run_chat_only_for_project(
            &user.id,
            &project,
            &download_base,
            Some(&conversation_id),
            &message,
            req.agent.as_deref(),
            None,
            Some(trace_id.as_str()),
            &state,
            tx,
        )
        .await;
    } else {
        run_project_agent_with_scheduler(
            state.clone(),
            user.id.clone(),
            project,
            download_base,
            conversation_id.clone(),
            message,
            req.project_icon_data_url,
            req.agent,
            attachments,
            ProjectExecutionMode::from_request(req.execution_mode.as_deref(), req.plan_mode),
            pc_runtime_route,
            req.direct_pc_cli.unwrap_or(false),
            None,
            Some(trace_id.clone()),
            tx,
        )
        .await;
    }
    let mut reply = String::new();
    let mut streamed_reply = String::new();
    let mut apk_url = None;
    let mut image_url = None;
    let mut error = None;
    while let Some(raw) = rx.recv().await {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
            record_server_message(&state, &trace_id, &value, raw.len());
            match value.get("type").and_then(|t| t.as_str()) {
                Some("assistant_message") => {
                    append_nonempty_ws_text(&mut streamed_reply, value["text"].as_str());
                }
                Some("assistant_chunk") => {
                    append_nonempty_ws_text(&mut streamed_reply, value["text"].as_str());
                }
                Some("done") => {
                    let message = value["message"].as_str().unwrap_or_default().trim();
                    reply = if message.is_empty() {
                        ai_cli::truncate_chars(streamed_reply.trim(), 12_000)
                    } else {
                        message.to_string()
                    };
                    apk_url = value["apk_url"].as_str().map(ToOwned::to_owned);
                    image_url = value["image_url"].as_str().map(ToOwned::to_owned);
                }
                Some("error") => {
                    let msg = value["message"].as_str().unwrap_or("发生错误").to_string();
                    reply = msg.clone();
                    error = Some(msg);
                }
                _ => {}
            }
        }
    }
    if reply.is_empty() && error.is_none() && !streamed_reply.trim().is_empty() {
        reply = ai_cli::truncate_chars(streamed_reply.trim(), 12_000);
    }

    let status = if error.is_some() { "failed" } else { "done" };
    let _ = state.store.finish_task(
        &task_id,
        status,
        Some(&reply),
        apk_url.as_deref(),
        error.as_deref(),
    );

    // create_task 已经写入本轮 user 消息；这里只补写 assistant，供历史记录和记忆提取使用。
    // 仅在有实质回复时写入，避免记录错误/空响应污染历史。
    if !reply.is_empty() && error.is_none() {
        if !original_user_message.trim().is_empty() {
            let _ = state.store.add_message(
                &project_id_for_history,
                Some(&conversation_id),
                Some(&task_id),
                None,
                "assistant",
                &reply,
            );
            // 异步提取长期记忆（不阻塞响应）
            {
                let state2 = state.clone();
                let uid = user.id.clone();
                let umsg = original_user_message.clone();
                let rep = reply.clone();
                let scope_id = Some(project_id_for_history.clone());
                let source_conv_id = Some(conversation_id.clone());
                tokio::spawn(async move {
                    crate::user_memory_extract::extract_and_save_memories_scoped(
                        state2,
                        uid,
                        umsg,
                        rep,
                        MEMORY_SCOPE_PROJECT.to_string(),
                        scope_id,
                        source_conv_id,
                    )
                    .await;
                });
            }
        }
    }
    state.server_traces.record(
        &trace_id,
        if error.is_some() {
            "http_project_task_failed"
        } else {
            "http_project_task_done"
        },
        serde_json::json!({
            "task_id": &task_id,
            "status": status,
            "has_apk_url": apk_url.is_some(),
            "has_image_url": image_url.is_some(),
        }),
    );
    Json(serde_json::json!({
        "task_id": task_id,
        "trace_id": trace_id,
        "conversation_id": conversation_id,
        "reply": reply,
        "apk_url": apk_url,
        "image_url": image_url,
    }))
    .into_response()
}

pub async fn recall_project_conversation_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath((project_id, conversation_id, message_id)): AxumPath<(String, String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    if !can_edit(&project.role) {
        return json_error(StatusCode::FORBIDDEN, "当前用户没有修改项目的权限");
    }
    match state.store.recall_user_conversation_message(
        &project.id,
        &user.id,
        &conversation_id,
        &message_id,
    ) {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub use crate::project_prewarm::{prewarm_project, prewarm_user_project};

/// POST /api/projects/:project_id/chat/stream
/// 与 chat_project 逻辑相同，但通过 SSE 实时推送 WsMessage 进度事件，
/// 让悬浮球等轻量客户端能看到 AI 处理进度，不必阻塞等待。
pub async fn chat_project_stream(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(project_id): AxumPath<String>,
    Json(req): Json<ProjectChatRequest>,
) -> Response {
    use axum::response::sse::{Event, Sse};
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    if !can_edit(&project.role) {
        return json_error(StatusCode::FORBIDDEN, "当前用户没有修改项目的权限");
    }
    let display_message = crate::project_attachment_notes::project_message_with_attachment_fallback(
        req.message.trim().to_string(),
        req.attachments.as_deref(),
    );
    if display_message.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "message 不能为空");
    }
    if looks_like_replaced_unicode_mojibake(&display_message) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "请求正文疑似发生字符编码损坏，中文已变成大量问号。请使用 UTF-8 发送 JSON；Windows 脚本建议使用 PowerShell 7，或先把 JSON 写成 UTF-8 文件后再用 curl.exe --data-binary @file 发送。",
        );
    }
    let pc_runtime_route = match req.pc_runtime_route() {
        Ok(route) => route,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };
    if let Some(msg) = chat_billing_block(&state, &user.id, &project, &req, pc_runtime_route) {
        return json_error(StatusCode::PAYMENT_REQUIRED, msg);
    }
    let conversation_id = match state.store.ensure_conversation(
        &project.id,
        &user.id,
        req.conversation_id.as_deref(),
        req.conversation_title.as_deref(),
    ) {
        Ok(id) => id,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let attachments = req.attachments.clone();
    let message = append_project_attachment_notes(
        &state,
        &project,
        &conversation_id,
        display_message.clone(),
        req.attachments.as_deref(),
    );
    let resolved_ui_route = resolve_ui_route_task(
        &state.store,
        &project.id,
        &display_message,
        req.ui_design_task.as_ref(),
        req.attachments.as_deref(),
    );
    let message = match append_ui_design_task_context(
        message,
        resolved_ui_route.task.as_ref(),
        req.attachments.as_deref(),
        !resolved_ui_route.suppress_inference,
    ) {
        Ok(message) => message,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };
    let trace_id = clean_trace_id(req.trace_id.as_deref());
    let task_id = match state.store.create_task_with_display_message(
        &project.id,
        &user.id,
        Some(&conversation_id),
        &message,
        &display_message,
    ) {
        Ok(id) => id,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    run_project_agent_with_scheduler(
        state.clone(),
        user.id.clone(),
        project,
        format!("{}/api/projects/{}/download", state.public_url, project_id),
        conversation_id,
        message,
        req.project_icon_data_url,
        req.agent,
        attachments,
        ProjectExecutionMode::from_request(req.execution_mode.as_deref(), req.plan_mode),
        pc_runtime_route,
        req.direct_pc_cli.unwrap_or(false),
        None,
        Some(trace_id),
        tx,
    )
    .await;

    // 把 mpsc receiver 转成 SSE stream。
    // 每条 WsMessage JSON 推一次 data event；channel 关闭时 stream 自然结束。
    let sse_stream = stream::unfold(
        (rx, state, task_id),
        |(mut rx, state, task_id)| async move {
            match rx.recv().await {
                None => None,
                Some(raw) => {
                    // done/error 时更新 task 状态（幂等，重复调用无害）
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                        match v.get("type").and_then(|t| t.as_str()) {
                            Some("done") => {
                                let reply = v["message"].as_str().unwrap_or("");
                                let _ = state.store.finish_task(
                                    &task_id,
                                    "done",
                                    Some(reply),
                                    None,
                                    None,
                                );
                            }
                            Some("error") => {
                                let msg = v["message"].as_str().unwrap_or("error");
                                let _ = state.store.finish_task(
                                    &task_id,
                                    "failed",
                                    Some(msg),
                                    None,
                                    Some(msg),
                                );
                            }
                            _ => {}
                        }
                    }
                    let event = Ok::<Event, Infallible>(Event::default().data(raw));
                    Some((event, (rx, state, task_id)))
                }
            }
        },
    );

    Sse::new(sse_stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response()
}

pub(crate) use crate::project_chat_runner::{
    looks_like_replaced_unicode_mojibake, run_project_agent_with_scheduler,
};
