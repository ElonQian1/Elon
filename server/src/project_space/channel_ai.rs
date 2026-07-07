use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

use crate::{
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    project_auth::{auth_from_headers, json_error, project_access},
    project_channel_summary::{spawn_channel_summary, ChannelSummaryTask},
    project_keys::clean_trace_id,
    project_space_task_control::take_channel_ai_task_control,
    project_tool_approval_recovery, project_tool_approvals, project_workspace_recovery,
    store::{ProjectAccess, CHANNEL_PERMISSION_START_AI, CHANNEL_PERMISSION_VIEW},
    types::AppState,
};

use super::{
    ensure_project_member_can_speak, ensure_user_project_for_space, project_member_can_use_channel,
    project_space_access, publish_channel_message_updated, DOCS_CHANNEL_KIND,
};

use super::channel_ai_spawn::{spawn_channel_ai_task, BlankFallback, ChannelAiTask};

#[derive(Deserialize)]
pub struct StartChannelAiTaskRequest {
    pub content: String,
    pub agent: Option<String>,
    #[serde(default, alias = "runtimeRoute", alias = "pcRoute", alias = "pc_route")]
    pub runtime_route: Option<String>,
    #[serde(default, alias = "conversationId")]
    pub conversation_id: Option<String>,
    #[serde(default, alias = "conversationTitle")]
    pub conversation_title: Option<String>,
    #[serde(
        default,
        alias = "localNodeId",
        alias = "currentNodeId",
        alias = "preferredNodeId",
        alias = "nodeId"
    )]
    pub local_node_id: Option<String>,
    #[serde(
        default,
        alias = "localWorkspacePath",
        alias = "currentWorkspacePath",
        alias = "preferredWorkspacePath",
        alias = "workspacePath"
    )]
    pub local_workspace_path: Option<String>,
    #[serde(default, alias = "directPcCli", alias = "pcDirectCli")]
    pub direct_pc_cli: Option<bool>,
    pub trace_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ToolApprovalDecisionRequest {
    pub decision: String,
}

#[derive(Deserialize)]
pub struct SummarizeChannelSelectionRequest {
    pub post_content: String,
    pub summary_prompt: String,
    pub agent: Option<String>,
    #[serde(default, alias = "runtimeRoute", alias = "pcRoute", alias = "pc_route")]
    pub runtime_route: Option<String>,
    pub trace_id: Option<String>,
}

pub async fn start_user_project_channel_ai_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id, channel_id)): Path<(String, String, String)>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<StartChannelAiTaskRequest>,
) -> Response {
    let (user, project) = match ensure_user_project_for_space(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    start_channel_ai_task_response(state, user.id, project, channel_id, req, true).await
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
    start_channel_ai_task_response(state, user.id, project, channel_id, req, false).await
}

pub async fn cancel_user_project_channel_ai_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id, channel_id, task_id)): Path<(String, String, String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let (user, project) = match ensure_user_project_for_space(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    cancel_channel_ai_task_response(state, user.id, project, channel_id, task_id)
}

pub async fn cancel_channel_ai_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id, task_id)): Path<(String, String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    cancel_channel_ai_task_response(state, user.id, project, channel_id, task_id)
}

pub async fn decide_channel_ai_tool_approval(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id, task_id, approval_id)): Path<(String, String, String, String)>,
    Json(req): Json<ToolApprovalDecisionRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    decide_channel_ai_tool_approval_response(
        state,
        user.id,
        project,
        channel_id,
        task_id,
        approval_id,
        req.decision,
    )
    .await
}

