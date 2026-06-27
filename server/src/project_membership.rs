/// project_membership.rs — 项目成员关系管理
///
/// 路由（均需登录）：
///   POST   /api/projects/:id/join                          加入公开项目（open=成员，readonly=只读成员）
///   DELETE /api/projects/:id/leave                         退出已加入的项目（owner 不可退出）
///   GET    /api/projects/:id/members                       列出项目所有成员（公开项目无需成员身份）
///   POST   /api/projects/:id/members                       管理员邀请/添加成员
///   GET    /api/projects/:id/member-audit                  owner/admin 查看成员管理日志
///   PATCH  /api/projects/:id/visibility                    设置公开/私有（仅 owner/admin）
///   PATCH  /api/projects/:id/icon                          修改项目 APK 图标（仅 owner）
///   PATCH  /api/projects/:id/brand                         修改项目展示别名与 logo（仅 owner）
///   PATCH  /api/projects/:id/members/:user_id              改成员角色（仅 owner/admin，不可改 owner/自己）
///   DELETE /api/projects/:id/members/:user_id              踢出成员（仅 owner/admin，不可踢 owner/自己）
///   PATCH  /api/projects/:id/members/:user_id/moderation   禁言/封禁/解除限制（仅 owner/admin）
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    store::{
        PERMISSION_INVITE_MEMBERS, PERMISSION_MANAGE_MEMBERS, PERMISSION_MANAGE_PROJECT_SETTINGS,
        PERMISSION_MANAGE_ROLES, PERMISSION_MODERATE_MEMBERS, PERMISSION_VIEW_AUDIT_LOG,
    },
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
pub struct UpdateProjectIconRequest {
    #[serde(default, alias = "iconDataUrl")]
    pub icon_data_url: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateMemberRoleRequest {
    /// "admin" | "editor" | "member" | "observer" | "viewer"（viewer 别名 → observer）或项目自定义角色 ID
    pub role: Option<String>,
    /// 多角色模式；为空时兼容旧的 role 字段。
    pub roles: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    /// 已注册用户的手机号、邮箱或 user_id
    pub account: String,
    /// 缺省为 member；支持 admin/editor/member/observer/viewer
    pub role: Option<String>,
}

#[derive(Deserialize)]
pub struct ListMemberAuditQuery {
    /// 返回最近多少条，默认 30，最大 100。
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct UpdateMemberModerationRequest {
    /// "mute" | "unmute" | "ban" | "unban"
    pub action: String,
    /// mute 使用；默认 60 分钟，最大 30 天。
    pub duration_minutes: Option<i64>,
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateProjectInviteLinkRequest {
    /// 缺省 member；支持内置角色或项目自定义角色，不能授予 owner。
    pub role: Option<String>,
    /// 多少小时后过期；空值为不过期。
    pub expires_in_hours: Option<i64>,
    /// 最大使用次数；空值为不限次数。
    pub max_uses: Option<i64>,
    /// 是否为临时邀请标记，当前先作为管理字段展示。
    pub temporary: Option<bool>,
}

#[derive(Deserialize)]
pub struct CreateProjectRoleRequest {
    pub name: String,
    pub color: Option<String>,
    pub position: Option<i64>,
    pub permissions: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct UpdateProjectRoleRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub position: Option<i64>,
    pub permissions: Option<Vec<String>>,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/projects/:id/join — 加入公开项目
const MAX_PROJECT_ICON_DATA_URL_BYTES: usize = 512 * 1024;
const MAX_PROJECT_DISPLAY_NAME_CHARS: usize = 80;

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
        Ok(already_member) => Json(serde_json::json!({
            "ok": true,
            "already_member": already_member,
            "message": if already_member { "你已经是该项目成员" } else { "已成功加入项目" },
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
        Ok(mut members) => {
            let online = state.online_users.read().await;
            for member in &mut members {
                apply_member_presence(member, online.contains_key(&member.user_id));
            }
            let total = members.len();
            Json(serde_json::json!({
                "members": members,
                "total": total,
                "project_id": project_id,
            }))
            .into_response()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// GET /api/projects/:id/invite-links — 项目邀请链接列表
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

/// GET /api/projects/:id/member-audit — owner/admin 查看成员管理日志
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

fn can_update_project_icon(role: &str) -> bool {
    role.trim().eq_ignore_ascii_case("owner")
}

fn can_update_project_brand(role: &str) -> bool {
    can_update_project_icon(role)
}

fn apply_member_presence(member: &mut crate::store::ProjectMemberEntry, connected: bool) {
    let configured = member.presence_status.trim().to_ascii_lowercase();
    if !connected || configured == "invisible" {
        member.is_online = false;
        member.presence_status = "offline".to_string();
        return;
    }
    member.is_online = true;
    member.presence_status = match configured.as_str() {
        "idle" | "dnd" | "online" => configured,
        _ => "online".to_string(),
    };
}

fn member_has_project_permission(
    state: &AppState,
    project_id: &str,
    user_id: &str,
    permission: &str,
) -> bool {
    state
        .store
        .project_member_has_permission(project_id, user_id, permission)
        .unwrap_or(false)
}

fn requested_member_roles(req: UpdateMemberRoleRequest) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    if let Some(roles) = req.roles {
        for role in roles {
            let role = role.trim();
            if role.is_empty()
                || out
                    .iter()
                    .any(|item: &String| item.eq_ignore_ascii_case(role))
            {
                continue;
            }
            out.push(role.to_string());
        }
    }
    if out.is_empty() {
        if let Some(role) = req.role {
            let role = role.trim();
            if !role.is_empty() {
                out.push(role.to_string());
            }
        }
    }
    if out.is_empty() {
        return Err("至少选择一个角色".into());
    }
    Ok(out)
}

fn project_role_permission_options() -> Value {
    serde_json::json!([
        { "key": "view_members", "label": "查看成员" },
        { "key": "send_messages", "label": "发送消息" },
        { "key": "invite_members", "label": "邀请成员" },
        { "key": "manage_members", "label": "管理成员" },
        { "key": "moderate_members", "label": "禁言/封禁" },
        { "key": "view_audit_log", "label": "查看日志" },
        { "key": "manage_roles", "label": "管理角色" },
        { "key": "manage_project_settings", "label": "项目设置" }
    ])
}

fn ensure_role_position_below_manager(
    state: &AppState,
    project_id: &str,
    manager_role: &str,
    position: i64,
    action_label: &str,
) -> Result<(), String> {
    let manager_level = state
        .store
        .project_role_level(project_id, manager_role)
        .unwrap_or(0);
    if manager_level <= 0 {
        return Err("当前角色无权管理项目角色".into());
    }
    if position >= manager_level {
        return Err(format!("当前角色不能{}同级或更高层级角色", action_label));
    }
    Ok(())
}

fn is_builtin_project_role(role: &str) -> bool {
    matches!(
        role.trim(),
        "owner"
            | "admin"
            | "editor"
            | "developer"
            | "maintainer"
            | "member"
            | "observer"
            | "viewer"
    )
}

fn ensure_role_management_allowed(
    state: &AppState,
    project_id: &str,
    manager_role: &str,
    target_role: Option<&str>,
    new_role: Option<&str>,
    action_label: &str,
) -> Result<(), String> {
    let manager_level = state
        .store
        .project_role_level(project_id, manager_role)
        .unwrap_or(0);
    let target_level = target_role.map(|role| {
        state
            .store
            .project_role_level(project_id, role)
            .unwrap_or(0)
    });
    let new_level = new_role.map(|role| {
        state
            .store
            .project_role_level(project_id, role)
            .unwrap_or(0)
    });
    ensure_role_management_allowed_by_level(
        manager_level,
        target_level,
        new_level,
        new_role,
        action_label,
    )
}

fn ensure_role_set_management_allowed(
    state: &AppState,
    project_id: &str,
    manager_role: &str,
    target_role: Option<&str>,
    new_roles: &[String],
    action_label: &str,
) -> Result<(), String> {
    let manager_level = state
        .store
        .project_role_level(project_id, manager_role)
        .unwrap_or(0);
    let target_level = target_role.map(|role| {
        state
            .store
            .project_role_level(project_id, role)
            .unwrap_or(0)
    });
    ensure_role_management_allowed_by_level(manager_level, target_level, None, None, action_label)?;
    for role in new_roles {
        let new_level = state
            .store
            .project_role_level(project_id, role)
            .unwrap_or(0);
        if new_level >= manager_level {
            return Err(format!("当前角色不能分配同级或更高角色（{}）", role.trim()));
        }
    }
    Ok(())
}

fn ensure_role_management_allowed_by_level(
    manager_level: i64,
    target_level: Option<i64>,
    new_level: Option<i64>,
    new_role: Option<&str>,
    action_label: &str,
) -> Result<(), String> {
    if manager_level <= 0 {
        return Err("当前角色无权管理成员".into());
    }
    if let Some(target_level) = target_level {
        if target_level >= manager_level {
            return Err(format!("当前角色不能{}同级或更高角色成员", action_label));
        }
    }
    if let Some(new_level) = new_level {
        if new_level >= manager_level {
            return Err(format!(
                "当前角色不能分配同级或更高角色（{}）",
                new_role.unwrap_or_default().trim()
            ));
        }
    }
    Ok(())
}

fn project_brand_field<'a>(
    obj: &'a Map<String, Value>,
    snake_case: &str,
    camel_case: &str,
) -> Option<&'a Value> {
    obj.get(snake_case).or_else(|| obj.get(camel_case))
}

fn clean_project_display_name_update(
    value: Option<&Value>,
) -> Result<Option<Option<String>>, (StatusCode, String)> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(Some(None));
    }
    let Some(value) = value.as_str() else {
        return Err((StatusCode::BAD_REQUEST, "项目别名必须是字符串".into()));
    };
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("null") {
        return Ok(Some(None));
    }
    if value.chars().count() > MAX_PROJECT_DISPLAY_NAME_CHARS {
        return Err((StatusCode::BAD_REQUEST, "项目别名不能超过 80 个字".into()));
    }
    Ok(Some(Some(value.to_string())))
}

fn clean_project_icon_data_url_update(
    value: Option<&Value>,
) -> Result<Option<Option<String>>, (StatusCode, String)> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(Some(None));
    }
    let Some(value) = value.as_str() else {
        return Err((StatusCode::BAD_REQUEST, "项目 logo 必须是字符串".into()));
    };
    clean_project_icon_data_url(Some(value.to_string())).map(Some)
}

fn clean_project_icon_data_url(
    icon_data_url: Option<String>,
) -> Result<Option<String>, (StatusCode, String)> {
    let Some(value) = icon_data_url.map(|value| value.trim().to_string()) else {
        return Ok(None);
    };
    if value.is_empty() || value.eq_ignore_ascii_case("null") {
        return Ok(None);
    }
    if value.len() > MAX_PROJECT_ICON_DATA_URL_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "APK 图标图片太大，请换一张较小的图片".into(),
        ));
    }
    if !value.starts_with("data:image/") || !value.contains(";base64,") {
        return Err((
            StatusCode::BAD_REQUEST,
            "APK 图标必须是 data:image/*;base64 格式".into(),
        ));
    }
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::{
        can_update_project_brand, can_update_project_icon, clean_project_display_name_update,
        clean_project_icon_data_url_update, ensure_role_management_allowed_by_level,
    };
    use serde_json::{json, Value};

    #[test]
    fn project_icon_update_is_owner_only() {
        assert!(can_update_project_icon("owner"));
        assert!(!can_update_project_icon("admin"));
        assert!(!can_update_project_icon("editor"));
        assert!(!can_update_project_icon("member"));
        assert!(!can_update_project_icon("observer"));
    }

    #[test]
    fn project_brand_update_is_owner_only() {
        assert!(can_update_project_brand("owner"));
        assert!(!can_update_project_brand("admin"));
        assert!(!can_update_project_brand("editor"));
    }

    #[test]
    fn project_display_name_update_distinguishes_missing_clear_and_set() {
        assert_eq!(clean_project_display_name_update(None).unwrap(), None);
        assert_eq!(
            clean_project_display_name_update(Some(&Value::Null)).unwrap(),
            Some(None)
        );
        assert_eq!(
            clean_project_display_name_update(Some(&json!(" 一龙网游加速器 "))).unwrap(),
            Some(Some("一龙网游加速器".to_string()))
        );
    }

    #[test]
    fn project_icon_update_accepts_null_as_clear() {
        assert_eq!(
            clean_project_icon_data_url_update(Some(&Value::Null)).unwrap(),
            Some(None)
        );
        assert_eq!(
            clean_project_icon_data_url_update(Some(&json!("null"))).unwrap(),
            Some(None)
        );
    }

    #[test]
    fn role_hierarchy_blocks_same_or_higher_management() {
        assert!(ensure_role_management_allowed_by_level(
            100,
            Some(80),
            Some(60),
            Some("editor"),
            "修改"
        )
        .is_ok());
        assert!(ensure_role_management_allowed_by_level(
            80,
            Some(60),
            Some(40),
            Some("member"),
            "修改"
        )
        .is_ok());
        assert!(ensure_role_management_allowed_by_level(80, Some(80), None, None, "移除").is_err());
        assert!(ensure_role_management_allowed_by_level(
            80,
            Some(60),
            Some(80),
            Some("admin"),
            "修改"
        )
        .is_err());
        assert!(ensure_role_management_allowed_by_level(0, Some(40), None, None, "移除").is_err());
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
