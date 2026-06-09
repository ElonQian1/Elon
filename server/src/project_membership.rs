/// project_membership.rs — 项目成员关系管理
///
/// 路由（均需登录）：
///   POST   /api/projects/:id/join                          加入公开项目（open=成员，readonly=只读成员）
///   DELETE /api/projects/:id/leave                         退出已加入的项目（owner 不可退出）
///   GET    /api/projects/:id/members                       列出项目所有成员（公开项目无需成员身份）
///   POST   /api/projects/:id/members                       管理员邀请/添加成员
///   PATCH  /api/projects/:id/visibility                    设置公开/私有（仅 owner/admin）
///   PATCH  /api/projects/:id/members/:user_id              改成员角色（仅 owner/admin，不可改 owner/自己）
///   DELETE /api/projects/:id/members/:user_id              踢出成员（仅 owner/admin，不可踢 owner/自己）
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, can_manage_project_members, json_error},
    types::AppState,
};

// ─── 请求体 ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct VisibilityRequest {
    /// true = 公开，false = 私有
    pub is_public: bool,
    /// "open" | "approval" | "invite" | "readonly"；默认 "open"
    pub join_mode: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateMemberRoleRequest {
    /// "admin" | "editor" | "member" | "observer" | "viewer"（viewer 别名 → observer）
    pub role: String,
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    /// 已注册用户的手机号、邮箱或 user_id
    pub account: String,
    /// 缺省为 member；支持 admin/editor/member/observer/viewer
    pub role: Option<String>,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/projects/:id/join — 加入公开项目
pub async fn join_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.join_project(&user.id, &project_id) {
        Ok(()) => Json(serde_json::json!({
            "ok": true,
            "message": "已成功加入项目",
            "project_id": project_id,
        }))
        .into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("不对外公开") {
                StatusCode::FORBIDDEN
            } else if msg.contains("需要审批") || msg.contains("join_mode=approval") {
                // 引导客户端改用 /request-join 接口
                return Json(serde_json::json!({
                    "ok": false,
                    "code": "approval_required",
                    "message": "该项目需要项目管理员审批才能加入，请使用「申请加入」功能",
                    "hint": "POST /api/projects/{id}/request-join"
                }))
                .into_response();
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, msg)
        }
    }
}

/// DELETE /api/projects/:id/leave — 退出项目
pub async fn leave_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.leave_project(&user.id, &project_id) {
        Ok(()) => Json(serde_json::json!({
            "ok": true,
            "message": "已退出项目",
        }))
        .into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不是该项目的成员") {
                StatusCode::NOT_FOUND
            } else if msg.contains("owner 不可退出") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, msg)
        }
    }
}

/// GET /api/projects/:id/members — 项目成员列表
///
/// - 公开项目：任何人（已登录或未登录）均可查看
/// - 私有项目：仅项目成员可查看（在此 handler 内校验）
pub async fn list_members(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    // 先尝试获取项目是否公开，若私有则需要校验成员身份
    let is_public = state
        .store
        .get_public_project(&project_id)
        .map(|_| true)
        .unwrap_or(false);

    if !is_public {
        // 私有项目：必须是登录用户且是项目成员才可查看
        let user = match auth_from_headers(&state, &headers) {
            Ok(u) => u,
            Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
        };
        if state
            .store
            .get_project_access(&user.id, &project_id)
            .is_err()
        {
            return json_error(StatusCode::FORBIDDEN, "无权查看该项目成员");
        }
    }

    match state.store.list_project_members(&project_id) {
        Ok(members) => Json(serde_json::json!({
            "members": members,
            "total": members.len(),
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
    if !can_manage_project_members(&access.role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目 owner 或管理员才可邀请成员");
    }

    match state.store.add_project_member_by_account(
        &project_id,
        req.account.trim(),
        req.role.as_deref().unwrap_or("member").trim(),
    ) {
        Ok(member) => Json(serde_json::json!({
            "ok": true,
            "project_id": project_id,
            "member": member,
        }))
        .into_response(),
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
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !can_manage_project_members(&access.role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目 owner 或管理员才可修改可见性");
    }

    let join_mode = req.join_mode.as_deref().unwrap_or("open");
    if !["open", "approval", "invite", "readonly"].contains(&join_mode) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "join_mode 必须为 open / approval / invite / readonly",
        );
    }

    match state
        .store
        .set_project_visibility(&project_id, req.is_public, join_mode)
    {
        Ok(()) => Json(serde_json::json!({
            "ok": true,
            "is_public": req.is_public,
            "join_mode": join_mode,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

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
    if !can_manage_project_members(&access.role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目 owner 或管理员才可修改成员角色");
    }
    if target_user_id == user.id {
        return json_error(StatusCode::BAD_REQUEST, "不能修改自己的角色");
    }
    match state
        .store
        .update_member_role(&project_id, &target_user_id, req.role.trim())
    {
        Ok(()) => Json(serde_json::json!({
            "ok": true,
            "project_id": project_id,
            "user_id": target_user_id,
            "role": req.role,
        }))
        .into_response(),
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
    if !can_manage_project_members(&access.role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目 owner 或管理员才可移除成员");
    }
    if target_user_id == user.id {
        return json_error(
            StatusCode::BAD_REQUEST,
            "不能移除自己；如要退出请使用 leave 接口",
        );
    }
    match state.store.remove_member(&project_id, &target_user_id) {
        Ok(()) => Json(serde_json::json!({
            "ok": true,
            "message": "成员已移除",
        }))
        .into_response(),
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
