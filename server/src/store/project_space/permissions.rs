use super::super::project_roles::{
    project_member_effective_role_locked, project_member_has_permission_locked,
    project_member_role_refs_locked,
};
use super::super::{
    ProjectChannelMemberPermissionOverride, ProjectChannelPermissions,
    ProjectChannelRolePermissionOverride, PERMISSION_MANAGE_PROJECT_SETTINGS,
    PERMISSION_SEND_MESSAGES, PERMISSION_VIEW_MEMBERS,
};
use super::{
    CHANNEL_PERMISSIONS, CHANNEL_PERMISSION_MANAGE, CHANNEL_PERMISSION_SEND,
    CHANNEL_PERMISSION_START_AI, CHANNEL_PERMISSION_VIEW,
};
use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

pub(super) fn project_space_is_public_for_visitors_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM projects
          WHERE id = ?1
            AND status != 'deleted'
            AND is_public = 1
            AND join_mode != 'invite'",
        params![project_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub(super) fn visitor_project_channel_permissions(channel_kind: &str) -> ProjectChannelPermissions {
    let can_view = matches!(
        channel_kind,
        "announcements" | "discussion" | "requirements" | "suggestions" | "issues"
    );
    let can_send = matches!(
        channel_kind,
        "discussion" | "requirements" | "suggestions" | "issues"
    );
    ProjectChannelPermissions {
        can_view,
        can_send,
        can_start_ai: false,
        can_manage: false,
    }
}

pub(super) fn project_member_channel_permissions_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    channel_id: &str,
    user_id: &str,
) -> Result<ProjectChannelPermissions> {
    let channel = project_channel_permission_context_locked(conn, project_id, channel_id)?;
    let channel_kind = channel.kind.as_str();
    if !project_member_exists_locked(conn, project_id, user_id)? {
        if project_space_is_public_for_visitors_locked(conn, project_id)? {
            return Ok(visitor_project_channel_permissions(channel_kind));
        }
        return Ok(ProjectChannelPermissions {
            can_view: false,
            can_send: false,
            can_start_ai: false,
            can_manage: false,
        });
    }
    let role_refs = project_member_role_refs_locked(conn, project_id, user_id)?;
    let role_ids: Vec<String> = role_refs.iter().map(|role| role.id.clone()).collect();
    let is_owner = role_ids.iter().any(|role| role == "owner");
    if is_owner {
        return Ok(ProjectChannelPermissions {
            can_view: true,
            can_send: channel_kind != "docs",
            can_start_ai: channel_kind == "ai_development",
            can_manage: true,
        });
    }

    let can_view_base =
        project_member_has_permission_locked(conn, project_id, user_id, PERMISSION_VIEW_MEMBERS)
            .unwrap_or(false);
    let can_send_base =
        project_member_has_permission_locked(conn, project_id, user_id, PERMISSION_SEND_MESSAGES)
            .unwrap_or(false)
            && channel_kind != "docs"
            && channel_kind != "announcements";
    let effective_role =
        project_member_effective_role_locked(conn, project_id, user_id)?.unwrap_or_default();
    let can_start_ai_base = matches!(
        effective_role.as_str(),
        "owner" | "admin" | "editor" | "developer" | "maintainer"
    ) && channel_kind == "ai_development";
    let can_manage_base = project_member_has_permission_locked(
        conn,
        project_id,
        user_id,
        PERMISSION_MANAGE_PROJECT_SETTINGS,
    )
    .unwrap_or(false);

    let can_view_base = apply_category_overrides_locked(
        conn,
        project_id,
        channel.category_id.as_deref(),
        channel.permission_sync,
        user_id,
        &role_ids,
        CHANNEL_PERMISSION_VIEW,
        can_view_base,
    )?;
    let can_view_after_roles = apply_channel_role_overrides_locked(
        conn,
        project_id,
        channel_id,
        &role_ids,
        CHANNEL_PERMISSION_VIEW,
        can_view_base,
    )?;
    let can_view = apply_channel_member_overrides_locked(
        conn,
        project_id,
        channel_id,
        user_id,
        CHANNEL_PERMISSION_VIEW,
        can_view_after_roles,
    )?;
    let can_send_base = apply_category_overrides_locked(
        conn,
        project_id,
        channel.category_id.as_deref(),
        channel.permission_sync,
        user_id,
        &role_ids,
        CHANNEL_PERMISSION_SEND,
        can_send_base,
    )?;
    let can_send_after_roles = apply_channel_role_overrides_locked(
        conn,
        project_id,
        channel_id,
        &role_ids,
        CHANNEL_PERMISSION_SEND,
        can_send_base,
    )?;
    let mut can_send = apply_channel_member_overrides_locked(
        conn,
        project_id,
        channel_id,
        user_id,
        CHANNEL_PERMISSION_SEND,
        can_send_after_roles,
    )?;
    if channel_kind == "docs" {
        can_send = false;
    }
    let can_start_ai_base = apply_category_overrides_locked(
        conn,
        project_id,
        channel.category_id.as_deref(),
        channel.permission_sync,
        user_id,
        &role_ids,
        CHANNEL_PERMISSION_START_AI,
        can_start_ai_base,
    )?;
    let can_start_ai_after_roles = apply_channel_role_overrides_locked(
        conn,
        project_id,
        channel_id,
        &role_ids,
        CHANNEL_PERMISSION_START_AI,
        can_start_ai_base,
    )?;
    let can_start_ai = channel_kind == "ai_development"
        && apply_channel_member_overrides_locked(
            conn,
            project_id,
            channel_id,
            user_id,
            CHANNEL_PERMISSION_START_AI,
            can_start_ai_after_roles,
        )?;
    let can_manage_base = apply_category_overrides_locked(
        conn,
        project_id,
        channel.category_id.as_deref(),
        channel.permission_sync,
        user_id,
        &role_ids,
        CHANNEL_PERMISSION_MANAGE,
        can_manage_base,
    )?;
    let can_manage_after_roles = apply_channel_role_overrides_locked(
        conn,
        project_id,
        channel_id,
        &role_ids,
        CHANNEL_PERMISSION_MANAGE,
        can_manage_base,
    )?;
    let can_manage = apply_channel_member_overrides_locked(
        conn,
        project_id,
        channel_id,
        user_id,
        CHANNEL_PERMISSION_MANAGE,
        can_manage_after_roles,
    )?;
    Ok(ProjectChannelPermissions {
        can_view,
        can_send,
        can_start_ai,
        can_manage,
    })
}

