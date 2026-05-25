use axum::{
    Json,
    body::{Body, Bytes},
    extract::{
        Path as AxumPath, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, broadcast, mpsc::UnboundedSender, watch};

use crate::{
    agent, ai_cli, intent_router,
    project_attachments::{
        MAX_PROJECT_ATTACHMENT_BYTES, append_project_attachment_notes, content_type_for_file,
        percent_encode_path_segment, safe_project_path_part, unique_project_attachment_name,
    },
    project_auth::{
        LoginRequest, RegisterRequest, auth_from_headers, auth_from_headers_or_query, can_edit,
        json_error, login_inner, project_access, register_inner,
    },
    project_chat_reply::chat_reply_after_intent_gate,
    project_conversation_workspace::{
        ProjectConversationWorkspace, merge_conversation_worktree,
        prepare_project_conversation_workspace, project_conversation_execution_key,
        project_merge_execution_key, project_shared_execution_key,
    },
    project_git::{configure_git_remote, ensure_project_deploy_key, project_git_status_json},
    project_mobile::ensure_mobile_project,
    project_ws_protocol::{
        PROJECT_WS_BACKLOG_LIMIT, ProjectChatRequest, ProjectPrewarmRequest,
        enrich_project_ws_event, is_done_project_ws_message, is_terminal_project_ws_message,
        is_terminal_task_status, parse_project_message, project_client_request_id,
        server_message_details, task_control_event, terminal_backlog_from_task,
    },
    store::{ProjectAccess, PublicUser},
    tools,
    types::{AppState, WsMessage},
};

static PROJECT_WS_JOBS: LazyLock<Mutex<HashMap<String, Arc<ProjectWsJob>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

struct ProjectWsJob {
    key: String,
    fingerprint: String,
    task_id: String,
    trace_id: Option<String>,
    cancel_tx: watch::Sender<bool>,
    backlog: Mutex<Vec<String>>,
    broadcaster: broadcast::Sender<String>,
    finished: AtomicBool,
}

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub template: Option<String>,
}

#[derive(Deserialize)]
pub struct GitConfigRequest {
    pub repo_url: String,
    pub branch: Option<String>,
    pub auth_type: Option<String>,
}

