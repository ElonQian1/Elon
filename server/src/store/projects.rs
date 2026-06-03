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
use rusqlite::{params, OptionalExtension};

use super::{now, ProjectDeletionTarget, ProjectMemberEntry, PublicProjectItem, Store};

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
              (SELECT t.apk_url FROM tasks t
               WHERE t.project_id = p.id AND t.apk_url IS NOT NULL AND t.apk_url != ''
               ORDER BY t.created_at DESC LIMIT 1) AS latest_apk_url,
              p.created_at,
              p.updated_at,
              p.created_by AS owner_id
            FROM projects p
            LEFT JOIN users u ON u.id = p.created_by
             WHERE p.is_public = 1
              AND p.join_mode != 'invite'
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
                    latest_apk_url: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                    owner_id: row.get(12).unwrap_or_default(),
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
               (SELECT t.apk_url FROM tasks t
                WHERE t.project_id = p.id AND t.apk_url IS NOT NULL AND t.apk_url != ''
                ORDER BY t.created_at DESC LIMIT 1),
               p.created_at,
               p.updated_at,
               p.created_by AS owner_id
             FROM projects p
             LEFT JOIN users u ON u.id = p.created_by
             WHERE p.id = ?1 AND p.is_public = 1 AND p.join_mode != 'invite' AND p.status != 'deleted'",
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
                    latest_apk_url: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                    owner_id: row.get(12).unwrap_or_default(),
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

    /// 加入项目（open/invite 作为 member；readonly 作为 observer；返回 Err 表示无法加入）
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
        if join_mode == "approval" {
            anyhow::bail!("该项目需要审批才能加入，请联系项目 owner");
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

    /// 修改成员角色（仅 viewer/editor/member/observer 之间互转；不可改 owner，不可改自己）
    pub fn update_member_role(
        &self,
        project_id: &str,
        target_user_id: &str,
        new_role: &str,
    ) -> Result<()> {
        if !["editor", "member", "observer", "viewer"].contains(&new_role) {
            anyhow::bail!("role 必须为 editor / member / observer / viewer");
        }
        // viewer 是 APK 端展示别名，落库统一为 observer（保持与 join_mode=readonly 一致）
        let role_db = if new_role == "viewer" {
            "observer"
        } else {
            new_role
        };
        let conn = self.conn()?;
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
            params![project_id, target_user_id, role_db],
        )?;
        Ok(())
    }

    /// 移除成员（owner 不可被移除，需要由 handler 层确保调用者是 owner）
    pub fn remove_member(&self, project_id: &str, target_user_id: &str) -> Result<()> {
        let conn = self.conn()?;
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
            "DELETE FROM project_members WHERE project_id = ?1 AND user_id = ?2",
            params![project_id, target_user_id],
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
             ORDER BY CASE pm.role WHEN 'owner' THEN 0 WHEN 'editor' THEN 1 WHEN 'member' THEN 2 WHEN 'observer' THEN 3 ELSE 4 END, pm.created_at",
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
               (SELECT t.apk_url FROM tasks t
                WHERE t.project_id = p.id AND t.apk_url IS NOT NULL AND t.apk_url != ''
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
                    latest_apk_url: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                    owner_id: row.get(12).unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
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
                "SELECT p.id, p.name, p.workspace_key, p.source_type, p.workspace_path, pm.role
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
                        },
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))?;

        if role != "owner" {
            anyhow::bail!("只有项目 owner 才能删除项目");
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

        let running_tasks: i64 = tx.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = 'running'",
            params![project_id],
            |row| row.get(0),
        )?;
        if running_tasks > 0 {
            anyhow::bail!("项目还有正在运行的开发任务，请等待结束后再删除");
        }

        tx.execute(
            "DELETE FROM task_events WHERE task_id IN (SELECT id FROM tasks WHERE project_id = ?1)",
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
            "DELETE FROM project_channels WHERE project_id = ?1",
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
            "DELETE FROM project_members WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute("DELETE FROM projects WHERE id = ?1", params![project_id])?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_project_delete_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn create_project_reuses_existing_owner_project_name() {
        let store = temp_store();
        let user = store
            .create_user("reuse-project-name@example.com", "secret1", None, None)
            .expect("user should be created");

        let first = store
            .create_project(&user.id, "Reusable Project", None, None)
            .expect("project should be created");
        let second = store
            .create_project(&user.id, "Reusable Project", None, None)
            .expect("existing project should be reused");

        assert!(!first.reused_existing);
        assert!(second.reused_existing);
        assert_eq!(second.project.id, first.project.id);
        assert_eq!(
            store
                .list_projects_for_user(&user.id)
                .expect("projects should list")
                .len(),
            1
        );
    }

    #[test]
    fn invite_projects_are_hidden_from_store_but_joinable_by_card() {
        let store = temp_store();
        let owner = store
            .create_user("invite-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let invited = store
            .create_user("invite-member@example.com", "secret1", None, None)
            .expect("invited user should be created");
        let project = store
            .create_project(&owner.id, "Invite Only", None, None)
            .expect("project should be created")
            .project;

        store
            .set_project_visibility(&project.id, true, "invite")
            .expect("project should become invite-only");

        assert!(store
            .list_public_projects(None, 10, 0)
            .expect("store projects should list")
            .is_empty());
        assert!(store.get_public_project(&project.id).is_err());

        store
            .join_project(&invited.id, &project.id)
            .expect("invited card recipient should join");
        let access = store
            .get_project_access(&invited.id, &project.id)
            .expect("joined user should have project access");
        assert_eq!(access.role, "member");
    }

    #[test]
    fn readonly_projects_are_public_but_join_as_observer() {
        let store = temp_store();
        let owner = store
            .create_user("readonly-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let viewer = store
            .create_user("readonly-viewer@example.com", "secret1", None, None)
            .expect("viewer should be created");
        let project = store
            .create_project(&owner.id, "Readonly Project", None, None)
            .expect("project should be created")
            .project;

        store
            .set_project_visibility(&project.id, true, "readonly")
            .expect("project should become readonly public");

        let public_projects = store
            .list_public_projects(None, 10, 0)
            .expect("store projects should list");
        assert_eq!(public_projects.len(), 1);
        assert_eq!(public_projects[0].join_mode, "readonly");

        store
            .join_project(&viewer.id, &project.id)
            .expect("readonly viewer should join");
        let access = store
            .get_project_access(&viewer.id, &project.id)
            .expect("viewer should have project access");
        assert_eq!(access.role, "observer");
    }

    #[test]
    fn public_projects_include_latest_apk_url() {
        let store = temp_store();
        let owner = store
            .create_user("apk-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let project = store
            .create_project(&owner.id, "APK Project", None, None)
            .expect("project should be created")
            .project;

        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should become public");
        let task = store
            .create_task(&project.id, &owner.id, Some("conv"), "build apk")
            .expect("task should be created");
        store
            .finish_task(
                &task,
                "done",
                Some("done"),
                Some("https://example.test/latest.apk"),
                None,
            )
            .expect("task should finish with apk url");

        let public_projects = store
            .list_public_projects(None, 10, 0)
            .expect("store projects should list");
        assert_eq!(
            public_projects[0].latest_apk_url.as_deref(),
            Some("https://example.test/latest.apk")
        );
    }

    #[test]
    fn deletion_target_rejects_running_tasks() {
        let store = temp_store();
        let user = store
            .create_user("delete-running@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Running Delete", None, None)
            .expect("project should be created")
            .project;
        store
            .create_task(&project.id, &user.id, Some("conv"), "run")
            .expect("task should be created");

        let err = store
            .project_deletion_target(&user.id, &project.id)
            .expect_err("running task should block deletion")
            .to_string();

        assert!(err.contains("正在运行"));
    }

    #[test]
    fn purge_project_records_removes_project_children() {
        let store = temp_store();
        let user = store
            .create_user("delete-purge@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Purge Delete", None, None)
            .expect("project should be created")
            .project;
        let task = store
            .create_task(&project.id, &user.id, Some("conv"), "run")
            .expect("task should be created");
        store
            .record_task_event(&task, r#"{"type":"progress","message":"step"}"#)
            .expect("event should be recorded");
        store
            .finish_task(&task, "done", Some("done"), None, None)
            .expect("task should finish");

        let target = store
            .project_deletion_target(&user.id, &project.id)
            .expect("target should be available");
        assert_eq!(target.id, project.id);

        store
            .purge_project_records(&user.id, &project.id)
            .expect("project records should purge");

        assert!(store.get_project_access(&user.id, &project.id).is_err());
        assert!(store
            .list_task_events(&task, 10)
            .expect("task events query should work")
            .is_empty());
    }
}
