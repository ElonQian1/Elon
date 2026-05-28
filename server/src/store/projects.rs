/// store/projects.rs — 项目商店 & 成员管理的数据库查询层
///
/// 职责：
///   - 列出公开项目（商店浏览）
///   - 获取单个公开项目详情
///   - 设置项目公开/私有（visibility）
///   - 加入 / 退出 项目
///   - 列出项目成员
///   - 列出用户已加入（非自建）的项目
use anyhow::{Result, anyhow};
use rusqlite::{params, OptionalExtension};

use super::{now, PublicProjectItem, ProjectMemberEntry, Store};

impl Store {
    // ─── 商店浏览 ────────────────────────────────────────────────────────────

    /// 列出所有公开项目，支持全文搜索（按名称/描述）和分页
    pub fn list_public_projects(
        &self,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PublicProjectItem>> {
        let conn = self.conn()?;
        let pattern = search
            .filter(|s| !s.trim().is_empty())
            .map(|s| format!("%{}%", s.to_ascii_lowercase()));

        let sql = "
            SELECT
              p.id,
              p.name,
              p.description,
              p.template,
              COALESCE(u.phone, u.email, p.created_by) AS owner_account,
              (SELECT COUNT(*) FROM project_members pm2
               WHERE pm2.project_id = p.id) AS member_count,
              p.is_public,
              p.join_mode,
              (SELECT t.status FROM tasks t
               WHERE t.project_id = p.id
               ORDER BY t.created_at DESC LIMIT 1) AS last_task_status,
              p.created_at,
              p.updated_at,
              p.created_by AS owner_id
            FROM projects p
            LEFT JOIN users u ON u.id = p.created_by
            WHERE p.is_public = 1
              AND p.status != 'deleted'
              AND (?1 IS NULL OR LOWER(p.name) LIKE ?1 OR LOWER(COALESCE(p.description,'')) LIKE ?1)
            ORDER BY p.updated_at DESC
            LIMIT ?2 OFFSET ?3";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt
            .query_map(params![pattern, limit, offset], |row| {
                Ok(PublicProjectItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    template: row.get(3)?,
                    owner_account: row.get(4)?,
                    member_count: row.get(5)?,
                    is_public: row.get::<_, i64>(6)? != 0,
                    join_mode: row.get(7)?,
                    last_task_status: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    owner_id: row.get(11).unwrap_or_default(),

                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// 获取单个公开项目详情（不要求是成员）
    pub fn get_public_project(&self, project_id: &str) -> Result<PublicProjectItem> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT
               p.id, p.name, p.description, p.template,
               COALESCE(u.phone, u.email, p.created_by) AS owner_account,
               (SELECT COUNT(*) FROM project_members pm2 WHERE pm2.project_id = p.id),
               p.is_public,
               p.join_mode,
               (SELECT t.status FROM tasks t WHERE t.project_id = p.id
                ORDER BY t.created_at DESC LIMIT 1),
               p.created_at,
               p.updated_at,
               p.created_by AS owner_id
             FROM projects p
             LEFT JOIN users u ON u.id = p.created_by
             WHERE p.id = ?1 AND p.is_public = 1 AND p.status != 'deleted'",
            params![project_id],
            |row| {
                Ok(PublicProjectItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    template: row.get(3)?,
                    owner_account: row.get(4)?,
                    member_count: row.get(5)?,
                    is_public: row.get::<_, i64>(6)? != 0,
                    join_mode: row.get(7)?,
                    last_task_status: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    owner_id: row.get(11).unwrap_or_default(),
                })
            },
        )
        .map_err(|_| anyhow!("项目不存在或未公开"))
    }

    // ─── 成员管理 ────────────────────────────────────────────────────────────

    /// 设置项目公开可见性（仅 owner 调用前需在 handler 层校验 role）
    pub fn set_project_visibility(
        &self,
        project_id: &str,
        is_public: bool,
        join_mode: &str,
    ) -> Result<()> {
        let n = self.conn()?.execute(
            "UPDATE projects SET is_public = ?1, join_mode = ?2, updated_at = ?3
             WHERE id = ?4 AND status != 'deleted'",
            params![is_public as i64, join_mode, now(), project_id],
        )?;
        if n == 0 {
            anyhow::bail!("项目不存在");
        }
        Ok(())
    }

    /// 加入项目（open 模式直接成功；返回 Err 表示无法加入）
    pub fn join_project(&self, user_id: &str, project_id: &str) -> Result<()> {
        let conn = self.conn()?;
        // 检查项目存在且公开
        let (is_public, join_mode): (i64, String) = conn
            .query_row(
                "SELECT is_public, join_mode FROM projects
                 WHERE id = ?1 AND status != 'deleted'",
                params![project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| anyhow!("项目不存在"))?;

        if is_public == 0 {
            anyhow::bail!("该项目不对外公开");
        }
        if join_mode != "open" {
            anyhow::bail!("该项目需要审批才能加入，请联系项目 owner");
        }
        // 已经是成员则幂等成功
        conn.execute(
            "INSERT OR IGNORE INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, 'member', ?3)",
            params![project_id, user_id, now()],
        )?;
        Ok(())
    }

    /// 退出项目（owner 不可退出，由 handler 层校验）
    pub fn leave_project(&self, user_id: &str, project_id: &str) -> Result<()> {
        // 禁止 owner 退出
        let role: Option<String> = self
            .conn()?
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, user_id],
                |row| row.get(0),
            )
            .optional()?;
        match role.as_deref() {
            None => anyhow::bail!("你不是该项目的成员"),
            Some("owner") => anyhow::bail!("项目 owner 不可退出，请先转让所有权或删除项目"),
            _ => {}
        }
        self.conn()?.execute(
            "DELETE FROM project_members WHERE project_id = ?1 AND user_id = ?2",
            params![project_id, user_id],
        )?;
        Ok(())
    }

    /// 列出项目所有成员（公开项目任何人可查；私有项目在 handler 层校验权限）
    pub fn list_project_members(&self, project_id: &str) -> Result<Vec<ProjectMemberEntry>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT pm.user_id,
                    COALESCE(u.nickname, u.phone, u.email, pm.user_id) AS account,
                    u.avatar_data_url,
                    pm.role,
                    pm.created_at
             FROM project_members pm
             LEFT JOIN users u ON u.id = pm.user_id
             WHERE pm.project_id = ?1
             ORDER BY CASE pm.role WHEN 'owner' THEN 0 WHEN 'member' THEN 1 ELSE 2 END, pm.created_at",
        )?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(ProjectMemberEntry {
                    user_id: row.get(0)?,
                    account: row.get(1)?,
                    avatar_data_url: row.get(2)?,
                    role: row.get(3)?,
                    joined_at: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// 列出用户已加入（但非 owner）的公开项目
    pub fn list_joined_projects(&self, user_id: &str) -> Result<Vec<PublicProjectItem>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT
               p.id, p.name, p.description, p.template,
               COALESCE(u.phone, u.email, p.created_by) AS owner_account,
               (SELECT COUNT(*) FROM project_members pm2 WHERE pm2.project_id = p.id),
               p.is_public, p.join_mode,
               (SELECT t.status FROM tasks t WHERE t.project_id = p.id
                ORDER BY t.created_at DESC LIMIT 1),
               p.created_at, p.updated_at,
               p.created_by AS owner_id
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id
             LEFT JOIN users u ON u.id = p.created_by
             WHERE pm.user_id = ?1
               AND pm.role != 'owner'
               AND p.status != 'deleted'
             ORDER BY p.updated_at DESC",
        )?;
        let rows = stmt
            .query_map(params![user_id], |row| {
                Ok(PublicProjectItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    template: row.get(3)?,
                    owner_account: row.get(4)?,
                    member_count: row.get(5)?,
                    is_public: row.get::<_, i64>(6)? != 0,
                    join_mode: row.get(7)?,
                    last_task_status: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    owner_id: row.get(11).unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}