pub async fn login(State(state): State<Arc<AppState>>, Json(req): Json<LoginRequest>) -> Response {
    match login_inner(&state, req) {
        Ok((token, expires_at, user)) => Json(serde_json::json!({
            "token": token,
            "expires_at": expires_at,
            "user": user,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    }
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Response {
    match register_inner(&state, req) {
        Ok((token, expires_at, user)) => Json(serde_json::json!({
            "token": token,
            "expires_at": expires_at,
            "user": user,
        }))
        .into_response(),
        Err(e) => {
            let message = e.to_string();
            if message.contains("UNIQUE constraint failed") {
                json_error(StatusCode::BAD_REQUEST, "账号已被注册")
            } else {
                json_error(StatusCode::BAD_REQUEST, message)
            }
        }
    }
}

pub async fn me(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match auth_from_headers(&state, &headers) {
        Ok(user) => Json(serde_json::json!({ "user": user })).into_response(),
        Err(e) => json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    }
}

pub async fn list_my_projects(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if let Err(e) = ensure_mobile_project(&state, &user.id, "elon-self", Some("一龙项目")) {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    match state.store.list_projects_for_user(&user.id) {
        Ok(projects) => Json(serde_json::json!({ "projects": projects })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn create_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateProjectRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    let project = match state.store.create_project(
        &user.id,
        &req.name,
        req.description.as_deref(),
        req.template.as_deref(),
    ) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };

    let workspace = state.get_project_workspace(&project.workspace_key);
    if let Err(e) =
        tools::create_project_workspace(&workspace, &project.template, &project.name, &user.id)
    {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    Json(serde_json::json!({ "project": project })).into_response()
}

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
        }),
    );

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
    run_project_agent_with_scheduler(
        state.clone(),
        user.id.clone(),
        project,
        download_base,
        conversation_id.clone(),
        message,
        req.agent,
        Some(trace_id.clone()),
        tx,
    )
    .await;

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

pub async fn prewarm_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(project_id): AxumPath<String>,
    Json(req): Json<ProjectPrewarmRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    prewarm_project_response(state, user, project, req).await
}

pub async fn prewarm_user_project(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<ProjectPrewarmRequest>,
) -> Response {
    let (user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    prewarm_project_response(state, user, project, req).await
}

async fn prewarm_project_response(
    state: Arc<AppState>,
    user: PublicUser,
    project: ProjectAccess,
    req: ProjectPrewarmRequest,
) -> Response {
    if !can_edit(&project.role) {
        return json_error(
            StatusCode::FORBIDDEN,
            "current user cannot edit this project",
        );
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

    let base_workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let conversation_workspace =
        match prepare_project_conversation_workspace(&state, &project, &conversation_id) {
            Ok(workspace) => workspace,
            Err(error) => {
                tracing::warn!(
                    project_id = %project.id,
                    conversation_id = %conversation_id,
                    error = %error,
                    "conversation worktree prewarm fell back to base workspace"
                );
                ProjectConversationWorkspace::shared(base_workspace.clone())
            }
        };
    let workspace = conversation_workspace.active_path().to_path_buf();
    let trace_id = req
        .trace_id
        .as_deref()
        .map(|value| clean_trace_id(Some(value)))
        .filter(|value| !value.is_empty());
    let requested_agent = req
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let agent = if state.ai_cli.codex_cli_only {
        None
    } else {
        requested_agent
    };
    let workspace_key = workspace.display().to_string();
    let throttle_key = codex_prewarm_key(
        &project.id,
        &user.id,
        &conversation_id,
        agent.as_deref(),
        &workspace_key,
    );
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "codex_prewarm_request",
            serde_json::json!({
                "project_id": &project.id,
                "user_id": &user.id,
                "conversation_id": &conversation_id,
                "workspace": &workspace_key,
                "agent": agent.as_deref(),
            }),
        );
    }
    if !state
        .codex_prewarm
        .start_if_allowed(&throttle_key, Duration::from_secs(120))
        .await
    {
        if let Some(trace_id) = trace_id.as_deref() {
            state.server_traces.record(
                trace_id,
                "codex_prewarm_skipped",
                serde_json::json!({
                    "reason": "cooldown",
                    "project_id": &project.id,
                    "conversation_id": &conversation_id,
                }),
            );
        }
        return Json(serde_json::json!({
            "status": "skipped",
            "reason": "cooldown",
            "project_id": project.id,
            "conversation_id": conversation_id,
        }))
        .into_response();
    }

    let scope = ai_cli::NativeSessionScope {
        project_id: project.id.clone(),
        user_id: user.id.clone(),
        conversation_id: conversation_id.clone(),
    };
    let state_for_task = state.clone();
    let workspace_for_task = workspace.clone();
    let agent_for_task = agent.clone();
    let project_id_for_log = project.id.clone();
    let conversation_id_for_log = conversation_id.clone();
    let prewarm_key_for_task = throttle_key.clone();
    let trace_id_for_task = trace_id.clone();
    tokio::spawn(async move {
        match ai_cli::prewarm_codex_session(
            &workspace_for_task,
            agent_for_task.as_deref(),
            scope,
            trace_id_for_task.as_deref(),
            &state_for_task,
        )
        .await
        {
            Ok(result) => tracing::info!(
                project_id = %project_id_for_log,
                conversation_id = %conversation_id_for_log,
                reused = result.reused,
                thread_id = ?result.thread_id,
                elapsed_ms = result.elapsed_ms,
                "Codex CLI session prewarm completed"
            ),
            Err(error) => tracing::warn!(
                project_id = %project_id_for_log,
                conversation_id = %conversation_id_for_log,
                error = %error,
                "Codex CLI session prewarm failed"
            ),
        }
        let accepted = state_for_task
            .codex_prewarm
            .finish(&prewarm_key_for_task)
            .await;
        if !accepted {
            tracing::info!(
                project_id = %project_id_for_log,
                conversation_id = %conversation_id_for_log,
                "Codex CLI session prewarm finished after real request started"
            );
        }
    });

    Json(serde_json::json!({
        "status": "accepted",
        "project_id": project.id,
        "conversation_id": conversation_id,
        "workspace": workspace_key,
    }))
    .into_response()
}

async fn run_project_agent_with_scheduler(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    download_base: String,
    conversation_id: String,
    message: String,
    agent_name: Option<String>,
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
            }),
        );
    }
    let routing_decision = intent_router::classify(&message);
    let needs_project_workflow =
        routing_decision.route != intent_router::CapabilityRoute::ChatAgent;
    // Phase 2 优化：本地分类置信度 >= 84 的明确代码任务（app_or_web_development=84、
    // image_asset_for_app=86、standalone_image=90），跳过 codex 二次意图门控，
    // 节省每请求 5-15 秒的冷启动+推理时间。confidence < 84 的模糊判定仍走门控防误判。
    let skip_intent_gate = needs_project_workflow && routing_decision.confidence >= 84;
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "server_intent_classified",
            serde_json::json!({
                "needs_project_workflow": needs_project_workflow,
                "local_confidence": routing_decision.confidence,
                "local_reason": routing_decision.reason,
                "skip_intent_gate": skip_intent_gate,
            }),
        );
    }
    let base_workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let prepared_execution_workspace = if needs_project_workflow {
        match prepare_project_conversation_workspace(&state, &project, &conversation_id) {
            Ok(workspace) => Some(workspace),
            Err(error) => {
                let _ = tx.send(
                    WsMessage::Error {
                        message: format!("创建会话 worktree 失败: {}", error),
                    }
                    .to_json(),
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
        None
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

    if skip_intent_gate {
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
        let _ = tx.send(
            WsMessage::Progress {
                message: "已识别为开发任务，直接进入项目工作流。".into(),
            }
            .to_json(),
        );
    } else {
        let _ = tx.send(
            WsMessage::Progress {
                message: "正在确认这是否需要进入开发流程。".into(),
            }
            .to_json(),
        );
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
                let _ = tx.send(
                    WsMessage::Error {
                        message: format!("Codex CLI 意图确认失败: {}", error),
                    }
                    .to_json(),
                );
                return;
            }
        }
    }

    let _ = tx.send(
        WsMessage::Progress {
            message: "通用项目工作流已启用：服务器会为本会话准备独立 worktree/分支；同一会话串行，编码阶段可跨会话并行，最终合并、版本号和发布仍串行。"
                .into(),
        }
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
                WsMessage::Progress {
                    message: "当前会话已有任务在运行，本次任务已进入该会话队列；其他会话仍可使用独立 worktree 并行开发。"
                        .into(),
                }
                .to_json(),
            );
        })
        .await;

    let execution_workspace = prepared_execution_workspace
        .unwrap_or_else(|| ProjectConversationWorkspace::shared(base_workspace.clone()));

    let shared_project_permit = if execution_workspace.is_isolated() {
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
                        WsMessage::Progress {
                            message: "当前项目无法创建独立 worktree，已退回共享工作区串行执行。"
                                .into(),
                        }
                        .to_json(),
                    );
                })
                .await,
        )
    };

    let message_text = if conversation_permit.was_queued() {
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
    let _ = tx.send(
        WsMessage::Progress {
            message: message_text.into(),
        }
        .to_json(),
    );

    let _keep_conversation_permit = conversation_permit;
    let _keep_shared_project_permit = shared_project_permit;
    run_project_agent_in_execution_workspace(
        state,
        user_id,
        project,
        download_base,
        conversation_id,
        message,
        agent_name,
        trace_id,
        execution_workspace,
        tx,
    )
    .await;
}

