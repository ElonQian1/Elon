/// store/project_space.rs — 项目空间频道与共享消息
///
/// 职责：
///   - 为项目补齐默认频道
///   - 返回项目空间首页需要的摘要、频道、成员
///   - 保存项目频道消息与 AI 任务状态
use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

use super::project_roles::{
    normalize_project_member_role_for_project, project_member_effective_role_locked,
    project_member_has_permission_locked, project_member_role_refs_locked,
};
use super::{
    new_id, now, project_branding, ProjectChannel, ProjectChannelCategory,
    ProjectChannelMemberPermissionOverride, ProjectChannelMessage, ProjectChannelPermissions,
    ProjectChannelRolePermissionOverride, ProjectSpaceSummary, Store,
    PERMISSION_MANAGE_PROJECT_SETTINGS, PERMISSION_SEND_MESSAGES, PERMISSION_VIEW_MEMBERS,
};

pub const CHANNEL_PERMISSION_VIEW: &str = "view_channel";
pub const CHANNEL_PERMISSION_SEND: &str = "send_messages";
pub const CHANNEL_PERMISSION_START_AI: &str = "start_ai_tasks";
pub const CHANNEL_PERMISSION_MANAGE: &str = "manage_channel";

const CHANNEL_PERMISSIONS: &[&str] = &[
    CHANNEL_PERMISSION_VIEW,
    CHANNEL_PERMISSION_SEND,
    CHANNEL_PERMISSION_START_AI,
    CHANNEL_PERMISSION_MANAGE,
];

const DEFAULT_CHANNEL_CATEGORIES: [(&str, &str, i64); 3] = [
    ("开始", "start", 10),
    ("项目资料", "info", 20),
    ("需求反馈", "feedback", 30),
];

const DEFAULT_CHANNELS: [(&str, &str, i64, &str); 8] = [
    ("公告", "announcements", 10, "info"),
    ("文档", "docs", 15, "info"),
    ("讨论", "discussion", 20, "info"),
    ("需求", "requirements", 30, "feedback"),
    ("意见", "suggestions", 35, "feedback"),
    ("问题反馈", "issues", 40, "feedback"),
    ("AI开发", "ai_development", 50, "start"),
    ("构建发布", "builds", 60, "start"),
];

fn project_channel_message_from_row(
    row: &rusqlite::Row<'_>,
    user_id: &str,
) -> rusqlite::Result<ProjectChannelMessage> {
    let sender_user_id: Option<String> = row.get(3)?;
    Ok(ProjectChannelMessage {
        id: row.get(0)?,
        project_id: row.get(1)?,
        channel_id: row.get(2)?,
        outgoing: sender_user_id.as_deref() == Some(user_id),
        sender_user_id,
        sender_name: row.get(4)?,
        sender_avatar_data_url: row.get(5)?,
        reply_to_message_id: row.get(6)?,
        kind: row.get(7)?,
        content: row.get(8)?,
        task_id: row.get(9)?,
        task_status: row.get(10)?,
        task_error: row.get(11)?,
        task_apk_url: row.get(12)?,
        suggestion_status: row.get(13)?,
        suggestion_resolved_by: row.get(14)?,
        suggestion_resolved_by_name: row.get(15)?,
        suggestion_resolved_at: row.get(16)?,
        created_at: row.get(17)?,
    })
}

