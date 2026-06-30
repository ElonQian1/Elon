use axum::{
    http::{HeaderMap, StatusCode},
    response::Response,
};
use std::sync::Arc;

use crate::{
    group_ai::types::{ProjectAiMatter, ProjectAiMatterAssignment},
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    store::{ProjectAccess, PublicUser},
    types::AppState,
};

pub(crate) fn authenticate_project_member(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<(PublicUser, ProjectAccess), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error.to_string()))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error.to_string()))?;
    Ok((user, access))
}

pub(crate) fn ensure_can_create_matter(access: &ProjectAccess) -> Result<(), Response> {
    if can_edit(&access.role) {
        return Ok(());
    }
    Err(json_error(
        StatusCode::FORBIDDEN,
        "只有项目 owner、管理员或编辑者可以创建群体 AI Matter",
    ))
}

pub(crate) fn ensure_can_authorize_node(access: &ProjectAccess) -> Result<(), Response> {
    if can_edit(&access.role) {
        return Ok(());
    }
    Err(json_error(
        StatusCode::FORBIDDEN,
        "只有项目 owner、管理员或编辑者可以把自己的节点授权给项目",
    ))
}

pub(crate) fn ensure_node_provider(
    state: &Arc<AppState>,
    user_id: &str,
    node_id: &str,
) -> Result<(), Response> {
    let node_id = node_id.trim();
    if node_id.is_empty() {
        return Err(json_error(StatusCode::BAD_REQUEST, "node_id 不能为空"));
    }
    match state.store.get_node_credential_owner(node_id) {
        Ok(Some(owner_user_id)) if owner_user_id == user_id => Ok(()),
        Ok(Some(_)) => Err(json_error(
            StatusCode::FORBIDDEN,
            "只能授权自己名下注册的 PC 节点",
        )),
        Ok(None) => Err(json_error(StatusCode::NOT_FOUND, "节点不存在或尚未注册")),
        Err(error) => Err(json_error(StatusCode::BAD_REQUEST, error.to_string())),
    }
}

pub(crate) fn ensure_can_decide_matter(
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

pub(crate) fn ensure_can_operate_assignment(
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
