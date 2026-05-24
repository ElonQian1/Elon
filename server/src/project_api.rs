use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path as AxumPath, Query, State,
    },
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::{collections::HashMap, path::Path, path::PathBuf, process::Command, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    agent, ai_cli, intent_router,
    store::{ProjectAccess, PublicUser},
    tools,
    types::{AppState, WsMessage},
};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
    pub device_name: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub account: String,
    pub password: String,
    pub nickname: Option<String>,
    pub device_name: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub template: Option<String>,
}

#[derive(Deserialize)]
pub struct ProjectChatRequest {
    pub message: String,
    pub agent: Option<String>,
    pub conversation_id: Option<String>,
    pub conversation_title: Option<String>,
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
        tx,
    )
    .await;

    let mut reply = String::new();
    let mut apk_url = None;
    let mut image_url = None;
    let mut error = None;
    while let Some(raw) = rx.recv().await {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
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

    Json(serde_json::json!({
        "task_id": task_id,
        "conversation_id": conversation_id,
        "reply": reply,
        "apk_url": apk_url,
        "image_url": image_url,
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
    tx: UnboundedSender<String>,
) {
    let needs_project_workflow =
        intent_router::classify(&message).route != intent_router::CapabilityRoute::ChatAgent;
    if !needs_project_workflow {
        agent::run_for_project(
            &user_id,
            &project,
            &download_base,
            Some(&conversation_id),
            &message,
            agent_name.as_deref(),
            &state,
            tx,
        )
        .await;
        return;
    }

    let _ = tx.send(
        WsMessage::Progress {
            message: "正在确认这是否需要进入开发流程。".into(),
        }
        .to_json(),
    );
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let native_session_scope = ai_cli::NativeSessionScope {
        project_id: project.id.clone(),
        user_id: user_id.clone(),
        conversation_id: conversation_id.clone(),
    };
    match ai_cli::confirm_project_intent(
        &workspace,
        &message,
        agent_name.as_deref(),
        Some(native_session_scope),
        &state,
    )
    .await
    {
        Ok(gate) if !gate.should_enter_development() => {
            tracing::info!(
                confidence = gate.confidence,
                reason = %gate.reason,
                "Codex CLI kept request in lightweight chat"
            );
            let reply = gate
                .chat_reply
                .filter(|reply| !reply.trim().is_empty())
                .unwrap_or_else(|| {
                    "我先按普通聊天处理。你如果要我开始改代码、编译或发布，可以直接说明。".into()
                });
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
            tracing::info!(
                confidence = gate.confidence,
                reason = %gate.reason,
                "Codex CLI confirmed development workflow"
            );
        }
        Err(error) => {
            let _ = tx.send(
                WsMessage::Error {
                    message: format!("Codex CLI 意图确认失败: {}", error),
                }
                .to_json(),
            );
            return;
        }
    }

    let _ = tx.send(
        WsMessage::Progress {
            message: "通用项目工作流已启用：先确认 Git/权限，再读取项目文档，按项目自己的规则修改；同一项目的共享工作区任务会排队，未来 task worktree 编码可并行，但合并、版本号和发布仍串行。"
                .into(),
        }
        .to_json(),
    );

    let queued_tx = tx.clone();
    let permit = state
        .project_task_scheduler
        .acquire(&project.id, move || {
            let _ = queued_tx.send(
                WsMessage::Progress {
                    message: "当前项目已有任务在运行，本次任务已进入队列。为了避免多个手机同时修改同一份项目工作区，服务器会按项目顺序执行。"
                        .into(),
                }
                .to_json(),
            );
        })
        .await;

    let message_text = if permit.was_queued() {
        "已轮到本次任务，开始同步代码并调用 AI 修改项目。"
    } else {
        "已获得项目执行权，开始同步代码并调用 AI 修改项目。"
    };
    let _ = tx.send(
        WsMessage::Progress {
            message: message_text.into(),
        }
        .to_json(),
    );

    agent::run_for_project(
        &user_id,
        &project,
        &download_base,
        Some(&conversation_id),
        &message,
        agent_name.as_deref(),
        &state,
        tx,
    )
    .await;
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
    ws.on_upgrade(move |socket| handle_project_ws(socket, state, user, project, download_base))
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
    ws.on_upgrade(move |socket| handle_project_ws(socket, state, user, project, download_base))
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
) {
    let (mut sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        let text = match msg {
            Message::Text(text) => text,
            Message::Ping(payload) => {
                if sender.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
                continue;
            }
            Message::Pong(_) => continue,
            Message::Close(_) => break,
            Message::Binary(_) => continue,
        };
        let request = parse_project_message(&text);
        let message = request.message.trim().to_string();
        if message.is_empty() {
            continue;
        }

        let conversation_id = state
            .store
            .ensure_conversation(
                &project.id,
                &user.id,
                request.conversation_id.as_deref(),
                request.conversation_title.as_deref(),
            )
            .unwrap_or_else(|_| "default".into());

        let task_id = state
            .store
            .create_task(&project.id, &user.id, Some(&conversation_id), &message)
            .unwrap_or_else(|_| "tsk_unknown".into());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let state_clone = state.clone();
        let user_id = user.id.clone();
        let project_for_task = project.clone();
        let download_base_for_task = download_base.clone();
        let agent_task = tokio::spawn(async move {
            run_project_agent_with_scheduler(
                state_clone,
                user_id,
                project_for_task,
                download_base_for_task,
                conversation_id,
                message,
                request.agent,
                tx,
            )
            .await;
        });

        let mut reply = String::new();
        let mut apk_url = None;
        let mut error = None;
        let mut client_disconnected = false;
        loop {
            tokio::select! {
                progress = rx.recv() => {
                    let Some(progress) = progress else {
                        break;
                    };
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&progress) {
                        match value.get("type").and_then(|t| t.as_str()) {
                            Some("done") => {
                                reply = value["message"].as_str().unwrap_or("完成").to_string();
                                apk_url = value["apk_url"].as_str().map(ToOwned::to_owned);
                            }
                            Some("error") => {
                                let msg = value["message"].as_str().unwrap_or("发生错误").to_string();
                                reply = msg.clone();
                                error = Some(msg);
                            }
                            _ => {}
                        }
                    }
                    if sender.send(Message::Text(progress)).await.is_err() {
                        client_disconnected = true;
                        break;
                    }
                }
                incoming = receiver.next() => {
                    match incoming {
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
                        Some(Ok(Message::Text(_))) => {}
                        Some(Ok(Message::Binary(_))) => {}
                        Some(Err(_)) => {
                            client_disconnected = true;
                            break;
                        }
                    }
                }
            }
        }
        if client_disconnected {
            agent_task.abort();
            let _ = agent_task.await;
            let _ = state.store.finish_task(
                &task_id,
                "failed",
                Some("连接已断开"),
                None,
                Some("client disconnected"),
            );
            break;
        }
        let _ = agent_task.await;

        let status = if error.is_some() { "failed" } else { "done" };
        let _ = state.store.finish_task(
            &task_id,
            status,
            Some(&reply),
            apk_url.as_deref(),
            error.as_deref(),
        );
    }
}

fn parse_project_message(raw: &str) -> ProjectChatRequest {
    serde_json::from_str::<ProjectChatRequest>(raw).unwrap_or_else(|_| ProjectChatRequest {
        message: raw.to_string(),
        agent: None,
        conversation_id: None,
        conversation_title: None,
    })
}

// ── 兼容旧 APK：旧入口会被映射到普通项目 elon-self ───────────────────────

/// WebSocket 入口：`GET /ws/elon`，无需 token。新客户端应使用
/// `/ws/user/:user_id/projects/:project_id`。
pub async fn ws_elon_self_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    let (user, project) = match ensure_mobile_project(&state, "elon-system", "elon-self", None) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let download_base = format!(
        "{}/api/user/{}/projects/{}/download",
        state.public_url, user.id, project.id
    );
    ws.on_upgrade(move |socket| handle_project_ws(socket, state, user, project, download_base))
        .into_response()
}

/// APK 下载：`GET /api/elon/download/:filename`
pub async fn download_elon_self_apk(AxumPath(filename): AxumPath<String>) -> Response {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return json_error(StatusCode::BAD_REQUEST, "invalid filename");
    }
    if !filename.ends_with(".apk") {
        return json_error(StatusCode::BAD_REQUEST, "only APK downloads are allowed");
    }

    let workspace = agent::elon_self_workspace();
    let Some(apk_path) = tools::find_apk_by_filename(&workspace.join("android"), &filename) else {
        return json_error(StatusCode::NOT_FOUND, "APK 文件不存在");
    };
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
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(data))
        .unwrap_or_else(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
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

fn ensure_mobile_project(
    state: &AppState,
    user_id: &str,
    project_id: &str,
    project_title: Option<&str>,
) -> anyhow::Result<(PublicUser, ProjectAccess)> {
    let user = state.store.ensure_device_user(user_id)?;
    let spec = mobile_project_spec(project_id, project_title);
    let project = state.store.ensure_project_for_user(
        &user.id,
        project_id,
        &spec.name,
        Some(spec.description),
        spec.source_type,
        spec.template,
        spec.workspace_path.as_deref(),
    )?;

    if project.source_type != "local_path" {
        let workspace = state
            .resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
        tools::create_project_workspace(&workspace, "android", &project.name, &user.id)?;
    }

    Ok((user, project))
}

struct MobileProjectSpec {
    name: String,
    description: &'static str,
    source_type: &'static str,
    template: &'static str,
    workspace_path: Option<String>,
}

fn mobile_project_spec(project_id: &str, project_title: Option<&str>) -> MobileProjectSpec {
    let workspace_path = configured_local_project_workspace(project_id)
        .map(|path| path.to_string_lossy().to_string());
    let name = project_title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            if project_id == "elon-self" {
                "一龙项目".into()
            } else {
                "移动端项目".into()
            }
        });

    if workspace_path.is_some() {
        MobileProjectSpec {
            name,
            description: "本地 Git 项目",
            source_type: "local_path",
            template: "local",
            workspace_path,
        }
    } else {
        MobileProjectSpec {
            name,
            description: "APK 创建的项目",
            source_type: "template",
            template: "android",
            workspace_path: None,
        }
    }
}

