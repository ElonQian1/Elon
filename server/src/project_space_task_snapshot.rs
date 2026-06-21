// server/src/project_space_task_snapshot.rs
//
// PC 开发任务恢复控制面：提供可持久重放的 task snapshot / events-since API。
// 这里不尝试伪装成“已接回原进程”，只把任务、频道消息和事件游标稳定暴露给前端。
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error, project_access},
    project_mobile::ensure_mobile_project,
    project_space,
    store::{ProjectAccess, ProjectChannelMessage, TaskEventRecord, TaskSnapshot},
    types::AppState,
};

const TASK_MESSAGE_LIMIT: i64 = 200;
const DEFAULT_EVENT_LIMIT: usize = 200;

#[derive(Deserialize)]
pub struct TaskSnapshotQuery {
    pub since: Option<i64>,
    pub limit: Option<usize>,
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
struct TaskAttachState {
    status: &'static str,
    live: bool,
    reason: &'static str,
}

#[derive(Debug, Serialize)]
struct RuntimeEventView {
    seq: i64,
    created_at: String,
    event: Value,
}

#[derive(Debug, Serialize)]
struct RuntimeEventsResponse {
    task_id: String,
    events: Vec<RuntimeEventView>,
    last_event_seq: i64,
    has_more: bool,
    attach: TaskAttachState,
}

#[derive(Debug, Serialize)]
struct RuntimeSnapshotResponse {
    task: TaskSnapshot,
    messages: Vec<ProjectChannelMessage>,
    events: Vec<RuntimeEventView>,
    last_event_seq: i64,
    has_more: bool,
    attach: TaskAttachState,
}

struct TaskSnapshotContext {
    project: ProjectAccess,
    task: TaskSnapshot,
    messages: Vec<ProjectChannelMessage>,
    events: Vec<TaskEventRecord>,
    last_event_seq: i64,
}

pub async fn snapshot_channel_ai_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id, task_id)): Path<(String, String, String)>,
    Query(query): Query<TaskSnapshotQuery>,
) -> Response {
    let (user_id, project) = match project_from_auth(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    snapshot_response(
        state,
        user_id,
        project,
        channel_id,
        task_id,
        query.since.unwrap_or(0),
        query.limit.unwrap_or(DEFAULT_EVENT_LIMIT),
    )
}

pub async fn list_channel_ai_task_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id, task_id)): Path<(String, String, String)>,
    Query(query): Query<TaskSnapshotQuery>,
) -> Response {
    let (user_id, project) = match project_from_auth(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    events_response(
        state,
        user_id,
        project,
        channel_id,
        task_id,
        query.since.unwrap_or(0),
        query.limit.unwrap_or(DEFAULT_EVENT_LIMIT),
    )
}

pub async fn snapshot_user_channel_ai_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id, channel_id, task_id)): Path<(String, String, String, String)>,
    Query(query): Query<TaskSnapshotQuery>,
) -> Response {
    let (user_id, project) = match user_project_from_auth(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.title.as_deref(),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    snapshot_response(
        state,
        user_id,
        project,
        channel_id,
        task_id,
        query.since.unwrap_or(0),
        query.limit.unwrap_or(DEFAULT_EVENT_LIMIT),
    )
}

pub async fn list_user_channel_ai_task_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id, channel_id, task_id)): Path<(String, String, String, String)>,
    Query(query): Query<TaskSnapshotQuery>,
) -> Response {
    let (user_id, project) = match user_project_from_auth(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.title.as_deref(),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    events_response(
        state,
        user_id,
        project,
        channel_id,
        task_id,
        query.since.unwrap_or(0),
        query.limit.unwrap_or(DEFAULT_EVENT_LIMIT),
    )
}

fn snapshot_response(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    channel_id: String,
    task_id: String,
    since: i64,
    limit: usize,
) -> Response {
    let context = match load_task_snapshot_context(
        &state,
        &user_id,
        project,
        &channel_id,
        &task_id,
        since,
        limit,
    ) {
        Ok(context) => context,
        Err(response) => return response,
    };
    let attach = attach_state(&context.project.id, &channel_id, &context.task);
    let events = render_events(context.events);
    let returned_last_seq = events.last().map(|event| event.seq).unwrap_or(since.max(0));
    Json(RuntimeSnapshotResponse {
        task: context.task,
        messages: context.messages,
        events,
        last_event_seq: context.last_event_seq,
        has_more: context.last_event_seq > returned_last_seq,
        attach,
    })
    .into_response()
}

fn events_response(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    channel_id: String,
    task_id: String,
    since: i64,
    limit: usize,
) -> Response {
    let context = match load_task_snapshot_context(
        &state,
        &user_id,
        project,
        &channel_id,
        &task_id,
        since,
        limit,
    ) {
        Ok(context) => context,
        Err(response) => return response,
    };
    let attach = attach_state(&context.project.id, &channel_id, &context.task);
    let events = render_events(context.events);
    let returned_last_seq = events.last().map(|event| event.seq).unwrap_or(since.max(0));
    Json(RuntimeEventsResponse {
        task_id: context.task.id,
        events,
        last_event_seq: context.last_event_seq,
        has_more: context.last_event_seq > returned_last_seq,
        attach,
    })
    .into_response()
}

fn load_task_snapshot_context(
    state: &AppState,
    user_id: &str,
    project: ProjectAccess,
    channel_id: &str,
    task_id: &str,
    since: i64,
    limit: usize,
) -> Result<TaskSnapshotContext, Response> {
    let channel_kind = state
        .store
        .get_project_channel_kind(&project.id, channel_id)
        .map_err(|e| json_error(StatusCode::NOT_FOUND, e.to_string()))?;
    if channel_kind != "ai_development" {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "只有 AI开发 频道支持任务快照",
        ));
    }
    let task = state
        .store
        .get_channel_task_snapshot(&project.id, channel_id, task_id)
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "任务不存在或不属于该频道"))?;
    let messages = state
        .store
        .list_project_channel_messages(user_id, &project.id, channel_id, TASK_MESSAGE_LIMIT)
        .map_err(|e| json_error(StatusCode::BAD_REQUEST, e.to_string()))?;
    let events = state
        .store
        .list_task_events_after(task_id, since, limit)
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let last_event_seq = state
        .store
        .latest_task_event_seq(task_id)
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(TaskSnapshotContext {
        project,
        task,
        messages,
        events,
        last_event_seq,
    })
}

