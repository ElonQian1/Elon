//! store/join_requests.rs — 项目加入申请 DB 操作
//!
//! 当项目 join_mode='approval' 时，用户提交申请，owner 审批通过后自动添加成员。

use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{
    common::{new_id, now},
    store_types::JoinRequestRecord,
    Store,
};

impl Store {
    /// 提交加入申请（幂等：已有 pending 申请则返回现有记录 ID）
    pub fn create_join_request(
        &self,
        user_id: &str,
        project_id: &str,
        message: Option<&str>,
    ) -> Result<JoinRequestRecord> {
        let conn = self.conn()?;

        // 检查项目是否存在且为 approval 模式
        let (is_public, join_mode): (i64, String) = conn
            .query_row(
                "SELECT is_public, join_mode FROM projects WHERE id = ?1 AND status != 'deleted'",
                params![project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| anyhow!("项目不存在"))?;

        if is_public == 0 {
            anyhow::bail!("该项目不对外公开");
        }
        if join_mode != "approval" {
            anyhow::bail!("该项目不需要申请，join_mode={join_mode}");
        }

        // 已是成员则直接拒绝
        let already_member: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, user_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        if already_member {
            anyhow::bail!("你已经是该项目成员");
        }

        // 幂等：已有 pending/rejected 申请时 upsert
        let now_str = now();
        let req_id: Option<String> = conn
            .query_row(
                "SELECT id FROM project_join_requests
                 WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, user_id],
                |row| row.get(0),
            )
            .optional()?;

        let id = if let Some(existing_id) = req_id {
            // 重新激活为 pending（若已 rejected 再次申请）
            conn.execute(
                "UPDATE project_join_requests
                 SET status='pending', message=?3, reviewed_by=NULL, reviewed_at=NULL, updated_at=?4
                 WHERE id=?1 AND project_id=?2",
                params![existing_id, project_id, message, now_str],
            )?;
            existing_id
        } else {
            let new_id = new_id("jr");
            conn.execute(
                "INSERT INTO project_join_requests
                 (id, project_id, user_id, message, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?5)",
                params![new_id, project_id, user_id, message, now_str],
            )?;
            new_id
        };

        drop(conn);
        self.get_join_request_by_id(&id)
    }

