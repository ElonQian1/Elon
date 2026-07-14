//! 项目成员角色修改、移除和禁言管理（从 project_membership.rs 拆分）。

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use super::{
    ensure_role_management_allowed, ensure_role_set_management_allowed,
    member_has_project_permission, publish_members_updated, requested_member_roles,
    UpdateMemberModerationRequest, UpdateMemberRoleRequest,
};
use crate::{
    project_auth::{auth_from_headers, json_error},
    store::{PERMISSION_MANAGE_MEMBERS, PERMISSION_MODERATE_MEMBERS},
    types::AppState,
};

/// PATCH /api/projects/:id/members/:user_id — 修改成员角色（仅 owner/admin）
pub async fn update_member_role(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, target_user_id)): Path<(String, String)>,
    Json(req): Json<UpdateMemberRoleRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_MANAGE_MEMBERS) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权修改成员角色");
    }
    if target_user_id == user.id {
        return json_error(StatusCode::BAD_REQUEST, "不能修改自己的角色");
    }
    let old_role = state
        .store
        .project_member_role(&project_id, &target_user_id)
        .ok()
        .flatten();
    let requested_roles = match requested_member_roles(req) {
        Ok(roles) => roles,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };
    if let Err(message) = ensure_role_set_management_allowed(
        &state,
        &project_id,
        &access.role,
        old_role.as_deref(),
        &requested_roles,
        "修改成员角色",
    ) {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    match state.store.set_project_member_roles(
        &project_id,
        &target_user_id,
        &requested_roles,
        Some(&user.id),
    ) {
        Ok(member) => {
            let new_role = member.role.clone();
            let audit_note = if member.roles.len() > 1 {
                Some(
                    member
                        .roles
                        .iter()
                        .map(|role| role.id.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                )
            } else {
                None
            };
            if let Err(err) = state.store.record_project_member_audit(
                &project_id,
                Some(&user.id),
                Some(&target_user_id),
                "update_role",
                old_role.as_deref(),
                Some(&new_role),
                audit_note.as_deref(),
            ) {
                tracing::warn!(?err, project_id = %project_id, "记录成员角色审计日志失败");
            }
            publish_members_updated(
                &state,
                &project_id,
                "update_member_role",
                Some(&target_user_id),
                Some(&user.id),
            );
            Json(serde_json::json!({
                "ok": true,
                "project_id": project_id,
                "user_id": target_user_id,
                "role": new_role,
                "roles": member.roles,
                "member": member,
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不是该项目成员") {
                StatusCode::NOT_FOUND
            } else if msg.contains("不能修改 owner") || msg.contains("role 必须") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            json_error(status, msg)
        }
    }
}

/// DELETE /api/projects/:id/members/:user_id — 移除成员（仅 owner/admin）
pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, target_user_id)): Path<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_MANAGE_MEMBERS) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权移除成员");
    }
    if target_user_id == user.id {
        return json_error(
            StatusCode::BAD_REQUEST,
            "不能移除自己；如要退出请使用 leave 接口",
        );
    }
    let old_role = state
        .store
        .project_member_role(&project_id, &target_user_id)
        .ok()
        .flatten();
    if let Err(message) = ensure_role_management_allowed(
        &state,
        &project_id,
        &access.role,
        old_role.as_deref(),
        None,
        "移除成员",
    ) {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    match state.store.remove_member(&project_id, &target_user_id) {
        Ok(()) => {
            if let Err(err) = state.store.record_project_member_audit(
                &project_id,
                Some(&user.id),
                Some(&target_user_id),
                "remove_member",
                old_role.as_deref(),
                None,
                None,
            ) {
                tracing::warn!(?err, project_id = %project_id, "记录成员移除审计日志失败");
            }
            publish_members_updated(
                &state,
                &project_id,
                "remove_member",
                Some(&target_user_id),
                Some(&user.id),
            );
            Json(serde_json::json!({
                "ok": true,
                "message": "成员已移除",
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不是该项目成员") {
                StatusCode::NOT_FOUND
            } else if msg.contains("不能移除项目 owner") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            json_error(status, msg)
        }
    }
}

/// PATCH /api/projects/:id/members/:user_id/moderation — 禁言/封禁/解除限制
pub async fn update_member_moderation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, target_user_id)): Path<(String, String)>,
    Json(req): Json<UpdateMemberModerationRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_MODERATE_MEMBERS) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权限制成员");
    }
    if target_user_id == user.id {
        return json_error(StatusCode::BAD_REQUEST, "不能限制自己");
    }
    let target_role = state
        .store
        .project_member_role(&project_id, &target_user_id)
        .ok()
        .flatten();
    if let Err(message) = ensure_role_management_allowed(
        &state,
        &project_id,
        &access.role,
        target_role.as_deref(),
        None,
        "限制成员",
    ) {
        return json_error(StatusCode::FORBIDDEN, message);
    }

    let action = req.action.trim().to_string();
    match state.store.update_project_member_moderation(
        &project_id,
        &target_user_id,
        &user.id,
        &action,
        req.duration_minutes,
        req.note.as_deref(),
    ) {
        Ok(moderation) => {
            let audit_action = moderation_audit_action(&action);
            let audit_note = moderation_audit_note(&action, &moderation, req.note.as_deref());
            if let Err(err) = state.store.record_project_member_audit(
                &project_id,
                Some(&user.id),
                Some(&target_user_id),
                audit_action,
                None,
                None,
                audit_note.as_deref(),
            ) {
                tracing::warn!(?err, project_id = %project_id, "记录成员限制审计日志失败");
            }
            publish_members_updated(
                &state,
                &project_id,
                audit_action,
                Some(&target_user_id),
                Some(&user.id),
            );
            Json(serde_json::json!({
                "ok": true,
                "project_id": project_id,
                "user_id": target_user_id,
                "moderation": moderation,
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不是该项目成员") || msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("owner")
                || msg.contains("不能限制")
                || msg.contains("action 必须")
            {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            json_error(status, msg)
        }
    }
}

fn moderation_audit_action(action: &str) -> &'static str {
    match action.trim() {
        "mute" => "mute_member",
        "unmute" => "unmute_member",
        "ban" => "ban_member",
        "unban" => "unban_member",
        _ => "moderate_member",
    }
}

fn moderation_audit_note(
    action: &str,
    moderation: &crate::store::ProjectMemberModerationEntry,
    note: Option<&str>,
) -> Option<String> {
    let clean_note = note.map(str::trim).filter(|value| !value.is_empty());
    match action.trim() {
        "mute" => moderation
            .muted_until
            .as_ref()
            .map(|until| match clean_note {
                Some(note) => format!("muted_until={until}; {note}"),
                None => format!("muted_until={until}"),
            }),
        "ban" => clean_note
            .map(|note| format!("ban; {note}"))
            .or_else(|| Some("ban".into())),
        "unmute" => clean_note
            .map(|note| format!("unmute; {note}"))
            .or_else(|| Some("unmute".into())),
        "unban" => clean_note
            .map(|note| format!("unban; {note}"))
            .or_else(|| Some("unban".into())),
        _ => clean_note.map(str::to_string),
    }
}