fn configured_local_project_workspace(project_id: &str) -> Option<std::path::PathBuf> {
    let env_key = format!("ELON_PROJECT_{}_PATH", env_key_suffix(project_id));
    if let Ok(path) = std::env::var(env_key) {
        let path = path.trim();
        if !path.is_empty() {
            return Some(path.into());
        }
    }

    if project_id == "elon-self" {
        return Some(agent::elon_self_workspace());
    }

    None
}

fn env_key_suffix(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn project_git_status_json(state: &AppState, project: &ProjectAccess) -> serde_json::Value {
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let has_git = workspace.join(".git").exists();
    let origin = git_output(&workspace, &["remote", "get-url", "origin"]).ok();
    let branch = git_output(&workspace, &["rev-parse", "--abbrev-ref", "HEAD"]).ok();
    let (public_key, has_deploy_key) = read_deploy_public_key(state, &project.id)
        .map(|key| (Some(key), true))
        .unwrap_or((None, false));
    let remote_check = if has_git && origin.is_some() {
        Some(check_remote_access(
            &workspace,
            branch.as_deref().unwrap_or("main"),
        ))
    } else {
        None
    };
    let deploy_keys_url = origin
        .as_deref()
        .and_then(github_deploy_keys_url)
        .unwrap_or_else(|| "https://github.com/settings/keys".into());

    serde_json::json!({
        "project_id": project.id,
        "source_type": project.source_type,
        "workspace": workspace.to_string_lossy(),
        "git": {
            "has_git": has_git,
            "origin": origin,
            "branch": branch,
            "remote_check": remote_check,
        },
        "deploy_key": {
            "exists": has_deploy_key,
            "public_key": public_key,
            "github_deploy_keys_url": deploy_keys_url,
        },
        "recommended_auth": "deploy_key",
        "github_app": {
            "enabled": false,
            "message": "GitHub App 授权适合多用户正式版；当前版本先使用每项目 Deploy Key。"
        },
        "workflow": project_workflow_json(),
    })
}

fn project_workflow_json() -> serde_json::Value {
    serde_json::json!({
        "title": "通用项目工作流",
        "summary": "所有项目都走同一套流程：先识别项目和授权，再读取项目文档，之后修改、验证、提交、推送；同项目共享动作由服务器排队保护。",
        "steps": [
            "项目准备：确认项目路径、Git 仓库、远端和写权限。",
            "读取文档：优先读取 AGENTS.md、CODEX.md、README.md、.github/instructions 和任务相关 docs。",
            "会话连续：其他 AI 模型以后只能作为旁路分析，结论必须回灌到当前 Codex CLI 原生 session。",
            "执行任务：按项目自己的技术栈修改代码，不把一龙自项目规则套到普通项目。",
            "验证保存：运行必要检查，commit；有可用远端时 push。",
            "共享动作：合并 main、版本号递增、APK 发布、服务器部署必须串行。"
        ],
        "codex_memory": "Codex CLI 不依赖长期记忆；服务器每次任务都会在提示词中注入这套通用流程，同时要求它读取当前项目仓库内的说明文档。以后接入的其他模型只能做旁路分析，结论会被整理后回灌到当前会话绑定的 Codex CLI 原生 session。"
    })
}

fn ensure_project_deploy_key(
    state: &AppState,
    project: &ProjectAccess,
    workspace: &Path,
) -> anyhow::Result<String> {
    std::fs::create_dir_all(workspace)?;
    if !workspace.join(".git").exists() {
        let _ = Command::new("git")
            .arg("init")
            .current_dir(workspace)
            .output();
    }

    let (private_key, _) = deploy_key_paths(state, &project.id);
    if !private_key.exists() {
        if let Some(parent) = private_key.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let comment = format!("elon-project-{}@server", project.id);
        let output = Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-N", "", "-C", &comment, "-f"])
            .arg(&private_key)
            .output()?;
        if !output.status.success() {
            anyhow::bail!(
                "生成 SSH key 失败: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&private_key, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    configure_deploy_key_ssh(workspace, &private_key)?;
    read_deploy_public_key(state, &project.id)
}

fn configure_git_remote(
    state: &AppState,
    project: &ProjectAccess,
    workspace: &Path,
    repo_url: &str,
    branch: &str,
    auth_type: &str,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(workspace)?;
    if !workspace.join(".git").exists() {
        let output = Command::new("git")
            .arg("init")
            .current_dir(workspace)
            .output()?;
        if !output.status.success() {
            anyhow::bail!("git init 失败: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    let remote_exists = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    let args = if remote_exists {
        vec!["remote", "set-url", "origin", repo_url]
    } else {
        vec!["remote", "add", "origin", repo_url]
    };
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "设置 Git 远端失败: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let _ = Command::new("git")
        .args(["branch", "-M", branch])
        .current_dir(workspace)
        .output();

    if auth_type == "deploy_key" {
        let _ = ensure_project_deploy_key(state, project, workspace)?;
    }

    Ok(())
}

fn deploy_key_paths(state: &AppState, project_id: &str) -> (PathBuf, PathBuf) {
    let private_key = state
        .data_dir
        .join("git-keys")
        .join(env_key_suffix(project_id).to_ascii_lowercase())
        .join("deploy_ed25519");
    let public_key = private_key.with_extension("pub");
    (private_key, public_key)
}

fn read_deploy_public_key(state: &AppState, project_id: &str) -> anyhow::Result<String> {
    let (_, public_key) = deploy_key_paths(state, project_id);
    Ok(std::fs::read_to_string(public_key)?.trim().to_string())
}

fn configure_deploy_key_ssh(workspace: &Path, private_key: &Path) -> anyhow::Result<()> {
    let ssh_command = format!(
        "ssh -i {} -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new",
        private_key.to_string_lossy()
    );
    let output = Command::new("git")
        .args(["config", "core.sshCommand", &ssh_command])
        .current_dir(workspace)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "配置项目 SSH key 失败: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn git_output(workspace: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn check_remote_access(workspace: &Path, branch: &str) -> serde_json::Value {
    let output = Command::new("git")
        .args(["ls-remote", "--heads", "origin", branch])
        .current_dir(workspace)
        .output();
    match output {
        Ok(out) if out.status.success() => serde_json::json!({
            "ok": true,
            "message": "远端读取正常"
        }),
        Ok(out) => serde_json::json!({
            "ok": false,
            "message": String::from_utf8_lossy(&out.stderr).trim()
        }),
        Err(e) => serde_json::json!({
            "ok": false,
            "message": e.to_string()
        }),
    }
}

fn github_deploy_keys_url(repo_url: &str) -> Option<String> {
    let trimmed = repo_url.trim().trim_end_matches(".git");
    let path = if let Some(path) = trimmed.strip_prefix("git@github.com:") {
        path
    } else if let Some(path) = trimmed.strip_prefix("https://github.com/") {
        path
    } else if let Some(path) = trimmed.strip_prefix("http://github.com/") {
        path
    } else {
        return None;
    };
    let mut parts = path.split('/').filter(|part| !part.is_empty());
    let owner = parts.next()?;
    let repo = parts.next()?;
    Some(format!("https://github.com/{owner}/{repo}/settings/keys"))
}

fn login_inner(
    state: &AppState,
    req: LoginRequest,
) -> anyhow::Result<(String, String, PublicUser)> {
    let user = state
        .store
        .authenticate_password(&req.account, &req.password)?;
    let (token, expires_at) = state
        .store
        .create_session(&user.id, req.device_name.as_deref())?;
    Ok((token, expires_at, user))
}

fn register_inner(
    state: &AppState,
    req: RegisterRequest,
) -> anyhow::Result<(String, String, PublicUser)> {
    let user = state.store.create_user(
        &req.account,
        &req.password,
        req.nickname.as_deref(),
        Some("user"),
    )?;
    let (token, expires_at) = state
        .store
        .create_session(&user.id, req.device_name.as_deref())?;
    Ok((token, expires_at, user))
}

pub fn auth_from_headers(state: &AppState, headers: &HeaderMap) -> anyhow::Result<PublicUser> {
    let token = bearer_token(headers).ok_or_else(|| anyhow::anyhow!("缺少 Authorization token"))?;
    state.store.authenticate_token(token)
}

fn auth_from_headers_or_query(
    state: &AppState,
    headers: &HeaderMap,
    query: &HashMap<String, String>,
) -> anyhow::Result<PublicUser> {
    if let Some(token) = bearer_token(headers) {
        return state.store.authenticate_token(token);
    }
    let token = query
        .get("token")
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!("缺少下载 token"))?;
    state.store.authenticate_token(token)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn project_access(
    state: &AppState,
    user_id: &str,
    project_id: &str,
) -> anyhow::Result<ProjectAccess> {
    state.store.get_project_access(user_id, project_id)
}

fn can_edit(role: &str) -> bool {
    matches!(role, "owner" | "editor")
}

fn json_error(status: StatusCode, message: impl ToString) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": message.to_string()
        })),
    )
        .into_response()
}
