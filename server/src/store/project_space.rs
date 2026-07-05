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
const PROJECT_GALLERY_IMAGE_LIMIT: usize = 4;
const PROJECT_GALLERY_IMAGE_URL_MAX: usize = 2048;

fn parse_project_gallery_images(json: Option<&str>) -> Vec<String> {
    let Some(json) = json else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(json)
        .unwrap_or_default()
        .into_iter()
        .take(PROJECT_GALLERY_IMAGE_LIMIT)
        .map(|value| {
            value
                .trim()
                .take_if_project_gallery_image_url()
                .unwrap_or_default()
        })
        .collect()
}

fn clean_project_gallery_image_url(value: &str) -> Result<Option<String>> {
    let clean = value
        .trim()
        .take_if_project_gallery_image_url()
        .unwrap_or_default();
    if clean.is_empty() {
        return Ok(None);
    }
    if clean.chars().count() > PROJECT_GALLERY_IMAGE_URL_MAX {
        anyhow::bail!(
            "项目图片地址不能超过 {} 个字符",
            PROJECT_GALLERY_IMAGE_URL_MAX
        );
    }
    Ok(Some(clean))
}

trait ProjectGalleryImageUrlExt {
    fn take_if_project_gallery_image_url(&self) -> Option<String>;
}

impl ProjectGalleryImageUrlExt for str {
    fn take_if_project_gallery_image_url(&self) -> Option<String> {
        let value = self.trim();
        let lower = value.to_ascii_lowercase();
        let is_image = lower.starts_with("https://")
            || lower.starts_with("http://")
            || lower.starts_with("data:image/");
        (is_image && !lower.eq("null")).then(|| value.to_string())
    }
}

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
        task_codex_thread_id: row.get(13)?,
        suggestion_status: row.get(14)?,
        suggestion_resolved_by: row.get(15)?,
        suggestion_resolved_by_name: row.get(16)?,
        suggestion_resolved_at: row.get(17)?,
        created_at: row.get(18)?,
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
                    p.display_name, p.gallery_images_json
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
                    member_count: row.get(4)?,
                    icon_data_url: row.get(5)?,
                    gallery_images: parse_project_gallery_images(row.get::<_, Option<String>>(10)?.as_deref()),
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

    pub fn update_project_gallery_image(
        &self,
        project_id: &str,
        slot: usize,
        image_url: Option<&str>,
    ) -> Result<Vec<String>> {
        if slot >= PROJECT_GALLERY_IMAGE_LIMIT {
            anyhow::bail!("项目图片最多支持 {} 张", PROJECT_GALLERY_IMAGE_LIMIT);
        }
        let clean_url = match image_url {
            Some(value) => clean_project_gallery_image_url(value)?,
            None => None,
        };
        let conn = self.conn()?;
        let current_json: Option<String> = conn
            .query_row(
                "SELECT gallery_images_json FROM projects WHERE id = ?1 AND status != 'deleted'",
                params![project_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在"))?;
        let mut images = parse_project_gallery_images(current_json.as_deref());
        while images.len() <= slot {
            images.push(String::new());
        }
        images[slot] = clean_url.unwrap_or_default();
        while images.last().is_some_and(|value| value.trim().is_empty()) {
            images.pop();
        }
        let next_json = if images.iter().any(|value| !value.trim().is_empty()) {
            Some(serde_json::to_string(&images)?)
        } else {
            None
        };
        let updated = conn.execute(
            "UPDATE projects
                SET gallery_images_json = ?1, updated_at = ?2
              WHERE id = ?3 AND status != 'deleted'",
            params![next_json, now(), project_id],
        )?;
        if updated == 0 {
            anyhow::bail!("项目不存在");
        }
        Ok(images)
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
}


mod messages;
mod permissions;
#[cfg(test)]
mod tests;

use self::permissions::*;
