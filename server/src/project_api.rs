use axum::{
    Json,
    extract::{
        Path as AxumPath, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
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
    time::Duration,
};
use tokio::sync::{Mutex, broadcast, watch};

use crate::{
    project_attachments::append_project_attachment_notes,
    project_auth::{
        LoginRequest, RegisterRequest, auth_from_headers, can_edit, json_error, login_inner,
        project_access, register_inner,
    },
    project_chat::run_project_agent_with_scheduler,
    project_keys::{clean_trace_id, project_ws_fingerprint, project_ws_job_key},
    project_mobile::ensure_mobile_project,
    project_trace_events::{record_server_message, record_server_transport},
    project_ws_protocol::{
        PROJECT_WS_BACKLOG_LIMIT, enrich_project_ws_event, is_terminal_project_ws_message,
        is_terminal_task_status, parse_project_message, project_client_request_id,
        task_control_event, terminal_backlog_from_task,
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
