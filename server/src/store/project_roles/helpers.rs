use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

use super::{
    builtin_project_role_level, builtin_project_role_permissions, new_id,
    normalize_builtin_project_member_role, now, project_role_level_locked, ProjectMemberRoleRef,
    ProjectRoleEntry, ALL_ROLE_PERMISSIONS, BUILTIN_ROLES, PERMISSION_INVITE_MEMBERS,
    PERMISSION_MANAGE_MEMBERS, PERMISSION_MANAGE_PROJECT_SETTINGS, PERMISSION_MANAGE_ROLES,
    PERMISSION_MODERATE_MEMBERS, PERMISSION_SEND_MESSAGES, PERMISSION_VIEW_AUDIT_LOG,
    PERMISSION_VIEW_MEMBERS,
};

pub(super) fn list_project_roles_locked(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<ProjectRoleEntry>> {
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

pub(super) fn project_role_entry_locked(
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

pub(super) fn project_role_entry_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProjectRoleEntry> {
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

pub(super) fn project_member_role_keys_locked(
    conn: &Connection,
    project_id: &str,
    user_id: &str,
) -> Result<Vec<String>> {
    let primary: Option<String> = conn
        .query_row(
            "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
            params![project_id, user_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(primary) = primary else {
        return Ok(Vec::new());
    };
    let primary = canonical_project_role_key(&primary);
    if primary == "owner" {
        return Ok(vec!["owner".to_string()]);
    }

    let mut seen = HashSet::new();
    let mut roles = Vec::new();
    if !primary.is_empty() && seen.insert(primary.clone()) {
        roles.push(primary);
    }

    let mut stmt = conn.prepare(
        "SELECT role_id
           FROM project_member_roles
          WHERE project_id = ?1 AND user_id = ?2",
    )?;
    let rows = stmt
        .query_map(params![project_id, user_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for role in rows {
        let role = canonical_project_role_key(&role);
        if role.is_empty() || role == "owner" {
            continue;
        }
        if seen.insert(role.clone()) {
            roles.push(role);
        }
    }
    if roles.is_empty() {
        roles.push("member".to_string());
    }
    sort_project_role_keys_locked(conn, project_id, &mut roles);
    Ok(roles)
}

pub(super) fn project_member_role_ref_locked(
    conn: &Connection,
    project_id: &str,
    role: &str,
) -> Result<ProjectMemberRoleRef> {
    let key = canonical_project_role_key(role);
    if let Some(role_ref) = builtin_project_member_role_ref(project_id, &key) {
        return Ok(role_ref);
    }
    let custom = conn
        .query_row(
            "SELECT id, name, color, position
               FROM project_roles
              WHERE project_id = ?1 AND id = ?2",
            params![project_id, key],
            |row| {
                Ok(ProjectMemberRoleRef {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    position: row.get(3)?,
                    builtin: false,
                })
            },
        )
        .optional()?;
    Ok(custom.unwrap_or_else(|| ProjectMemberRoleRef {
        id: key.clone(),
        name: key,
        color: None,
        position: 0,
        builtin: false,
    }))
}

pub(super) fn builtin_project_member_role_ref(
    project_id: &str,
    role: &str,
) -> Option<ProjectMemberRoleRef> {
    let key = canonical_project_role_key(role);
    BUILTIN_ROLES
        .iter()
        .find(|(id, _, _, _)| *id == key)
        .map(|(id, name, color, position)| ProjectMemberRoleRef {
            id: (*id).to_string(),
            name: (*name).to_string(),
            color: color.map(str::to_string),
            position: *position,
            builtin: true,
        })
        .or_else(|| {
            if key == "owner" {
                Some(ProjectMemberRoleRef {
                    id: "owner".to_string(),
                    name: "拥有者".to_string(),
                    color: Some("#f0b232".to_string()),
                    position: 100,
                    builtin: true,
                })
            } else {
                let _ = project_id;
                None
            }
        })
}

pub(super) fn custom_role_exists_locked(
    conn: &Connection,
    project_id: &str,
    role_id: &str,
) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM project_roles WHERE project_id = ?1 AND id = ?2",
        params![project_id, role_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub(super) fn ensure_custom_role_exists(
    conn: &Connection,
    project_id: &str,
    role_id: &str,
) -> Result<()> {
    if !custom_role_exists_locked(conn, project_id, role_id)? {
        anyhow::bail!("角色不存在或不是自定义角色");
    }
    Ok(())
}

pub(super) fn custom_role_permissions_locked(
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

pub(super) fn project_role_member_count_locked(
    conn: &Connection,
    project_id: &str,
    role_id: &str,
) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM (
             SELECT user_id FROM project_members WHERE project_id = ?1 AND role = ?2
             UNION
             SELECT user_id FROM project_member_roles WHERE project_id = ?1 AND role_id = ?2
           )",
        params![project_id, role_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub(super) fn sort_project_role_keys_locked(
    conn: &Connection,
    project_id: &str,
    roles: &mut [String],
) {
    roles.sort_by(|left, right| {
        let left_level = project_role_level_locked(conn, project_id, left).unwrap_or(0);
        let right_level = project_role_level_locked(conn, project_id, right).unwrap_or(0);
        right_level.cmp(&left_level).then_with(|| left.cmp(right))
    });
}

pub(super) fn ensure_project_exists(conn: &Connection, project_id: &str) -> Result<()> {
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

pub(super) fn clean_role_name(name: &str) -> Result<String> {
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

pub(super) fn clean_role_color(color: Option<&str>) -> Result<Option<String>> {
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

pub(super) fn clean_role_position(position: i64) -> i64 {
    position.clamp(1, 79)
}

pub(super) fn clean_permissions(input: &[String]) -> Vec<String> {
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

pub(super) fn normalize_role_key(role: &str) -> String {
    role.trim().to_ascii_lowercase()
}

pub(super) fn canonical_project_role_key(role: &str) -> String {
    let key = normalize_role_key(role);
    if key == "owner" {
        return key;
    }
    normalize_builtin_project_member_role(&key)
        .unwrap_or(key.as_str())
        .to_string()
}
