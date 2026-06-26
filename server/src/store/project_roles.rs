use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

use super::{new_id, now, ProjectRoleEntry, Store};

pub const PERMISSION_VIEW_MEMBERS: &str = "view_members";
pub const PERMISSION_SEND_MESSAGES: &str = "send_messages";
pub const PERMISSION_INVITE_MEMBERS: &str = "invite_members";
pub const PERMISSION_MANAGE_MEMBERS: &str = "manage_members";
pub const PERMISSION_MODERATE_MEMBERS: &str = "moderate_members";
pub const PERMISSION_VIEW_AUDIT_LOG: &str = "view_audit_log";
pub const PERMISSION_MANAGE_ROLES: &str = "manage_roles";
pub const PERMISSION_MANAGE_PROJECT_SETTINGS: &str = "manage_project_settings";

const ALL_ROLE_PERMISSIONS: &[&str] = &[
    PERMISSION_VIEW_MEMBERS,
    PERMISSION_SEND_MESSAGES,
    PERMISSION_INVITE_MEMBERS,
    PERMISSION_MANAGE_MEMBERS,
    PERMISSION_MODERATE_MEMBERS,
    PERMISSION_VIEW_AUDIT_LOG,
    PERMISSION_MANAGE_ROLES,
    PERMISSION_MANAGE_PROJECT_SETTINGS,
];

