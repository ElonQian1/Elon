/// project_membership.rs — 项目成员关系管理
///
/// 路由（均需登录）：
///   POST   /api/projects/:id/join                          加入公开项目（open=成员，readonly=只读成员）
///   DELETE /api/projects/:id/leave                         退出已加入的项目（owner 不可退出）
///   GET    /api/projects/:id/members                       列出项目所有成员（公开项目无需成员身份）
///   POST   /api/projects/:id/members                       管理员邀请/添加成员
///   PATCH  /api/projects/:id/visibility                    设置公开/私有（仅 owner/admin）
///   PATCH  /api/projects/:id/icon                          修改项目 APK 图标（仅 owner）
///   PATCH  /api/projects/:id/brand                         修改项目展示别名与 logo（仅 owner）
///   PATCH  /api/projects/:id/members/:user_id              改成员角色（仅 owner/admin，不可改 owner/自己）
///   DELETE /api/projects/:id/members/:user_id              踢出成员（仅 owner/admin，不可踢 owner/自己）
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{Map, Value};
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
pub struct UpdateProjectIconRequest {
    #[serde(default, alias = "iconDataUrl")]
    pub icon_data_url: Option<String>,
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
        return json_error(
            StatusCode::FORBIDDEN,
            "只有项目 owner 或管理员才可修改可见性",
        );
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
        clean_project_icon_data_url_update,
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
        return json_error(
            StatusCode::FORBIDDEN,
            "只有项目 owner 或管理员才可修改成员角色",
        );
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