pub async fn decide_user_project_channel_ai_tool_approval(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id, channel_id, task_id, approval_id)): Path<(
        String,
        String,
        String,
        String,
        String,
    )>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<ToolApprovalDecisionRequest>,
) -> Response {
    let (user, project) = match ensure_user_project_for_space(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    decide_channel_ai_tool_approval_response(
        state,
        user.id,
        project,
        channel_id,
        task_id,
        approval_id,
        req.decision,
    )
    .await
}

async fn decide_channel_ai_tool_approval_response(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    channel_id: String,
    task_id: String,
    approval_id: String,
    decision: String,
) -> Response {
    if !project_member_can_use_channel(
        &state,
        &project.id,
        &channel_id,
        &user_id,
        CHANNEL_PERMISSION_START_AI,
    ) {
        return json_error(
            StatusCode::FORBIDDEN,
            "当前成员角色不能审批项目 AI 工具调用",
        );
    }
    let project_id = project.id.clone();
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
            "只有 AI开发 频道可以审批项目 AI 工具调用",
        );
    }
    let claim = match claim_channel_ai_tool_approval(
        &state,
        &project_id,
        &channel_id,
        &task_id,
        &approval_id,
        &decision,
    ) {
        Ok(claim) => claim,
        Err(e) => {
            let status = match e.kind() {
                project_tool_approvals::ToolApprovalErrorKind::BadRequest => {
                    StatusCode::BAD_REQUEST
                }
                project_tool_approvals::ToolApprovalErrorKind::Conflict => StatusCode::CONFLICT,
                project_tool_approvals::ToolApprovalErrorKind::NotFound => StatusCode::NOT_FOUND,
            };
            return json_error(status, e.to_string());
        }
    };
    let target = match claim {
        project_tool_approvals::ToolApprovalClaim::Dispatch(target) => target,
        project_tool_approvals::ToolApprovalClaim::AlreadyDecided { decision } => {
            return Json(serde_json::json!({
                "ok": true,
                "task_id": task_id,
                "approval_id": approval_id,
                "decision": decision,
                "status": "already_decided",
            }))
            .into_response();
        }
    };
    let accepted = match state
        .agent_manager
        .send_tool_approval_decision(&target.req_id, &approval_id, &target.decision)
        .await
    {
        Ok(accepted) => accepted,
        Err(e) => {
            project_tool_approvals::mark_dispatch_failed(&task_id, &approval_id, &target.decision);
            return json_error(StatusCode::CONFLICT, e.to_string());
        }
    };
    if !accepted {
        project_tool_approvals::mark_dispatch_failed(&task_id, &approval_id, &target.decision);
        return json_error(StatusCode::CONFLICT, "PC 节点未接受该工具审批决定，请重试");
    }
    project_tool_approvals::mark_decided(&task_id, &approval_id, &target.decision);
    Json(serde_json::json!({
        "ok": true,
        "accepted": true,
        "task_id": task_id,
        "approval_id": approval_id,
        "decision": target.decision,
        "status": "sent",
    }))
    .into_response()
}

fn claim_channel_ai_tool_approval(
    state: &AppState,
    project_id: &str,
    channel_id: &str,
    task_id: &str,
    approval_id: &str,
    decision: &str,
) -> Result<project_tool_approvals::ToolApprovalClaim, project_tool_approvals::ToolApprovalError> {
    match project_tool_approvals::claim_decision_target(
        project_id,
        channel_id,
        task_id,
        approval_id,
        decision,
    ) {
        Err(e) if e.kind() == project_tool_approvals::ToolApprovalErrorKind::NotFound => {
            // 服务端重启或单进程内存丢失后，用已落库的任务事件重建审批状态。
            if let Ok(events) = state.store.list_task_events(task_id, 1000) {
                project_tool_approval_recovery::recover_from_task_events(
                    project_id, channel_id, task_id, &events,
                );
            }
            project_tool_approvals::claim_decision_target(
                project_id,
                channel_id,
                task_id,
                approval_id,
                decision,
            )
        }
        result => result,
    }
}

fn cancel_channel_ai_task_response(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    channel_id: String,
    task_id: String,
) -> Response {
    if !project_member_can_use_channel(
        &state,
        &project.id,
        &channel_id,
        &user_id,
        CHANNEL_PERMISSION_START_AI,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前成员角色不能停止项目 AI 开发");
    }
    let project_id = project.id.clone();
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
            "只有 AI开发 频道可以停止项目 AI 开发任务",
        );
    }
    let control = match take_channel_ai_task_control(&task_id, &project_id, &channel_id) {
        Some(control) => control,
        None => return json_error(StatusCode::NOT_FOUND, "任务不在运行中或已结束"),
    };
    control.request_cancel();
    Json(serde_json::json!({
        "ok": true,
        "task_id": task_id,
        "status": "cancel_requested",
    }))
    .into_response()
}