impl Store {
    pub fn project_space_summary(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<ProjectSpaceSummary> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        let mut project = conn
            .query_row(
            "SELECT p.id, p.name, p.description, COALESCE(pm.role, 'visitor') AS role,
                    (SELECT COUNT(*) FROM project_members count_pm WHERE count_pm.project_id = p.id),
                    p.icon_data_url, p.updated_at, p.source_type, p.workspace_path,
                    p.display_name, p.is_public, p.join_mode
             FROM projects p
             LEFT JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = ?2
             WHERE p.id = ?1
               AND p.status != 'deleted'
               AND (
                    pm.user_id IS NOT NULL
                    OR (p.is_public = 1 AND p.join_mode != 'invite')
               )",
            params![project_id, user_id],
            |row| {
                let mut project = ProjectSpaceSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    display_name: row.get(9)?,
                    description: row.get(2)?,
                    role: row.get(3)?,
                    is_public: row.get::<_, i64>(10)? != 0,
                    join_mode: row.get(11)?,
                    member_count: row.get(4)?,
                    icon_data_url: row.get(5)?,
                    updated_at: row.get(6)?,
                };
                let source_type: String = row.get(7)?;
                let workspace_path: Option<String> = row.get(8)?;
                project_branding::apply_project_space_branding(
                    &mut project,
                    &source_type,
                    workspace_path.as_deref(),
                );
                Ok(project)
            },
        )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))?;
        if project.role != "visitor" {
            if let Some(role) = project_member_effective_role_locked(&conn, project_id, user_id)? {
                project.role = role;
            }
        }
        Ok(project)
    }

    pub fn update_project_description(
        &self,
        user_id: &str,
        project_id: &str,
        description: &str,
    ) -> Result<ProjectSpaceSummary> {
        let clean = description.trim();
        if clean.chars().count() > 240 {
            anyhow::bail!("项目简介不能超过 240 个字");
        }
        let _ = self.project_space_summary(user_id, project_id)?;
        let description_value = if clean.is_empty() {
            None
        } else {
            Some(clean.to_string())
        };
        let conn = self.conn()?;
        let updated = conn.execute(
            "UPDATE projects
                SET description = ?1, updated_at = ?2
              WHERE id = ?3 AND status != 'deleted'",
            params![description_value, now(), project_id],
        )?;
        if updated == 0 {
            anyhow::bail!("项目不存在");
        }
        drop(conn);
        self.project_space_summary(user_id, project_id)
    }

    pub fn list_project_space_channels(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<Vec<ProjectChannel>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT c.id, c.project_id, c.name, c.kind, c.position,
                    c.category_id,
                    cat.name AS category_name,
                    cat.kind AS category_kind,
                    COALESCE(cat.position, 9999) AS category_position,
                    COALESCE(c.permission_sync, 1) AS permission_sync,
                    (
                      SELECT m.content
                      FROM project_channel_messages m
                      WHERE m.project_id = c.project_id AND m.channel_id = c.id
                      ORDER BY m.created_at DESC LIMIT 1
                    ) AS last_message,
                    (
                      SELECT m.created_at
                      FROM project_channel_messages m
                      WHERE m.project_id = c.project_id AND m.channel_id = c.id
                      ORDER BY m.created_at DESC LIMIT 1
                    ) AS last_message_at,
                    (
                      SELECT COUNT(*)
                      FROM project_channel_messages unread
                      LEFT JOIN project_channel_read_states rs
                        ON rs.project_id = unread.project_id
                       AND rs.channel_id = unread.channel_id
                       AND rs.user_id = ?1
                      WHERE unread.project_id = c.project_id
                        AND unread.channel_id = c.id
                        AND COALESCE(rs.last_read_at, '') < unread.created_at
                        AND COALESCE(unread.sender_user_id, '') != ?1
                    ) AS unread_count
             FROM project_channels c
             LEFT JOIN project_channel_categories cat ON cat.id = c.category_id
             WHERE c.project_id = ?2
             ORDER BY COALESCE(cat.position, 9999), c.position, c.created_at",
        )?;
        let mut rows = stmt
            .query_map(params![user_id, project_id], |row| {
                Ok(ProjectChannel {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    kind: row.get(3)?,
                    position: row.get(4)?,
                    category_id: row.get(5)?,
                    category_name: row.get(6)?,
                    category_kind: row.get(7)?,
                    category_position: row.get(8)?,
                    permission_sync: row.get::<_, i64>(9)? != 0,
                    permissions: ProjectChannelPermissions {
                        can_view: false,
                        can_send: false,
                        can_start_ai: false,
                        can_manage: false,
                    },
                    role_overrides: Vec::new(),
                    member_overrides: Vec::new(),
                    last_message: row.get(10)?,
                    last_message_at: row.get(11)?,
                    unread_count: row.get(12)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        let mut visible = Vec::new();
        for mut channel in rows.drain(..) {
            let permissions =
                project_member_channel_permissions_locked(&conn, project_id, &channel.id, user_id)?;
            if !permissions.can_view {
                continue;
            }
            let can_manage_channel = permissions.can_manage;
            channel.permissions = permissions;
            if can_manage_channel {
                channel.role_overrides = list_project_channel_role_permission_overrides_locked(
                    &conn,
                    project_id,
                    &channel.id,
                )?;
                channel.member_overrides = list_project_channel_member_permission_overrides_locked(
                    &conn,
                    project_id,
                    &channel.id,
                )?;
            }
            visible.push(channel);
        }
        Ok(visible)
    }

    pub fn get_project_channel_kind(&self, project_id: &str, channel_id: &str) -> Result<String> {
        self.ensure_project_default_channels(project_id)?;
        self.conn()?
            .query_row(
                "SELECT kind FROM project_channels WHERE project_id = ?1 AND id = ?2",
                params![project_id, channel_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("频道不存在"))
    }

    pub fn project_member_channel_permissions(
        &self,
        project_id: &str,
        channel_id: &str,
        user_id: &str,
    ) -> Result<ProjectChannelPermissions> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        project_member_channel_permissions_locked(&conn, project_id, channel_id, user_id)
    }

    pub fn list_project_channel_categories(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectChannelCategory>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, kind, position
               FROM project_channel_categories
              WHERE project_id = ?1
              ORDER BY position, created_at",
        )?;
        let categories = stmt
            .query_map(params![project_id], |row| {
                Ok(ProjectChannelCategory {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    kind: row.get(3)?,
                    position: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(categories)
    }

    pub fn project_member_channel_category_permissions(
        &self,
        project_id: &str,
        category_id: &str,
        user_id: &str,
    ) -> Result<ProjectChannelPermissions> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        project_member_channel_category_permissions_locked(&conn, project_id, category_id, user_id)
    }

    pub fn list_project_channel_category_role_permission_overrides(
        &self,
        project_id: &str,
        category_id: &str,
    ) -> Result<Vec<ProjectChannelRolePermissionOverride>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        ensure_project_channel_category_exists_locked(&conn, project_id, category_id)?;
        list_project_channel_category_role_permission_overrides_locked(
            &conn,
            project_id,
            category_id,
        )
    }

    pub fn set_project_channel_category_role_permission_override(
        &self,
        project_id: &str,
        category_id: &str,
        role_id: &str,
        allow: &[String],
        deny: &[String],
        updated_by: Option<&str>,
    ) -> Result<Vec<ProjectChannelRolePermissionOverride>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        ensure_project_channel_category_exists_locked(&conn, project_id, category_id)?;
        let role_id = normalize_project_member_role_for_project(&conn, project_id, role_id)?;
        if role_id == "owner" {
            anyhow::bail!("不能覆盖拥有者权限");
        }
        let mut allow = clean_channel_permissions(allow);
        let deny = clean_channel_permissions(deny);
        allow.retain(|permission| !deny.iter().any(|item| item == permission));
        let now_str = now();
        conn.execute(
            "DELETE FROM project_channel_category_role_permissions
             WHERE project_id = ?1 AND category_id = ?2 AND role_id = ?3",
            params![project_id, category_id, role_id],
        )?;
        for permission in deny {
            conn.execute(
                "INSERT INTO project_channel_category_role_permissions
                 (project_id, category_id, role_id, permission, effect, updated_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'deny', ?5, ?6)",
                params![
                    project_id,
                    category_id,
                    role_id,
                    permission,
                    updated_by,
                    &now_str
                ],
            )?;
        }
        for permission in allow {
            conn.execute(
                "INSERT INTO project_channel_category_role_permissions
                 (project_id, category_id, role_id, permission, effect, updated_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'allow', ?5, ?6)",
                params![
                    project_id,
                    category_id,
                    role_id,
                    permission,
                    updated_by,
                    &now_str
                ],
            )?;
        }
        list_project_channel_category_role_permission_overrides_locked(
            &conn,
            project_id,
            category_id,
        )
    }

    pub fn list_project_channel_category_member_permission_overrides(
        &self,
        project_id: &str,
        category_id: &str,
    ) -> Result<Vec<ProjectChannelMemberPermissionOverride>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        ensure_project_channel_category_exists_locked(&conn, project_id, category_id)?;
        list_project_channel_category_member_permission_overrides_locked(
            &conn,
            project_id,
            category_id,
        )
    }

    pub fn set_project_channel_category_member_permission_override(
        &self,
        project_id: &str,
        category_id: &str,
        user_id: &str,
        allow: &[String],
        deny: &[String],
        updated_by: Option<&str>,
    ) -> Result<Vec<ProjectChannelMemberPermissionOverride>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        ensure_project_channel_category_exists_locked(&conn, project_id, category_id)?;
        ensure_project_member_exists_locked(&conn, project_id, user_id)?;
        let mut allow = clean_channel_permissions(allow);
        let deny = clean_channel_permissions(deny);
        allow.retain(|permission| !deny.iter().any(|item| item == permission));
        let now_str = now();
        conn.execute(
            "DELETE FROM project_channel_category_member_permissions
             WHERE project_id = ?1 AND category_id = ?2 AND user_id = ?3",
            params![project_id, category_id, user_id],
        )?;
        for permission in deny {
            conn.execute(
                "INSERT INTO project_channel_category_member_permissions
                 (project_id, category_id, user_id, permission, effect, updated_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'deny', ?5, ?6)",
                params![
                    project_id,
                    category_id,
                    user_id,
                    permission,
                    updated_by,
                    &now_str
                ],
            )?;
        }
        for permission in allow {
            conn.execute(
                "INSERT INTO project_channel_category_member_permissions
                 (project_id, category_id, user_id, permission, effect, updated_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'allow', ?5, ?6)",
                params![
                    project_id,
                    category_id,
                    user_id,
                    permission,
                    updated_by,
                    &now_str
                ],
            )?;
        }
        list_project_channel_category_member_permission_overrides_locked(
            &conn,
            project_id,
            category_id,
        )
    }

    pub fn list_project_channel_role_permission_overrides(
        &self,
        project_id: &str,
        channel_id: &str,
    ) -> Result<Vec<ProjectChannelRolePermissionOverride>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        ensure_project_channel_exists_locked(&conn, project_id, channel_id)?;
        list_project_channel_role_permission_overrides_locked(&conn, project_id, channel_id)
    }

    pub fn set_project_channel_role_permission_override(
        &self,
        project_id: &str,
        channel_id: &str,
        role_id: &str,
        allow: &[String],
        deny: &[String],
        updated_by: Option<&str>,
    ) -> Result<Vec<ProjectChannelRolePermissionOverride>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        ensure_project_channel_exists_locked(&conn, project_id, channel_id)?;
        let role_id = normalize_project_member_role_for_project(&conn, project_id, role_id)?;
        if role_id == "owner" {
            anyhow::bail!("不能覆盖拥有者权限");
        }
        let mut allow = clean_channel_permissions(allow);
        let deny = clean_channel_permissions(deny);
        allow.retain(|permission| !deny.iter().any(|item| item == permission));
        let now_str = now();
        conn.execute(
            "DELETE FROM project_channel_role_permissions
             WHERE project_id = ?1 AND channel_id = ?2 AND role_id = ?3",
            params![project_id, channel_id, role_id],
        )?;
        for permission in deny {
            conn.execute(
                "INSERT INTO project_channel_role_permissions
                 (project_id, channel_id, role_id, permission, effect, updated_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'deny', ?5, ?6)",
                params![project_id, channel_id, role_id, permission, updated_by, &now_str],
            )?;
        }
        for permission in allow {
            conn.execute(
                "INSERT INTO project_channel_role_permissions
                 (project_id, channel_id, role_id, permission, effect, updated_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'allow', ?5, ?6)",
                params![project_id, channel_id, role_id, permission, updated_by, &now_str],
            )?;
        }
        list_project_channel_role_permission_overrides_locked(&conn, project_id, channel_id)
    }

    pub fn list_project_channel_member_permission_overrides(
        &self,
        project_id: &str,
        channel_id: &str,
    ) -> Result<Vec<ProjectChannelMemberPermissionOverride>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        ensure_project_channel_exists_locked(&conn, project_id, channel_id)?;
        list_project_channel_member_permission_overrides_locked(&conn, project_id, channel_id)
    }

    pub fn set_project_channel_member_permission_override(
        &self,
        project_id: &str,
        channel_id: &str,
        user_id: &str,
        allow: &[String],
        deny: &[String],
        updated_by: Option<&str>,
    ) -> Result<Vec<ProjectChannelMemberPermissionOverride>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        ensure_project_channel_exists_locked(&conn, project_id, channel_id)?;
        ensure_project_member_exists_locked(&conn, project_id, user_id)?;
        let mut allow = clean_channel_permissions(allow);
        let deny = clean_channel_permissions(deny);
        allow.retain(|permission| !deny.iter().any(|item| item == permission));
        let now_str = now();
        conn.execute(
            "DELETE FROM project_channel_member_permissions
             WHERE project_id = ?1 AND channel_id = ?2 AND user_id = ?3",
            params![project_id, channel_id, user_id],
        )?;
        for permission in deny {
            conn.execute(
                "INSERT INTO project_channel_member_permissions
                 (project_id, channel_id, user_id, permission, effect, updated_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'deny', ?5, ?6)",
                params![project_id, channel_id, user_id, permission, updated_by, &now_str],
            )?;
        }
        for permission in allow {
            conn.execute(
                "INSERT INTO project_channel_member_permissions
                 (project_id, channel_id, user_id, permission, effect, updated_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'allow', ?5, ?6)",
                params![project_id, channel_id, user_id, permission, updated_by, &now_str],
            )?;
        }
        list_project_channel_member_permission_overrides_locked(&conn, project_id, channel_id)
    }

    pub fn list_project_channel_messages(
        &self,
        user_id: &str,
        project_id: &str,
        channel_id: &str,
        limit: i64,
    ) -> Result<Vec<ProjectChannelMessage>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT m.id, m.project_id, m.channel_id, m.sender_user_id,
                    COALESCE(u.nickname, u.phone, u.email, m.sender_user_id) AS sender_name,
                    u.avatar_data_url,
                    m.reply_to_message_id,
                    m.kind, m.content, m.task_id,
                    t.status AS task_status,
                    t.error AS task_error,
                    t.apk_url AS task_apk_url,
                    m.suggestion_status,
                    m.suggestion_resolved_by,
                    COALESCE(resolver.nickname, resolver.phone, resolver.email, m.suggestion_resolved_by)
                      AS suggestion_resolved_by_name,
                    m.suggestion_resolved_at,
                    m.created_at
             FROM project_channel_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             LEFT JOIN users resolver ON resolver.id = m.suggestion_resolved_by
             LEFT JOIN tasks t ON t.id = m.task_id
             WHERE m.project_id = ?1 AND m.channel_id = ?2
             ORDER BY m.created_at DESC
             LIMIT ?3",
        )?;
        let mut rows = stmt
            .query_map(
                params![project_id, channel_id, limit.clamp(1, 200)],
                |row| project_channel_message_from_row(row, user_id),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.reverse();
        drop(stmt);
        self.mark_project_channel_read_locked(&conn, user_id, project_id, channel_id)?;
        Ok(rows)
    }

    pub fn insert_project_channel_message(
        &self,
        project_id: &str,
        channel_id: &str,
        sender_user_id: Option<&str>,
        kind: &str,
        content: &str,
        task_id: Option<&str>,
        reply_to_message_id: Option<&str>,
    ) -> Result<ProjectChannelMessage> {
        let content = content.trim();
        if content.is_empty() {
            return Err(anyhow!("消息内容不能为空"));
        }
        let kind = match kind {
            "text" | "suggestion" | "ai_task" | "ai_progress" | "ai_result" | "system" => kind,
            _ => "text",
        };
        let suggestion_status = if kind == "suggestion" {
            Some("open")
        } else {
            None
        };
        let id = new_id("pcm");
        let created_at = now();
        let conn = self.conn()?;
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM project_channels WHERE project_id = ?1 AND id = ?2",
                params![project_id, channel_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(anyhow!("频道不存在"));
        }
        conn.execute(
            "INSERT INTO project_channel_messages (
                id, project_id, channel_id, sender_user_id, kind, content, task_id,
                suggestion_status, reply_to_message_id, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                project_id,
                channel_id,
                sender_user_id,
                kind,
                content,
                task_id,
                suggestion_status,
                reply_to_message_id,
                created_at
            ],
        )?;
        conn.execute(
            "UPDATE project_channels SET updated_at = ?1 WHERE id = ?2",
            params![created_at, channel_id],
        )?;
        conn.query_row(
            "SELECT m.id, m.project_id, m.channel_id, m.sender_user_id,
                    COALESCE(u.nickname, u.phone, u.email, m.sender_user_id) AS sender_name,
                    u.avatar_data_url,
                    m.reply_to_message_id,
                    m.kind, m.content, m.task_id,
                    t.status AS task_status,
                    t.error AS task_error,
                    t.apk_url AS task_apk_url,
                    m.suggestion_status,
                    m.suggestion_resolved_by,
                    COALESCE(resolver.nickname, resolver.phone, resolver.email, m.suggestion_resolved_by)
                      AS suggestion_resolved_by_name,
                    m.suggestion_resolved_at,
                    m.created_at
             FROM project_channel_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             LEFT JOIN users resolver ON resolver.id = m.suggestion_resolved_by
             LEFT JOIN tasks t ON t.id = m.task_id
             WHERE m.project_id = ?1 AND m.channel_id = ?2 AND m.id = ?3",
            params![project_id, channel_id, id],
            |row| project_channel_message_from_row(row, sender_user_id.unwrap_or_default()),
        )
        .map_err(Into::into)
    }

    pub fn insert_project_channel_ai_result_once(
        &self,
        project_id: &str,
        channel_id: &str,
        content: &str,
        task_id: &str,
    ) -> Result<bool> {
        let content = content.trim();
        if content.is_empty() {
            return Err(anyhow!("消息内容不能为空"));
        }
        let created_at = now();
        let conn = self.conn()?;
        let exists: Option<String> = conn
            .query_row(
                "SELECT id
                 FROM project_channel_messages
                 WHERE project_id = ?1
                   AND channel_id = ?2
                   AND task_id = ?3
                   AND kind = 'ai_result'
                 LIMIT 1",
                params![project_id, channel_id, task_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_some() {
            return Ok(false);
        }
        let channel_exists: Option<String> = conn
            .query_row(
                "SELECT id FROM project_channels WHERE project_id = ?1 AND id = ?2",
                params![project_id, channel_id],
                |row| row.get(0),
            )
            .optional()?;
        if channel_exists.is_none() {
            return Err(anyhow!("频道不存在"));
        }

        conn.execute(
            "INSERT INTO project_channel_messages (
                id, project_id, channel_id, sender_user_id, kind, content, task_id, created_at
             )
             VALUES (?1, ?2, ?3, NULL, 'ai_result', ?4, ?5, ?6)",
            params![
                new_id("pcm"),
                project_id,
                channel_id,
                content,
                task_id,
                created_at
            ],
        )?;
        conn.execute(
            "UPDATE project_channels SET updated_at = ?1 WHERE project_id = ?2 AND id = ?3",
            params![created_at, project_id, channel_id],
        )?;
        Ok(true)
    }

    pub fn mark_project_suggestion_updated(
        &self,
        user_id: &str,
        project_id: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<ProjectChannelMessage> {
        self.ensure_project_default_channels(project_id)?;
        let resolved_at = now();
        let conn = self.conn()?;
        let channel_kind: Option<String> = conn
            .query_row(
                "SELECT kind FROM project_channels WHERE project_id = ?1 AND id = ?2",
                params![project_id, channel_id],
                |row| row.get(0),
            )
            .optional()?;
        if channel_kind.as_deref() != Some("suggestions") {
            return Err(anyhow!("只有意见频道的建议可以标记为已更新"));
        }

        let changed = conn.execute(
            "UPDATE project_channel_messages
                SET suggestion_status = 'updated',
                    suggestion_resolved_by = ?1,
                    suggestion_resolved_at = ?2
              WHERE project_id = ?3
                AND channel_id = ?4
                AND id = ?5
                AND kind = 'suggestion'",
            params![user_id, resolved_at, project_id, channel_id, message_id],
        )?;
        if changed == 0 {
            return Err(anyhow!("建议不存在"));
        }
        conn.execute(
            "UPDATE project_channels SET updated_at = ?1 WHERE project_id = ?2 AND id = ?3",
            params![resolved_at, project_id, channel_id],
        )?;

        conn.query_row(
            "SELECT m.id, m.project_id, m.channel_id, m.sender_user_id,
                    COALESCE(u.nickname, u.phone, u.email, m.sender_user_id) AS sender_name,
                    u.avatar_data_url,
                    m.reply_to_message_id,
                    m.kind, m.content, m.task_id,
                    t.status AS task_status,
                    t.error AS task_error,
                    t.apk_url AS task_apk_url,
                    m.suggestion_status,
                    m.suggestion_resolved_by,
                    COALESCE(resolver.nickname, resolver.phone, resolver.email, m.suggestion_resolved_by)
                      AS suggestion_resolved_by_name,
                    m.suggestion_resolved_at,
                    m.created_at
             FROM project_channel_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             LEFT JOIN users resolver ON resolver.id = m.suggestion_resolved_by
             LEFT JOIN tasks t ON t.id = m.task_id
            WHERE m.project_id = ?1 AND m.channel_id = ?2 AND m.id = ?3",
            params![project_id, channel_id, message_id],
            |row| project_channel_message_from_row(row, user_id),
        )
        .map_err(Into::into)
    }

    pub fn ensure_project_default_channels(&self, project_id: &str) -> Result<()> {
        let conn = self.conn()?;
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM projects WHERE id = ?1 AND status != 'deleted'",
                params![project_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(anyhow!("项目不存在"));
        }
        let created_at = now();
        for (name, kind, position) in DEFAULT_CHANNEL_CATEGORIES {
            conn.execute(
                "INSERT OR IGNORE INTO project_channel_categories (
                    id, project_id, name, kind, position, created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![new_id("pcc"), project_id, name, kind, position, created_at],
            )?;
        }
        let mut category_stmt = conn.prepare(
            "SELECT kind, id
               FROM project_channel_categories
              WHERE project_id = ?1",
        )?;
        let category_rows = category_stmt
            .query_map(params![project_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(category_stmt);
        let category_ids: HashMap<String, String> = category_rows.into_iter().collect();
        for (name, kind, position, category_kind) in DEFAULT_CHANNELS {
            let category_id = category_ids.get(category_kind).map(String::as_str);
            conn.execute(
                "INSERT OR IGNORE INTO project_channels (
                    id, project_id, category_id, name, kind, position, created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    new_id("pch"),
                    project_id,
                    category_id,
                    name,
                    kind,
                    position,
                    created_at
                ],
            )?;
            if let Some(category_id) = category_id {
                conn.execute(
                    "UPDATE project_channels
                        SET category_id = ?1,
                            permission_sync = COALESCE(permission_sync, 1)
                      WHERE project_id = ?2
                        AND kind = ?3
                        AND (category_id IS NULL OR category_id = '')",
                    params![category_id, project_id, kind],
                )?;
            }
        }
        Ok(())
    }

    fn mark_project_channel_read_locked(
        &self,
        conn: &rusqlite::Connection,
        user_id: &str,
        project_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        conn.execute(
            "INSERT INTO project_channel_read_states (
                project_id, channel_id, user_id, last_read_at
             )
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id, channel_id, user_id)
             DO UPDATE SET last_read_at = excluded.last_read_at",
            params![project_id, channel_id, user_id, now()],
        )?;
        Ok(())
    }
}