pub(super) fn project_member_channel_category_permissions_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    category_id: &str,
    user_id: &str,
) -> Result<ProjectChannelPermissions> {
    ensure_project_channel_category_exists_locked(conn, project_id, category_id)?;
    let role_refs = project_member_role_refs_locked(conn, project_id, user_id)?;
    let role_ids: Vec<String> = role_refs.iter().map(|role| role.id.clone()).collect();
    let is_owner = role_ids.iter().any(|role| role == "owner");
    if is_owner {
        return Ok(ProjectChannelPermissions {
            can_view: true,
            can_send: true,
            can_start_ai: true,
            can_manage: true,
        });
    }

    let can_view_base =
        project_member_has_permission_locked(conn, project_id, user_id, PERMISSION_VIEW_MEMBERS)
            .unwrap_or(false);
    let can_send_base =
        project_member_has_permission_locked(conn, project_id, user_id, PERMISSION_SEND_MESSAGES)
            .unwrap_or(false);
    let effective_role =
        project_member_effective_role_locked(conn, project_id, user_id)?.unwrap_or_default();
    let can_start_ai_base = matches!(
        effective_role.as_str(),
        "owner" | "admin" | "editor" | "developer" | "maintainer"
    );
    let can_manage_base = project_member_has_permission_locked(
        conn,
        project_id,
        user_id,
        PERMISSION_MANAGE_PROJECT_SETTINGS,
    )
    .unwrap_or(false);

    Ok(ProjectChannelPermissions {
        can_view: apply_category_overrides_locked(
            conn,
            project_id,
            Some(category_id),
            true,
            user_id,
            &role_ids,
            CHANNEL_PERMISSION_VIEW,
            can_view_base,
        )?,
        can_send: apply_category_overrides_locked(
            conn,
            project_id,
            Some(category_id),
            true,
            user_id,
            &role_ids,
            CHANNEL_PERMISSION_SEND,
            can_send_base,
        )?,
        can_start_ai: apply_category_overrides_locked(
            conn,
            project_id,
            Some(category_id),
            true,
            user_id,
            &role_ids,
            CHANNEL_PERMISSION_START_AI,
            can_start_ai_base,
        )?,
        can_manage: apply_category_overrides_locked(
            conn,
            project_id,
            Some(category_id),
            true,
            user_id,
            &role_ids,
            CHANNEL_PERMISSION_MANAGE,
            can_manage_base,
        )?,
    })
}

pub(super) fn apply_category_overrides_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    category_id: Option<&str>,
    permission_sync: bool,
    user_id: &str,
    role_ids: &[String],
    permission: &str,
    base: bool,
) -> Result<bool> {
    let Some(category_id) = category_id else {
        return Ok(base);
    };
    if !permission_sync {
        return Ok(base);
    }
    let after_roles = apply_channel_category_role_overrides_locked(
        conn,
        project_id,
        category_id,
        role_ids,
        permission,
        base,
    )?;
    apply_channel_category_member_overrides_locked(
        conn,
        project_id,
        category_id,
        user_id,
        permission,
        after_roles,
    )
}