async fn start_channel_ai_task_response(
    state: Arc<AppState>,
    user_id: String,
    mut project: ProjectAccess,
    channel_id: String,
    req: StartChannelAiTaskRequest,
    use_user_download_route: bool,
) -> Response {
    if !project_member_can_use_channel(
        &state,
        &project.id,
        &channel_id,
        &user_id,
        CHANNEL_PERMISSION_VIEW,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权查看该频道");
    }
    if !project_member_can_use_channel(
        &state,
        &project.id,
        &channel_id,
        &user_id,
        CHANNEL_PERMISSION_START_AI,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前成员角色不能发起项目 AI 开发");
    }
    let project_id = project.id.clone();
    if let Err(response) = ensure_project_member_can_speak(&state, &project_id, &user_id) {
        return response;
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
    let runtime_route = match req.runtime_route.as_deref() {
        Some(value) => match PcRuntimeRoutePreference::from_request(value) {
            Ok(route) => route,
            Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
        },
        None => None,
    };
    let direct_pc_cli = req.direct_pc_cli.unwrap_or(false);
    if should_auto_bind_local_node(runtime_route) {
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
                    &user_id,
                    &project.id,
                    &project.role,
                    node_id,
                    workspace_path,
                )
                .await
                {
                    Ok(_) => {
                        if let Ok(updated) = project_access(&state, &user_id, &project_id) {
                            project = updated;
                        }
                    }
                    Err((status, message)) => return json_error(status, message),
                }
            }
        }
    }

    let fallback_conversation_id = format!("channel-{}", channel_id);
    let fallback_conversation_title = format!("项目频道 {}", channel_id);
    let conversation_id_hint = req
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback_conversation_id.as_str());
    let conversation_title_hint = req
        .conversation_title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback_conversation_title.as_str());
    let conversation_id = match state.store.ensure_conversation(
        &project.id,
        &user_id,
        Some(conversation_id_hint),
        Some(conversation_title_hint),
    ) {
        Ok(id) => id,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let task_id =
        match state
            .store
            .create_task(&project.id, &user_id, Some(&conversation_id), &content)
        {
            Ok(id) => id,
            Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
    let trace_id = clean_trace_id(req.trace_id.as_deref());
    let download_base = if use_user_download_route {
        format!(
            "{}/api/user/{}/projects/{}/download",
            state.public_url, user_id, project.id
        )
    } else {
        format!("{}/api/projects/{}/download", state.public_url, project.id)
    };
    let task_message = match state.store.insert_project_channel_message(
        &project_id,
        &channel_id,
        Some(&user_id),
        "ai_task",
        &format!("发起 AI 开发任务：{}", content),
        Some(&task_id),
        None,
    ) {
        Ok(message) => message,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    publish_channel_message_updated(
        state.as_ref(),
        &project_id,
        &channel_id,
        Some(&conversation_id),
        Some(&task_id),
        "ai_task",
    );

    spawn_channel_ai_task(ChannelAiTask {
        state: state.clone(),
        user_id,
        project,
        project_id,
        channel_id,
        conversation_id: conversation_id.clone(),
        task_id: task_id.clone(),
        download_base,
        content,
        agent: req.agent,
        runtime_route,
        direct_pc_cli,
        trace_id: trace_id.clone(),
    });

    Json(serde_json::json!({
        "task_id": task_id,
        "trace_id": trace_id,
        "conversation_id": conversation_id,
        "message": task_message,
    }))
    .into_response()
}

fn should_auto_bind_local_node(route: Option<PcRuntimeRoutePreference>) -> bool {
    matches!(
        route,
        Some(PcRuntimeRoutePreference::RouteA | PcRuntimeRoutePreference::RouteB)
    )
}

pub async fn summarize_user_project_channel_selection(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id, channel_id)): Path<(String, String, String)>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<SummarizeChannelSelectionRequest>,
) -> Response {
    let (user, project) = match ensure_user_project_for_space(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    summarize_channel_selection_response(state, user.id, project, channel_id, req)
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
    summarize_channel_selection_response(state, user.id, project, channel_id, req)
}

fn summarize_channel_selection_response(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    channel_id: String,
    req: SummarizeChannelSelectionRequest,
) -> Response {
    let project_id = project.id.clone();
    let post_content = req.post_content.trim().to_string();
    let summary_prompt = req.summary_prompt.trim().to_string();
    if post_content.is_empty() || summary_prompt.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "summary content 不能为空");
    }
    if let Err(response) = ensure_project_member_can_speak(&state, &project_id, &user_id) {
        return response;
    }
    let channel_kind = match state
        .store
        .get_project_channel_kind(&project_id, &channel_id)
    {
        Ok(kind) => kind,
        Err(e) => return json_error(StatusCode::NOT_FOUND, e.to_string()),
    };
    if channel_kind == DOCS_CHANNEL_KIND {
        return json_error(
            StatusCode::BAD_REQUEST,
            "文档频道是固定只读频道，不能发帖总结",
        );
    }
    let runtime_route = match req.runtime_route.as_deref() {
        Some(value) => match PcRuntimeRoutePreference::from_request(value) {
            Ok(route) => route,
            Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
        },
        None => None,
    };

    let post_message = match state.store.insert_project_channel_message(
        &project_id,
        &channel_id,
        Some(&user_id),
        "text",
        &post_content,
        None,
        None,
    ) {
        Ok(message) => message,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    publish_channel_message_updated(state.as_ref(), &project_id, &channel_id, None, None, "text");
    let trace_id = clean_trace_id(req.trace_id.as_deref());
    spawn_channel_summary(ChannelSummaryTask {
        state: state.clone(),
        user_id,
        project,
        project_id,
        channel_id,
        prompt: summary_prompt,
        agent: req.agent,
        runtime_route,
        trace_id: trace_id.clone(),
    });

    Json(serde_json::json!({
        "trace_id": trace_id,
        "message": post_message,
    }))
    .into_response()
}
