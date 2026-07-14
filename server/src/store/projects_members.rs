// server/src/store/projects_members.rs
//! 项目成员管理，从 projects.rs 提取。

use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::project_roles::{
    normalize_project_member_role_for_project, normalize_project_member_roles_for_project,
    project_member_effective_role_locked, project_member_role_refs_locked,
    sync_project_member_roles_locked,
};
use super::{
    clean_optional, is_system_project_source_type, normalize_account, now, project_branding,
    ProjectDeletionTarget, ProjectMemberEntry, PublicProjectItem, Store,
};
use crate::store::projects::member_ops::{ensure_project_not_system, project_member_entry};

impl Store {
    /// 管理员邀请/添加已注册用户为项目成员；若已是非 owner 成员则更新角色。
    pub fn add_project_member_by_account(
        &self,
        project_id: &str,
        account: &str,
        role: &str,
    ) -> Result<ProjectMemberEntry> {
        let account = normalize_account(account)?;
        let conn = self.conn()?;
        let role_db = normalize_project_member_role_for_project(&conn, project_id, role)?;
        ensure_project_not_system(&conn, project_id, "系统归档项目不能添加成员")?;
        let now_str = now();
        let target_user_id: String = conn
            .query_row(
                "SELECT id
                 FROM users
                 WHERE status = 'active'
                   AND (phone = ?1 OR email = ?1 OR id = ?1 OR nickname = ?1)
                 ORDER BY CASE WHEN phone = ?1 OR email = ?1 OR id = ?1 THEN 0 ELSE 1 END
                 LIMIT 1",
                params![account],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("目标账号不存在或未激活"))?;

        let current_role: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, target_user_id],
                |row| row.get(0),
            )
            .optional()?;
        if current_role.as_deref() == Some("owner") {
            anyhow::bail!("不能修改 owner 的角色");
        }
        let banned_count: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM project_member_restrictions
             WHERE project_id = ?1
               AND user_id = ?2
               AND banned_at IS NOT NULL
               AND (banned_until IS NULL OR banned_until > ?3)",
            params![project_id, target_user_id, &now_str],
            |row| row.get(0),
        )?;
        if banned_count > 0 {
            anyhow::bail!("目标用户已被封禁，请先解除封禁");
        }

        conn.execute(
            "INSERT INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id, user_id) DO UPDATE SET role = excluded.role",
            params![project_id, target_user_id, role_db, &now_str],
        )?;
        sync_project_member_roles_locked(
            &conn,
            project_id,
            &target_user_id,
            std::slice::from_ref(&role_db),
            None,
        )?;
        project_member_entry(&conn, project_id, &target_user_id)
    }

    pub fn find_active_user_id_by_account(&self, account: &str) -> Result<String> {
        let account = normalize_account(account)?;
        let conn = self.conn()?;
        conn.query_row(
            "SELECT id
             FROM users
             WHERE status = 'active'
               AND (phone = ?1 OR email = ?1 OR id = ?1 OR nickname = ?1)
             ORDER BY CASE WHEN phone = ?1 OR email = ?1 OR id = ?1 THEN 0 ELSE 1 END
             LIMIT 1",
            params![account],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("目标账号不存在或未激活"))
    }

    pub fn project_member_role(
        &self,
        project_id: &str,
        target_user_id: &str,
    ) -> Result<Option<String>> {
        let conn = self.conn()?;
        project_member_effective_role_locked(&conn, project_id, target_user_id)
    }

    /// 修改成员角色（仅 admin/editor/member/observer 之间互转；不可改 owner，不可改自己）
    pub fn update_member_role(
        &self,
        project_id: &str,
        target_user_id: &str,
        new_role: &str,
    ) -> Result<()> {
        let roles = vec![new_role.to_string()];
        self.set_project_member_roles(project_id, target_user_id, &roles, None)
            .map(|_| ())
    }

    /// 修改成员持有的所有角色；`project_members.role` 保留最高角色以兼容旧客户端。
    pub fn set_project_member_roles(
        &self,
        project_id: &str,
        target_user_id: &str,
        roles: &[String],
        assigned_by: Option<&str>,
    ) -> Result<ProjectMemberEntry> {
        let conn = self.conn()?;
        ensure_project_not_system(&conn, project_id, "系统归档项目不能修改成员角色")?;
        let role_dbs = normalize_project_member_roles_for_project(&conn, project_id, roles)?;
        let primary_role = role_dbs
            .first()
            .cloned()
            .unwrap_or_else(|| "member".to_string());
        let current_role: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, target_user_id],
                |row| row.get(0),
            )
            .optional()?;
        match current_role.as_deref() {
            None => anyhow::bail!("目标用户不是该项目成员"),
            Some("owner") => anyhow::bail!("不能修改 owner 的角色"),
            _ => {}
        }
        conn.execute(
            "UPDATE project_members SET role = ?3 WHERE project_id = ?1 AND user_id = ?2",
            params![project_id, target_user_id, primary_role],
        )?;
        sync_project_member_roles_locked(
            &conn,
            project_id,
            target_user_id,
            &role_dbs,
            assigned_by,
        )?;
        project_member_entry(&conn, project_id, target_user_id)
    }

    /// 移除成员（owner 不可被移除，需要由 handler 层确保调用者可管理成员）
    pub fn remove_member(&self, project_id: &str, target_user_id: &str) -> Result<()> {
        let conn = self.conn()?;
        ensure_project_not_system(&conn, project_id, "系统归档项目不能移除成员")?;
        let current_role: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, target_user_id],
                |row| row.get(0),
            )
            .optional()?;
        match current_role.as_deref() {
            None => anyhow::bail!("目标用户不是该项目成员"),
            Some("owner") => anyhow::bail!("不能移除项目 owner"),
            _ => {}
        }
        conn.execute(
            "DELETE FROM project_member_roles WHERE project_id = ?1 AND user_id = ?2",
            params![project_id, target_user_id],
        )?;
        conn.execute(
            "DELETE FROM project_members WHERE project_id = ?1 AND user_id = ?2",
            params![project_id, target_user_id],
        )?;
        Ok(())
    }

    /// 列出项目所有成员（公开项目任何人可查；私有项目在 handler 层校验权限）
    pub fn list_project_members(&self, project_id: &str) -> Result<Vec<ProjectMemberEntry>> {
        let conn = self.conn()?;
        let now = now();
        let mut stmt = conn.prepare(
            "SELECT pm.user_id,
                    COALESCE(NULLIF(trim(pm.display_name), ''), u.nickname, u.phone, u.email, pm.user_id) AS account,
                    COALESCE(u.nickname, u.phone, u.email, pm.user_id) AS global_account,
                    pm.display_name,
                    pm.admin_note,
                    u.avatar_data_url,
                    pm.role,
                    pm.created_at,
                    r.muted_until,
                    r.banned_at,
                    r.banned_until,
                    CASE WHEN r.muted_until IS NOT NULL AND r.muted_until > ?2 THEN 1 ELSE 0 END AS is_muted,
                    CASE WHEN r.banned_at IS NOT NULL AND (r.banned_until IS NULL OR r.banned_until > ?2) THEN 1 ELSE 0 END AS is_banned,
                    COALESCE(ps.status, 'online') AS presence_status,
                    ps.custom_status,
                    ps.activity
             FROM project_members pm
             LEFT JOIN users u ON u.id = pm.user_id
             LEFT JOIN project_member_restrictions r
               ON r.project_id = pm.project_id AND r.user_id = pm.user_id
             LEFT JOIN user_presence_settings ps ON ps.user_id = pm.user_id
             LEFT JOIN project_roles pr
               ON pr.project_id = pm.project_id AND pr.id = pm.role
             WHERE pm.project_id = ?1
             ORDER BY
               CASE pm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'editor' THEN 2 WHEN 'member' THEN 3 WHEN 'observer' THEN 4 ELSE 5 END,
               COALESCE(pr.position, 0) DESC,
               pm.created_at",
        )?;
        let mut rows = stmt
            .query_map(params![project_id, now], |row| {
                Ok(ProjectMemberEntry {
                    user_id: row.get(0)?,
                    account: row.get(1)?,
                    global_account: row.get(2)?,
                    member_display_name: row.get(3)?,
                    admin_note: row.get(4)?,
                    avatar_data_url: row.get(5)?,
                    role: row.get(6)?,
                    roles: Vec::new(),
                    joined_at: row.get(7)?,
                    is_online: false,
                    presence_status: row.get(13)?,
                    custom_status: row.get(14)?,
                    activity: row.get(15)?,
                    muted_until: row.get(8)?,
                    banned_at: row.get(9)?,
                    banned_until: row.get(10)?,
                    is_muted: row.get::<_, i64>(11)? != 0,
                    is_banned: row.get::<_, i64>(12)? != 0,
                    channel_permissions: None,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for member in &mut rows {
            let roles = project_member_role_refs_locked(&conn, project_id, &member.user_id)?;
            if let Some(effective) = roles.first() {
                member.role = effective.id.clone();
            }
            member.roles = roles;
        }
        rows.sort_by(|left, right| {
            let left_level = left.roles.first().map(|role| role.position).unwrap_or(0);
            let right_level = right.roles.first().map(|role| role.position).unwrap_or(0);
            right_level
                .cmp(&left_level)
                .then_with(|| left.joined_at.cmp(&right.joined_at))
                .then_with(|| left.account.cmp(&right.account))
        });
        Ok(rows)
    }
}
