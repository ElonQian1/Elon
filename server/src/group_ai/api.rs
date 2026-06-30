use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    group_ai::{
        actions::{
            accept_matter as accept_matter_action, approve_matter as approve_matter_action,
            cancel_matter as cancel_matter_action,
            complete_assignment as complete_assignment_action,
            fail_assignment as fail_assignment_action, matter_detail, record_assignment_settlement,
            request_changes, retry_assignment as retry_assignment_action,
            start_matter as start_matter_action, AssignmentActionInput, MatterDetail,
        },
        bot_selector::{available_nodes_for_project, bots_for_project},
        permissions::{
            authenticate_project_member, ensure_can_authorize_node, ensure_can_create_matter,
            ensure_node_provider,
        },
        planner::build_matter_plan,
        types::{
            CreateMatterPlanRequest, CreateMatterRecord, ProjectAiMatter,
            ProjectAiMatterAssignment, UpsertNodeAuthorizationRequest,
        },
    },
    project_auth::{can_edit, json_error},
    store::ProjectAccess,
    types::AppState,
};

#[derive(Debug, Deserialize)]
pub(crate) struct ListMattersQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MatterActionRequest {
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct AssignmentActionRequest {
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default, alias = "resultSummary")]
    pub result_summary: Option<String>,
    #[serde(default, alias = "computeCallId")]
    pub compute_call_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default, alias = "accountingStatus")]
    pub accounting_status: Option<String>,
    #[serde(default, alias = "billedCostRmbFen")]
    pub billed_cost_rmb_fen: Option<i64>,
    #[serde(default, alias = "providerEarnedFen")]
    pub provider_earned_fen: Option<i64>,
}