async fn run_project_agent_in_execution_workspace(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    download_base: String,
    conversation_id: String,
    message: String,
    agent_name: Option<String>,
    trace_id: Option<String>,
    execution_workspace: ProjectConversationWorkspace,
    tx: UnboundedSender<String>,
) {
    let (agent_tx, mut agent_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let agent_state = state.clone();
    let agent_user_id = user_id.clone();
    let agent_project = project.clone();
    let agent_download_base = download_base.clone();
    let agent_conversation_id = conversation_id.clone();
    let agent_message = message.clone();
    let agent_name_for_task = agent_name.clone();
    let agent_trace_id = trace_id.clone();
    let agent_workspace = execution_workspace.active_workspace.clone();

    let agent_task = tokio::spawn(async move {
        agent::run_for_project_in_workspace(
            &agent_user_id,
            &agent_project,
            &agent_workspace,
            &agent_download_base,
            Some(&agent_conversation_id),
            &agent_message,
            agent_name_for_task.as_deref(),
            agent_trace_id.as_deref(),
            &agent_state,
            agent_tx,
        )
        .await;
    });

    let mut terminal_raw = None;
    let mut terminal_is_done = false;
    while let Some(raw) = agent_rx.recv().await {
        if is_terminal_project_ws_message(&raw) {
            terminal_is_done = is_done_project_ws_message(&raw);
            terminal_raw = Some(raw);
            break;
        }
        let _ = tx.send(raw);
    }

    if let Err(error) = agent_task.await {
        let _ = tx.send(
            WsMessage::Error {
                message: format!("AI 任务异常结束: {}", error),
            }
            .to_json(),
        );
        return;
    }

    while let Ok(raw) = agent_rx.try_recv() {
        if is_terminal_project_ws_message(&raw) {
            terminal_is_done = is_done_project_ws_message(&raw);
            terminal_raw = Some(raw);
        } else {
            let _ = tx.send(raw);
        }
    }

    if terminal_is_done && execution_workspace.is_isolated() {
        let merge_key = project_merge_execution_key(&project.id);
        let merge_tx = tx.clone();
        let merge_state = state.clone();
        let merge_trace_id = trace_id.clone();
        let merge_project_id = project.id.clone();
        let merge_permit = state
            .project_task_scheduler
            .acquire(&merge_key, move || {
                if let Some(trace_id) = merge_trace_id.as_deref() {
                    merge_state.server_traces.record(
                        trace_id,
                        "server_project_merge_queue_wait",
                        serde_json::json!({ "project_id": &merge_project_id }),
                    );
                }
                let _ = merge_tx.send(
                    WsMessage::Progress {
                        message: "代码已在会话分支完成，正在等待项目合并锁。".into(),
                    }
                    .to_json(),
                );
            })
            .await;
        let _keep_merge_permit = merge_permit;
        let _ = tx.send(
            WsMessage::Progress {
                message: "正在把会话分支串行合并回项目主工作区。".into(),
            }
            .to_json(),
        );
        match merge_conversation_worktree(&execution_workspace) {
            Ok(summary) => {
                if let Some(trace_id) = trace_id.as_deref() {
                    state.server_traces.record(
                        trace_id,
                        "server_project_merge_done",
                        serde_json::json!({
                            "project_id": &project.id,
                            "conversation_id": &conversation_id,
                            "summary": summary,
                        }),
                    );
                }
                let _ = tx.send(WsMessage::Progress { message: summary }.to_json());
            }
            Err(error) => {
                if let Some(trace_id) = trace_id.as_deref() {
                    state.server_traces.record(
                        trace_id,
                        "server_project_merge_failed",
                        serde_json::json!({
                            "project_id": &project.id,
                            "conversation_id": &conversation_id,
                            "error": error.to_string(),
                        }),
                    );
                }
                let _ = tx.send(
                    WsMessage::Error {
                        message: format!(
                            "会话代码已完成，但合并回项目主分支失败: {}。请处理冲突后重试。",
                            error
                        ),
                    }
                    .to_json(),
                );
                return;
            }
        }
    }

    if let Some(raw) = terminal_raw {
        let _ = tx.send(raw);
    } else {
        let _ = tx.send(
            WsMessage::Error {
                message: "AI 任务结束但没有返回完成状态。".into(),
            }
            .to_json(),
        );
    }
}

pub async fn ws_project_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    AxumPath(project_id): AxumPath<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let token = query.get("token").map(String::as_str).unwrap_or("");
    let user = match state.store.authenticate_token(token) {
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

    let download_base = format!("{}/api/projects/{}/download", state.public_url, project.id);
    let client_version_code = query
        .get("app_version_code")
        .and_then(|value| value.parse::<i64>().ok());
    ws.on_upgrade(move |socket| {
        handle_project_ws(
            socket,
            state,
            user,
            project,
            download_base,
            client_version_code,
        )
    })
    .into_response()
}

pub async fn ws_user_project_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let (user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let download_base = format!(
        "{}/api/user/{}/projects/{}/download",
        state.public_url, user.id, project.id
    );
    let client_version_code = query
        .get("app_version_code")
        .and_then(|value| value.parse::<i64>().ok());
    ws.on_upgrade(move |socket| {
        handle_project_ws(
            socket,
            state,
            user,
            project,
            download_base,
            client_version_code,
        )
    })
    .into_response()
}

pub async fn user_project_git_status(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let (_user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };

    Json(project_git_status_json(&state, &project)).into_response()
}

pub async fn upload_project_attachment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(project_id): AxumPath<String>,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };

    upload_project_attachment_impl(
        state,
        user.id,
        project,
        query.get("conversation_id").map(String::as_str),
        query.get("conversation_title").map(String::as_str),
        query.get("kind").map(String::as_str),
        query.get("display_name").map(String::as_str),
        query.get("file_name").map(String::as_str),
        query.get("mime_type").map(String::as_str),
        body,
        true,
    )
    .await
}

