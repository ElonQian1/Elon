use super::{
    UpdateMemberRoleRequest, MAX_PROJECT_DISPLAY_NAME_CHARS, MAX_PROJECT_ICON_DATA_URL_BYTES,
};
use crate::{
    project_auth::{can_edit, json_error},
    project_events,
    project_ws_protocol::enrich_project_ws_event,
    store::{
        ProjectAccess, ProjectMemberEntry, PERMISSION_INVITE_MEMBERS, PERMISSION_MANAGE_MEMBERS,
        PERMISSION_MANAGE_PROJECT_SETTINGS, PERMISSION_MANAGE_ROLES,
    },
    types::AppState,
};
use axum::{http::StatusCode, response::Response, Json};
use serde_json::{Map, Value};
use std::sync::Arc;

pub(crate) fn can_update_project_icon(role: &str) -> bool {
    role.trim().eq_ignore_ascii_case("owner")
}

pub(crate) fn can_update_project_brand(role: &str) -> bool {
    can_update_project_icon(role)
}

pub(crate) fn apply_member_presence(
    member: &mut crate::store::ProjectMemberEntry,
    connected: bool,
) {
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

pub(crate) fn publish_members_updated(
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

pub(crate) fn member_has_project_permission(
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

pub(crate) fn requested_member_roles(req: UpdateMemberRoleRequest) -> Result<Vec<String>, String> {
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

pub(crate) fn project_role_permission_options() -> Value {
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

pub(crate) fn ensure_role_position_below_manager(
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

pub(crate) fn is_builtin_project_role(role: &str) -> bool {
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

pub(crate) fn ensure_role_management_allowed(
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

pub(crate) fn ensure_role_set_management_allowed(
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

pub(crate) fn ensure_role_management_allowed_by_level(
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

pub(crate) fn project_brand_field<'a>(
    obj: &'a Map<String, Value>,
    snake_case: &str,
    camel_case: &str,
) -> Option<&'a Value> {
    obj.get(snake_case).or_else(|| obj.get(camel_case))
}

pub(crate) fn clean_project_display_name_update(
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

pub(crate) fn clean_member_text_update(
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

pub(crate) fn clean_project_icon_data_url_update(
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

pub(crate) fn clean_project_icon_data_url(
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
