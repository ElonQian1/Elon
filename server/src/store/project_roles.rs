use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

use super::{new_id, now, ProjectMemberRoleRef, ProjectRoleEntry, Store};

pub const PERMISSION_VIEW_MEMBERS: &str = "view_members";
pub const PERMISSION_SEND_MESSAGES: &str = "send_messages";
pub const PERMISSION_INVITE_MEMBERS: &str = "invite_members";
pub const PERMISSION_MANAGE_MEMBERS: &str = "manage_members";
pub const PERMISSION_MODERATE_MEMBERS: &str = "moderate_members";
pub const PERMISSION_VIEW_AUDIT_LOG: &str = "view_audit_log";
pub const PERMISSION_MANAGE_ROLES: &str = "manage_roles";
pub const PERMISSION_MANAGE_PROJECT_SETTINGS: &str = "manage_project_settings";

pub(super) const ALL_ROLE_PERMISSIONS: &[&str] = &[
    PERMISSION_VIEW_MEMBERS,
    PERMISSION_SEND_MESSAGES,
    PERMISSION_INVITE_MEMBERS,
    PERMISSION_MANAGE_MEMBERS,
    PERMISSION_MODERATE_MEMBERS,
    PERMISSION_VIEW_AUDIT_LOG,
    PERMISSION_MANAGE_ROLES,
    PERMISSION_MANAGE_PROJECT_SETTINGS,
];

pub(super) const BUILTIN_ROLES: &[(&str, &str, Option<&str>, i64)] = &[
    ("owner", "拥有者", Some("#f0b232"), 100),
    ("admin", "管理员", Some("#ed4245"), 80),
    ("editor", "协作者", Some("#5865f2"), 60),
    ("member", "成员", Some("#43b581"), 40),
    ("observer", "只读成员", Some("#747f8d"), 20),
];

impl Store {
    pub fn list_project_roles(&self, project_id: &str) -> Result<Vec<ProjectRoleEntry>> {
        let conn = self.conn()?;
        list_project_roles_locked(&conn, project_id)
    }

    pub fn create_project_role(
        &self,
        project_id: &str,
        name: &str,
        color: Option<&str>,
        position: Option<i64>,
        permissions: Option<&[String]>,
        created_by: Option<&str>,
    ) -> Result<ProjectRoleEntry> {
        let conn = self.conn()?;
        ensure_project_exists(&conn, project_id)?;
        let name = clean_role_name(name)?;
        let color = clean_role_color(color)?;
        let position = clean_role_position(position.unwrap_or(30));
        let permissions = clean_permissions(permissions.unwrap_or(&[]));
        let permissions_json = serde_json::to_string(&permissions)?;
        let id = new_id("prole");
        let now_str = now();
        conn.execute(
            "INSERT INTO project_roles
             (id, project_id, name, color, position, permissions_json, created_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                id,
                project_id,
                name,
                color,
                position,
                permissions_json,
                created_by,
                now_str
            ],
        )
        .map_err(|err| {
            if err.to_string().contains("UNIQUE") {
                anyhow!("该项目已存在同名角色")
            } else {
                anyhow!(err)
            }
        })?;
        project_role_entry_locked(&conn, project_id, &id)
    }

    pub fn update_project_role(
        &self,
        project_id: &str,
        role_id: &str,
        name: Option<&str>,
        color: Option<Option<&str>>,
        position: Option<i64>,
        permissions: Option<&[String]>,
    ) -> Result<ProjectRoleEntry> {
        let conn = self.conn()?;
        ensure_custom_role_exists(&conn, project_id, role_id)?;
        let current = project_role_entry_locked(&conn, project_id, role_id)?;
        let name = match name {
            Some(value) => clean_role_name(value)?,
            None => current.name,
        };
        let color = match color {
            Some(value) => clean_role_color(value)?,
            None => current.color,
        };
        let position = position
            .map(clean_role_position)
            .unwrap_or(current.position);
        let permissions = permissions
            .map(clean_permissions)
            .unwrap_or(current.permissions);
        let permissions_json = serde_json::to_string(&permissions)?;
        conn.execute(
            "UPDATE project_roles
                SET name = ?3,
                    color = ?4,
                    position = ?5,
                    permissions_json = ?6,
                    updated_at = ?7
              WHERE project_id = ?1 AND id = ?2",
            params![
                project_id,
                role_id,
                name,
                color,
                position,
                permissions_json,
                now()
            ],
        )?;
        project_role_entry_locked(&conn, project_id, role_id)
    }

