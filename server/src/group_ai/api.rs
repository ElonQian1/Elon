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
        bot_selector::{available_nodes_for_project, bots_for_project},
        permissions::{
            authenticate_project_member, ensure_can_authorize_node, ensure_can_create_matter,
            ensure_node_provider,
        },
        planner::build_matter_plan,
        types::{CreateMatterPlanRequest, CreateMatterRecord, UpsertNodeAuthorizationRequest},
    },
    project_auth::{can_edit, json_error},
    types::AppState,
};

#[derive(Debug, Deserialize)]
pub(crate) struct ListMattersQuery {
    pub limit: Option<i64>,
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
    match state.store.get_project_ai_matter(&project_id, &matter_id) {
        Ok(Some(matter)) => Json(json!({
            "ok": true,
            "matter": matter,
        }))
        .into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Matter 不存在"),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
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
