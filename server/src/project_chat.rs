use axum::{
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::stream;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    agent, agent_intent,
    agent_routing::is_local_cli_option,
    ai_cli, intent_router,
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
    project_ws_protocol::{ProjectAttachmentRef, ProjectChatRequest},
    store::ProjectAccess,
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
            req.agent,
            attachments,
            ProjectExecutionMode::from_request(req.execution_mode.as_deref(), req.plan_mode),
            Some(trace_id.clone()),
            tx,
        )
        .await;
    }

    let mut reply = String::new();
    let mut apk_url = None;
    let mut image_url = None;
    let mut error = None;
    while let Some(raw) = rx.recv().await {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
            record_server_message(&state, &trace_id, &value, raw.len());
            match value.get("type").and_then(|t| t.as_str()) {
                Some("done") => {
                    reply = value["message"].as_str().unwrap_or("完成").to_string();
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

    let status = if error.is_some() { "failed" } else { "done" };
    let _ = state.store.finish_task(
        &task_id,
        status,
        Some(&reply),
        apk_url.as_deref(),
        error.as_deref(),
    );

    // 把本轮对话写入 messages 表，供历史记录和记忆提取使用。
    // 仅在有实质回复时写入，避免记录错误/空响应污染历史。
    if !reply.is_empty() && error.is_none() {
        if !original_user_message.trim().is_empty() {
            let _ = state.store.add_message(
                &project_id_for_history,
                Some(&conversation_id),
                Some(&task_id),
                Some(&user.id),
                "user",
                original_user_message.trim(),
            );
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
                tokio::spawn(async move {
                    crate::user_memory_extract::extract_and_save_memories(state2, uid, umsg, rep).await;
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
        req.agent,
        attachments,
        ProjectExecutionMode::from_request(req.execution_mode.as_deref(), req.plan_mode),
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

pub(crate) async fn run_project_agent_with_scheduler(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    download_base: String,
    conversation_id: String,
    message: String,
    agent_name: Option<String>,
    attachments: Option<Vec<ProjectAttachmentRef>>,
    execution_mode: ProjectExecutionMode,
    trace_id: Option<String>,
    tx: UnboundedSender<String>,
) {
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "server_workflow_start",
            serde_json::json!({
                "project_id": &project.id,
                "user_id": &user_id,
                "conversation_id": &conversation_id,
                "message_chars": message.chars().count(),
                "agent": agent_name.as_deref(),
                "execution_mode": execution_mode.as_str(),
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
    let is_pc_node_project = project.node_id.as_deref()
        .map(|n| !n.is_empty())
        .unwrap_or(false);

    if is_pc_node_project {
        // PC 节点项目快速路径：跳过 worktree/调度器，直接交给 agent
        agent::run_for_project(
            &user_id,
            &project,
            &download_base,
            Some(&conversation_id),
            &message,
            agent_name.as_deref(),
            trace_id.as_deref(),
            &state,
            tx,
        )
        .await;
        return;
    }

    let prepared_execution_workspace = if needs_project_workflow && !execution_mode.is_plan() && !is_pc_node_project {
        match prepare_project_conversation_workspace(&state, &project, &conversation_id) {
            Ok(workspace) => Some(workspace),
            Err(error) => {
                let _ = tx
                    .send(WsMessage::error(format!("创建会话 worktree 失败: {}", error)).to_json());
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
            trace_id.as_deref(),
            &state,
            tx,
        )
        .await;
        return;
    }

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
