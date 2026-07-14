//! 项目成员审计、添加、可见性和角色管理（从 project_membership.rs 拆分）。

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

use super::{
    can_update_project_brand, can_update_project_icon, clean_project_display_name_update,
    clean_project_icon_data_url, clean_project_icon_data_url_update,
    ensure_role_management_allowed, ensure_role_management_allowed_by_level,
    ensure_role_position_below_manager, ensure_role_set_management_allowed,
    is_builtin_project_role, member_has_project_permission, project_brand_field,
    project_role_permission_options, publish_members_updated, AddMemberRequest,
    CreateProjectRoleRequest, ListMemberAuditQuery, UpdateProjectIconRequest,
    UpdateProjectRoleRequest, VisibilityRequest,
};
use crate::{
    project_auth::{auth_from_headers, json_error},
    store::{
        PERMISSION_INVITE_MEMBERS, PERMISSION_MANAGE_MEMBERS, PERMISSION_MANAGE_PROJECT_SETTINGS,
        PERMISSION_MANAGE_ROLES, PERMISSION_VIEW_AUDIT_LOG,
    },
    types::AppState,
};

pub async fn list_member_audit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Query(q): Query<ListMemberAuditQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let _access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_VIEW_AUDIT_LOG)
        && !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_MANAGE_MEMBERS)
    {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权查看成员日志");
    }

    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    match state.store.list_project_member_audit(&project_id, limit) {
        Ok(entries) => Json(serde_json::json!({
            "entries": entries,
            "total": entries.len(),
            "project_id": project_id,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// POST /api/projects/:id/members — 管理员邀请/添加已注册成员
pub async fn add_member(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_INVITE_MEMBERS) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权邀请成员");
    }

    let account = req.account.trim().to_string();
    let role = req.role.as_deref().unwrap_or("member").trim().to_string();
    let audit_target_user_id = state.store.find_active_user_id_by_account(&account).ok();
    let audit_old_role = audit_target_user_id.as_deref().and_then(|target_user_id| {
        state
            .store
            .project_member_role(&project_id, target_user_id)
            .ok()
            .flatten()
    });
    if let Err(message) = ensure_role_management_allowed(
        &state,
        &project_id,
        &access.role,
        audit_old_role.as_deref(),
        Some(&role),
        "邀请或调整成员角色",
    ) {
        return json_error(StatusCode::FORBIDDEN, message);
    }

    match state
        .store
        .add_project_member_by_account(&project_id, &account, &role)
    {
        Ok(member) => {
            let action = if audit_old_role.is_some() {
                "update_role"
            } else {
                "invite_member"
            };
            if let Err(err) = state.store.record_project_member_audit(
                &project_id,
                Some(&user.id),
                Some(&member.user_id),
                action,
                audit_old_role.as_deref(),
                Some(&member.role),
                None,
            ) {
                tracing::warn!(?err, project_id = %project_id, "记录成员邀请审计日志失败");
            }
            publish_members_updated(
                &state,
                &project_id,
                action,
                Some(&member.user_id),
                Some(&user.id),
            );
            Json(serde_json::json!({
                "ok": true,
                "project_id": project_id,
                "member": member,
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") || msg.contains("账号") {
                StatusCode::NOT_FOUND
            } else if msg.contains("owner") || msg.contains("role 必须") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            json_error(status, msg)
        }
    }
}

/// PATCH /api/projects/:id/visibility — 设置项目公开/私有（仅 owner/admin）
pub async fn update_visibility(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<VisibilityRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    // 仅 owner/admin 可修改
    let _access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !member_has_project_permission(
        &state,
        &project_id,
        &user.id,
        PERMISSION_MANAGE_PROJECT_SETTINGS,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权修改项目可见性");
    }

    let join_mode = req.join_mode.as_deref().unwrap_or("open");
    if !["open", "approval", "invite", "readonly"].contains(&join_mode) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "join_mode 必须为 open / approval / invite / readonly",
        );
    }

    let effective_is_public = if project_id == "elon-self" {
        true
    } else {
        req.is_public
    };
    let effective_join_mode = if project_id == "elon-self" {
        "approval"
    } else {
        join_mode
    };

    match state
        .store
        .set_project_visibility(&project_id, effective_is_public, effective_join_mode)
    {
        Ok(()) => Json(serde_json::json!({
            "ok": true,
            "is_public": effective_is_public,
            "join_mode": effective_join_mode,
        }))
        .into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("系统归档项目") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, msg)
        }
    }
}