fn project_space_is_public_for_visitors_locked(
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

fn visitor_project_channel_permissions(channel_kind: &str) -> ProjectChannelPermissions {
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

fn project_member_channel_permissions_locked(
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

fn project_member_channel_category_permissions_locked(
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

fn apply_category_overrides_locked(
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

fn apply_channel_category_role_overrides_locked(
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

fn apply_channel_category_member_overrides_locked(
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

fn apply_channel_role_overrides_locked(
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

fn apply_channel_member_overrides_locked(
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

struct ProjectChannelPermissionContext {
    kind: String,
    category_id: Option<String>,
    permission_sync: bool,
}

fn project_channel_permission_context_locked(
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

fn ensure_project_channel_exists_locked(
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

fn ensure_project_channel_category_exists_locked(
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

fn project_member_exists_locked(
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

fn ensure_project_member_exists_locked(
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

fn list_project_channel_category_role_permission_overrides_locked(
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

fn list_project_channel_category_member_permission_overrides_locked(
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

fn list_project_channel_role_permission_overrides_locked(
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

fn list_project_channel_member_permission_overrides_locked(
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

fn clean_channel_permissions(input: &[String]) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_project_suggestions_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn default_channels_include_suggestions() {
        let store = temp_store();
        let owner = store
            .create_user("suggestions-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let project = store
            .create_project(&owner.id, "Suggestions Project", None, None)
            .expect("project should be created")
            .project;

        let channels = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("channels should list");

        assert!(channels.iter().any(|channel| channel.kind == "suggestions"));
        assert!(channels.iter().any(|channel| channel.kind == "docs"));
    }

    #[test]
    fn suggestion_message_can_be_marked_updated() {
        let store = temp_store();
        let owner = store
            .create_user("suggestions-resolver@example.com", "secret1", None, None)
            .expect("owner should be created");
        let project = store
            .create_project(&owner.id, "Suggestion Resolve", None, None)
            .expect("project should be created")
            .project;
        let channel = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("channels should list")
            .into_iter()
            .find(|channel| channel.kind == "suggestions")
            .expect("suggestions channel should exist");

        let message = store
            .insert_project_channel_message(
                &project.id,
                &channel.id,
                Some(&owner.id),
                "suggestion",
                "希望增加深色模式",
                None,
                None,
            )
            .expect("suggestion should insert");
        assert_eq!(message.suggestion_status.as_deref(), Some("open"));

        let updated = store
            .mark_project_suggestion_updated(&owner.id, &project.id, &channel.id, &message.id)
            .expect("suggestion should update");

        assert_eq!(updated.suggestion_status.as_deref(), Some("updated"));
        assert_eq!(
            updated.suggestion_resolved_by.as_deref(),
            Some(owner.id.as_str())
        );
    }

    #[test]
    fn project_description_can_be_updated_and_cleared() {
        let store = temp_store();
        let owner = store
            .create_user("intro-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let project = store
            .create_project(&owner.id, "Intro Project", Some("旧简介"), None)
            .expect("project should be created")
            .project;

        let updated = store
            .update_project_description(&owner.id, &project.id, "  一款太逃杀类型的卡牌游戏  ")
            .expect("description should update");
        assert_eq!(
            updated.description.as_deref(),
            Some("一款太逃杀类型的卡牌游戏")
        );

        let cleared = store
            .update_project_description(&owner.id, &project.id, "   ")
            .expect("description should clear");
        assert!(cleared.description.is_none());
    }

    #[test]
    fn channel_role_permission_overrides_hide_and_allow_channels() {
        let store = temp_store();
        let owner = store
            .create_user("channel-perm-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let viewer = store
            .create_user("channel-perm-viewer@example.com", "secret1", None, None)
            .expect("viewer should be created");
        let project = store
            .create_project(&owner.id, "Channel Permissions", None, None)
            .expect("project should be created")
            .project;
        store
            .add_project_member_by_account(
                &project.id,
                "channel-perm-viewer@example.com",
                "observer",
            )
            .expect("viewer should be invited");
        let channel = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("owner channels should list")
            .into_iter()
            .find(|channel| channel.kind == "discussion")
            .expect("discussion channel should exist");

        store
            .set_project_channel_role_permission_override(
                &project.id,
                &channel.id,
                "observer",
                &[],
                &[CHANNEL_PERMISSION_VIEW.to_string()],
                Some(&owner.id),
            )
            .expect("deny override should save");
        let permissions = store
            .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
            .expect("permissions should load");
        assert!(!permissions.can_view);
        assert!(!store
            .list_project_space_channels(&viewer.id, &project.id)
            .expect("viewer channels should list")
            .iter()
            .any(|item| item.id == channel.id));

        store
            .set_project_channel_role_permission_override(
                &project.id,
                &channel.id,
                "observer",
                &[
                    CHANNEL_PERMISSION_VIEW.to_string(),
                    CHANNEL_PERMISSION_SEND.to_string(),
                ],
                &[],
                Some(&owner.id),
            )
            .expect("allow override should save");
        let permissions = store
            .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
            .expect("permissions should reload");
        assert!(permissions.can_view);
        assert!(permissions.can_send);
    }

    #[test]
    fn channel_category_permissions_are_inherited_before_channel_overrides() {
        let store = temp_store();
        let owner = store
            .create_user("category-perm-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let viewer = store
            .create_user("category-perm-viewer@example.com", "secret1", None, None)
            .expect("viewer should be created");
        let project = store
            .create_project(&owner.id, "Category Permissions", None, None)
            .expect("project should be created")
            .project;
        store
            .add_project_member_by_account(
                &project.id,
                "category-perm-viewer@example.com",
                "observer",
            )
            .expect("viewer should be invited");
        let categories = store
            .list_project_channel_categories(&project.id)
            .expect("categories should list");
        let feedback = categories
            .iter()
            .find(|category| category.kind == "feedback")
            .expect("feedback category should exist");
        let channel = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("owner channels should list")
            .into_iter()
            .find(|channel| channel.kind == "requirements")
            .expect("requirements channel should exist");

        store
            .set_project_channel_category_role_permission_override(
                &project.id,
                &feedback.id,
                "observer",
                &[],
                &[CHANNEL_PERMISSION_VIEW.to_string()],
                Some(&owner.id),
            )
            .expect("category deny should save");
        assert!(
            !store
                .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
                .expect("permissions should load")
                .can_view
        );

        store
            .set_project_channel_role_permission_override(
                &project.id,
                &channel.id,
                "observer",
                &[CHANNEL_PERMISSION_VIEW.to_string()],
                &[],
                Some(&owner.id),
            )
            .expect("channel allow should save");
        assert!(
            store
                .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
                .expect("permissions should reload")
                .can_view
        );
    }

    #[test]
    fn channel_member_permission_overrides_can_override_role_denies() {
        let store = temp_store();
        let owner = store
            .create_user(
                "channel-member-perm-owner@example.com",
                "secret1",
                None,
                None,
            )
            .expect("owner should be created");
        let viewer = store
            .create_user(
                "channel-member-perm-viewer@example.com",
                "secret1",
                None,
                None,
            )
            .expect("viewer should be created");
        let project = store
            .create_project(&owner.id, "Channel Member Permissions", None, None)
            .expect("project should be created")
            .project;
        store
            .add_project_member_by_account(
                &project.id,
                "channel-member-perm-viewer@example.com",
                "observer",
            )
            .expect("viewer should be invited");
        let channel = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("owner channels should list")
            .into_iter()
            .find(|channel| channel.kind == "discussion")
            .expect("discussion channel should exist");

        store
            .set_project_channel_role_permission_override(
                &project.id,
                &channel.id,
                "observer",
                &[],
                &[CHANNEL_PERMISSION_VIEW.to_string()],
                Some(&owner.id),
            )
            .expect("role deny override should save");
        assert!(
            !store
                .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
                .expect("permissions should load")
                .can_view
        );

        store
            .set_project_channel_member_permission_override(
                &project.id,
                &channel.id,
                &viewer.id,
                &[
                    CHANNEL_PERMISSION_VIEW.to_string(),
                    CHANNEL_PERMISSION_SEND.to_string(),
                ],
                &[],
                Some(&owner.id),
            )
            .expect("member allow override should save");
        let permissions = store
            .project_member_channel_permissions(&project.id, &channel.id, &viewer.id)
            .expect("permissions should reload");
        assert!(permissions.can_view);
        assert!(permissions.can_send);
    }
}
