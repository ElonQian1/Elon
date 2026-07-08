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

use super::super::project_roles::{
    normalize_project_member_role_for_project, normalize_project_member_roles_for_project,
    project_member_effective_role_locked, project_member_role_refs_locked,
    sync_project_member_roles_locked,
};
use super::super::{
    clean_optional, is_system_project_source_type, normalize_account, now, project_branding,
    ProjectDeletionTarget, ProjectMemberEntry, PublicProjectItem, Store,
};

impl Store {
    // ─── 商店浏览 ────────────────────────────────────────────────────────────

    /// 列出所有公开项目，支持全文搜索（按名称/描述）和分页

    pub fn update_project_member_profile(
        &self,
        project_id: &str,
        target_user_id: &str,
        display_name: Option<Option<&str>>,
        admin_note: Option<Option<&str>>,
    ) -> Result<ProjectMemberEntry> {
        let conn = self.conn()?;
        let (current_display_name, current_admin_note): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT display_name, admin_note
                   FROM project_members
                  WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, target_user_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("不是该项目成员"))?;
        let next_display_name = display_name
            .map(|value| {
                value
                    .and_then(|value| clean_optional(Some(value)))
                    .map(ToOwned::to_owned)
            })
            .unwrap_or(current_display_name);
        let next_admin_note = admin_note
            .map(|value| {
                value
                    .and_then(|value| clean_optional(Some(value)))
                    .map(ToOwned::to_owned)
            })
            .unwrap_or(current_admin_note);
        conn.execute(
            "UPDATE project_members
                SET display_name = ?3,
                    admin_note = ?4
              WHERE project_id = ?1 AND user_id = ?2",
            params![
                project_id,
                target_user_id,
                next_display_name,
                next_admin_note
            ],
        )?;
        project_member_entry(&conn, project_id, target_user_id)
    }

    /// 列出用户已加入或拥有的公开项目；项目广场用它判断“加入”还是“进入空间”。
    pub fn list_joined_projects(&self, user_id: &str) -> Result<Vec<PublicProjectItem>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT
               p.id, p.name, p.description, p.template,
               COALESCE(u.nickname, u.phone, u.email, p.created_by) AS owner_account,
               (SELECT COUNT(*) FROM project_members pm2 WHERE pm2.project_id = p.id),
               p.is_public, p.join_mode,
               (SELECT t.status FROM tasks t WHERE t.project_id = p.id
                ORDER BY t.created_at DESC LIMIT 1),
               (SELECT t.apk_url FROM tasks t
                WHERE t.project_id = p.id AND t.apk_url IS NOT NULL AND t.apk_url != ''
                ORDER BY t.created_at DESC LIMIT 1),
               p.icon_data_url,
               p.created_at, p.updated_at,
               p.created_by AS owner_id,
               p.source_type,
               p.workspace_path,
               p.display_name,
               pm.role
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id
             LEFT JOIN users u ON u.id = p.created_by
             WHERE pm.user_id = ?1
               AND p.status != 'deleted'
             ORDER BY p.updated_at DESC",
        )?;
        let mut rows = stmt
            .query_map(params![user_id], |row| {
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
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        for project in &mut rows {
            project.viewer_role =
                project_member_effective_role_locked(&conn, &project.id, user_id)?;
        }
        Ok(rows)
    }

    /// 返回一个用户拥有的项目删除目标。只允许 owner 删除；运行中的任务会阻止删除。
    pub fn project_deletion_target(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<ProjectDeletionTarget> {
        if project_id == "elon-self" {
            anyhow::bail!("一龙项目是平台自身项目，不能从手机端删除");
        }

        let conn = self.conn()?;
        let (target, role) = conn
            .query_row(
                "SELECT p.id, p.name, p.workspace_key, p.source_type, p.workspace_path, p.node_id, pm.role
                 FROM projects p
                 JOIN project_members pm ON pm.project_id = p.id
                 WHERE p.id = ?1 AND pm.user_id = ?2 AND p.status != 'deleted'",
                params![project_id, user_id],
                |row| {
                    Ok((
                        ProjectDeletionTarget {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            workspace_key: row.get(2)?,
                            source_type: row.get(3)?,
                            workspace_path: row.get(4)?,
                            node_id: row.get(5)?,
                        },
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))?;

        if role != "owner" {
            anyhow::bail!("只有项目 owner 才能删除项目");
        }
        if is_system_project_source_type(&target.source_type) {
            anyhow::bail!("系统归档项目不能删除");
        }

        let running_tasks: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = 'running'",
            params![project_id],
            |row| row.get(0),
        )?;
        if running_tasks > 0 {
            anyhow::bail!("项目还有正在运行的开发任务，请等待结束后再删除");
        }

        Ok(target)
    }

    /// 彻底删除项目在数据库中的产品记录。调用前应已经完成文件清理。
    pub fn purge_project_records(&self, user_id: &str, project_id: &str) -> Result<()> {
        if project_id == "elon-self" {
            anyhow::bail!("一龙项目是平台自身项目，不能从手机端删除");
        }

        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let role: Option<String> = tx
            .query_row(
                "SELECT pm.role
                 FROM projects p
                 JOIN project_members pm ON pm.project_id = p.id
                 WHERE p.id = ?1 AND pm.user_id = ?2 AND p.status != 'deleted'",
                params![project_id, user_id],
                |row| row.get(0),
            )
            .optional()?;
        match role.as_deref() {
            Some("owner") => {}
            Some(_) => anyhow::bail!("只有项目 owner 才能删除项目"),
            None => anyhow::bail!("项目不存在，或当前用户无权访问"),
        }

        let source_type: String = tx.query_row(
            "SELECT source_type FROM projects WHERE id = ?1 AND status != 'deleted'",
            params![project_id],
            |row| row.get(0),
        )?;
        if is_system_project_source_type(&source_type) {
            anyhow::bail!("系统归档项目不能删除");
        }

        let running_tasks: i64 = tx.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = 'running'",
            params![project_id],
            |row| row.get(0),
        )?;
        if running_tasks > 0 {
            anyhow::bail!("项目还有正在运行的开发任务，请等待结束后再删除");
        }

        tx.execute(
            "DELETE FROM project_ai_reviews
              WHERE matter_id IN (SELECT id FROM project_ai_matters WHERE project_id = ?1)
                 OR target_assignment_id IN (
                    SELECT a.id
                      FROM project_ai_matter_assignments a
                      JOIN project_ai_matters m ON m.id = a.matter_id
                     WHERE m.project_id = ?1
                 )",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_ai_matter_assignments
              WHERE matter_id IN (SELECT id FROM project_ai_matters WHERE project_id = ?1)",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_ai_events
              WHERE project_id = ?1
                 OR matter_id IN (SELECT id FROM project_ai_matters WHERE project_id = ?1)",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_ai_matters WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_ai_bots WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_ai_node_authorizations WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_runtime_permission_audit WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_runtime_permissions WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_landing_upload_tokens WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_dev_profiles WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_identities WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_execution_sessions WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_member_conversation_discussion_messages WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_join_requests WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM task_events WHERE task_id IN (SELECT id FROM tasks WHERE project_id = ?1)",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_workspace_health_snapshots WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_channel_read_states WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_channel_messages WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_channel_role_permissions WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_channel_member_permissions WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_channel_category_role_permissions WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_channel_category_member_permissions WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_channels WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_channel_categories WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM agent_native_sessions WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM messages WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM artifacts WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_events WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM conversations WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM tasks WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_member_restrictions WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_member_audit WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_invite_links WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_member_roles WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_members WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM project_roles WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute("DELETE FROM projects WHERE id = ?1", params![project_id])?;
        tx.commit()?;
        Ok(())
    }
}

pub(crate) fn project_member_entry(
    conn: &Connection,
    project_id: &str,
    user_id: &str,
) -> Result<ProjectMemberEntry> {
    let now = now();
    let mut entry = conn.query_row(
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
                CASE WHEN r.muted_until IS NOT NULL AND r.muted_until > ?3 THEN 1 ELSE 0 END AS is_muted,
                CASE WHEN r.banned_at IS NOT NULL AND (r.banned_until IS NULL OR r.banned_until > ?3) THEN 1 ELSE 0 END AS is_banned,
                COALESCE(ps.status, 'online') AS presence_status,
                ps.custom_status,
                ps.activity
           FROM project_members pm
           LEFT JOIN users u ON u.id = pm.user_id
           LEFT JOIN project_member_restrictions r
             ON r.project_id = pm.project_id AND r.user_id = pm.user_id
           LEFT JOIN user_presence_settings ps ON ps.user_id = pm.user_id
           WHERE pm.project_id = ?1 AND pm.user_id = ?2",
        params![project_id, user_id, now],
        |row| {
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
        },
    )?;
    let roles = project_member_role_refs_locked(conn, project_id, user_id)?;
    if let Some(effective) = roles.first() {
        entry.role = effective.id.clone();
    }
    entry.roles = roles;
    Ok(entry)
}

pub(crate) fn ensure_project_not_system(
    conn: &rusqlite::Connection,
    project_id: &str,
    message: &str,
) -> Result<()> {
    let source_type: String = conn
        .query_row(
            "SELECT source_type FROM projects WHERE id = ?1 AND status != 'deleted'",
            params![project_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("项目不存在"))?;
    if is_system_project_source_type(&source_type) {
        anyhow::bail!(message.to_string());
    }
    Ok(())
}