    pub fn delete_project_role(&self, project_id: &str, role_id: &str) -> Result<()> {
        let conn = self.conn()?;
        ensure_custom_role_exists(&conn, project_id, role_id)?;
        let member_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND role = ?2",
            params![project_id, role_id],
            |row| row.get(0),
        )?;
        if member_count > 0 {
            anyhow::bail!("该角色仍有成员使用，请先调整这些成员角色");
        }
        conn.execute(
            "DELETE FROM project_roles WHERE project_id = ?1 AND id = ?2",
            params![project_id, role_id],
        )?;
        Ok(())
    }

    pub fn project_role_level(&self, project_id: &str, role: &str) -> Result<i64> {
        let conn = self.conn()?;
        project_role_level_locked(&conn, project_id, role)
    }

    pub fn project_role_has_permission(
        &self,
        project_id: &str,
        role: &str,
        permission: &str,
    ) -> Result<bool> {
        let role = normalize_role_key(role);
        if role == "owner" {
            return Ok(true);
        }
        let permission = permission.trim();
        if permission.is_empty() {
            return Ok(false);
        }
        if let Some(permissions) = builtin_project_role_permissions(&role) {
            return Ok(permissions.iter().any(|item| item == permission));
        }
        let conn = self.conn()?;
        let permissions = custom_role_permissions_locked(&conn, project_id, &role)?;
        Ok(permissions.iter().any(|item| item == permission))
    }

    pub fn project_member_roles(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<Vec<ProjectMemberRoleRef>> {
        let conn = self.conn()?;
        project_member_role_refs_locked(&conn, project_id, user_id)
    }

    pub fn project_member_effective_role(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<Option<String>> {
        let conn = self.conn()?;
        project_member_effective_role_locked(&conn, project_id, user_id)
    }

    pub fn project_member_effective_level(&self, project_id: &str, user_id: &str) -> Result<i64> {
        let conn = self.conn()?;
        project_member_effective_level_locked(&conn, project_id, user_id)
    }

    pub fn project_member_has_permission(
        &self,
        project_id: &str,
        user_id: &str,
        permission: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        project_member_has_permission_locked(&conn, project_id, user_id, permission)
    }
}

pub(super) fn normalize_project_member_role_for_project(
    conn: &Connection,
    project_id: &str,
    role: &str,
) -> Result<String> {
    let key = normalize_role_key(role);
    if let Some(builtin) = normalize_builtin_project_member_role(&key) {
        return Ok(builtin.to_string());
    }
    if custom_role_exists_locked(conn, project_id, &key)? {
        return Ok(key);
    }
    anyhow::bail!("role 必须为 admin / editor / member / observer / viewer 或项目自定义角色");
}

pub(super) fn normalize_project_member_roles_for_project(
    conn: &Connection,
    project_id: &str,
    roles: &[String],
) -> Result<Vec<String>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for role in roles {
        let role = role.trim();
        if role.is_empty() {
            continue;
        }
        let normalized = normalize_project_member_role_for_project(conn, project_id, role)?;
        if seen.insert(normalized.clone()) {
            out.push(normalized);
        }
    }
    if out.is_empty() {
        out.push("member".to_string());
    }
    sort_project_role_keys_locked(conn, project_id, &mut out);
    Ok(out)
}

pub(super) fn project_member_role_refs_locked(
    conn: &Connection,
    project_id: &str,
    user_id: &str,
) -> Result<Vec<ProjectMemberRoleRef>> {
    let keys = project_member_role_keys_locked(conn, project_id, user_id)?;
    keys.iter()
        .map(|role| project_member_role_ref_locked(conn, project_id, role))
        .collect()
}

pub(super) fn project_member_effective_role_locked(
    conn: &Connection,
    project_id: &str,
    user_id: &str,
) -> Result<Option<String>> {
    Ok(project_member_role_keys_locked(conn, project_id, user_id)?
        .into_iter()
        .next())
}

pub(super) fn project_member_effective_level_locked(
    conn: &Connection,
    project_id: &str,
    user_id: &str,
) -> Result<i64> {
    let roles = project_member_role_keys_locked(conn, project_id, user_id)?;
    Ok(roles
        .first()
        .map(|role| project_role_level_locked(conn, project_id, role).unwrap_or(0))
        .unwrap_or(0))
}

