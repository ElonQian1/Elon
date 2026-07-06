//! store/join_requests.rs — 项目加入申请 DB 操作
//!
//! 当项目 join_mode='approval' 时，用户提交申请，owner/admin 审批通过后自动添加成员。

use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{
    common::{new_id, now},
    store_types_project::JoinRequestRecord,
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
            anyhow::bail!("你已被该项目封禁，无法申请加入");
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

    /// 列出项目所有待审批（或全部）申请（owner/admin 使用）
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
    /// - reviewer_user_id 必须是该项目 owner/admin
    pub fn approve_join_request(
        &self,
        req_id: &str,
        project_id: &str,
        reviewer_user_id: &str,
    ) -> Result<JoinRequestRecord> {
        let conn = self.conn()?;
        let now_str = now();

        // 校验申请存在且 pending
        let (req_project_id, applicant_user_id, status): (String, String, String) = conn
            .query_row(
                "SELECT project_id, user_id, status FROM project_join_requests WHERE id = ?1",
                params![req_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|_| anyhow!("申请记录不存在"))?;

        if req_project_id != project_id {
            anyhow::bail!("申请记录不属于当前项目");
        }

        // 仅 owner/admin 可审批
        let reviewer_role: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, reviewer_user_id],
                |row| row.get(0),
            )
            .optional()?;
        if !matches!(reviewer_role.as_deref(), Some("owner" | "admin")) {
            anyhow::bail!("仅项目 owner 或管理员可审批加入申请");
        }

        if status != "pending" {
            anyhow::bail!("申请已处理（当前状态：{status}）");
        }
        let banned_count: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM project_member_restrictions
             WHERE project_id = ?1
               AND user_id = ?2
               AND banned_at IS NOT NULL
               AND (banned_until IS NULL OR banned_until > ?3)",
            params![project_id, applicant_user_id, &now_str],
            |row| row.get(0),
        )?;
        if banned_count > 0 {
            anyhow::bail!("申请人已被该项目封禁，请先解除封禁");
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
            params![req_project_id, applicant_user_id, now_str],
        )?;

        drop(conn);
        self.get_join_request_by_id(req_id)
    }

    /// 拒绝申请
    ///
    /// 安全要求：
    /// - req_id 必须属于 project_id（防止跨项目审批）
    /// - reviewer_user_id 必须是该项目 owner/admin
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

        // 仅 owner/admin 可审批
        let reviewer_role: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, reviewer_user_id],
                |row| row.get(0),
            )
            .optional()?;
        if !matches!(reviewer_role.as_deref(), Some("owner" | "admin")) {
            anyhow::bail!("仅项目 owner 或管理员可审批加入申请");
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

    /// 列出某 owner/admin 管理项目的待审批数（含 0 也返回，便于前端展示项目列表）
    ///
    /// 返回 `Vec<(project_id, project_name, pending_count)>`
    pub fn list_owned_projects_with_pending_counts(
        &self,
        manager_user_id: &str,
    ) -> Result<Vec<(String, String, i64)>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name,
                    (SELECT COUNT(*) FROM project_join_requests jr
                     WHERE jr.project_id = p.id AND jr.status = 'pending') AS pending
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id
             WHERE pm.user_id = ?1 AND pm.role IN ('owner', 'admin')
                   AND p.status != 'deleted'
             ORDER BY pending DESC, p.created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![manager_user_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// 撤销自己的申请（仅 pending 状态可撤销）
    pub fn cancel_my_join_request(&self, req_id: &str, user_id: &str) -> Result<()> {
        let conn = self.conn()?;
        let (req_user_id, status): (String, String) = conn
            .query_row(
                "SELECT user_id, status FROM project_join_requests WHERE id = ?1",
                params![req_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| anyhow!("申请记录不存在"))?;
        if req_user_id != user_id {
            anyhow::bail!("仅可撤销自己提交的申请");
        }
        if status != "pending" {
            anyhow::bail!("申请已处理（当前状态：{status}），无法撤销");
        }
        conn.execute(
            "DELETE FROM project_join_requests WHERE id = ?1",
            params![req_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path =
            std::env::temp_dir().join(format!("elon_join_request_{}.db", Uuid::new_v4().simple()));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn admin_can_approve_join_request() {
        let store = temp_store();
        let owner = store
            .create_user("join-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let admin = store
            .create_user("join-admin@example.com", "secret1", None, None)
            .expect("admin should be created");
        let applicant = store
            .create_user("join-applicant@example.com", "secret1", None, None)
            .expect("applicant should be created");
        let project = store
            .create_project(&owner.id, "Admin Approval", None, None)
            .expect("project should be created")
            .project;
        store
            .set_project_visibility(&project.id, true, "approval")
            .expect("project should require approval");
        store
            .add_project_member_by_account(&project.id, &admin.id, "admin")
            .expect("admin should be added");

        let request = store
            .create_join_request(&applicant.id, &project.id, Some("please add me"))
            .expect("request should be created");
        let approved = store
            .approve_join_request(&request.id, &project.id, &admin.id)
            .expect("admin should approve request");

        assert_eq!(approved.status, "approved");
        let access = store
            .get_project_access(&applicant.id, &project.id)
            .expect("applicant should join after approval");
        assert_eq!(access.role, "member");
    }
}
