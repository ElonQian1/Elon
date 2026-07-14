use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error, project_access},
    store::{
        ProjectChannelMemberPermissionOverride, ProjectChannelRolePermissionOverride,
        CHANNEL_PERMISSION_MANAGE,
    },
    types::AppState,
};

use super::{
    project_member_can_use_channel, project_member_can_use_channel_category,
    UpdateChannelRolePermissionRequest,
};

pub async fn get_channel_permissions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id)): Path<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    if !project_member_can_use_channel(
        &state,
        &project.id,
        &channel_id,
        &user.id,
        CHANNEL_PERMISSION_MANAGE,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权管理该频道权限");
    }
    let current_user_permissions =
        match state
            .store
            .project_member_channel_permissions(&project.id, &channel_id, &user.id)
        {
            Ok(permissions) => permissions,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        };
    let role_overrides = match state
        .store
        .list_project_channel_role_permission_overrides(&project.id, &channel_id)
    {
        Ok(overrides) => overrides,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    match state
        .store
        .list_project_channel_member_permission_overrides(&project.id, &channel_id)
    {
        Ok(member_overrides) => Json(serde_json::json!({
            "project_id": project.id,
            "channel_id": channel_id,
            "overrides": role_overrides,
            "member_overrides": member_overrides,
            "current_user_permissions": current_user_permissions,
            "permissions": channel_permission_options(),
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn update_channel_permissions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id)): Path<(String, String)>,
    Json(req): Json<UpdateChannelRolePermissionRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    if !project_member_can_use_channel(
        &state,
        &project.id,
        &channel_id,
        &user.id,
        CHANNEL_PERMISSION_MANAGE,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权管理该频道权限");
    }
    let clean_member_id = req.member_id.as_deref().map(str::trim).unwrap_or("");
    if !clean_member_id.is_empty() {
        return match state.store.set_project_channel_member_permission_override(
            &project.id,
            &channel_id,
            clean_member_id,
            &req.allow,
            &req.deny,
            Some(&user.id),
        ) {
            Ok(member_overrides) => {
                record_channel_permission_audit(
                    &state,
                    &project.id,
                    &channel_id,
                    &user.id,
                    Some(clean_member_id),
                    "update_channel_member_permission",
                    "member",
                    clean_member_id,
                    &req.allow,
                    &req.deny,
                );
                channel_permissions_payload(
                    &state,
                    &project.id,
                    &channel_id,
                    &user.id,
                    Some(member_overrides),
                    None,
                )
            }
            Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
        };
    }
    match state.store.set_project_channel_role_permission_override(
        &project.id,
        &channel_id,
        &req.role_id,
        &req.allow,
        &req.deny,
        Some(&user.id),
    ) {
        Ok(overrides) => {
            record_channel_permission_audit(
                &state,
                &project.id,
                &channel_id,
                &user.id,
                None,
                "update_channel_role_permission",
                "role",
                &req.role_id,
                &req.allow,
                &req.deny,
            );
            channel_permissions_payload(
                &state,
                &project.id,
                &channel_id,
                &user.id,
                None,
                Some(overrides),
            )
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn get_channel_category_permissions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, category_id)): Path<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    if !project_member_can_use_channel_category(
        &state,
        &project.id,
        &category_id,
        &user.id,
        CHANNEL_PERMISSION_MANAGE,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权管理该频道分类权限");
    }
    let current_user_permissions = match state.store.project_member_channel_category_permissions(
        &project.id,
        &category_id,
        &user.id,
    ) {
        Ok(permissions) => permissions,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let role_overrides = match state
        .store
        .list_project_channel_category_role_permission_overrides(&project.id, &category_id)
    {
        Ok(overrides) => overrides,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    match state
        .store
        .list_project_channel_category_member_permission_overrides(&project.id, &category_id)
    {
        Ok(member_overrides) => Json(serde_json::json!({
            "project_id": project.id,
            "category_id": category_id,
            "overrides": role_overrides,
            "member_overrides": member_overrides,
            "current_user_permissions": current_user_permissions,
            "permissions": channel_permission_options(),
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn update_channel_category_permissions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, category_id)): Path<(String, String)>,
    Json(req): Json<UpdateChannelRolePermissionRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    if !project_member_can_use_channel_category(
        &state,
        &project.id,
        &category_id,
        &user.id,
        CHANNEL_PERMISSION_MANAGE,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权管理该频道分类权限");
    }
    let clean_member_id = req.member_id.as_deref().map(str::trim).unwrap_or("");
    if !clean_member_id.is_empty() {
        return match state
            .store
            .set_project_channel_category_member_permission_override(
                &project.id,
                &category_id,
                clean_member_id,
                &req.allow,
                &req.deny,
                Some(&user.id),
            ) {
            Ok(member_overrides) => {
                record_channel_category_permission_audit(
                    &state,
                    &project.id,
                    &category_id,
                    &user.id,
                    Some(clean_member_id),
                    "update_channel_category_member_permission",
                    "member",
                    clean_member_id,
                    &req.allow,
                    &req.deny,
                );
                channel_category_permissions_payload(
                    &state,
                    &project.id,
                    &category_id,
                    &user.id,
                    Some(member_overrides),
                    None,
                )
            }
            Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
        };
    }
    match state
        .store
        .set_project_channel_category_role_permission_override(
            &project.id,
            &category_id,
            &req.role_id,
            &req.allow,
            &req.deny,
            Some(&user.id),
        ) {
        Ok(overrides) => {
            record_channel_category_permission_audit(
                &state,
                &project.id,
                &category_id,
                &user.id,
                None,
                "update_channel_category_role_permission",
                "role",
                &req.role_id,
                &req.allow,
                &req.deny,
            );
            channel_category_permissions_payload(
                &state,
                &project.id,
                &category_id,
                &user.id,
                None,
                Some(overrides),
            )
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

fn channel_permissions_payload(
    state: &AppState,
    project_id: &str,
    channel_id: &str,
    user_id: &str,
    member_overrides: Option<Vec<ProjectChannelMemberPermissionOverride>>,
    role_overrides: Option<Vec<ProjectChannelRolePermissionOverride>>,
) -> Response {
    let role_overrides = match role_overrides {
        Some(overrides) => overrides,
        None => match state
            .store
            .list_project_channel_role_permission_overrides(project_id, channel_id)
        {
            Ok(overrides) => overrides,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        },
    };
    let member_overrides = match member_overrides {
        Some(overrides) => overrides,
        None => match state
            .store
            .list_project_channel_member_permission_overrides(project_id, channel_id)
        {
            Ok(overrides) => overrides,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        },
    };
    let current_user_permissions = match state
        .store
        .project_member_channel_permissions(project_id, channel_id, user_id)
    {
        Ok(permissions) => permissions,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    Json(serde_json::json!({
        "ok": true,
        "project_id": project_id,
        "channel_id": channel_id,
        "overrides": role_overrides,
        "member_overrides": member_overrides,
        "current_user_permissions": current_user_permissions,
        "permissions": channel_permission_options(),
    }))
    .into_response()
}

fn channel_category_permissions_payload(
    state: &AppState,
    project_id: &str,
    category_id: &str,
    user_id: &str,
    member_overrides: Option<Vec<ProjectChannelMemberPermissionOverride>>,
    role_overrides: Option<Vec<ProjectChannelRolePermissionOverride>>,
) -> Response {
    let role_overrides = match role_overrides {
        Some(overrides) => overrides,
        None => match state
            .store
            .list_project_channel_category_role_permission_overrides(project_id, category_id)
        {
            Ok(overrides) => overrides,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        },
    };
    let member_overrides = match member_overrides {
        Some(overrides) => overrides,
        None => match state
            .store
            .list_project_channel_category_member_permission_overrides(project_id, category_id)
        {
            Ok(overrides) => overrides,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        },
    };
    let current_user_permissions = match state.store.project_member_channel_category_permissions(
        project_id,
        category_id,
        user_id,
    ) {
        Ok(permissions) => permissions,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    Json(serde_json::json!({
        "ok": true,
        "project_id": project_id,
        "category_id": category_id,
        "overrides": role_overrides,
        "member_overrides": member_overrides,
        "current_user_permissions": current_user_permissions,
        "permissions": channel_permission_options(),
    }))
    .into_response()
}

fn record_channel_permission_audit(
    state: &AppState,
    project_id: &str,
    channel_id: &str,
    actor_user_id: &str,
    target_user_id: Option<&str>,
    action: &str,
    scope: &str,
    target: &str,
    allow: &[String],
    deny: &[String],
) {
    let channel_kind = state
        .store
        .get_project_channel_kind(project_id, channel_id)
        .unwrap_or_default();
    let note = format!(
        "channel_id={};channel_kind={};scope={};target={};allow={};deny={}",
        audit_note_value(channel_id),
        audit_note_value(&channel_kind),
        audit_note_value(scope),
        audit_note_value(target),
        audit_permission_list(allow),
        audit_permission_list(deny),
    );
    if let Err(err) = state.store.record_project_member_audit(
        project_id,
        Some(actor_user_id),
        target_user_id,
        action,
        None,
        None,
        Some(&note),
    ) {
        tracing::warn!(
            ?err,
            project_id = %project_id,
            channel_id = %channel_id,
            action = %action,
            "记录频道权限审计日志失败"
        );
    }
}

fn record_channel_category_permission_audit(
    state: &AppState,
    project_id: &str,
    category_id: &str,
    actor_user_id: &str,
    target_user_id: Option<&str>,
    action: &str,
    scope: &str,
    target: &str,
    allow: &[String],
    deny: &[String],
) {
    let category_label = state
        .store
        .list_project_channel_categories(project_id)
        .ok()
        .and_then(|categories| {
            categories
                .into_iter()
                .find(|category| category.id == category_id)
                .map(|category| format!("{}:{}", category.kind, category.name))
        })
        .unwrap_or_default();
    let note = format!(
        "category_id={};category={};scope={};target={};allow={};deny={}",
        audit_note_value(category_id),
        audit_note_value(&category_label),
        audit_note_value(scope),
        audit_note_value(target),
        audit_permission_list(allow),
        audit_permission_list(deny),
    );
    if let Err(err) = state.store.record_project_member_audit(
        project_id,
        Some(actor_user_id),
        target_user_id,
        action,
        None,
        None,
        Some(&note),
    ) {
        tracing::warn!(
            ?err,
            project_id = %project_id,
            category_id = %category_id,
            action = %action,
            "记录频道分类权限审计日志失败"
        );
    }
}

fn audit_permission_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| audit_note_value(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

fn audit_note_value(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| match ch {
            ';' | '=' | '\n' | '\r' => ' ',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn channel_permission_options() -> serde_json::Value {
    serde_json::json!([
        { "key": "view_channel", "label": "查看频道" },
        { "key": "send_messages", "label": "发送消息" },
        { "key": "start_ai_tasks", "label": "发起 AI 任务" },
        { "key": "manage_channel", "label": "管理频道权限" }
    ])
}
