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
use std::{collections::HashMap, sync::Arc};

use crate::{
    agent,
    store::{ProjectAccess, PublicUser},
    tools,
    types::AppState,
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

    let task_id = match state.store.create_task(&project.id, &user.id, &message) {
        Ok(id) => id,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    agent::run_for_project(
        &user.id,
        &project.id,
        &project.workspace_key,
        &message,
        req.agent.as_deref(),
        &state,
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
        "reply": reply,
        "apk_url": apk_url,
        "image_url": image_url,
    }))
    .into_response()
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

    ws.on_upgrade(move |socket| handle_project_ws(socket, state, user, project))
        .into_response()
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

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return json_error(StatusCode::BAD_REQUEST, "invalid filename");
    }
    if !filename.ends_with(".apk") {
        return json_error(StatusCode::BAD_REQUEST, "only APK downloads are allowed");
    }

    let workspace = state.get_project_workspace(&project.workspace_key);
    let Some(apk_path) = tools::find_download_apk(&workspace, &filename) else {
        return json_error(StatusCode::NOT_FOUND, "APK 文件不存在");
    };
    let download_name = apk_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&filename);
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

async fn handle_project_ws(
    socket: WebSocket,
    state: Arc<AppState>,
    user: PublicUser,
    project: ProjectAccess,
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

        let task_id = state
            .store
            .create_task(&project.id, &user.id, &message)
            .unwrap_or_else(|_| "tsk_unknown".into());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let state_clone = state.clone();
        let user_id = user.id.clone();
        let project_id = project.id.clone();
        let workspace_key = project.workspace_key.clone();
        let agent_task = tokio::spawn(async move {
            agent::run_for_project(
                &user_id,
                &project_id,
                &workspace_key,
                &message,
                request.agent.as_deref(),
                &state_clone,
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
    })
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
