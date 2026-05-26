use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path as AxumPath, Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{atomic::Ordering, Arc},
};
use tokio::sync::broadcast;

use crate::{
    project_attachment_notes::append_project_attachment_notes,
    project_auth::{
        auth_from_headers, can_edit, json_error, login_inner, project_access, register_inner,
        LoginRequest, RegisterRequest,
    },
    project_keys::{clean_trace_id, project_ws_fingerprint},
    project_mobile::ensure_mobile_project,
    project_trace_events::record_server_transport,
    project_ws_job::{cancel_project_ws_job, get_or_start_project_ws_job},
    project_ws_protocol::{
        is_terminal_project_ws_message, parse_project_message, project_client_request_id,
        task_control_event,
    },
    store::{ProjectAccess, PublicUser},
    tools,
    types::AppState,
};

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub template: Option<String>,
}

#[derive(Deserialize)]
pub struct FriendSearchQuery {
    pub phone: String,
}

#[derive(Deserialize)]
pub struct AddFriendRequest {
    pub phone: String,
}

#[derive(Deserialize)]
pub struct FriendMessagesQuery {
    pub after: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct SendFriendMessageRequest {
    pub content: String,
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

pub async fn list_friends(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    match state.store.list_friends(&user.id) {
        Ok(friends) => Json(serde_json::json!({ "friends": friends })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn search_friend_by_phone(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<FriendSearchQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    match state.store.search_friend_by_phone(&user.id, &query.phone) {
        Ok(Some(result)) => Json(serde_json::json!({
            "found": true,
            "user": result.user,
            "already_friend": result.already_friend,
            "is_self": result.is_self,
        }))
        .into_response(),
        Ok(None) => Json(serde_json::json!({ "found": false })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn add_friend_by_phone(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<AddFriendRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    match state.store.add_friend_by_phone(&user.id, &req.phone) {
        Ok(result) => Json(serde_json::json!({
            "friend": result.friend,
            "already_friend": result.already_friend,
        }))
        .into_response(),
        Err(e) => {
            let message = e.to_string();
            let status = if message.contains("未找到") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, message)
        }
    }
}

pub async fn list_friend_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(friend_id): AxumPath<String>,
    Query(query): Query<FriendMessagesQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    match state.store.list_friend_messages(
        &user.id,
        &friend_id,
        query.after.as_deref(),
        query.limit.unwrap_or(80),
    ) {
        Ok(messages) => Json(serde_json::json!({ "messages": messages })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn send_friend_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(friend_id): AxumPath<String>,
    Json(req): Json<SendFriendMessageRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    match state
        .store
        .send_friend_message(&user.id, &friend_id, &req.content)
    {
        Ok(message) => Json(serde_json::json!({ "message": message })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
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
    // ── APK 最低版本门控 ────────────────────────────────────────────────────
    if state.min_apk_version_code > 0 {
        let client_vc = query
            .get("app_version_code")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);
        if client_vc > 0 && client_vc < state.min_apk_version_code {
            return json_error(
                StatusCode::UPGRADE_REQUIRED,
                format!(
                    "您的 APP 版本过低（当前 {}，最低要求 {}），请前往 {}/app/download 升级后再使用",
                    client_vc, state.min_apk_version_code, state.public_url
                ),
            );
        }
    }

    // ── 身份验证（REQUIRE_LOGIN=true 时强制） ───────────────────────────────
    let (user, project) = if state.require_login {
        let token = query.get("token").map(String::as_str).unwrap_or("");
        let authed_user = match state.store.authenticate_token(token) {
            Ok(u) => u,
            Err(e) => {
                return json_error(
                    StatusCode::UNAUTHORIZED,
                    format!("请先登录后再使用（{}）", e),
                );
            }
        };
        // token 验证通过后，用 token 对应的用户身份获取/创建项目
        match ensure_mobile_project(
            &state,
            &authed_user.id,
            &project_id,
            query.get("title").map(String::as_str),
        ) {
            Ok(pair) => pair,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        }
    } else {
        // 旧兼容模式：device UUID 直接当 user_id，自动创建账号
        match ensure_mobile_project(
            &state,
            &user_id,
            &project_id,
            query.get("title").map(String::as_str),
        ) {
            Ok(pair) => pair,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        }
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
        let attachments = request.attachments.clone();
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
            attachments,
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
