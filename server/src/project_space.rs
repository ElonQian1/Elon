/// project_space.rs — 项目空间频道 API
///
/// 这是商城“加入项目”之后的协作空间入口。普通频道消息写入共享频道；
/// AI 开发频道可以把一次成员发起的开发任务写回同一频道，供项目成员共同跟进。
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error, project_access},
    project_channel_summary::{ChannelSummaryTask, spawn_channel_summary},
    project_chat::run_project_agent_with_scheduler,
    project_keys::clean_trace_id,
    types::AppState,
};

#[derive(Deserialize)]
pub struct ChannelMessagesQuery {
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct SendChannelMessageRequest {
    pub content: String,
}

#[derive(Deserialize)]
pub struct StartChannelAiTaskRequest {
    pub content: String,
    pub agent: Option<String>,
    pub trace_id: Option<String>,
}

#[derive(Deserialize)]
pub struct SummarizeChannelSelectionRequest {
    pub post_content: String,
    pub summary_prompt: String,
    pub agent: Option<String>,
    pub trace_id: Option<String>,
}

pub async fn get_project_space(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match state.store.project_space_summary(&user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    let channels = match state
        .store
        .list_project_space_channels(&user.id, &project_id)
    {
        Ok(channels) => channels,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let members = match state.store.list_project_members(&project_id) {
        Ok(members) => members,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    Json(serde_json::json!({
        "project": project,
        "channels": channels,
        "members": members,
    }))
    .into_response()
}

pub async fn list_channel_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id)): Path<(String, String)>,
    Query(query): Query<ChannelMessagesQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if let Err(e) = project_access(&state, &user.id, &project_id) {
        return json_error(StatusCode::FORBIDDEN, e.to_string());
    }
    match state.store.list_project_channel_messages(
        &user.id,
        &project_id,
        &channel_id,
        query.limit.unwrap_or(120),
    ) {
        Ok(messages) => Json(serde_json::json!({ "messages": messages })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn send_channel_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id)): Path<(String, String)>,
    Json(req): Json<SendChannelMessageRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if let Err(e) = project_access(&state, &user.id, &project_id) {
        return json_error(StatusCode::FORBIDDEN, e.to_string());
    }
    match state.store.insert_project_channel_message(
        &project_id,
        &channel_id,
        Some(&user.id),
        "text",
        &req.content,
        None,
    ) {
        Ok(message) => Json(serde_json::json!({ "message": message })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn start_channel_ai_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id)): Path<(String, String)>,
    Json(req): Json<StartChannelAiTaskRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    if !can_start_channel_ai(&project.role) {
        return json_error(StatusCode::FORBIDDEN, "当前成员角色不能发起项目 AI 开发");
    }
    let channel_kind = match state
        .store
        .get_project_channel_kind(&project_id, &channel_id)
    {
        Ok(kind) => kind,
        Err(e) => return json_error(StatusCode::NOT_FOUND, e.to_string()),
    };
    if channel_kind != "ai_development" {
        return json_error(
            StatusCode::BAD_REQUEST,
            "只有 AI开发 频道可以发起项目 AI 开发任务",
        );
    }
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "content 不能为空");
    }

    let conversation_id = format!("channel-{}", channel_id);
    let conversation_title = format!("项目频道 {}", channel_id);
    let conversation_id = match state.store.ensure_conversation(
        &project.id,
        &user.id,
        Some(&conversation_id),
        Some(&conversation_title),
    ) {
        Ok(id) => id,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let task_id =
        match state
            .store
            .create_task(&project.id, &user.id, Some(&conversation_id), &content)
        {
            Ok(id) => id,
            Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
    let trace_id = clean_trace_id(req.trace_id.as_deref());
    let task_message = match state.store.insert_project_channel_message(
        &project_id,
        &channel_id,
        Some(&user.id),
        "ai_task",
        &format!("发起 AI 开发任务：{}", content),
        Some(&task_id),
    ) {
        Ok(message) => message,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };

    spawn_channel_ai_task(ChannelAiTask {
        state: state.clone(),
        user_id: user.id,
        project,
        project_id,
        channel_id,
        conversation_id,
        task_id: task_id.clone(),
        content,
        agent: req.agent,
        trace_id: trace_id.clone(),
    });

    Json(serde_json::json!({
        "task_id": task_id,
        "trace_id": trace_id,
        "message": task_message,
    }))
    .into_response()
}

pub async fn summarize_channel_selection(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id)): Path<(String, String)>,
    Json(req): Json<SummarizeChannelSelectionRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    let post_content = req.post_content.trim().to_string();
    let summary_prompt = req.summary_prompt.trim().to_string();
    if post_content.is_empty() || summary_prompt.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "summary content 不能为空");
    }

    let post_message = match state.store.insert_project_channel_message(
        &project_id,
        &channel_id,
        Some(&user.id),
        "text",
        &post_content,
        None,
    ) {
        Ok(message) => message,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let trace_id = clean_trace_id(req.trace_id.as_deref());
    spawn_channel_summary(ChannelSummaryTask {
        state: state.clone(),
        user_id: user.id,
        project,
        project_id,
        channel_id,
        prompt: summary_prompt,
        agent: req.agent,
        trace_id: trace_id.clone(),
    });

    Json(serde_json::json!({
        "trace_id": trace_id,
        "message": post_message,
    }))
    .into_response()
}