pub async fn upload_user_project_attachment(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let (user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };

    upload_project_attachment_impl(
        state,
        user.id,
        project,
        query.get("conversation_id").map(String::as_str),
        query.get("conversation_title").map(String::as_str),
        query.get("kind").map(String::as_str),
        query.get("display_name").map(String::as_str),
        query.get("file_name").map(String::as_str),
        query.get("mime_type").map(String::as_str),
        body,
        false,
    )
    .await
}

async fn upload_project_attachment_impl(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    conversation_id_hint: Option<&str>,
    conversation_title_hint: Option<&str>,
    kind_hint: Option<&str>,
    display_name_hint: Option<&str>,
    file_name_hint: Option<&str>,
    mime_type_hint: Option<&str>,
    body: Bytes,
    include_project_api_url: bool,
) -> Response {
    if body.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "attachment body is empty");
    }
    if body.len() > MAX_PROJECT_ATTACHMENT_BYTES {
        return json_error(StatusCode::PAYLOAD_TOO_LARGE, "attachment is too large");
    }

    let conversation_id = match state.store.ensure_conversation(
        &project.id,
        &user_id,
        conversation_id_hint,
        conversation_title_hint,
    ) {
        Ok(id) => id,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let attachments_dir = workspace
        .join("attachments")
        .join(safe_project_path_part(&conversation_id, 80));
    if let Err(error) = tokio::fs::create_dir_all(&attachments_dir).await {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }

    let display_name = display_name_hint
        .or(file_name_hint)
        .unwrap_or("attachment.bin");
    let original_name = file_name_hint.unwrap_or(display_name);
    let file_name = unique_project_attachment_name(&attachments_dir, original_name);
    let path = attachments_dir.join(&file_name);
    if let Err(error) = tokio::fs::write(&path, body.as_ref()).await {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }

    let mut urls = vec![format!(
        "{}/api/user/{}/projects/{}/attachments/{}/{}",
        state.public_url.trim_end_matches('/'),
        percent_encode_path_segment(&user_id),
        percent_encode_path_segment(&project.id),
        percent_encode_path_segment(&conversation_id),
        percent_encode_path_segment(&file_name)
    )];
    if include_project_api_url {
        urls.push(format!(
            "{}/api/projects/{}/attachments/{}/{}",
            state.public_url.trim_end_matches('/'),
            percent_encode_path_segment(&project.id),
            percent_encode_path_segment(&conversation_id),
            percent_encode_path_segment(&file_name)
        ));
    }

    let attachment = serde_json::json!({
        "kind": kind_hint.unwrap_or("attachment"),
        "display_name": display_name,
        "file_name": file_name,
        "mime_type": mime_type_hint.unwrap_or("application/octet-stream"),
        "path": path.to_string_lossy(),
        "url": urls[0],
        "urls": urls,
        "size_bytes": body.len(),
    });
    Json(serde_json::json!({
        "status": "uploaded",
        "project_id": project.id,
        "conversation_id": conversation_id,
        "attachment": attachment,
    }))
    .into_response()
}

pub async fn download_user_project_attachment(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id, conversation_id, filename)): AxumPath<(
        String,
        String,
        String,
        String,
    )>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return json_error(StatusCode::BAD_REQUEST, "invalid filename");
    }

    let (_user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    download_project_attachment_impl(&state, &project, &conversation_id, &filename).await
}

pub async fn download_project_attachment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath((project_id, conversation_id, filename)): AxumPath<(String, String, String)>,
) -> Response {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return json_error(StatusCode::BAD_REQUEST, "invalid filename");
    }

    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    download_project_attachment_impl(&state, &project, &conversation_id, &filename).await
}