pub(super) fn apply_channel_category_role_overrides_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    category_id: &str,
    role_ids: &[String],
    permission: &str,
    base: bool,
) -> Result<bool> {
    if role_ids.is_empty() {
        return Ok(false);
    }
    let mut denied = false;
    let mut allowed = false;
    let mut stmt = conn.prepare(
        "SELECT effect
           FROM project_channel_category_role_permissions
          WHERE project_id = ?1
            AND category_id = ?2
            AND permission = ?3
            AND role_id = ?4",
    )?;
    for role_id in role_ids {
        let effects = stmt
            .query_map(
                params![project_id, category_id, permission, role_id],
                |row| row.get::<_, String>(0),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for effect in effects {
            if effect == "deny" {
                denied = true;
            } else if effect == "allow" {
                allowed = true;
            }
        }
    }
    if denied {
        return Ok(false);
    }
    if allowed {
        return Ok(true);
    }
    Ok(base)
}

pub(super) fn apply_channel_category_member_overrides_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    category_id: &str,
    user_id: &str,
    permission: &str,
    base: bool,
) -> Result<bool> {
    let effects = conn
        .prepare(
            "SELECT effect
               FROM project_channel_category_member_permissions
              WHERE project_id = ?1
                AND category_id = ?2
                AND user_id = ?3
                AND permission = ?4",
        )?
        .query_map(
            params![project_id, category_id, user_id, permission],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if effects.iter().any(|effect| effect == "deny") {
        return Ok(false);
    }
    if effects.iter().any(|effect| effect == "allow") {
        return Ok(true);
    }
    Ok(base)
}

pub(super) fn apply_channel_role_overrides_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    channel_id: &str,
    role_ids: &[String],
    permission: &str,
    base: bool,
) -> Result<bool> {
    if role_ids.is_empty() {
        return Ok(false);
    }
    let mut denied = false;
    let mut allowed = false;
    let mut stmt = conn.prepare(
        "SELECT effect
           FROM project_channel_role_permissions
          WHERE project_id = ?1
            AND channel_id = ?2
            AND permission = ?3
            AND role_id = ?4",
    )?;
    for role_id in role_ids {
        let effects = stmt
            .query_map(
                params![project_id, channel_id, permission, role_id],
                |row| row.get::<_, String>(0),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for effect in effects {
            if effect == "deny" {
                denied = true;
            } else if effect == "allow" {
                allowed = true;
            }
        }
    }
    if denied {
        return Ok(false);
    }
    if allowed {
        return Ok(true);
    }
    Ok(base)
}

pub(super) fn apply_channel_member_overrides_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    channel_id: &str,
    user_id: &str,
    permission: &str,
    base: bool,
) -> Result<bool> {
    let effects = conn
        .prepare(
            "SELECT effect
               FROM project_channel_member_permissions
              WHERE project_id = ?1
                AND channel_id = ?2
                AND user_id = ?3
                AND permission = ?4",
        )?
        .query_map(
            params![project_id, channel_id, user_id, permission],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if effects.iter().any(|effect| effect == "deny") {
        return Ok(false);
    }
    if effects.iter().any(|effect| effect == "allow") {
        return Ok(true);
    }
    Ok(base)
}

pub(super) struct ProjectChannelPermissionContext {
    kind: String,
    category_id: Option<String>,
    permission_sync: bool,
}

pub(super) fn project_channel_permission_context_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    channel_id: &str,
) -> Result<ProjectChannelPermissionContext> {
    conn.query_row(
        "SELECT kind, category_id, COALESCE(permission_sync, 1)
           FROM project_channels
          WHERE project_id = ?1 AND id = ?2",
        params![project_id, channel_id],
        |row| {
            Ok(ProjectChannelPermissionContext {
                kind: row.get(0)?,
                category_id: row.get(1)?,
                permission_sync: row.get::<_, i64>(2)? != 0,
            })
        },
    )
    .optional()?
    .ok_or_else(|| anyhow!("频道不存在"))
}

pub(super) fn ensure_project_channel_exists_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    channel_id: &str,
) -> Result<String> {
    conn.query_row(
        "SELECT kind FROM project_channels WHERE project_id = ?1 AND id = ?2",
        params![project_id, channel_id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| anyhow!("频道不存在"))
}

pub(super) fn ensure_project_channel_category_exists_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    category_id: &str,
) -> Result<String> {
    conn.query_row(
        "SELECT kind
           FROM project_channel_categories
          WHERE project_id = ?1 AND id = ?2",
        params![project_id, category_id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| anyhow!("频道分类不存在"))
}

pub(super) fn project_member_exists_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    user_id: &str,
) -> Result<bool> {
    conn.query_row(
        "SELECT 1 FROM project_members WHERE project_id = ?1 AND user_id = ?2",
        params![project_id, user_id],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(Into::into)
}

pub(super) fn ensure_project_member_exists_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    user_id: &str,
) -> Result<()> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM project_members WHERE project_id = ?1 AND user_id = ?2",
            params![project_id, user_id],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        anyhow::bail!("成员不存在");
    }
    Ok(())
}