pub(super) fn project_member_has_permission_locked(
    conn: &Connection,
    project_id: &str,
    user_id: &str,
    permission: &str,
) -> Result<bool> {
    let roles = project_member_role_keys_locked(conn, project_id, user_id)?;
    project_roles_have_permission_locked(conn, project_id, &roles, permission)
}

pub(super) fn project_roles_have_permission_locked(
    conn: &Connection,
    project_id: &str,
    roles: &[String],
    permission: &str,
) -> Result<bool> {
    let permission = permission.trim();
    if permission.is_empty() {
        return Ok(false);
    }
    for role in roles {
        let key = canonical_project_role_key(role);
        if key == "owner" {
            return Ok(true);
        }
        if let Some(permissions) = builtin_project_role_permissions(&key) {
            if permissions.iter().any(|item| item == permission) {
                return Ok(true);
            }
            continue;
        }
        let permissions = custom_role_permissions_locked(conn, project_id, &key)?;
        if permissions.iter().any(|item| item == permission) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn sync_project_member_roles_locked(
    conn: &Connection,
    project_id: &str,
    user_id: &str,
    roles: &[String],
    assigned_by: Option<&str>,
) -> Result<()> {
    conn.execute(
        "DELETE FROM project_member_roles WHERE project_id = ?1 AND user_id = ?2",
        params![project_id, user_id],
    )?;
    let now_str = now();
    for role in roles {
        let role = canonical_project_role_key(role);
        if role.is_empty() || role == "owner" {
            continue;
        }
        conn.execute(
            "INSERT OR IGNORE INTO project_member_roles
             (project_id, user_id, role_id, assigned_by, assigned_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![project_id, user_id, role, assigned_by, &now_str],
        )?;
    }
    Ok(())
}

pub(super) fn project_role_level_locked(
    conn: &Connection,
    project_id: &str,
    role: &str,
) -> Result<i64> {
    let key = normalize_role_key(role);
    if let Some(level) = builtin_project_role_level(&key) {
        return Ok(level);
    }
    let level: Option<i64> = conn
        .query_row(
            "SELECT position FROM project_roles WHERE project_id = ?1 AND id = ?2",
            params![project_id, key],
            |row| row.get(0),
        )
        .optional()?;
    Ok(level.unwrap_or(0))
}

pub(super) fn normalize_builtin_project_member_role(role: &str) -> Option<&'static str> {
    match normalize_role_key(role).as_str() {
        "admin" => Some("admin"),
        "editor" | "developer" | "maintainer" => Some("editor"),
        "member" => Some("member"),
        "observer" | "viewer" => Some("observer"),
        _ => None,
    }
}

pub(super) fn builtin_project_role_level(role: &str) -> Option<i64> {
    match normalize_role_key(role).as_str() {
        "owner" => Some(100),
        "admin" => Some(80),
        "editor" | "developer" | "maintainer" => Some(60),
        "member" => Some(40),
        "observer" | "viewer" => Some(20),
        _ => None,
    }
}

pub(super) fn builtin_project_role_permissions(role: &str) -> Option<Vec<String>> {
    let permissions = match normalize_role_key(role).as_str() {
        "owner" => ALL_ROLE_PERMISSIONS,
        "admin" => &[
            PERMISSION_VIEW_MEMBERS,
            PERMISSION_SEND_MESSAGES,
            PERMISSION_INVITE_MEMBERS,
            PERMISSION_MANAGE_MEMBERS,
            PERMISSION_MODERATE_MEMBERS,
            PERMISSION_VIEW_AUDIT_LOG,
            PERMISSION_MANAGE_ROLES,
            PERMISSION_MANAGE_PROJECT_SETTINGS,
        ][..],
        "editor" | "developer" | "maintainer" => {
            &[PERMISSION_VIEW_MEMBERS, PERMISSION_SEND_MESSAGES][..]
        }
        "member" => &[PERMISSION_VIEW_MEMBERS, PERMISSION_SEND_MESSAGES][..],
        "observer" | "viewer" => &[PERMISSION_VIEW_MEMBERS][..],
        _ => return None,
    };
    Some(
        permissions
            .iter()
            .map(|permission| permission.to_string())
            .collect(),
    )
}


mod helpers;
use self::helpers::*;