fn project_from_auth(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<(String, ProjectAccess), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|e| json_error(StatusCode::UNAUTHORIZED, e.to_string()))?;
    let project = project_access(state, &user.id, project_id)
        .map_err(|e| json_error(StatusCode::FORBIDDEN, e.to_string()))?;
    Ok((user.id, project))
}

fn user_project_from_auth(
    state: &AppState,
    headers: &HeaderMap,
    user_id: &str,
    project_id: &str,
    project_title: Option<&str>,
) -> Result<(String, ProjectAccess), Response> {
    let effective_user_id = if state.require_login {
        auth_from_headers(state, headers)
            .map_err(|e| json_error(StatusCode::UNAUTHORIZED, e.to_string()))?
            .id
    } else {
        user_id.to_string()
    };
    ensure_mobile_project(state, &effective_user_id, project_id, project_title)
        .map(|(user, project)| (user.id, project))
        .map_err(|e| json_error(StatusCode::BAD_REQUEST, e.to_string()))
}

fn attach_state(project_id: &str, channel_id: &str, task: &TaskSnapshot) -> TaskAttachState {
    if project_space::is_channel_ai_task_active(&task.id, project_id, channel_id) {
        return TaskAttachState {
            status: "live",
            live: true,
            reason: "服务端仍持有该任务的运行控制句柄",
        };
    }
    if task.status == "running" {
        return TaskAttachState {
            status: "detached",
            live: false,
            reason: "任务仍标记为运行中，但当前服务端没有运行控制句柄；需要重新 attach 或继续",
        };
    }
    TaskAttachState {
        status: "terminal",
        live: false,
        reason: "任务已经进入终态，只能基于快照继续新一轮处理",
    }
}

fn render_events(events: Vec<TaskEventRecord>) -> Vec<RuntimeEventView> {
    events
        .into_iter()
        .map(|event| RuntimeEventView {
            seq: event.seq,
            created_at: event.created_at,
            event: serde_json::from_str(&event.event_json)
                .unwrap_or_else(|_| json!({ "type": "invalid_json", "raw": event.event_json })),
        })
        .collect()
}