async fn download_project_attachment_impl(
    state: &AppState,
    project: &ProjectAccess,
    conversation_id: &str,
    filename: &str,
) -> Response {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return json_error(StatusCode::BAD_REQUEST, "invalid filename");
    }

    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let attachments_dir = workspace
        .join("attachments")
        .join(safe_project_path_part(conversation_id, 80));
    let path = attachments_dir.join(&filename);
    let valid_path = std::fs::canonicalize(&attachments_dir)
        .ok()
        .and_then(|root| {
            std::fs::canonicalize(&path)
                .ok()
                .filter(|canonical| canonical.starts_with(root))
        })
        .is_some();
    if !valid_path {
        return json_error(StatusCode::NOT_FOUND, "attachment not found");
    }

    let data = match tokio::fs::read(&path).await {
        Ok(data) => data,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let content_type = content_type_for_file(&filename);
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", filename),
        )
        .body(Body::from(data))
        .unwrap_or_else(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn user_project_deploy_key(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let (_user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());

    match ensure_project_deploy_key(&state, &project, &workspace) {
        Ok(public_key) => Json(serde_json::json!({
            "project_id": project.id,
            "public_key": public_key,
            "status": project_git_status_json(&state, &project),
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn user_project_git_config(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<GitConfigRequest>,
) -> Response {
    let (user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if !can_edit(&project.role) {
        return json_error(StatusCode::FORBIDDEN, "当前用户没有配置项目的权限");
    }

    let repo_url = req.repo_url.trim();
    if repo_url.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "Git 仓库地址不能为空");
    }
    let branch = req.branch.as_deref().unwrap_or("main").trim();
    let branch = if branch.is_empty() { "main" } else { branch };
    let auth_type = req.auth_type.as_deref().unwrap_or("deploy_key");

    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    if let Err(e) = configure_git_remote(&state, &project, &workspace, repo_url, branch, auth_type)
    {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    let project =
        match state
            .store
            .update_project_git_config(&user.id, &project.id, repo_url, branch)
        {
            Ok(project) => project,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        };

    Json(project_git_status_json(&state, &project)).into_response()
}

pub async fn download_project_apk(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath((project_id, filename)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let user = match auth_from_headers_or_query(&state, &headers, &query) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };

    serve_project_apk(&state, &project, &filename).await
}

pub async fn download_user_project_apk(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id, filename)): AxumPath<(String, String, String)>,
) -> Response {
    let user = match state.store.ensure_device_user(&user_id) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    serve_project_apk(&state, &project, &filename).await
}

async fn handle_project_ws(
    socket: WebSocket,
    state: Arc<AppState>,
    user: PublicUser,
    project: ProjectAccess,
    download_base: String,
    client_version_code: Option<i64>,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut update_rx = crate::app_update::subscribe();

    if let Some(event) =
        crate::app_update::latest_update_event_for_client(&state, client_version_code).await
    {
        if sender.send(Message::Text(event)).await.is_err() {
            return;
        }
    }

    loop {
        let text = tokio::select! {
            update = update_rx.recv() => {
                if let Ok(event) = update {
                    if crate::app_update::is_newer_for_client(&event, client_version_code)
                        && sender.send(Message::Text(event)).await.is_err()
                    {
                        break;
                    }
                }
                continue;
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Ping(payload))) => {
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                        continue;
                    }
                    Some(Ok(Message::Pong(_))) => continue,
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Binary(_))) => continue,
                    Some(Err(_)) => break,
                }
            }
        };

        let request = parse_project_message(&text);
        let op = request.op.as_deref().unwrap_or("run").to_ascii_lowercase();

        let conversation_id = state
            .store
            .ensure_conversation(
                &project.id,
                &user.id,
                request.conversation_id.as_deref(),
                request.conversation_title.as_deref(),
            )
            .unwrap_or_else(|_| "default".into());
        if op == "cancel" {
            let canceled_task_id = cancel_project_ws_job(
                &project.id,
                &user.id,
                &conversation_id,
                request.task_id.as_deref(),
                request.client_request_id.as_deref(),
            )
            .await;
            let payload = match canceled_task_id.as_deref() {
                Some(task_id) => task_control_event(
                    "cancel_requested",
                    Some(task_id),
                    request.client_request_id.as_deref(),
                    Some(&conversation_id),
                    "已接收取消请求，任务会尽快停止。",
                ),
                None => task_control_event(
                    "cancel_ignored",
                    request.task_id.as_deref(),
                    request.client_request_id.as_deref(),
                    Some(&conversation_id),
                    "没有找到可取消的运行中任务。",
                ),
            };
            if sender.send(Message::Text(payload)).await.is_err() {
                break;
            }
            continue;
        }

        let message = request.message.trim().to_string();
        if message.is_empty() {
            continue;
        }
        let message = append_project_attachment_notes(
            &state,
            &project,
            &conversation_id,
            message,
            request.attachments.as_deref(),
        );

        let trace_id = clean_trace_id(request.trace_id.as_deref());
        let client_request_id =
            project_client_request_id(&request, &project.id, &user.id, &conversation_id, &message);
        state.server_traces.record(
            &trace_id,
            "ws_project_message_received",
            serde_json::json!({
                "project_id": &project.id,
                "user_id": &user.id,
                "conversation_id": &conversation_id,
                "client_request_id": &client_request_id,
                "message_chars": message.chars().count(),
                "agent": request.agent.as_deref(),
            }),
        );
        let fingerprint =
            project_ws_fingerprint(&conversation_id, request.agent.as_deref(), &message);
        let job = get_or_start_project_ws_job(
            state.clone(),
            user.id.clone(),
            project.clone(),
            download_base.clone(),
            conversation_id.clone(),
            message,
            request.agent,
            Some(trace_id.clone()),
            client_request_id.clone(),
            fingerprint,
        )
        .await;

        if sender
            .send(Message::Text(task_control_event(
                "accepted",
                Some(&job.task_id),
                Some(&client_request_id),
                Some(&conversation_id),
                "请求已进入任务队列。",
            )))
            .await
            .is_err()
        {
            break;
        }

        let mut job_rx = job.broadcaster.subscribe();
        let backlog = job.backlog.lock().await.clone();
        let mut replayed_terminal = false;
        let mut replay_failed = false;
        for progress in backlog {
            if sender.send(Message::Text(progress.clone())).await.is_err() {
                record_server_transport(
                    &state,
                    &trace_id,
                    "server_replay_to_phone_failed",
                    &progress,
                    &job.task_id,
                );
                replay_failed = true;
                break;
            }
            record_server_transport(
                &state,
                &trace_id,
                "server_message_replayed_to_phone",
                &progress,
                &job.task_id,
            );
            if is_terminal_project_ws_message(&progress) {
                replayed_terminal = true;
                break;
            }
        }
        if replay_failed {
            break;
        }
        if replayed_terminal {
            continue;
        }

        let mut client_disconnected = false;
        loop {
            tokio::select! {
                progress = job_rx.recv() => {
                    match progress {
                        Ok(progress) => {
                            let terminal = is_terminal_project_ws_message(&progress);
                            if sender.send(Message::Text(progress.clone())).await.is_err() {
                                record_server_transport(
                                    &state,
                                    &trace_id,
                                    "server_send_to_phone_failed",
                                    &progress,
                                    &job.task_id,
                                );
                                client_disconnected = true;
                                break;
                            }
                            record_server_transport(
                                &state,
                                &trace_id,
                                "server_message_forwarded_to_phone",
                                &progress,
                                &job.task_id,
                            );
                            if terminal || job.finished.load(Ordering::SeqCst) {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                incoming = receiver.next() => {
                    match incoming {
                        Some(Ok(Message::Text(text))) => {
                            tracing::info!(
                                task_id = %job.task_id,
                                "received project WebSocket message while request was running; ignoring {} bytes",
                                text.len()
                            );
                        }
                        Some(Ok(Message::Ping(payload))) => {
                            if sender.send(Message::Pong(payload)).await.is_err() {
                                client_disconnected = true;
                                break;
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {}
                        Some(Ok(Message::Close(_))) | None => {
                            client_disconnected = true;
                            break;
                        }
                        Some(Ok(Message::Binary(_))) => {}
                        Some(Err(_)) => {
                            client_disconnected = true;
                            break;
                        }
                    }
                }
                update = update_rx.recv() => {
                    if let Ok(event) = update {
                        if crate::app_update::is_newer_for_client(&event, client_version_code)
                            && sender.send(Message::Text(event)).await.is_err()
                        {
                            client_disconnected = true;
                            break;
                        }
                    }
                }
            }
        }
        if client_disconnected {
            state.server_traces.record(
                &trace_id,
                "server_client_disconnected",
                serde_json::json!({
                    "task_id": &job.task_id,
                    "background_job_continues": !job.finished.load(Ordering::SeqCst),
                }),
            );
            tracing::info!(
                task_id = %job.task_id,
                "project WebSocket disconnected while task was running; background job continues"
            );
            break;
        }
    }
}

async fn get_or_start_project_ws_job(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    download_base: String,
    conversation_id: String,
    message: String,
    agent_name: Option<String>,
    trace_id: Option<String>,
    client_request_id: String,
    fingerprint: String,
) -> Arc<ProjectWsJob> {
    let key = project_ws_job_key(&project.id, &user_id, &conversation_id, &client_request_id);
    let mut jobs = PROJECT_WS_JOBS.lock().await;
    if let Some(existing) = jobs.get(&key) {
        if existing.fingerprint == fingerprint {
            if let Some(trace_id) = trace_id.as_deref() {
                state.server_traces.record(
                    trace_id,
                    "ws_project_join_existing_job",
                    serde_json::json!({
                        "task_id": &existing.task_id,
                        "finished": existing.finished.load(Ordering::SeqCst),
                    }),
                );
            }
            return existing.clone();
        }
        if !existing.finished.load(Ordering::SeqCst) {
            if let Some(trace_id) = trace_id.as_deref() {
                state.server_traces.record(
                    trace_id,
                    "ws_project_attach_running_job",
                    serde_json::json!({
                        "task_id": &existing.task_id,
                        "reason": "different_fingerprint",
                    }),
                );
            }
            let notice = WsMessage::Progress {
                message: "同一个请求仍在后台处理，正在继续同步已有任务进度。".into(),
            }
            .to_json();
            let _ = existing.broadcaster.send(notice);
            return existing.clone();
        }
        jobs.remove(&key);
    }

    let persisted = state
        .store
        .get_task_by_client_request(
            &project.id,
            &user_id,
            Some(&conversation_id),
            &client_request_id,
        )
        .ok()
        .flatten();
    if let Some(task) = persisted
        .as_ref()
        .filter(|task| is_terminal_task_status(&task.status))
    {
        let events = state
            .store
            .list_task_events(&task.id, PROJECT_WS_BACKLOG_LIMIT)
            .unwrap_or_default();
        let backlog = terminal_backlog_from_task(task, events);
        let (broadcast_tx, _) = broadcast::channel::<String>(256);
        let (cancel_tx, _cancel_rx) = watch::channel(false);
        let job = Arc::new(ProjectWsJob {
            key: key.clone(),
            fingerprint,
            task_id: task.id.clone(),
            trace_id: trace_id.clone(),
            cancel_tx,
            backlog: Mutex::new(backlog),
            broadcaster: broadcast_tx,
            finished: AtomicBool::new(true),
        });
        if let Some(trace_id) = trace_id.as_deref() {
            state.server_traces.record(
                trace_id,
                "ws_project_restore_terminal_task",
                serde_json::json!({
                    "task_id": &task.id,
                    "status": &task.status,
                }),
            );
        }
        jobs.insert(key.clone(), job.clone());
        schedule_project_job_cleanup(key, job.clone());
        return job;
    }

    let (task_id, restart_notice) = if let Some(task) = persisted {
        let notice = if task.status == "interrupted" {
            Some("上次任务被服务器重启中断，正在用同一个任务记录继续处理。".to_string())
        } else {
            Some("正在恢复服务器中已有的运行中任务。".to_string())
        };
        let _ = state.store.set_task_running(&task.id);
        (task.id, notice)
    } else {
        match state.store.create_task_with_client_request(
            &project.id,
            &user_id,
            Some(&conversation_id),
            Some(&client_request_id),
            &message,
        ) {
            Ok(task_id) => (task_id, None),
            Err(error) => {
                let raw = WsMessage::Error {
                    message: format!("创建任务记录失败: {}", error),
                }
                .to_json();
                let (broadcast_tx, _) = broadcast::channel::<String>(256);
                let (cancel_tx, _cancel_rx) = watch::channel(false);
                let job = Arc::new(ProjectWsJob {
                    key: key.clone(),
                    fingerprint,
                    task_id: "tsk_unknown".into(),
                    trace_id: trace_id.clone(),
                    cancel_tx,
                    backlog: Mutex::new(vec![raw]),
                    broadcaster: broadcast_tx,
                    finished: AtomicBool::new(true),
                });
                jobs.insert(key.clone(), job.clone());
                schedule_project_job_cleanup(key, job.clone());
                return job;
            }
        }
    };

    let (broadcast_tx, _) = broadcast::channel::<String>(256);
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let job = Arc::new(ProjectWsJob {
        key: key.clone(),
        fingerprint,
        task_id: task_id.clone(),
        trace_id: trace_id.clone(),
        cancel_tx,
        backlog: Mutex::new(Vec::new()),
        broadcaster: broadcast_tx,
        finished: AtomicBool::new(false),
    });
    jobs.insert(key.clone(), job.clone());

    let job_for_task = job.clone();
    tokio::spawn(async move {
        run_project_ws_job(
            state,
            user_id,
            project,
            download_base,
            conversation_id,
            message,
            agent_name,
            trace_id,
            task_id,
            job_for_task,
            restart_notice,
            cancel_rx,
        )
        .await;
    });

    job
}

async fn run_project_ws_job(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    download_base: String,
    conversation_id: String,
    message: String,
    agent_name: Option<String>,
    trace_id: Option<String>,
    task_id: String,
    job: Arc<ProjectWsJob>,
    restart_notice: Option<String>,
    mut cancel_rx: watch::Receiver<bool>,
) {
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "ws_project_job_start",
            serde_json::json!({
                "task_id": &task_id,
                "project_id": &project.id,
                "conversation_id": &conversation_id,
                "message_chars": message.chars().count(),
                "agent": agent_name.as_deref(),
            }),
        );
    }
    if let Some(message) = restart_notice {
        emit_project_job_event(
            &state,
            &task_id,
            &job,
            WsMessage::Progress { message }.to_json(),
        )
        .await;
    }
    emit_project_job_event(
        &state,
        &task_id,
        &job,
        task_control_event(
            "started",
            Some(&task_id),
            None,
            Some(&conversation_id),
            "任务开始执行。",
        ),
    )
    .await;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let state_clone = state.clone();
    let project_for_task = project.clone();
    let task_conversation_id = conversation_id.clone();
    let task_message = message.clone();
    let task_agent_name = agent_name.clone();
    let task_trace_id = trace_id.clone();
    let agent_task = tokio::spawn(async move {
        run_project_agent_with_scheduler(
            state_clone,
            user_id,
            project_for_task,
            download_base,
            task_conversation_id,
            task_message,
            task_agent_name,
            task_trace_id,
            tx,
        )
        .await;
    });

    let mut reply = String::new();
    let mut apk_url = None;
    let mut error = None;
    let mut saw_terminal = false;
    loop {
        tokio::select! {
            changed = cancel_rx.changed() => {
                if changed.is_ok() && *cancel_rx.borrow() {
                    agent_task.abort();
                    let msg = "任务已取消。".to_string();
                    emit_project_job_event(
                        &state,
                        &task_id,
                        &job,
                        task_control_event(
                            "canceled",
                            Some(&task_id),
                            None,
                            Some(&conversation_id),
                            &msg,
                        ),
                    )
                    .await;
                    emit_project_job_event(
                        &state,
                        &task_id,
                        &job,
                        WsMessage::Error {
                            message: msg.clone(),
                        }
                        .to_json(),
                    )
                    .await;
                    reply = msg.clone();
                    error = Some(msg);
                    saw_terminal = true;
                    break;
                }
            }
            next = rx.recv() => {
                let Some(progress) = next else {
                    break;
                };
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&progress) {
            match value.get("type").and_then(|t| t.as_str()) {
                Some("done") => {
                    reply = value["message"].as_str().unwrap_or("完成").to_string();
                    apk_url = value["apk_url"].as_str().map(ToOwned::to_owned);
                    saw_terminal = true;
                }
                Some("error") => {
                    let msg = value["message"].as_str().unwrap_or("发生错误").to_string();
                    reply = msg.clone();
                    error = Some(msg);
                    saw_terminal = true;
                }
                _ => {}
            }
        }
        let terminal = is_terminal_project_ws_message(&progress);
        emit_project_job_event(&state, &task_id, &job, progress).await;
        if terminal {
            break;
        }
            }
        }
    }
    let _ = agent_task.await;

    if !saw_terminal {
        let msg = "任务没有返回最终结果，请稍后重试或查看服务端日志。".to_string();
        let raw = WsMessage::Error {
            message: msg.clone(),
        }
        .to_json();
        emit_project_job_event(&state, &task_id, &job, raw).await;
        reply = msg.clone();
        error = Some(msg);
    }

    let status = if error.is_some() { "failed" } else { "done" };
    let _ = state.store.finish_task(
        &task_id,
        status,
        Some(&reply),
        apk_url.as_deref(),
        error.as_deref(),
    );
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            if error.is_some() {
                "ws_project_task_failed"
            } else {
                "ws_project_task_done"
            },
            serde_json::json!({
                "task_id": &task_id,
                "status": status,
                "has_apk_url": apk_url.is_some(),
            }),
        );
    }
    job.finished.store(true, Ordering::SeqCst);
    schedule_project_job_cleanup(job.key.clone(), job);
}