const BUILTIN_ROLES: &[(&str, &str, Option<&str>, i64)] = &[
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

fn list_project_roles_locked(conn: &Connection, project_id: &str) -> Result<Vec<ProjectRoleEntry>> {
    ensure_project_exists(conn, project_id)?;
    let mut entries = Vec::new();
    for (id, name, color, position) in BUILTIN_ROLES {
        let member_count = project_role_member_count_locked(conn, project_id, id)?;
        entries.push(ProjectRoleEntry {
            id: (*id).to_string(),
            project_id: project_id.to_string(),
            name: (*name).to_string(),
            color: color.map(str::to_string),
            position: *position,
            permissions: builtin_project_role_permissions(id).unwrap_or_default(),
            builtin: true,
            member_count,
        });
    }
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, color, position, permissions_json
           FROM project_roles
          WHERE project_id = ?1
          ORDER BY position DESC, created_at ASC",
    )?;
    let custom_roles = stmt
        .query_map(params![project_id], |row| project_role_entry_from_row(row))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for mut role in custom_roles {
        role.member_count = project_role_member_count_locked(conn, project_id, &role.id)?;
        entries.push(role);
    }
    entries.sort_by(|left, right| {
        right
            .position
            .cmp(&left.position)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(entries)
}

fn project_role_entry_locked(
    conn: &Connection,
    project_id: &str,
    role_id: &str,
) -> Result<ProjectRoleEntry> {
    let mut entry = conn
        .query_row(
            "SELECT id, project_id, name, color, position, permissions_json
               FROM project_roles
              WHERE project_id = ?1 AND id = ?2",
            params![project_id, role_id],
            |row| project_role_entry_from_row(row),
        )
        .optional()?
        .ok_or_else(|| anyhow!("角色不存在"))?;
    entry.member_count = project_role_member_count_locked(conn, project_id, role_id)?;
    Ok(entry)
}

fn project_role_entry_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRoleEntry> {
    let permissions_json: String = row.get(5)?;
    let permissions = serde_json::from_str::<Vec<String>>(&permissions_json).unwrap_or_default();
    Ok(ProjectRoleEntry {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        color: row.get(3)?,
        position: row.get(4)?,
        permissions,
        builtin: false,
        member_count: 0,
    })
}

fn custom_role_exists_locked(conn: &Connection, project_id: &str, role_id: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM project_roles WHERE project_id = ?1 AND id = ?2",
        params![project_id, role_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn ensure_custom_role_exists(conn: &Connection, project_id: &str, role_id: &str) -> Result<()> {
    if !custom_role_exists_locked(conn, project_id, role_id)? {
        anyhow::bail!("角色不存在或不是自定义角色");
    }
    Ok(())
}

fn custom_role_permissions_locked(
    conn: &Connection,
    project_id: &str,
    role_id: &str,
) -> Result<Vec<String>> {
    let permissions_json: Option<String> = conn
        .query_row(
            "SELECT permissions_json FROM project_roles WHERE project_id = ?1 AND id = ?2",
            params![project_id, role_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(permissions_json
        .and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok())
        .unwrap_or_default())
}

fn project_role_member_count_locked(
    conn: &Connection,
    project_id: &str,
    role_id: &str,
) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND role = ?2",
        params![project_id, role_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn ensure_project_exists(conn: &Connection, project_id: &str) -> Result<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM projects WHERE id = ?1 AND status != 'deleted'",
        params![project_id],
        |row| row.get(0),
    )?;
    if count == 0 {
        anyhow::bail!("项目不存在");
    }
    Ok(())
}

fn clean_role_name(name: &str) -> Result<String> {
    let value = name.trim();
    if value.is_empty() {
        anyhow::bail!("角色名称不能为空");
    }
    if value.chars().count() > 32 {
        anyhow::bail!("角色名称不能超过 32 个字");
    }
    if BUILTIN_ROLES.iter().any(|(_, label, _, _)| *label == value)
        || builtin_project_role_level(value).is_some()
    {
        anyhow::bail!("不能使用内置角色名称");
    }
    Ok(value.to_string())
}

fn clean_role_color(color: Option<&str>) -> Result<Option<String>> {
    let Some(value) = color.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let valid = value.len() == 7
        && value.starts_with('#')
        && value.chars().skip(1).all(|ch| ch.is_ascii_hexdigit());
    if !valid {
        anyhow::bail!("角色颜色必须是 #RRGGBB 格式");
    }
    Ok(Some(value.to_ascii_lowercase()))
}

fn clean_role_position(position: i64) -> i64 {
    position.clamp(1, 79)
}

fn clean_permissions(input: &[String]) -> Vec<String> {
    let allowed: HashSet<&str> = ALL_ROLE_PERMISSIONS.iter().copied().collect();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for permission in input {
        let permission = permission.trim();
        if permission.is_empty() || !allowed.contains(permission) || !seen.insert(permission) {
            continue;
        }
        out.push(permission.to_string());
    }
    out
}

fn normalize_role_key(role: &str) -> String {
    role.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path =
            std::env::temp_dir().join(format!("elon_project_roles_{}.db", Uuid::new_v4().simple()));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn custom_project_role_can_be_assigned_and_checked() {
        let store = temp_store();
        let owner = store
            .create_user("role-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let member = store
            .create_user("role-member@example.com", "secret1", None, None)
            .expect("member should be created");
        let project = store
            .create_project(&owner.id, "Role Project", None, None)
            .expect("project should be created")
            .project;
        let permissions = vec![PERMISSION_INVITE_MEMBERS.to_string()];

        let role = store
            .create_project_role(
                &project.id,
                "审核员",
                Some("#43b581"),
                Some(55),
                Some(&permissions),
                Some(&owner.id),
            )
            .expect("custom role should be created");

        let assigned = store
            .add_project_member_by_account(&project.id, "role-member@example.com", &role.id)
            .expect("custom role should be assignable");
        assert_eq!(assigned.role, role.id);
        assert_eq!(
            store
                .get_project_access(&member.id, &project.id)
                .expect("member access should load")
                .role,
            role.id
        );
        assert_eq!(
            store
                .project_role_level(&project.id, &role.id)
                .expect("role level should load"),
            55
        );
        assert!(store
            .project_role_has_permission(&project.id, &role.id, PERMISSION_INVITE_MEMBERS)
            .expect("role permission should load"));

        let roles = store
            .list_project_roles(&project.id)
            .expect("roles should list");
        let custom = roles
            .iter()
            .find(|item| item.id == role.id)
            .expect("custom role should be listed");
        assert_eq!(custom.member_count, 1);
        assert!(store
            .delete_project_role(&project.id, &role.id)
            .expect_err("role in use should not delete")
            .to_string()
            .contains("成员"));
    }
}