    /// 获取单条申请详情
    pub fn get_join_request_by_id(&self, req_id: &str) -> Result<JoinRequestRecord> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT jr.id, jr.project_id, p.name, jr.user_id,
                    COALESCE(u.nickname, u.phone, u.email, jr.user_id),
                    u.avatar_data_url,
                    jr.message, jr.status, jr.reviewed_by, jr.reviewed_at, jr.created_at
             FROM project_join_requests jr
             JOIN projects p ON p.id = jr.project_id
             LEFT JOIN users u ON u.id = jr.user_id
             WHERE jr.id = ?1",
            params![req_id],
            |row| {
                Ok(JoinRequestRecord {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    project_name: row.get(2)?,
                    user_id: row.get(3)?,
                    user_account: row.get(4)?,
                    user_avatar: row.get(5)?,
                    message: row.get(6)?,
                    status: row.get(7)?,
                    reviewed_by: row.get(8)?,
                    reviewed_at: row.get(9)?,
                    created_at: row.get(10)?,
                })
            },
        )
        .map_err(|_| anyhow!("申请记录不存在"))
    }

    /// 列出项目所有待审批（或全部）申请（owner 使用）
    pub fn list_join_requests(
        &self,
        project_id: &str,
        only_pending: bool,
    ) -> Result<Vec<JoinRequestRecord>> {
        let conn = self.conn()?;
        let sql = if only_pending {
            "SELECT jr.id, jr.project_id, p.name, jr.user_id,
                    COALESCE(u.nickname, u.phone, u.email, jr.user_id),
                    u.avatar_data_url,
                    jr.message, jr.status, jr.reviewed_by, jr.reviewed_at, jr.created_at
             FROM project_join_requests jr
             JOIN projects p ON p.id = jr.project_id
             LEFT JOIN users u ON u.id = jr.user_id
             WHERE jr.project_id = ?1 AND jr.status = 'pending'
             ORDER BY jr.created_at ASC"
        } else {
            "SELECT jr.id, jr.project_id, p.name, jr.user_id,
                    COALESCE(u.nickname, u.phone, u.email, jr.user_id),
                    u.avatar_data_url,
                    jr.message, jr.status, jr.reviewed_by, jr.reviewed_at, jr.created_at
             FROM project_join_requests jr
             JOIN projects p ON p.id = jr.project_id
             LEFT JOIN users u ON u.id = jr.user_id
             WHERE jr.project_id = ?1
             ORDER BY jr.created_at DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok(JoinRequestRecord {
                id: row.get(0)?,
                project_id: row.get(1)?,
                project_name: row.get(2)?,
                user_id: row.get(3)?,
                user_account: row.get(4)?,
                user_avatar: row.get(5)?,
                message: row.get(6)?,
                status: row.get(7)?,
                reviewed_by: row.get(8)?,
                reviewed_at: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 列出当前用户自己的所有申请
    pub fn list_my_join_requests(&self, user_id: &str) -> Result<Vec<JoinRequestRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT jr.id, jr.project_id, p.name, jr.user_id,
                    COALESCE(u.nickname, u.phone, u.email, jr.user_id),
                    u.avatar_data_url,
                    jr.message, jr.status, jr.reviewed_by, jr.reviewed_at, jr.created_at
             FROM project_join_requests jr
             JOIN projects p ON p.id = jr.project_id
             LEFT JOIN users u ON u.id = jr.user_id
             WHERE jr.user_id = ?1
             ORDER BY jr.created_at DESC",
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(JoinRequestRecord {
                id: row.get(0)?,
                project_id: row.get(1)?,
                project_name: row.get(2)?,
                user_id: row.get(3)?,
                user_account: row.get(4)?,
                user_avatar: row.get(5)?,
                message: row.get(6)?,
                status: row.get(7)?,
                reviewed_by: row.get(8)?,
                reviewed_at: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 审批通过：将申请标记 approved，并添加成员（role=member）
    ///
    /// 安全要求：
    /// - req_id 必须属于 project_id（防止跨项目审批）
    /// - reviewer_user_id 必须是该项目 owner
    pub fn approve_join_request(
        &self,
        req_id: &str,
        project_id: &str,
        reviewer_user_id: &str,
    ) -> Result<JoinRequestRecord> {
        let conn = self.conn()?;
        let now_str = now();

        // 校验申请存在且 pending
        let (req_project_id, user_id, status): (String, String, String) = conn
            .query_row(
                "SELECT project_id, user_id, status FROM project_join_requests WHERE id = ?1",
                params![req_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|_| anyhow!("申请记录不存在"))?;

        if req_project_id != project_id {
            anyhow::bail!("申请记录不属于当前项目");
        }

        // 仅 owner 可审批
        let reviewer_role: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, reviewer_user_id],
                |row| row.get(0),
            )
            .optional()?;
        if reviewer_role.as_deref() != Some("owner") {
            anyhow::bail!("仅项目 owner 可审批加入申请");
        }

        if status != "pending" {
            anyhow::bail!("申请已处理（当前状态：{status}）");
        }

        // 更新申请状态
        conn.execute(
            "UPDATE project_join_requests
             SET status='approved', reviewed_by=?2, reviewed_at=?3, updated_at=?3
             WHERE id=?1",
            params![req_id, reviewer_user_id, now_str],
        )?;

        // 添加为项目成员（幂等）
        conn.execute(
            "INSERT OR IGNORE INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, 'member', ?3)",
            params![req_project_id, user_id, now_str],
        )?;

        drop(conn);
        self.get_join_request_by_id(req_id)
    }

    /// 拒绝申请
    ///
    /// 安全要求：
    /// - req_id 必须属于 project_id（防止跨项目审批）
    /// - reviewer_user_id 必须是该项目 owner
    pub fn reject_join_request(
        &self,
        req_id: &str,
        project_id: &str,
        reviewer_user_id: &str,
    ) -> Result<JoinRequestRecord> {
        let conn = self.conn()?;
        let now_str = now();

        let (req_project_id, status): (String, String) = conn
            .query_row(
                "SELECT project_id, status FROM project_join_requests WHERE id = ?1",
                params![req_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| anyhow!("申请记录不存在"))?;

        if req_project_id != project_id {
            anyhow::bail!("申请记录不属于当前项目");
        }

        // 仅 owner 可审批
        let reviewer_role: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, reviewer_user_id],
                |row| row.get(0),
            )
            .optional()?;
        if reviewer_role.as_deref() != Some("owner") {
            anyhow::bail!("仅项目 owner 可审批加入申请");
        }

        if status != "pending" {
            anyhow::bail!("申请已处理（当前状态：{status}）");
        }

        conn.execute(
            "UPDATE project_join_requests
             SET status='rejected', reviewed_by=?2, reviewed_at=?3, updated_at=?3
             WHERE id=?1",
            params![req_id, reviewer_user_id, now_str],
        )?;

        drop(conn);
        self.get_join_request_by_id(req_id)
    }

    /// 某项目待审批数量（用于 badge 提醒）
    pub fn pending_join_request_count(&self, project_id: &str) -> i64 {
        self.conn()
            .and_then(|c| {
                c.query_row(
                    "SELECT COUNT(*) FROM project_join_requests WHERE project_id=?1 AND status='pending'",
                    params![project_id],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(Into::into)
            })
            .unwrap_or(0)
    }
}