async fn emit_project_job_event(
    state: &AppState,
    task_id: &str,
    job: &Arc<ProjectWsJob>,
    raw: String,
) {
    let raw = enrich_project_ws_event(raw, task_id);
    {
        let mut backlog = job.backlog.lock().await;
        backlog.push(raw.clone());
        if backlog.len() > PROJECT_WS_BACKLOG_LIMIT {
            let overflow = backlog.len() - PROJECT_WS_BACKLOG_LIMIT;
            backlog.drain(0..overflow);
        }
    }
    let _ = state.store.record_task_event(task_id, &raw);
    if let Some(trace_id) = job.trace_id.as_deref() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
            record_server_message(state, trace_id, &value, raw.len());
        }
    }
    let _ = job.broadcaster.send(raw);
}

async fn cancel_project_ws_job(
    project_id: &str,
    user_id: &str,
    conversation_id: &str,
    task_id: Option<&str>,
    client_request_id: Option<&str>,
) -> Option<String> {
    let jobs = PROJECT_WS_JOBS.lock().await;
    if let Some(task_id) = task_id {
        for job in jobs.values() {
            if job.task_id == task_id && !job.finished.load(Ordering::SeqCst) {
                let _ = job.cancel_tx.send(true);
                return Some(job.task_id.clone());
            }
        }
    }
    if let Some(client_request_id) = client_request_id {
        let key = project_ws_job_key(project_id, user_id, conversation_id, client_request_id);
        if let Some(job) = jobs.get(&key) {
            if !job.finished.load(Ordering::SeqCst) {
                let _ = job.cancel_tx.send(true);
                return Some(job.task_id.clone());
            }
        }
    }
    None
}

