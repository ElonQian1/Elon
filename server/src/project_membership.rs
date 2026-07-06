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
///   PATCH  /api/projects/:id/members/:user_id/profile      改项目内昵称/管理员备注
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
const MAX_MEMBER_DISPLAY_NAME_CHARS: usize = 40;
const MAX_MEMBER_ADMIN_NOTE_CHARS: usize = 160;

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
        Ok(already_member) => {
            if !already_member {
                publish_members_updated(
                    &state,
                    &project_id,
                    "join_project",
                    Some(&user.id),
                    Some(&user.id),
                );
            }
            Json(serde_json::json!({
                "ok": true,
                "already_member": already_member,
                "message": if already_member { "你已经是该项目成员" } else { "已成功加入项目" },
                "project_id": project_id,
            }))
            .into_response()
        }
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
        Ok(()) => {
            publish_members_updated(
                &state,
                &project_id,
                "leave_project",
                Some(&user.id),
                Some(&user.id),
            );
            Json(serde_json::json!({
                "ok": true,
                "message": "已退出项目",
            }))
            .into_response()
        }
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
            let can_view_admin_notes = auth_from_headers(&state, &headers)
                .ok()
                .map(|user| {
                    member_has_project_permission(
                        &state,
                        &project_id,
                        &user.id,
                        PERMISSION_MANAGE_MEMBERS,
                    )
                })
                .unwrap_or(false);
            let online = state.online_users.read().await;
            for member in &mut members {
                apply_member_presence(member, online.contains_key(&member.user_id));
                if !can_view_admin_notes {
                    member.admin_note = None;
                }
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

/// PATCH /api/projects/:id/members/:user_id/profile — 修改项目内昵称与管理员备注
pub async fn update_member_profile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, target_user_id)): Path<(String, String)>,
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
    if !member_has_project_permission(&state, &project_id, &user.id, PERMISSION_MANAGE_MEMBERS) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权修改成员资料");
    }

    let Some(obj) = req.as_object() else {
        return json_error(StatusCode::BAD_REQUEST, "请求体必须是 JSON 对象");
    };
    let display_name_update = match clean_member_text_update(
        project_brand_field(obj, "display_name", "displayName"),
        MAX_MEMBER_DISPLAY_NAME_CHARS,
        "成员昵称",
    ) {
        Ok(value) => value,
        Err((status, message)) => return json_error(status, message),
    };
    let admin_note_update = match clean_member_text_update(
        project_brand_field(obj, "admin_note", "adminNote"),
        MAX_MEMBER_ADMIN_NOTE_CHARS,
        "管理员备注",
    ) {
        Ok(value) => value,
        Err((status, message)) => return json_error(status, message),
    };
    if display_name_update.is_none() && admin_note_update.is_none() {
        return json_error(
            StatusCode::BAD_REQUEST,
            "至少需要提供 display_name/displayName 或 admin_note/adminNote",
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
        "修改成员资料",
    ) {
        return json_error(StatusCode::FORBIDDEN, message);
    }

    let display_name_arg = display_name_update.as_ref().map(|value| value.as_deref());
    let admin_note_arg = admin_note_update.as_ref().map(|value| value.as_deref());
    match state.store.update_project_member_profile(
        &project_id,
        &target_user_id,
        display_name_arg,
        admin_note_arg,
    ) {
        Ok(member) => {
            let mut note_parts = Vec::new();
            if let Some(value) = display_name_update {
                note_parts.push(format!(
                    "display_name={}",
                    value.unwrap_or_else(|| "cleared".to_string())
                ));
            }
            if let Some(value) = admin_note_update {
                note_parts.push(format!(
                    "admin_note={}",
                    value
                        .as_deref()
                        .map(|note| format!("{} chars", note.chars().count()))
                        .unwrap_or_else(|| "cleared".to_string())
                ));
            }
            if let Err(err) = state.store.record_project_member_audit(
                &project_id,
                Some(&user.id),
                Some(&target_user_id),
                "update_member_profile",
                old_role.as_deref(),
                old_role.as_deref(),
                Some(&note_parts.join(";")),
            ) {
                tracing::warn!(?err, project_id = %project_id, "记录成员资料更新审计日志失败");
            }
            publish_members_updated(
                &state,
                &project_id,
                "update_member_profile",
                Some(&target_user_id),
                Some(&user.id),
            );
            Json(serde_json::json!({
                "ok": true,
                "project_id": project_id,
                "user_id": target_user_id,
                "member": member,
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不是该项目成员") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, msg)
        }
    }
}


#[path = "project_membership_invites.rs"]
mod invites;
pub use invites::{
    list_project_invite_links, create_project_invite_link,
    revoke_project_invite_link, get_project_invite_preview,
    join_project_by_invite_link,
};

#[path = "project_membership_roles.rs"]
mod roles;
pub use roles::{
    list_member_audit, add_member, update_visibility,
    list_project_roles, create_project_role, update_project_role,
    delete_project_role, update_project_icon, update_project_brand,
};

#[path = "project_membership_edits.rs"]
mod edits;
pub use edits::{update_member_role, remove_member, update_member_moderation};
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

fn publish_members_updated(
    state: &AppState,
    project_id: &str,
    action: &str,
    target_user_id: Option<&str>,
    actor_user_id: Option<&str>,
) {
    crate::project_events::publish_members_updated(
        state,
        project_id,
        action,
        target_user_id,
        actor_user_id,
    );
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

fn clean_member_text_update(
    value: Option<&Value>,
    max_chars: usize,
    label: &str,
) -> Result<Option<Option<String>>, (StatusCode, String)> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(Some(None));
    }
    let Some(value) = value.as_str() else {
        return Err((StatusCode::BAD_REQUEST, format!("{}必须是字符串", label)));
    };
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("null") {
        return Ok(Some(None));
    }
    if value.chars().count() > max_chars {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("{}不能超过 {} 个字", label, max_chars),
        ));
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
#[path = "project_membership_tests.rs"]
mod project_membership_tests;
