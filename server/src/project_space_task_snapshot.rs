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
use homecli_proto::AgentToServer;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};

use crate::{
    project_auth::{auth_from_headers, json_error, project_access},
    project_mobile::ensure_mobile_project,
    project_space,
    store::{ProjectAccess, ProjectChannelMessage, TaskEventRecord, TaskSnapshot},
    types::AppState,
};

pub(super) const TASK_MESSAGE_LIMIT: i64 = 200;
pub(super) const DEFAULT_EVENT_LIMIT: usize = 200;
pub(super) const LOCAL_JOURNAL_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
pub(super) const LOCAL_JOURNAL_PROBE_RETRY_DELAY: Duration = Duration::from_millis(400);

#[derive(Deserialize)]
pub struct TaskSnapshotQuery {
    pub since: Option<i64>,
    pub limit: Option<usize>,
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct TaskAttachState {
    status: &'static str,
    live: bool,
    reason: &'static str,
}

#[derive(Debug, Serialize)]
pub(super) struct RuntimeEventView {
    seq: i64,
    created_at: String,
    event: Value,
}

#[derive(Debug, Serialize)]
pub(super) struct RuntimeEventsResponse {
    task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pc_req_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_id: Option<String>,
    events: Vec<RuntimeEventView>,
    last_event_seq: i64,
    has_more: bool,
    attach: Value,
    cloud_attach: TaskAttachState,
    local_journal: LocalJournalProbe,
    #[serde(skip_serializing_if = "Option::is_none")]
    resume: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    approval_state: Option<Value>,
}

#[derive(Debug, Serialize)]
pub(super) struct RuntimeSnapshotResponse {
    task: TaskSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pc_req_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_id: Option<String>,
    messages: Vec<ProjectChannelMessage>,
    events: Vec<RuntimeEventView>,
    last_event_seq: i64,
    has_more: bool,
    attach: Value,
    cloud_attach: TaskAttachState,
    local_journal: LocalJournalProbe,
    #[serde(skip_serializing_if = "Option::is_none")]
    resume: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    approval_state: Option<Value>,
}

pub(super) struct TaskSnapshotContext {
    project: ProjectAccess,
    task: TaskSnapshot,
    messages: Vec<ProjectChannelMessage>,
    events: Vec<TaskEventRecord>,
    last_event_seq: i64,
    pc_dispatch: Option<PcDispatchContext>,
}

#[derive(Debug, Clone)]
pub(super) struct PcDispatchContext {
    pc_req_id: String,
    agent_id: Option<String>,
    node_display_name: Option<String>,
    cli: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct LocalJournalProbe {
    status: String,
    reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pc_req_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    node_display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cli: Option<String>,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<Value>,
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
    .await
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
    .await
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
    .await
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
    .await
}

async fn snapshot_response(
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
    let cloud_attach = attach_state(&context.project.id, &channel_id, &context.task);
    let local_journal =
        resolve_local_journal(&state, context.pc_dispatch.as_ref(), since, limit).await;
    let attach = local_journal
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.get("attach").cloned())
        .unwrap_or_else(|| serde_json::to_value(&cloud_attach).unwrap_or_else(|_| json!({})));
    let resume = local_journal
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.get("resume").cloned());
    let approval_state = local_journal
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.get("approval_state").cloned());
    let pc_req_id = context
        .pc_dispatch
        .as_ref()
        .map(|dispatch| dispatch.pc_req_id.clone());
    let agent_id = context
        .pc_dispatch
        .as_ref()
        .and_then(|dispatch| dispatch.agent_id.clone());
    let events = render_events(context.events);
    let returned_last_seq = events.last().map(|event| event.seq).unwrap_or(since.max(0));
    Json(RuntimeSnapshotResponse {
        task: context.task,
        pc_req_id,
        agent_id,
        messages: context.messages,
        events,
        last_event_seq: context.last_event_seq,
        has_more: context.last_event_seq > returned_last_seq,
        cloud_attach,
        attach,
        local_journal,
        resume,
        approval_state,
    })
    .into_response()
}

async fn events_response(
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
    let cloud_attach = attach_state(&context.project.id, &channel_id, &context.task);
    let local_journal =
        resolve_local_journal(&state, context.pc_dispatch.as_ref(), since, limit).await;
    let attach = local_journal
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.get("attach").cloned())
        .unwrap_or_else(|| serde_json::to_value(&cloud_attach).unwrap_or_else(|_| json!({})));
    let resume = local_journal
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.get("resume").cloned());
    let approval_state = local_journal
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.get("approval_state").cloned());
    let pc_req_id = context
        .pc_dispatch
        .as_ref()
        .map(|dispatch| dispatch.pc_req_id.clone());
    let agent_id = context
        .pc_dispatch
        .as_ref()
        .and_then(|dispatch| dispatch.agent_id.clone());
    let events = render_events(context.events);
    let returned_last_seq = events.last().map(|event| event.seq).unwrap_or(since.max(0));
    Json(RuntimeEventsResponse {
        task_id: context.task.id,
        pc_req_id,
        agent_id,
        events,
        last_event_seq: context.last_event_seq,
        has_more: context.last_event_seq > returned_last_seq,
        cloud_attach,
        attach,
        local_journal,
        resume,
        approval_state,
    })
    .into_response()
}

mod helpers;
use self::helpers::*;
