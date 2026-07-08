/// store/projects.rs — 项目商店 & 成员管理的数据库查询层
///
/// 职责：
///   - 列出公开项目（商店浏览）
///   - 获取单个公开项目详情
///   - 设置项目公开/私有（visibility）
///   - 加入 / 退出 项目
///   - 列出项目成员
///   - 列出用户已加入（非自建）的项目
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::project_roles::{
    normalize_project_member_role_for_project, normalize_project_member_roles_for_project,
    project_member_effective_role_locked, project_member_role_refs_locked,
    sync_project_member_roles_locked,
};
use super::{
    clean_optional, is_system_project_source_type, normalize_account, now, project_branding,
    ProjectDeletionTarget, ProjectMemberEntry, PublicProjectItem, Store,
};

impl Store {
    // ─── 商店浏览 ────────────────────────────────────────────────────────────

    /// 列出所有公开项目，支持全文搜索（按名称/描述）和分页
    pub fn list_public_projects(
        &self,
        search: Option<&str>,
        join_mode: Option<&str>,
        has_apk: Option<bool>,
        sort: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PublicProjectItem>> {
        self.list_public_projects_for_viewer(search, join_mode, has_apk, sort, limit, offset, None)
    }

    /// 列出公开项目；登录用户传入 `viewer_user_id` 时返回该用户在每个项目中的角色。
    pub fn list_public_projects_for_viewer(
        &self,
        search: Option<&str>,
        join_mode: Option<&str>,
        has_apk: Option<bool>,
        sort: Option<&str>,
        limit: i64,
        offset: i64,
        viewer_user_id: Option<&str>,
    ) -> Result<Vec<PublicProjectItem>> {
        let conn = self.conn()?;
        let pattern = search
            .filter(|s| !s.trim().is_empty())
            .map(|s| format!("%{}%", s.to_ascii_lowercase()));
        let join_mode_filter = join_mode.and_then(|mode| match mode.trim() {
            "open" | "approval" | "readonly" => Some(mode.trim().to_string()),
            _ => None,
        });
        let has_apk_filter = has_apk.map(|value| if value { 1_i64 } else { 0_i64 });
        let order_by = match sort.map(str::trim) {
            Some("created") => "p.created_at DESC",
            Some("members") => "member_count DESC, p.updated_at DESC",
            _ => "p.updated_at DESC",
        };

        let sql = format!(
            "
            SELECT
              p.id,
              p.name,
              p.description,
              p.template,
              COALESCE(u.nickname, u.phone, u.email, p.created_by) AS owner_account,
              (SELECT COUNT(*) FROM project_members pm2
               WHERE pm2.project_id = p.id) AS member_count,
              p.is_public,
              p.join_mode,
              (SELECT t.status FROM tasks t
               WHERE t.project_id = p.id
               ORDER BY t.created_at DESC LIMIT 1) AS last_task_status,
              (SELECT t.apk_url FROM tasks t
               WHERE t.project_id = p.id AND t.apk_url IS NOT NULL AND t.apk_url != ''
               ORDER BY t.created_at DESC LIMIT 1) AS latest_apk_url,
              p.icon_data_url,
              p.created_at,
              p.updated_at,
              p.created_by AS owner_id,
              p.source_type,
              p.workspace_path,
              p.display_name,
              (SELECT pm.role FROM project_members pm
               WHERE pm.project_id = p.id AND pm.user_id = ?6
               LIMIT 1) AS viewer_role
            FROM projects p
            LEFT JOIN users u ON u.id = p.created_by
             WHERE p.is_public = 1
              AND p.join_mode != 'invite'
              AND p.status != 'deleted'
              AND p.source_type NOT IN ('agent_balloon', 'chat_memory')
              AND (
                ?1 IS NULL
                OR LOWER(p.name) LIKE ?1
                OR LOWER(COALESCE(p.display_name,'')) LIKE ?1
                OR LOWER(COALESCE(p.description,'')) LIKE ?1
                OR LOWER(COALESCE(u.nickname, '')) LIKE ?1
                OR LOWER(COALESCE(u.phone, u.email, p.created_by)) LIKE ?1
              )
              AND (?2 IS NULL OR p.join_mode = ?2)
              AND (
                ?3 IS NULL
                OR (?3 = 1 AND EXISTS (
                  SELECT 1 FROM tasks t_apk
                  WHERE t_apk.project_id = p.id
                    AND t_apk.apk_url IS NOT NULL
                    AND t_apk.apk_url != ''
                ))
                OR (?3 = 0 AND NOT EXISTS (
                  SELECT 1 FROM tasks t_apk
                  WHERE t_apk.project_id = p.id
                    AND t_apk.apk_url IS NOT NULL
                    AND t_apk.apk_url != ''
                ))
              )
            ORDER BY {order_by}
            LIMIT ?4 OFFSET ?5"
        );

        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt
            .query_map(
                params![
                    pattern,
                    join_mode_filter,
                    has_apk_filter,
                    limit,
                    offset,
                    viewer_user_id
                ],
                |row| {
                    let mut project = PublicProjectItem {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        display_name: row.get(16)?,
                        description: row.get(2)?,
                        template: row.get(3)?,
                        owner_account: row.get(4)?,
                        member_count: row.get(5)?,
                        is_public: row.get::<_, i64>(6)? != 0,
                        join_mode: row.get(7)?,
                        viewer_role: row.get(17)?,
                        last_task_status: row.get(8)?,
                        latest_apk_url: row.get(9)?,
                        icon_data_url: row.get(10)?,
                        created_at: row.get(11)?,
                        updated_at: row.get(12)?,
                        owner_id: row.get(13).unwrap_or_default(),
                    };
                    let source_type: String = row.get(14)?;
                    let workspace_path: Option<String> = row.get(15)?;
                    project_branding::apply_public_project_branding(
                        &mut project,
                        &source_type,
                        workspace_path.as_deref(),
                    );
                    Ok(project)
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        if let Some(viewer_user_id) = viewer_user_id {
            for project in &mut rows {
                project.viewer_role =
                    project_member_effective_role_locked(&conn, &project.id, viewer_user_id)?;
            }
        }
        Ok(rows)
    }

    pub fn count_public_projects(
        &self,
        search: Option<&str>,
        join_mode: Option<&str>,
        has_apk: Option<bool>,
    ) -> Result<i64> {
        let conn = self.conn()?;
        let pattern = search
            .filter(|s| !s.trim().is_empty())
            .map(|s| format!("%{}%", s.to_ascii_lowercase()));
        let join_mode_filter = join_mode.and_then(|mode| match mode.trim() {
            "open" | "approval" | "readonly" => Some(mode.trim().to_string()),
            _ => None,
        });
        let has_apk_filter = has_apk.map(|value| if value { 1_i64 } else { 0_i64 });

        conn.query_row(
            "
            SELECT COUNT(*)
            FROM projects p
            LEFT JOIN users u ON u.id = p.created_by
             WHERE p.is_public = 1
              AND p.join_mode != 'invite'
              AND p.status != 'deleted'
              AND p.source_type NOT IN ('agent_balloon', 'chat_memory')
              AND (
                ?1 IS NULL
                OR LOWER(p.name) LIKE ?1
                OR LOWER(COALESCE(p.display_name,'')) LIKE ?1
                OR LOWER(COALESCE(p.description,'')) LIKE ?1
                OR LOWER(COALESCE(u.nickname, '')) LIKE ?1
                OR LOWER(COALESCE(u.phone, u.email, p.created_by)) LIKE ?1
              )
              AND (?2 IS NULL OR p.join_mode = ?2)
              AND (
                ?3 IS NULL
                OR (?3 = 1 AND EXISTS (
                  SELECT 1 FROM tasks t_apk
                  WHERE t_apk.project_id = p.id
                    AND t_apk.apk_url IS NOT NULL
                    AND t_apk.apk_url != ''
                ))
                OR (?3 = 0 AND NOT EXISTS (
                  SELECT 1 FROM tasks t_apk
                  WHERE t_apk.project_id = p.id
                    AND t_apk.apk_url IS NOT NULL
                    AND t_apk.apk_url != ''
                ))
              )",
            params![pattern, join_mode_filter, has_apk_filter],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    /// 获取单个公开项目详情（不要求是成员）
    pub fn get_public_project(&self, project_id: &str) -> Result<PublicProjectItem> {
        self.get_public_project_for_viewer(project_id, None)
    }

    /// 获取单个公开项目详情；登录用户传入 `viewer_user_id` 时返回该用户角色。
    pub fn get_public_project_for_viewer(
        &self,
        project_id: &str,
        viewer_user_id: Option<&str>,
    ) -> Result<PublicProjectItem> {
        let conn = self.conn()?;
        let mut project = conn
            .query_row(
                "SELECT
               p.id, p.name, p.description, p.template,
               COALESCE(u.nickname, u.phone, u.email, p.created_by) AS owner_account,
               (SELECT COUNT(*) FROM project_members pm2 WHERE pm2.project_id = p.id),
               p.is_public,
               p.join_mode,
               (SELECT t.status FROM tasks t WHERE t.project_id = p.id
                ORDER BY t.created_at DESC LIMIT 1),
               (SELECT t.apk_url FROM tasks t
                WHERE t.project_id = p.id AND t.apk_url IS NOT NULL AND t.apk_url != ''
                ORDER BY t.created_at DESC LIMIT 1),
               p.icon_data_url,
               p.created_at,
               p.updated_at,
               p.created_by AS owner_id,
               p.source_type,
               p.workspace_path,
               p.display_name,
               (SELECT pm.role FROM project_members pm
                WHERE pm.project_id = p.id AND pm.user_id = ?2
                LIMIT 1) AS viewer_role
             FROM projects p
             LEFT JOIN users u ON u.id = p.created_by
             WHERE p.id = ?1
               AND p.is_public = 1
               AND p.join_mode != 'invite'
               AND p.status != 'deleted'
               AND p.source_type NOT IN ('agent_balloon', 'chat_memory')",
                params![project_id, viewer_user_id],
                |row| {
                    let mut project = PublicProjectItem {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        display_name: row.get(16)?,
                        description: row.get(2)?,
                        template: row.get(3)?,
                        owner_account: row.get(4)?,
                        member_count: row.get(5)?,
                        is_public: row.get::<_, i64>(6)? != 0,
                        join_mode: row.get(7)?,
                        viewer_role: row.get(17)?,
                        last_task_status: row.get(8)?,
                        latest_apk_url: row.get(9)?,
                        icon_data_url: row.get(10)?,
                        created_at: row.get(11)?,
                        updated_at: row.get(12)?,
                        owner_id: row.get(13).unwrap_or_default(),
                    };
                    let source_type: String = row.get(14)?;
                    let workspace_path: Option<String> = row.get(15)?;
                    project_branding::apply_public_project_branding(
                        &mut project,
                        &source_type,
                        workspace_path.as_deref(),
                    );
                    Ok(project)
                },
            )
            .map_err(|_| anyhow!("项目不存在或未公开"))?;
        if let Some(viewer_user_id) = viewer_user_id {
            project.viewer_role =
                project_member_effective_role_locked(&conn, &project.id, viewer_user_id)?;
        }
        Ok(project)
    }

    // ─── 成员管理 ────────────────────────────────────────────────────────────

    /// 设置项目公开可见性（调用前需在 handler 层校验 owner/admin）
    pub fn set_project_visibility(
        &self,
        project_id: &str,
        is_public: bool,
        join_mode: &str,
    ) -> Result<()> {
        let conn = self.conn()?;
        ensure_project_not_system(&conn, project_id, "系统归档项目不能公开到项目广场")?;
        let (is_public, join_mode) = if project_id == "elon-self" {
            (true, "approval")
        } else {
            (is_public, join_mode)
        };
        let n = conn.execute(
            "UPDATE projects SET is_public = ?1, join_mode = ?2, updated_at = ?3
             WHERE id = ?4 AND status != 'deleted'",
            params![is_public as i64, join_mode, now(), project_id],
        )?;
        if n == 0 {
            anyhow::bail!("项目不存在");
        }
        Ok(())
    }

    /// 加入项目（open/invite 作为 member；readonly 作为 observer；返回 Err 表示无法加入）
    pub fn set_project_icon_data_url(
        &self,
        project_id: &str,
        icon_data_url: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn()?;
        ensure_project_not_system(&conn, project_id, "系统归档项目不能设置 APK 图标")?;
        let icon_data_url = icon_data_url
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let n = conn.execute(
            "UPDATE projects SET icon_data_url = ?1, updated_at = ?2
             WHERE id = ?3 AND status != 'deleted'",
            params![icon_data_url, now(), project_id],
        )?;
        if n == 0 {
            anyhow::bail!("项目不存在");
        }
        Ok(())
    }

    pub fn join_project(&self, user_id: &str, project_id: &str) -> Result<bool> {
        let conn = self.conn()?;
        // 检查项目存在且公开
        let (is_public, join_mode, source_type): (i64, String, String) = conn
            .query_row(
                "SELECT is_public, join_mode, source_type FROM projects
                 WHERE id = ?1 AND status != 'deleted'",
                params![project_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|_| anyhow!("项目不存在"))?;
        if is_system_project_source_type(&source_type) {
            anyhow::bail!("系统归档项目不支持加入");
        }

        if is_public == 0 {
            anyhow::bail!("该项目不对外公开");
        }
        let banned_count: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM project_member_restrictions
             WHERE project_id = ?1
               AND user_id = ?2
               AND banned_at IS NOT NULL
               AND (banned_until IS NULL OR banned_until > ?3)",
            params![project_id, user_id, now()],
            |row| row.get(0),
        )?;
        if banned_count > 0 {
            anyhow::bail!("你已被该项目封禁，无法加入");
        }
        let existing_role: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, user_id],
                |row| row.get(0),
            )
            .optional()?;
        if existing_role.is_some() {
            return Ok(true);
        }
        if join_mode == "approval" {
            anyhow::bail!("该项目需要审批才能加入，请联系项目管理员");
        }
        if join_mode != "open" && join_mode != "invite" && join_mode != "readonly" {
            anyhow::bail!("该项目只能通过邀请加入");
        }
        let role = if join_mode == "readonly" {
            "observer"
        } else {
            "member"
        };
        // 已经是成员则幂等成功
        conn.execute(
            "INSERT OR IGNORE INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![project_id, user_id, role, now()],
        )?;
        Ok(false)
    }

    /// 退出项目（owner 不可退出，由 handler 层校验）
    pub fn leave_project(&self, user_id: &str, project_id: &str) -> Result<()> {
        // 禁止 owner 退出
        let project_info: Option<(String, String)> = self
            .conn()?
            .query_row(
                "SELECT pm.role, p.source_type
                 FROM project_members pm
                 JOIN projects p ON p.id = pm.project_id
                 WHERE pm.project_id = ?1 AND pm.user_id = ?2 AND p.status != 'deleted'",
                params![project_id, user_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (role, source_type) = match project_info {
            Some(info) => info,
            None => anyhow::bail!("你不是该项目的成员"),
        };
        if is_system_project_source_type(&source_type) {
            anyhow::bail!("系统归档项目不能退出");
        }
        match role.as_str() {
            "owner" => anyhow::bail!("项目 owner 不可退出，请先转让所有权或删除项目"),
            _ => {}
        }
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM project_member_roles WHERE project_id = ?1 AND user_id = ?2",
            params![project_id, user_id],
        )?;
        conn.execute(
            "DELETE FROM project_members WHERE project_id = ?1 AND user_id = ?2",
            params![project_id, user_id],
        )?;
        Ok(())
    }

}

pub(crate) mod listing_cursor;
#[cfg(test)]
mod listing_cursor_tests;
pub(crate) mod member_ops;
#[cfg(test)]
mod tests;

pub(crate) use self::member_ops::*;