pub(super) fn list_project_channel_category_role_permission_overrides_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    category_id: &str,
) -> Result<Vec<ProjectChannelRolePermissionOverride>> {
    let mut stmt = conn.prepare(
        "SELECT role_id, permission, effect
           FROM project_channel_category_role_permissions
          WHERE project_id = ?1 AND category_id = ?2
          ORDER BY role_id, permission",
    )?;
    let rows = stmt
        .query_map(params![project_id, category_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out: Vec<ProjectChannelRolePermissionOverride> = Vec::new();
    for (role_id, permission, effect) in rows {
        let entry = match out.iter_mut().find(|entry| entry.role_id == role_id) {
            Some(entry) => entry,
            None => {
                out.push(ProjectChannelRolePermissionOverride {
                    role_id: role_id.clone(),
                    allow: Vec::new(),
                    deny: Vec::new(),
                });
                out.last_mut().expect("entry should exist")
            }
        };
        if effect == "deny" {
            entry.deny.push(permission);
        } else {
            entry.allow.push(permission);
        }
    }
    Ok(out)
}

pub(super) fn list_project_channel_category_member_permission_overrides_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    category_id: &str,
) -> Result<Vec<ProjectChannelMemberPermissionOverride>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, permission, effect
           FROM project_channel_category_member_permissions
          WHERE project_id = ?1 AND category_id = ?2
          ORDER BY user_id, permission",
    )?;
    let rows = stmt
        .query_map(params![project_id, category_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out: Vec<ProjectChannelMemberPermissionOverride> = Vec::new();
    for (user_id, permission, effect) in rows {
        let entry = match out.iter_mut().find(|entry| entry.user_id == user_id) {
            Some(entry) => entry,
            None => {
                out.push(ProjectChannelMemberPermissionOverride {
                    user_id: user_id.clone(),
                    allow: Vec::new(),
                    deny: Vec::new(),
                });
                out.last_mut().expect("entry should exist")
            }
        };
        if effect == "deny" {
            entry.deny.push(permission);
        } else {
            entry.allow.push(permission);
        }
    }
    Ok(out)
}

pub(super) fn list_project_channel_role_permission_overrides_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    channel_id: &str,
) -> Result<Vec<ProjectChannelRolePermissionOverride>> {
    let mut stmt = conn.prepare(
        "SELECT role_id, permission, effect
           FROM project_channel_role_permissions
          WHERE project_id = ?1 AND channel_id = ?2
          ORDER BY role_id, permission",
    )?;
    let rows = stmt
        .query_map(params![project_id, channel_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out: Vec<ProjectChannelRolePermissionOverride> = Vec::new();
    for (role_id, permission, effect) in rows {
        let entry = match out.iter_mut().find(|entry| entry.role_id == role_id) {
            Some(entry) => entry,
            None => {
                out.push(ProjectChannelRolePermissionOverride {
                    role_id: role_id.clone(),
                    allow: Vec::new(),
                    deny: Vec::new(),
                });
                out.last_mut().expect("entry should exist")
            }
        };
        if effect == "deny" {
            entry.deny.push(permission);
        } else {
            entry.allow.push(permission);
        }
    }
    Ok(out)
}

pub(super) fn list_project_channel_member_permission_overrides_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    channel_id: &str,
) -> Result<Vec<ProjectChannelMemberPermissionOverride>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, permission, effect
           FROM project_channel_member_permissions
          WHERE project_id = ?1 AND channel_id = ?2
          ORDER BY user_id, permission",
    )?;
    let rows = stmt
        .query_map(params![project_id, channel_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out: Vec<ProjectChannelMemberPermissionOverride> = Vec::new();
    for (user_id, permission, effect) in rows {
        let entry = match out.iter_mut().find(|entry| entry.user_id == user_id) {
            Some(entry) => entry,
            None => {
                out.push(ProjectChannelMemberPermissionOverride {
                    user_id: user_id.clone(),
                    allow: Vec::new(),
                    deny: Vec::new(),
                });
                out.last_mut().expect("entry should exist")
            }
        };
        if effect == "deny" {
            entry.deny.push(permission);
        } else {
            entry.allow.push(permission);
        }
    }
    Ok(out)
}

pub(super) fn clean_channel_permissions(input: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for permission in input {
        let permission = permission.trim();
        if permission.is_empty()
            || !CHANNEL_PERMISSIONS.contains(&permission)
            || out.iter().any(|item: &String| item == permission)
        {
            continue;
        }
        out.push(permission.to_string());
    }
    out
}