struct ChannelAiTask {
    state: Arc<AppState>,
    user_id: String,
    project: crate::store::ProjectAccess,
    project_id: String,
    channel_id: String,
    conversation_id: String,
    task_id: String,
    content: String,
    agent: Option<String>,
    trace_id: String,
}

fn spawn_channel_ai_task(task: ChannelAiTask) {
    tokio::spawn(async move {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let run_state = task.state.clone();
        let run_project = task.project.clone();
        let run_user_id = task.user_id.clone();
        let run_conversation_id = task.conversation_id.clone();
        let run_content = task.content.clone();
        let run_agent = task.agent.clone();
        let run_trace_id = task.trace_id.clone();
        let download_base = format!(
            "{}/api/projects/{}/download",
            task.state.public_url, task.project.id
        );
        let runner = tokio::spawn(async move {
            run_project_agent_with_scheduler(
                run_state,
                run_user_id,
                run_project,
                download_base,
                run_conversation_id,
                run_content,
                run_agent,
                None,
                Some(run_trace_id),
                tx,
            )
            .await;
        });

        let mut final_reply = String::new();
        let mut final_status = "done".to_string();
        let mut apk_url = None;
        let mut error = None;
        while let Some(raw) = rx.recv().await {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
                let event_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let message = value
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                match event_type {
                    "progress" if !message.is_empty() => {
                        let _ = task.state.store.insert_project_channel_message(
                            &task.project_id,
                            &task.channel_id,
                            None,
                            "ai_progress",
                            message,
                            Some(&task.task_id),
                        );
                    }
                    "done" => {
                        final_reply = message.if_blank("AI 开发任务已完成。").to_string();
                        apk_url = value
                            .get("apk_url")
                            .and_then(|v| v.as_str())
                            .map(ToOwned::to_owned);
                        let result = result_message(message, apk_url.as_deref(), None);
                        let _ = task.state.store.insert_project_channel_message(
                            &task.project_id,
                            &task.channel_id,
                            None,
                            "ai_result",
                            &result,
                            Some(&task.task_id),
                        );
                    }
                    "error" => {
                        final_status = "failed".to_string();
                        let msg = message.if_blank("AI 开发任务失败。").to_string();
                        final_reply = msg.clone();
                        error = Some(msg.clone());
                        let _ = task.state.store.insert_project_channel_message(
                            &task.project_id,
                            &task.channel_id,
                            None,
                            "ai_result",
                            &result_message(&msg, None, Some("失败")),
                            Some(&task.task_id),
                        );
                    }
                    _ => {}
                }
            }
        }
        let _ = runner.await;
        if final_reply.is_empty() {
            final_reply = "AI 开发任务已结束。".to_string();
        }
        let _ = task.state.store.finish_task(
            &task.task_id,
            &final_status,
            Some(&final_reply),
            apk_url.as_deref(),
            error.as_deref(),
        );
    });
}

fn can_start_channel_ai(role: &str) -> bool {
    matches!(role, "owner" | "editor" | "member")
}

fn result_message(message: &str, apk_url: Option<&str>, status: Option<&str>) -> String {
    let mut parts = Vec::new();
    if let Some(status) = status {
        parts.push(format!("AI 开发任务{}。", status));
    }
    if !message.trim().is_empty() {
        parts.push(message.trim().to_string());
    }
    if let Some(apk_url) = apk_url.filter(|value| !value.is_empty()) {
        parts.push(format!("APK 下载：{}", apk_url));
    }
    parts.join("\n")
}

trait BlankFallback {
    fn if_blank<'a>(&'a self, fallback: &'a str) -> &'a str;
}

impl BlankFallback for str {
    fn if_blank<'a>(&'a self, fallback: &'a str) -> &'a str {
        if self.trim().is_empty() {
            fallback
        } else {
            self
        }
    }
}
