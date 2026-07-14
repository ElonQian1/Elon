//! 项目邀请链接管理（从 project_membership.rs 拆分）。

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use super::{
    ensure_role_management_allowed, member_has_project_permission, publish_members_updated,
    CreateProjectInviteLinkRequest,
};
use crate::{
    project_auth::{auth_from_headers, json_error},
    store::PERMISSION_INVITE_MEMBERS,
    types::AppState,
};

pub async fn list_project_invite_links(
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
    if !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_INVITE_MEMBERS) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权管理邀请链接");
    }

    match state.store.list_project_invite_links(&project_id) {
        Ok(invites) => Json(serde_json::json!({
            "ok": true,
            "project_id": project_id,
            "invites": invites,
        }))
        .into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("系统归档项目") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            json_error(status, msg)
        }
    }
}

/// POST /api/projects/:id/invite-links — 创建项目邀请链接
pub async fn create_project_invite_link(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<CreateProjectInviteLinkRequest>,
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
        return json_error(StatusCode::FORBIDDEN, "当前角色无权创建邀请链接");
    }

    let role = req.role.as_deref().unwrap_or("member").trim().to_string();
    if let Err(message) = ensure_role_management_allowed(
        &state,
        &project_id,
        &access.role,
        None,
        Some(&role),
        "创建邀请链接",
    ) {
        return json_error(StatusCode::FORBIDDEN, message);
    }

    match state.store.create_project_invite_link(
        &project_id,
        &user.id,
        &role,
        req.expires_in_hours,
        req.max_uses,
        req.temporary.unwrap_or(false),
    ) {
        Ok(invite) => {
            let audit_note = format!(
                "code={},role={},max_uses={},expires_at={}",
                invite.code,
                invite.role,
                invite
                    .max_uses
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unlimited".to_string()),
                invite
                    .expires_at
                    .clone()
                    .unwrap_or_else(|| "never".to_string())
            );
            if let Err(err) = state.store.record_project_member_audit(
                &project_id,
                Some(&user.id),
                None,
                "create_invite_link",
                None,
                Some(&invite.role),
                Some(&audit_note),
            ) {
                tracing::warn!(?err, project_id = %project_id, "记录邀请链接创建审计日志失败");
            }
            Json(serde_json::json!({
                "ok": true,
                "project_id": project_id,
                "invite": invite,
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("系统归档项目") || msg.contains("owner") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, msg)
        }
    }
}

/// DELETE /api/projects/:id/invite-links/:code — 撤销项目邀请链接
pub async fn revoke_project_invite_link(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, code)): Path<(String, String)>,
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
    if !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_INVITE_MEMBERS) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权撤销邀请链接");
    }

    match state.store.revoke_project_invite_link(&project_id, &code) {
        Ok(invite) => {
            if let Err(err) = state.store.record_project_member_audit(
                &project_id,
                Some(&user.id),
                None,
                "revoke_invite_link",
                None,
                Some(&invite.role),
                Some(&format!("code={}", invite.code)),
            ) {
                tracing::warn!(?err, project_id = %project_id, "记录邀请链接撤销审计日志失败");
            }
            Json(serde_json::json!({
                "ok": true,
                "project_id": project_id,
                "invite": invite,
            }))
            .into_response()
        }
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

/// GET /api/project-invites/:code — 邀请链接预览
pub async fn get_project_invite_preview(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> Response {
    match state.store.get_project_invite_preview(&code) {
        Ok(preview) => Json(serde_json::json!({
            "ok": true,
            "code": code,
            "invite": preview,
        }))
        .into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, msg)
        }
    }
}

/// POST /api/project-invites/:code/join — 通过邀请链接加入项目
pub async fn join_project_by_invite_link(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.join_project_by_invite_link(&user.id, &code) {
        Ok((already_member, preview)) => {
            if !already_member {
                if let Err(err) = state.store.record_project_member_audit(
                    &preview.project_id,
                    Some(&user.id),
                    Some(&user.id),
                    "join_by_invite_link",
                    None,
                    Some(&preview.role),
                    Some(&format!("code={}", code)),
                ) {
                    tracing::warn!(?err, project_id = %preview.project_id, "记录邀请链接加入审计日志失败");
                }
                publish_members_updated(
                    &state,
                    &preview.project_id,
                    "join_by_invite_link",
                    Some(&user.id),
                    Some(&user.id),
                );
            }
            Json(serde_json::json!({
                "ok": true,
                "already_member": already_member,
                "message": if already_member { "你已经是该项目成员" } else { "已通过邀请加入项目" },
                "project_id": preview.project_id,
                "invite": preview,
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("封禁") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, msg)
        }
    }
}
