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
    agent_routing::{is_local_cli_option, quick_casual_reply},
    ai_cli, billing, intent_router,
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    project_attachment_notes::{
        append_project_attachment_notes, append_project_cli_attachment_artifacts,
    },
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    project_chat_executor::run_project_agent_in_execution_workspace,
    project_chat_reply::chat_reply_after_intent_gate,
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
    let message = req.message.trim().to_string();
    if message.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "message 不能为空");
    }
    if looks_like_replaced_unicode_mojibake(&message) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "请求正文疑似发生字符编码损坏，中文已变成大量问号。请使用 UTF-8 发送 JSON；Windows 脚本建议使用 PowerShell 7，或先把 JSON 写成 UTF-8 文件后再用 curl.exe --data-binary @file 发送。",
        );
    }
    if let Err(msg) = billing::check_can_call(&state.store, &user.id) {
        return json_error(StatusCode::PAYMENT_REQUIRED, msg);
    }
    let pc_runtime_route = match req.pc_runtime_route() {
        Ok(route) => route,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };
    let skip_auto_bind_for_casual_chat =
        req.chat_only.unwrap_or(false) || quick_casual_reply(&message).is_some();
    if should_auto_bind_local_node(pc_runtime_route) && !skip_auto_bind_for_casual_chat {
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
        message,
        req.attachments.as_deref(),
    );
    let trace_id = clean_trace_id(req.trace_id.as_deref());
    state.server_traces.record(
        &trace_id,
        "http_project_message_received",
        serde_json::json!({
            "project_id": &project.id,
            "user_id": &user.id,
            "conversation_id": &conversation_id,
            "message_chars": message.chars().count(),
            "agent": req.agent.as_deref(),
            "pc_runtime_route": pc_runtime_route.map(|route| route.as_request_value()),
            "execution_mode": req.execution_mode.as_deref(),
            "plan_mode": req.plan_mode,
        }),
    );

    // 提前保存 project_id 和原始消息，因为后面 project / message 会被 move 进调度器。
    let project_id_for_history = project.id.clone();
    let original_user_message = message.clone();

    let task_id =
        match state
            .store
            .create_task(&project.id, &user.id, Some(&conversation_id), &message)
        {
            Ok(id) => id,
            Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let download_base = format!("{}/api/projects/{}/download", state.public_url, project.id);
    if req.chat_only.unwrap_or(false) {
        // 轻量对话：agent 子系统（悬浮球语音）借用服务器 AI 的对话能力，
        // 强制走 casual chat，绝不触发项目 Codex 工作流（避免误判开发任务而超时）。
        agent::run_for_project(
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

fn should_auto_bind_local_node(route: Option<PcRuntimeRoutePreference>) -> bool {
    !matches!(
        route,
        Some(PcRuntimeRoutePreference::RouteC2 | PcRuntimeRoutePreference::RouteC3)
    )
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
    let message = req.message.trim().to_string();
    if message.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "message 不能为空");
    }
    if looks_like_replaced_unicode_mojibake(&message) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "请求正文疑似发生字符编码损坏，中文已变成大量问号。请使用 UTF-8 发送 JSON；Windows 脚本建议使用 PowerShell 7，或先把 JSON 写成 UTF-8 文件后再用 curl.exe --data-binary @file 发送。",
        );
    }
    if let Err(msg) = billing::check_can_call(&state.store, &user.id) {
        return json_error(StatusCode::PAYMENT_REQUIRED, msg);
    }
    let pc_runtime_route = match req.pc_runtime_route() {
        Ok(route) => route,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };
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
        message,
        req.attachments.as_deref(),
    );
    let trace_id = clean_trace_id(req.trace_id.as_deref());
    let task_id =
        match state
            .store
            .create_task(&project.id, &user.id, Some(&conversation_id), &message)
        {
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

const MAX_PROJECT_ICON_CONTEXT_DATA_URL_BYTES: usize = 512 * 1024;

fn append_project_icon_context(
    state: &AppState,
    project: &ProjectAccess,
    workspace: &Path,
    message: String,
    project_icon_data_url: Option<&str>,
) -> String {
    let Some(icon_data_url) = clean_project_icon_context_data_url(project_icon_data_url) else {
        return message;
    };
    if can_edit(&project.role) {
        let _ = state
            .store
            .set_project_icon_data_url(&project.id, Some(&icon_data_url));
    }
    let wrote_metadata = write_project_icon_metadata(workspace, project, &icon_data_url);
    let note = if wrote_metadata {
        "用户已上传这个项目的 APK 图标。图标元数据已写入 `.elon/project-icon.json`；后续生成、修改或打包 Android APK 时，必须读取该文件并把其中的 `icon_data_url` 用作 launcher icon（含 `android:icon` / `android:roundIcon` / adaptive icon），应用内所有展示该用户 APK 的位置也使用同一图标。".to_string()
    } else {
        format!(
            "用户已上传这个项目的 APK 图标。后续生成、修改或打包 Android APK 时，必须把下面的 `icon_data_url` 用作 launcher icon（含 `android:icon` / `android:roundIcon` / adaptive icon），应用内所有展示该用户 APK 的位置也使用同一图标。\n\nicon_data_url:\n{}",
            icon_data_url
        )
    };
    format!("{message}\n\n[项目 APK 图标]\n{note}")
}

fn should_append_project_icon_context_for_pc_fast_path(needs_project_workflow: bool) -> bool {
    needs_project_workflow
}

fn pc_node_fast_path_route(
    needs_project_workflow: bool,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Option<PcRuntimeRoutePreference> {
    if !needs_project_workflow && pc_runtime_route.is_none() {
        return Some(PcRuntimeRoutePreference::RouteA);
    }
    pc_runtime_route
}

fn looks_like_replaced_unicode_mojibake(message: &str) -> bool {
    let mut total = 0usize;
    let mut question_marks = 0usize;
    let mut replacement_chars = 0usize;
    let mut cjk = 0usize;

    for ch in message.chars() {
        if ch.is_whitespace() {
            continue;
        }
        total += 1;
        match ch {
            '?' => question_marks += 1,
            '\u{FFFD}' => replacement_chars += 1,
            '\u{4E00}'..='\u{9FFF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{20000}'..='\u{2A6DF}'
            | '\u{2A700}'..='\u{2B73F}'
            | '\u{2B740}'..='\u{2B81F}'
            | '\u{2B820}'..='\u{2CEAF}'
            | '\u{F900}'..='\u{FAFF}' => cjk += 1,
            _ => {}
        }
    }

    if total < 40 || cjk > 0 {
        return false;
    }
    let damaged = question_marks + replacement_chars;
    damaged >= 12 && damaged * 100 >= total * 20
}

#[cfg(test)]
mod tests {
    use super::{
        looks_like_replaced_unicode_mojibake, pc_node_fast_path_route,
        should_append_project_icon_context_for_pc_fast_path,
    };
    use crate::pc_agent_runtime_choice::PcRuntimeRoutePreference;

    #[test]
    fn detects_windows_question_mark_mojibake() {
        let message = "?????????? Win ? Codex ????????????????????????????????\n\
            1. ?? AGENTS.md ? .github/copilot-instructions.md???????????\n\
            2. ?? server/src/git_command_error.rs?server/src/node_agent_main.rs?";

        assert!(looks_like_replaced_unicode_mojibake(message));
    }

    #[test]
    fn allows_normal_chinese_and_question_marks() {
        assert!(!looks_like_replaced_unicode_mojibake(
            "这是一次 Win 端 Codex 产品链路实测，请读取项目源码并运行 git status？"
        ));
        assert!(!looks_like_replaced_unicode_mojibake(
            "Why??? Can Codex read AGENTS.md and run cargo check?"
        ));
    }

    #[test]
    fn pc_node_fast_path_keeps_lightweight_chat_message_plain() {
        assert!(!should_append_project_icon_context_for_pc_fast_path(false));
        assert!(should_append_project_icon_context_for_pc_fast_path(true));
    }

    #[test]
    fn pc_node_fast_path_defaults_lightweight_chat_to_route_a() {
        assert_eq!(
            pc_node_fast_path_route(false, None),
            Some(PcRuntimeRoutePreference::RouteA)
        );
        assert_eq!(
            pc_node_fast_path_route(false, Some(PcRuntimeRoutePreference::RouteC3)),
            Some(PcRuntimeRoutePreference::RouteC3)
        );
        assert_eq!(pc_node_fast_path_route(true, None), None);
    }
}

fn clean_project_icon_context_data_url(project_icon_data_url: Option<&str>) -> Option<String> {
    let value = project_icon_data_url?.trim();
    if value.is_empty() || value.len() > MAX_PROJECT_ICON_CONTEXT_DATA_URL_BYTES {
        return None;
    }
    if !value.starts_with("data:image/") || !value.contains(";base64,") {
        return None;
    }
    Some(value.to_string())
}

fn append_nonempty_ws_text(buffer: &mut String, text: Option<&str>) {
    let Some(text) = text.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if !buffer.is_empty() && !buffer.ends_with('\n') {
        buffer.push('\n');
    }
    buffer.push_str(text);
}

fn write_project_icon_metadata(
    workspace: &Path,
    project: &ProjectAccess,
    icon_data_url: &str,
) -> bool {
    let dir = workspace.join(".elon");
    if std::fs::create_dir_all(&dir).is_err() {
        return false;
    }
    let payload = serde_json::json!({
        "project_id": &project.id,
        "project_name": &project.name,
        "icon_data_url": icon_data_url,
        "usage": "Use this image as the Android APK launcher icon, including android:icon, android:roundIcon, adaptive icon foreground/background if present, and all in-app surfaces that represent this user APK."
    });
    serde_json::to_string_pretty(&payload)
        .ok()
        .and_then(|json| std::fs::write(dir.join("project-icon.json"), json).ok())
        .is_some()
}

pub(crate) async fn run_project_agent_with_scheduler(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    download_base: String,
    conversation_id: String,
    message: String,
    project_icon_data_url: Option<String>,
    agent_name: Option<String>,
    attachments: Option<Vec<ProjectAttachmentRef>>,
    execution_mode: ProjectExecutionMode,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    trace_id: Option<String>,
    tx: UnboundedSender<String>,
) {
    if let Err(msg) = billing::check_can_call(&state.store, &user_id) {
        let _ = tx.send(WsMessage::error(msg).to_json());
        return;
    }
    let project_icon_data_url = project_icon_data_url.or_else(|| {
        state
            .store
            .project_space_summary(&user_id, &project.id)
            .ok()
            .and_then(|project| project.icon_data_url)
    });

    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "server_workflow_start",
            serde_json::json!({
                "project_id": &project.id,
                "user_id": &user_id,
                "conversation_id": &conversation_id,
                "message_chars": message.chars().count(),
                "has_project_icon": project_icon_data_url
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty()),
                "agent": agent_name.as_deref(),
                "execution_mode": execution_mode.as_str(),
                "pc_runtime_route": pc_runtime_route.map(|route| route.as_request_value()),
            }),
        );
    }
    let routing_decision = intent_router::classify(&message);
    // force_cli: 悬浮球手机控制专用模式，绕过本地 intent_router 分流，
    // 直接进入 Codex CLI 意图门控，由 Codex 自己判断"闲聊还是生成脚本"。
    let needs_project_workflow = execution_mode.is_plan()
        || execution_mode.is_force_cli()
        || routing_decision.route != intent_router::CapabilityRoute::ChatAgent;
    let base_workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    if needs_project_workflow && !can_edit(&project.role) {
        let apk_url = if agent_intent::is_project_delivery_request(&message, &base_workspace)
            && tools::find_latest_apk(&base_workspace).is_some()
        {
            Some(tools::stable_apk_url(&download_base))
        } else {
            None
        };
        let message = if apk_url.is_some() {
            "当前项目已有可下载 APK。你是只读成员，可以下载体验，但不能发起修改代码、编译或发布。"
                .to_string()
        } else {
            "你当前是只读成员，可以在项目频道里询问 AI、查看讨论和结果，但不能发起修改代码、编译或发布。请联系项目 owner 获取协作权限。".to_string()
        };
        let _ = tx.send(
            WsMessage::Done {
                message,
                apk_url,
                image_url: None,
                model_used: None,
                node_id: None,
            }
            .to_json(),
        );
        return;
    }
    // Phase 2 优化：本地分类置信度 >= 84 的明确代码任务跳过 codex 意图门控。
    // force_cli 模式（悬浮球手机控制）强制走意图门控，让 Codex 自己判断是闲聊还是生成脚本，
    // 不能跳过（否则 Codex 不读 AGENTS.md，无法生成手机控制 JSON）。
    let skip_intent_gate = needs_project_workflow
        && !execution_mode.is_force_cli()
        && routing_decision.confidence >= 84;
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "server_intent_classified",
            serde_json::json!({
                "needs_project_workflow": needs_project_workflow,
                "local_confidence": routing_decision.confidence,
                "local_reason": routing_decision.reason,
                "skip_intent_gate": skip_intent_gate,
                "execution_mode": execution_mode.as_str(),
            }),
        );
    }
    // PC 节点项目（有 node_id）的路径在用户 PC 上，不在服务器本地。
    // 服务器上不应创建 worktree——直接透传给 agent 层，由 pc_project_binding 接管。
    // 同时 bypass 整个 scheduler（PC项目无需 worktree/合并锁），减少不必要的等待。
    let is_pc_node_project = project
        .node_id
        .as_deref()
        .map(|n| !n.is_empty())
        .unwrap_or(false);

    if is_pc_node_project {
        // PC 节点项目快速路径：服务器不创建 worktree，但仍按会话串行，避免同一
        // conversation 的多个 CLI 进程同时写同一个 PC 会话 worktree。
        if needs_project_workflow {
            let _ = tx.send(
                WsMessage::progress(
                    "PC 节点项目已启用本机会话隔离：代码会在你的 PC 节点上创建/复用会话 worktree 后执行。",
                )
                .to_json(),
            );
        }
        let queued_tx = tx.clone();
        let trace_state = state.clone();
        let queued_trace_id = trace_id.clone();
        let queued_project_id = project.id.clone();
        let queued_conversation_id = conversation_id.clone();
        let conversation_execution_key =
            project_conversation_execution_key(&project.id, &conversation_id);
        let conversation_permit = state
            .project_task_scheduler
            .acquire(&conversation_execution_key, move || {
                if let Some(trace_id) = queued_trace_id.as_deref() {
                    trace_state.server_traces.record(
                        trace_id,
                        "server_pc_conversation_queue_wait",
                        serde_json::json!({
                            "project_id": &queued_project_id,
                            "conversation_id": &queued_conversation_id,
                        }),
                    );
                }
                let _ = queued_tx.send(
                    WsMessage::progress("当前 PC 会话已有任务在运行，本次消息已进入该会话队列；其他会话仍可并行执行。")
                        .to_json(),
                );
            })
            .await;
        if needs_project_workflow {
            let message = if conversation_permit.was_queued() {
                "已轮到本 PC 会话任务，开始交给 PC 节点执行。"
            } else {
                "已获得本 PC 会话执行权，开始交给 PC 节点执行。"
            };
            let _ = tx.send(WsMessage::progress(message).to_json());
        }
        let message = if should_append_project_icon_context_for_pc_fast_path(needs_project_workflow)
        {
            append_project_icon_context(
                &state,
                &project,
                &base_workspace,
                message,
                project_icon_data_url.as_deref(),
            )
        } else {
            message
        };
        let _keep_conversation_permit = conversation_permit;
        if execution_mode.is_plan() {
            agent::plan_for_project_in_workspace(
                &user_id,
                &project,
                &base_workspace,
                &download_base,
                Some(&conversation_id),
                &message,
                agent_name.as_deref(),
                pc_node_fast_path_route(needs_project_workflow, pc_runtime_route),
                trace_id.as_deref(),
                &state,
                tx,
            )
            .await;
            return;
        }
        agent::run_for_project(
            &user_id,
            &project,
            &download_base,
            Some(&conversation_id),
            &message,
            agent_name.as_deref(),
            pc_node_fast_path_route(needs_project_workflow, pc_runtime_route),
            trace_id.as_deref(),
            &state,
            tx,
        )
        .await;
        return;
    }

    let prepared_execution_workspace =
        if needs_project_workflow && !execution_mode.is_plan() && !is_pc_node_project {
            match prepare_project_conversation_workspace(&state, &project, &conversation_id) {
                Ok(workspace) => Some(workspace),
                Err(error) => {
                    let _ = tx.send(
                        WsMessage::error(format!("创建会话 worktree 失败: {}", error)).to_json(),
                    );
                    return;
                }
            }
        } else {
            None
        };
    let workspace = prepared_execution_workspace
        .as_ref()
        .map(|workspace| workspace.active_path())
        .unwrap_or(base_workspace.as_path());
    let workspace_key = workspace.display().to_string();
    let prewarm_agent = if state.ai_cli.codex_cli_only {
        agent_name
            .as_deref()
            .filter(|name| is_local_cli_option(&state, name))
    } else {
        agent_name.as_deref()
    };
    let prewarm_key = codex_prewarm_key(
        &project.id,
        &user_id,
        &conversation_id,
        prewarm_agent,
        &workspace_key,
    );
    state.codex_prewarm.cancel(&prewarm_key).await;
    if !needs_project_workflow {
        agent::run_for_project(
            &user_id,
            &project,
            &download_base,
            Some(&conversation_id),
            &message,
            agent_name.as_deref(),
            pc_runtime_route,
            trace_id.as_deref(),
            &state,
            tx,
        )
        .await;
        return;
    }

    let message = append_project_icon_context(
        &state,
        &project,
        workspace,
        message,
        project_icon_data_url.as_deref(),
    );

    if execution_mode.is_plan() {
        let _ =
            tx.send(WsMessage::progress("已开启先规划模式：本轮只生成计划，不改代码。").to_json());
    } else if skip_intent_gate {
        if let Some(trace_id) = trace_id.as_deref() {
            state.server_traces.record(
                trace_id,
                "server_intent_gate_skipped",
                serde_json::json!({
                    "confidence": routing_decision.confidence,
                    "reason": routing_decision.reason,
                }),
            );
        }
        tracing::info!(
            confidence = routing_decision.confidence,
            reason = routing_decision.reason,
            "Skipped codex intent gate (high local confidence)"
        );
        let _ = tx.send(WsMessage::progress("已识别为开发任务，直接进入项目工作流。").to_json());
    } else {
        let _ = tx.send(WsMessage::progress("正在确认这是否需要进入开发流程。").to_json());
        let native_session_scope = ai_cli::NativeSessionScope {
            project_id: project.id.clone(),
            user_id: user_id.clone(),
            conversation_id: conversation_id.clone(),
            runtime_permission: project.runtime_permission.clone(),
        };
        match ai_cli::confirm_project_intent(
            workspace,
            &message,
            agent_name.as_deref(),
            Some(native_session_scope),
            trace_id.as_deref(),
            &state,
        )
        .await
        {
            Ok(gate) if !gate.should_enter_development() => {
                if let Some(trace_id) = trace_id.as_deref() {
                    state.server_traces.record(
                        trace_id,
                        "server_intent_kept_chat",
                        serde_json::json!({
                            "confidence": gate.confidence,
                            "reason": gate.reason,
                        }),
                    );
                }
                tracing::info!(
                    confidence = gate.confidence,
                    reason = %gate.reason,
                    "Codex CLI kept request in lightweight chat"
                );
                let reply = chat_reply_after_intent_gate(&message, gate.chat_reply);
                let _ = tx.send(
                    WsMessage::Done {
                        message: reply,
                        apk_url: None,
                        image_url: None,
                        model_used: None,
                        node_id: None,
                    }
                    .to_json(),
                );
                return;
            }
            Ok(gate) => {
                if let Some(trace_id) = trace_id.as_deref() {
                    state.server_traces.record(
                        trace_id,
                        "server_intent_enter_development",
                        serde_json::json!({
                            "confidence": gate.confidence,
                            "reason": gate.reason,
                        }),
                    );
                }
                tracing::info!(
                    confidence = gate.confidence,
                    reason = %gate.reason,
                    "Codex CLI confirmed development workflow"
                );
            }
            Err(error) => {
                if let Some(trace_id) = trace_id.as_deref() {
                    state.server_traces.record(
                        trace_id,
                        "server_intent_error",
                        serde_json::json!({
                            "error": error.to_string(),
                        }),
                    );
                }
                let _ = tx
                    .send(WsMessage::error(format!("Codex CLI 意图确认失败: {}", error)).to_json());
                return;
            }
        }
    }

    let _ = tx.send(
        WsMessage::progress("通用项目工作流已启用：服务器会为本会话准备独立 worktree/分支；同一会话串行，编码阶段可跨会话并行，最终合并、版本号和发布仍串行。"
                )
        .to_json(),
    );

    let queued_tx = tx.clone();
    let trace_state = state.clone();
    let queued_trace_id = trace_id.clone();
    let queued_project_id = project.id.clone();
    let queued_conversation_id = conversation_id.clone();
    let conversation_execution_key =
        project_conversation_execution_key(&project.id, &conversation_id);
    let conversation_permit = state
        .project_task_scheduler
        .acquire(&conversation_execution_key, move || {
            if let Some(trace_id) = queued_trace_id.as_deref() {
                trace_state.server_traces.record(
                    trace_id,
                    "server_conversation_queue_wait",
                    serde_json::json!({
                        "project_id": &queued_project_id,
                        "conversation_id": &queued_conversation_id,
                    }),
                );
            }
            let _ = queued_tx.send(
                WsMessage::progress("当前会话已有任务在运行，本次任务已进入该会话队列；其他会话仍可使用独立 worktree 并行开发。"
                        )
                .to_json(),
            );
        })
        .await;

    let execution_workspace = prepared_execution_workspace
        .unwrap_or_else(|| ProjectConversationWorkspace::shared(base_workspace.clone()));

    let shared_project_permit = if execution_mode.is_plan() || execution_workspace.is_isolated() {
        None
    } else {
        let queued_tx = tx.clone();
        let trace_state = state.clone();
        let queued_trace_id = trace_id.clone();
        let queued_project_id = project.id.clone();
        let shared_key = project_shared_execution_key(&project.id);
        Some(
            state
                .project_task_scheduler
                .acquire(&shared_key, move || {
                    if let Some(trace_id) = queued_trace_id.as_deref() {
                        trace_state.server_traces.record(
                            trace_id,
                            "server_project_queue_wait",
                            serde_json::json!({ "project_id": &queued_project_id }),
                        );
                    }
                    let _ = queued_tx.send(
                        WsMessage::progress(
                            "当前项目无法创建独立 worktree，已退回共享工作区串行执行。",
                        )
                        .to_json(),
                    );
                })
                .await,
        )
    };

    let message_text = if execution_mode.is_plan() && conversation_permit.was_queued() {
        "已轮到本会话规划任务，开始生成计划。"
    } else if execution_mode.is_plan() {
        "已获得本会话规划执行权，开始生成计划。"
    } else if conversation_permit.was_queued() {
        "已轮到本会话任务，开始在会话 worktree 中调用 AI 修改项目。"
    } else if execution_workspace.is_isolated() {
        "已获得本会话执行权，开始在独立 worktree 中调用 AI 修改项目。"
    } else {
        "已获得项目执行权，开始在共享工作区中调用 AI 修改项目。"
    };
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "server_conversation_execution_granted",
            serde_json::json!({
                "project_id": &project.id,
                "conversation_id": &conversation_id,
                "was_queued": conversation_permit.was_queued(),
                "workspace": execution_workspace.active_path().display().to_string(),
                "isolated": execution_workspace.is_isolated(),
            }),
        );
    }
    let _ = tx.send(WsMessage::progress(message_text).to_json());

    let _keep_conversation_permit = conversation_permit;
    let _keep_shared_project_permit = shared_project_permit;
    let message = append_project_cli_attachment_artifacts(
        state.as_ref(),
        &project,
        &conversation_id,
        message,
        attachments.as_deref(),
        execution_workspace.active_path(),
    )
    .await;
    run_project_agent_in_execution_workspace(
        state,
        user_id,
        project,
        download_base,
        conversation_id,
        message,
        agent_name,
        execution_mode,
        trace_id,
        execution_workspace,
        tx,
    )
    .await;
}