fn schedule_project_job_cleanup(key: String, job: Arc<ProjectWsJob>) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30 * 60)).await;
        let mut jobs = PROJECT_WS_JOBS.lock().await;
        if jobs
            .get(&key)
            .map(|existing| Arc::ptr_eq(existing, &job))
            .unwrap_or(false)
        {
            jobs.remove(&key);
        }
    });
}

fn project_ws_job_key(
    project_id: &str,
    user_id: &str,
    conversation_id: &str,
    client_request_id: &str,
) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        project_id, user_id, conversation_id, client_request_id
    )
}

fn project_ws_fingerprint(
    conversation_id: &str,
    agent_name: Option<&str>,
    message: &str,
) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}",
        conversation_id,
        agent_name.unwrap_or(""),
        message
    )
}

fn clean_trace_id(input: Option<&str>) -> String {
    let cleaned = input
        .unwrap_or_default()
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
        .take(120)
        .collect::<String>();
    if cleaned.is_empty() {
        format!("srv_{}", current_wall_time_ms())
    } else {
        cleaned
    }
}

fn record_server_message(
    state: &AppState,
    trace_id: &str,
    value: &serde_json::Value,
    bytes: usize,
) {
    if trace_id.trim().is_empty() {
        return;
    }
    let details = server_message_details(value, bytes);
    state
        .server_traces
        .record(trace_id, "server_message_to_phone", details.clone());
    match value.get("type").and_then(|kind| kind.as_str()) {
        Some("done") => state.server_traces.record(trace_id, "server_done", details),
        Some("error") => state
            .server_traces
            .record(trace_id, "server_error", details),
        _ => {}
    }
}