/// GET /api/projects/:id/roles — 项目角色目录（内置 + 自定义）
pub async fn list_project_roles(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if state
        .store
        .get_project_access(&user.id, &project_id)
        .is_err()
    {
        return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问");
    }
    match state.store.list_project_roles(&project_id) {
        Ok(roles) => Json(serde_json::json!({
            "roles": roles,
            "permissions": project_role_permission_options(),
            "project_id": project_id,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// POST /api/projects/:id/roles — 创建项目自定义角色
pub async fn create_project_role(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<CreateProjectRoleRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_MANAGE_ROLES) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权管理项目角色");
    }
    let position = req.position.unwrap_or(30).clamp(1, 79);
    if let Err(message) =
        ensure_role_position_below_manager(&state, &project_id, &access.role, position, "创建")
    {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    match state.store.create_project_role(
        &project_id,
        &req.name,
        req.color.as_deref(),
        Some(position),
        req.permissions.as_deref(),
        Some(&user.id),
    ) {
        Ok(role) => {
            if let Err(err) = state.store.record_project_member_audit(
                &project_id,
                Some(&user.id),
                None,
                "create_role",
                None,
                Some(&role.id),
                Some(&role.name),
            ) {
                tracing::warn!(?err, project_id = %project_id, "记录角色创建审计日志失败");
            }
            publish_members_updated(&state, &project_id, "create_role", None, Some(&user.id));
            Json(serde_json::json!({
                "ok": true,
                "project_id": project_id,
                "role": role,
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("角色") || msg.contains("颜色") || msg.contains("同名") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            json_error(status, msg)
        }
    }
}

/// PATCH /api/projects/:id/roles/:role_id — 更新项目自定义角色
pub async fn update_project_role(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, role_id)): Path<(String, String)>,
    Json(req): Json<UpdateProjectRoleRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_MANAGE_ROLES) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权管理项目角色");
    }
    if is_builtin_project_role(&role_id) {
        return json_error(StatusCode::BAD_REQUEST, "内置角色不能编辑");
    }
    let target_level = state
        .store
        .project_role_level(&project_id, &role_id)
        .unwrap_or(0);
    let manager_level = state
        .store
        .project_role_level(&project_id, &access.role)
        .unwrap_or(0);
    if target_level >= manager_level {
        return json_error(StatusCode::FORBIDDEN, "当前角色不能编辑同级或更高角色");
    }
    if let Some(position) = req.position {
        if let Err(message) = ensure_role_position_below_manager(
            &state,
            &project_id,
            &access.role,
            position.clamp(1, 79),
            "调整",
        ) {
            return json_error(StatusCode::FORBIDDEN, message);
        }
    }
    let color_update = if req.color.is_some() {
        Some(req.color.as_deref())
    } else {
        None
    };
    match state.store.update_project_role(
        &project_id,
        &role_id,
        req.name.as_deref(),
        color_update,
        req.position,
        req.permissions.as_deref(),
    ) {
        Ok(role) => {
            if let Err(err) = state.store.record_project_member_audit(
                &project_id,
                Some(&user.id),
                None,
                "update_role_definition",
                Some(&role_id),
                Some(&role.id),
                Some(&role.name),
            ) {
                tracing::warn!(?err, project_id = %project_id, "记录角色更新审计日志失败");
            }
            publish_members_updated(
                &state,
                &project_id,
                "update_role_definition",
                None,
                Some(&user.id),
            );
            Json(serde_json::json!({
                "ok": true,
                "project_id": project_id,
                "role": role,
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("角色") || msg.contains("颜色") || msg.contains("同名") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            json_error(status, msg)
        }
    }
}

/// DELETE /api/projects/:id/roles/:role_id — 删除项目自定义角色
pub async fn delete_project_role(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, role_id)): Path<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_MANAGE_ROLES) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权管理项目角色");
    }
    if is_builtin_project_role(&role_id) {
        return json_error(StatusCode::BAD_REQUEST, "内置角色不能删除");
    }
    let target_level = state
        .store
        .project_role_level(&project_id, &role_id)
        .unwrap_or(0);
    let manager_level = state
        .store
        .project_role_level(&project_id, &access.role)
        .unwrap_or(0);
    if target_level >= manager_level {
        return json_error(StatusCode::FORBIDDEN, "当前角色不能删除同级或更高角色");
    }
    match state.store.delete_project_role(&project_id, &role_id) {
        Ok(()) => {
            if let Err(err) = state.store.record_project_member_audit(
                &project_id,
                Some(&user.id),
                None,
                "delete_role",
                Some(&role_id),
                None,
                None,
            ) {
                tracing::warn!(?err, project_id = %project_id, "记录角色删除审计日志失败");
            }
            publish_members_updated(&state, &project_id, "delete_role", None, Some(&user.id));
            Json(serde_json::json!({
                "ok": true,
                "project_id": project_id,
                "role_id": role_id,
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("角色") || msg.contains("成员") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            json_error(status, msg)
        }
    }
}

/// PATCH /api/projects/:id/icon — 修改项目 APK 图标（仅 owner）
pub async fn update_project_icon(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<UpdateProjectIconRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !can_update_project_icon(&access.role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目创建者才能修改 APK 图标");
    }
    let icon_data_url = match clean_project_icon_data_url(req.icon_data_url) {
        Ok(value) => value,
        Err((status, message)) => return json_error(status, message),
    };
    match state
        .store
        .set_project_icon_data_url(&project_id, icon_data_url.as_deref())
    {
        Ok(()) => Json(serde_json::json!({
            "ok": true,
            "project_id": project_id,
            "icon_data_url": icon_data_url,
        }))
        .into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("系统归档项目") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, msg)
        }
    }
}

/// PATCH /api/projects/:id/brand — 修改项目展示别名与 logo（仅 owner）
pub async fn update_project_brand(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<Value>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !can_update_project_brand(&access.role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目创建者才能修改项目展示资料");
    }

    let Some(obj) = req.as_object() else {
        return json_error(StatusCode::BAD_REQUEST, "请求体必须是 JSON 对象");
    };
    let display_name_update = match clean_project_display_name_update(project_brand_field(
        obj,
        "display_name",
        "displayName",
    )) {
        Ok(value) => value,
        Err((status, message)) => return json_error(status, message),
    };
    let icon_data_url_update = match clean_project_icon_data_url_update(project_brand_field(
        obj,
        "icon_data_url",
        "iconDataUrl",
    )) {
        Ok(value) => value,
        Err((status, message)) => return json_error(status, message),
    };
    if display_name_update.is_none() && icon_data_url_update.is_none() {
        return json_error(
            StatusCode::BAD_REQUEST,
            "至少需要提供 display_name/displayName 或 icon_data_url/iconDataUrl",
        );
    }

    let display_name_arg = display_name_update.as_ref().map(|value| value.as_deref());
    let icon_data_url_arg = icon_data_url_update.as_ref().map(|value| value.as_deref());
    match state
        .store
        .update_project_branding(&project_id, display_name_arg, icon_data_url_arg)
    {
        Ok(()) => Json(serde_json::json!({
            "ok": true,
            "project_id": project_id,
            "display_name": display_name_update.flatten(),
            "icon_data_url": icon_data_url_update.flatten(),
        }))
        .into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("系统归档项目") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, msg)
        }
    }
}