pub(crate) async fn available_nodes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match available_nodes_for_project(&state, &user.id, &project_id).await {
        Ok(nodes) => Json(json!({
            "ok": true,
            "project_id": project_id,
            "can_authorize_nodes": can_edit(&access.role),
            "nodes": nodes,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn upsert_node_authorization(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<UpsertNodeAuthorizationRequest>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    if let Err(response) = ensure_can_authorize_node(&access) {
        return response;
    }
    if let Err(response) = ensure_node_provider(&state, &user.id, &req.node_id) {
        return response;
    }

    match state
        .store
        .upsert_project_ai_node_authorization(&project_id, &user.id, &user.id, req)
    {
        Ok(authorization) => Json(json!({
            "ok": true,
            "authorization": authorization,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn list_bots(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = authenticate_project_member(&state, &headers, &project_id) {
        return response;
    }
    match bots_for_project(&state, &project_id).await {
        Ok(bots) => Json(json!({
            "ok": true,
            "project_id": project_id,
            "bots": bots,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn create_matter_plan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<CreateMatterPlanRequest>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    if let Err(response) = ensure_can_create_matter(&access) {
        return response;
    }
    if req.brief.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "brief 不能为空");
    }
    if let Err(response) =
        ensure_ai_development_channel(&state, &project_id, &req.channel_id, &user.id)
    {
        return response;
    }

    let bots = match bots_for_project(&state, &project_id).await {
        Ok(bots) => bots,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    let draft = build_matter_plan(&req, &user.id, &bots);
    let record = CreateMatterRecord {
        project_id: project_id.clone(),
        channel_id: req.channel_id.trim().to_string(),
        requester_user_id: user.id.clone(),
        source_message_id: clean_optional(req.source_message_id.as_deref()),
        title: draft.title,
        brief: req.brief.trim().to_string(),
        collaboration_mode: draft.collaboration_mode,
        participant_user_ids: draft.participant_user_ids,
        node_policy_json: draft.node_policy_json,
        acceptance_criteria: draft.acceptance_criteria,
        plan_json: draft.plan_json,
    };
    match state.store.create_project_ai_matter(record) {
        Ok(matter) => Json(json!({
            "ok": true,
            "matter": matter,
            "bots": bots,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn list_matters(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Query(query): Query<ListMattersQuery>,
) -> Response {
    if let Err(response) = authenticate_project_member(&state, &headers, &project_id) {
        return response;
    }
    match state
        .store
        .list_project_ai_matters(&project_id, query.limit.unwrap_or(50))
    {
        Ok(matters) => Json(json!({
            "ok": true,
            "project_id": project_id,
            "matters": matters,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn get_matter(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = authenticate_project_member(&state, &headers, &project_id) {
        return response;
    }
    match matter_detail(&state, &project_id, &matter_id) {
        Ok(Some(detail)) => matter_detail_response(detail),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Matter 不存在"),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn approve_matter(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
    Json(req): Json<MatterActionRequest>,
) -> Response {
    action_response(
        state,
        headers,
        project_id,
        matter_id,
        req,
        approve_matter_action,
    )
}

pub(crate) async fn start_matter(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
    Json(req): Json<MatterActionRequest>,
) -> Response {
    action_response(
        state,
        headers,
        project_id,
        matter_id,
        req,
        start_matter_action,
    )
}

pub(crate) async fn request_matter_changes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
    Json(req): Json<MatterActionRequest>,
) -> Response {
    action_response(state, headers, project_id, matter_id, req, request_changes)
}

pub(crate) async fn accept_matter(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
    Json(req): Json<MatterActionRequest>,
) -> Response {
    action_response(
        state,
        headers,
        project_id,
        matter_id,
        req,
        accept_matter_action,
    )
}

pub(crate) async fn cancel_matter(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
    Json(req): Json<MatterActionRequest>,
) -> Response {
    action_response(
        state,
        headers,
        project_id,
        matter_id,
        req,
        cancel_matter_action,
    )
}

pub(crate) async fn complete_assignment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id, assignment_id)): Path<(String, String, String)>,
    Json(req): Json<AssignmentActionRequest>,
) -> Response {
    assignment_action_response(
        state,
        headers,
        project_id,
        matter_id,
        assignment_id,
        req,
        complete_assignment_action,
    )
}

pub(crate) async fn fail_assignment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id, assignment_id)): Path<(String, String, String)>,
    Json(req): Json<AssignmentActionRequest>,
) -> Response {
    assignment_action_response(
        state,
        headers,
        project_id,
        matter_id,
        assignment_id,
        req,
        fail_assignment_action,
    )
}

pub(crate) async fn retry_assignment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id, assignment_id)): Path<(String, String, String)>,
    Json(req): Json<AssignmentActionRequest>,
) -> Response {
    assignment_action_response(
        state,
        headers,
        project_id,
        matter_id,
        assignment_id,
        req,
        retry_assignment_action,
    )
}

pub(crate) async fn record_assignment_settlement_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id, assignment_id)): Path<(String, String, String)>,
    Json(req): Json<AssignmentActionRequest>,
) -> Response {
    assignment_action_response(
        state,
        headers,
        project_id,
        matter_id,
        assignment_id,
        req,
        record_assignment_settlement,
    )
}

fn ensure_ai_development_channel(
    state: &AppState,
    project_id: &str,
    channel_id: &str,
    user_id: &str,
) -> Result<(), Response> {
    let channel_kind = state
        .store
        .get_project_channel_kind(project_id, channel_id)
        .map_err(|error| json_error(StatusCode::NOT_FOUND, error.to_string()))?;
    if channel_kind != "ai_development" {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "群体 AI Matter 第一版只能从 AI 开发频道创建",
        ));
    }
    let permissions = state
        .store
        .project_member_channel_permissions(project_id, channel_id, user_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error.to_string()))?;
    if permissions.can_start_ai {
        Ok(())
    } else {
        Err(json_error(
            StatusCode::FORBIDDEN,
            "当前成员没有在该频道启动 AI 开发的权限",
        ))
    }
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn action_response(
    state: Arc<AppState>,
    headers: HeaderMap,
    project_id: String,
    matter_id: String,
    req: MatterActionRequest,
    action: fn(&AppState, &str, &str, &str, Option<&str>) -> anyhow::Result<MatterDetail>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let matter = match state.store.get_project_ai_matter(&project_id, &matter_id) {
        Ok(Some(matter)) => matter,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Matter 不存在"),
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    if let Err(response) = ensure_can_decide_matter(&access, &user.id, &matter) {
        return response;
    }
    match action(
        &state,
        &project_id,
        &matter_id,
        &user.id,
        req.comment.as_deref(),
    ) {
        Ok(detail) => matter_detail_response(detail),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

fn assignment_action_response(
    state: Arc<AppState>,
    headers: HeaderMap,
    project_id: String,
    matter_id: String,
    assignment_id: String,
    req: AssignmentActionRequest,
    action: for<'a> fn(
        &AppState,
        &str,
        &str,
        &str,
        &str,
        AssignmentActionInput<'a>,
    ) -> anyhow::Result<MatterDetail>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let matter = match state.store.get_project_ai_matter(&project_id, &matter_id) {
        Ok(Some(matter)) => matter,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Matter 不存在"),
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    let assignment = match state.store.get_project_ai_matter_assignment(&assignment_id) {
        Ok(Some(assignment)) if assignment.matter_id == matter_id => assignment,
        Ok(Some(_)) => return json_error(StatusCode::BAD_REQUEST, "Assignment 不属于当前 Matter"),
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Assignment 不存在"),
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    if let Err(response) = ensure_can_operate_assignment(&access, &user.id, &matter, &assignment) {
        return response;
    }
    match action(
        &state,
        &project_id,
        &matter_id,
        &assignment_id,
        &user.id,
        req.as_input(),
    ) {
        Ok(detail) => matter_detail_response(detail),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

fn ensure_can_decide_matter(
    access: &ProjectAccess,
    user_id: &str,
    matter: &ProjectAiMatter,
) -> Result<(), Response> {
    if can_edit(&access.role) || matter.requester_user_id == user_id {
        return Ok(());
    }
    Err(json_error(
        StatusCode::FORBIDDEN,
        "只有项目编辑者或 Matter 创建者可以操作该 Matter",
    ))
}

fn ensure_can_operate_assignment(
    access: &ProjectAccess,
    user_id: &str,
    matter: &ProjectAiMatter,
    assignment: &ProjectAiMatterAssignment,
) -> Result<(), Response> {
    if can_edit(&access.role)
        || matter.requester_user_id == user_id
        || assignment.provider_user_id == user_id
        || assignment.assignee_user_id.as_deref() == Some(user_id)
    {
        return Ok(());
    }
    Err(json_error(
        StatusCode::FORBIDDEN,
        "只有项目编辑者、Matter 创建者或 Assignment 节点提供者可以操作该 Assignment",
    ))
}

impl AssignmentActionRequest {
    fn as_input(&self) -> AssignmentActionInput<'_> {
        AssignmentActionInput {
            comment: self.comment.as_deref(),
            result_summary: self.result_summary.as_deref(),
            compute_call_id: self.compute_call_id.as_deref(),
            status: self.status.as_deref(),
            accounting_status: self.accounting_status.as_deref(),
            billed_cost_rmb_fen: self.billed_cost_rmb_fen,
            provider_earned_fen: self.provider_earned_fen,
        }
    }
}

fn matter_detail_response(detail: MatterDetail) -> Response {
    Json(json!({
        "ok": true,
        "matter": detail.matter,
        "assignments": detail.assignments,
        "events": detail.events,
    }))
    .into_response()
}