fn record_server_transport(
    state: &AppState,
    trace_id: &str,
    phase: &str,
    raw: &str,
    task_id: &str,
) {
    if trace_id.trim().is_empty() {
        return;
    }
    let mut details = serde_json::from_str::<serde_json::Value>(raw)
        .map(|value| server_message_details(&value, raw.len()))
        .unwrap_or_else(|_| {
            serde_json::json!({
                "type": "invalid_json",
                "bytes": raw.len(),
            })
        });
    if let Some(object) = details.as_object_mut() {
        object.insert("task_id".into(), serde_json::json!(task_id));
    }
    state.server_traces.record(trace_id, phase, details);
}

fn current_wall_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

async fn serve_project_apk(state: &AppState, project: &ProjectAccess, filename: &str) -> Response {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return json_error(StatusCode::BAD_REQUEST, "invalid filename");
    }
    if !filename.ends_with(".apk") {
        return json_error(StatusCode::BAD_REQUEST, "only APK downloads are allowed");
    }
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let Some(apk_path) = tools::find_download_apk(&workspace, filename) else {
        return json_error(StatusCode::NOT_FOUND, "APK 文件不存在");
    };
    let download_name = apk_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(filename);
    let data = match tokio::fs::read(&apk_path).await {
        Ok(data) => data,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.android.package-archive",
        )
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", download_name),
        )
        .body(Body::from(data))
        .unwrap_or_else(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

fn codex_prewarm_key(
    project_id: &str,
    user_id: &str,
    conversation_id: &str,
    agent: Option<&str>,
    workspace_key: &str,
) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        project_id,
        user_id,
        conversation_id,
        agent.unwrap_or("default"),
        workspace_key
    )
}
