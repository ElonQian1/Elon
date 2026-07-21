use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::common::{clean_optional, new_id, now, safe_external_id};
use super::project_helpers::*;
use super::store_types::*;
use super::store_types_project::*;
use super::{pc_project_binding, project_branding, project_identities, project_roles};

impl super::Store {
    pub fn ensure_project_for_user(
        &self,
        user_id: &str,
        project_id: &str,
        name: &str,
        description: Option<&str>,
        source_type: &str,
        template: &str,
        workspace_path: Option<&str>,
    ) -> Result<ProjectAccess> {
        let user = self.ensure_device_user(user_id)?;
        let id = safe_external_id(project_id, "project");
        let name = name.trim();
        let name = if name.is_empty() {
            "移动端项目"
        } else {
            name
        };
        let source_type = match source_type.trim() {
            "local_path" => "local_path",
            "github" => "github",
            _ => "template",
        };
        let template = match template.trim() {
            "local" => "local",
            "github" => "github",
            _ => "android",
        };
        let now = now();

        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO projects (
                id, name, description, workspace_key, template, source_type, workspace_path,
                status, created_by, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?1, ?4, ?5, ?6, 'active', ?7, ?8, ?8)",
            params![
                id,
                name,
                clean_optional(description),
                template,
                source_type,
                clean_optional(workspace_path),
                user.id,
                now
            ],
        )?;
        tx.execute(
            "UPDATE projects
             SET source_type = CASE
                     WHEN ?2 != 'template' OR source_type = 'template' THEN ?2
                     ELSE source_type
                 END,
                 template = CASE
                     WHEN ?2 != 'template' OR source_type = 'template' THEN ?3
                     ELSE template
                 END,
                 workspace_path = CASE
                     WHEN node_id IS NOT NULL AND node_id != '' THEN workspace_path
                     ELSE COALESCE(?4, workspace_path)
                 END
             WHERE id = ?1",
            params![id, source_type, template, clean_optional(workspace_path)],
        )?;
        if id == "elon-self" {
            tx.execute(
                "UPDATE projects
                 SET is_public = 1,
                     join_mode = 'approval',
                     updated_at = ?2
                 WHERE id = ?1 AND status != 'deleted'",
                params![id, now],
            )?;
        }
        tx.execute(
            "INSERT OR IGNORE INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, 'owner', ?3)",
            params![id, user.id, now],
        )?;
        tx.commit()?;
        drop(conn);

        // 如果项目被标记为 deleted（例如被迁移脚本误删），尝试透明重定向到同名 active 项目，
        // 或将其恢复为 active，避免 APK 收到"项目不存在"错误。
        {
            let conn2 = self.conn()?;
            let is_deleted: bool = conn2
                .query_row(
                    "SELECT status = 'deleted' FROM projects WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if is_deleted {
                let active_id: Option<String> = conn2
                    .query_row(
                        "SELECT id FROM projects \
                         WHERE created_by = ?1 AND name = ?2 AND status != 'deleted' LIMIT 1",
                        params![user.id, name],
                        |row| row.get(0),
                    )
                    .optional()?;
                drop(conn2);
                if let Some(redirect_id) = active_id {
                    // 同名 active 项目已存在，直接返回它（透明重定向）
                    return self.get_project_access(&user.id, &redirect_id);
                } else {
                    // 无同名 active 项目，将本项目恢复为 active
                    self.conn()?.execute(
                        "UPDATE projects SET status = 'active' WHERE id = ?1",
                        params![id],
                    )?;
                }
            }
        }

        self.get_project_access(&user.id, &id)
    }

    pub fn list_projects_for_user(&self, user_id: &str) -> Result<Vec<ProjectSummary>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                    p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id,
                    p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                    p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
                    pm.role,
                    (SELECT COUNT(*) FROM project_members pm2 WHERE pm2.project_id = p.id) AS member_count,
                    p.is_public,
                    p.join_mode,
                    (
                        SELECT t.status FROM tasks t
                        WHERE t.project_id = p.id
                        ORDER BY t.created_at DESC
                        LIMIT 1
                    ) AS last_task_status,
                    (
                        SELECT t.apk_url FROM tasks t
                        WHERE t.project_id = p.id AND t.apk_url IS NOT NULL
                        ORDER BY t.created_at DESC
                        LIMIT 1
                    ) AS last_apk_url,
                    p.icon_data_url,
                    p.updated_at,
                    COALESCE(
                        (SELECT prp.mode
                           FROM project_runtime_permissions prp
                          WHERE prp.project_id = p.id),
                        'full_access'
                    ) AS runtime_permission,
                    p.display_name
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id
             WHERE pm.user_id = ?1 AND p.status != 'deleted'
             ORDER BY p.updated_at DESC",
        )?;

        let mut projects = stmt
            .query_map(params![user_id], project_summary_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        for project in &mut projects {
            apply_effective_project_summary_role(&conn, user_id, project)?;
            pc_project_binding::apply_user_pc_workspace_binding_to_summary(
                &conn, user_id, project,
            )?;
        }

        Ok(projects)
    }

    pub fn get_project_access(&self, user_id: &str, project_id: &str) -> Result<ProjectAccess> {
        let conn = self.conn()?;
        let mut access = conn
            .query_row(
                "SELECT p.id, p.name, p.workspace_key, p.template, p.source_type, p.repo_url, p.branch,
                        p.workspace_path, p.node_id,
                        p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                        p.storage_worktree_path, COALESCE(p.storage_status, 'none'), pm.role, p.status,
                        COALESCE(
                            (SELECT prp.mode
                               FROM project_runtime_permissions prp
                              WHERE prp.project_id = p.id),
                            'full_access'
                        ) AS runtime_permission
                 FROM projects p
                 JOIN project_members pm ON pm.project_id = p.id
                 WHERE p.id = ?1 AND pm.user_id = ?2 AND p.status != 'deleted'",
                params![project_id, user_id],
                |row| {
                    Ok(ProjectAccess {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        workspace_key: row.get(2)?,
                        template: row.get(3)?,
                        source_type: row.get(4)?,
                        repo_url: row.get(5)?,
                        branch: row.get(6)?,
                        workspace_path: row.get(7)?,
                        node_id: row.get(8)?,
                        storage_node_id: row.get(9)?,
                        storage_repo_path: row.get(10)?,
                        storage_repo_url: row.get(11)?,
                        storage_worktree_path: row.get(12)?,
                        storage_status: row.get(13)?,
                        role: row.get(14)?,
                        status: row.get(15)?,
                        runtime_permission: row.get(16)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))?;
        if let Some(effective_role) =
            project_roles::project_member_effective_role_locked(&conn, project_id, user_id)?
        {
            access.role = effective_role;
        }
        pc_project_binding::apply_user_pc_workspace_binding_to_access(&conn, user_id, &mut access)?;
        drop(conn);
        if self.project_member_is_banned(project_id, user_id)? {
            anyhow::bail!("你已被该项目封禁，无法访问项目空间");
        }
        Ok(access)
    }

    pub fn get_project_space_access(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<ProjectAccess> {
        if self.project_member_is_banned(project_id, user_id)? {
            anyhow::bail!("你已被该项目封禁，无法访问项目空间");
        }
        if let Ok(access) = self.get_project_access(user_id, project_id) {
            return Ok(access);
        }
        self.conn()?
            .query_row(
                "SELECT p.id, p.name, p.workspace_key, p.template, p.source_type, p.repo_url, p.branch,
                        p.workspace_path, p.node_id,
                        p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                        p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
                        COALESCE(
                            (SELECT prp.mode
                               FROM project_runtime_permissions prp
                              WHERE prp.project_id = p.id),
                            'full_access'
                        ) AS runtime_permission
                 FROM projects p
                 WHERE p.id = ?1
                   AND p.status != 'deleted'
                   AND p.is_public = 1
                   AND p.join_mode != 'invite'",
                params![project_id],
                |row| {
                    Ok(ProjectAccess {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        workspace_key: row.get(2)?,
                        template: row.get(3)?,
                        source_type: row.get(4)?,
                        repo_url: row.get(5)?,
                        branch: row.get(6)?,
                        workspace_path: row.get(7)?,
                        node_id: row.get(8)?,
                        storage_node_id: row.get(9)?,
                        storage_repo_path: row.get(10)?,
                        storage_repo_url: row.get(11)?,
                        storage_worktree_path: row.get(12)?,
                        storage_status: row.get(13)?,
                        status: row.get(14)?,
                        runtime_permission: row.get(15)?,
                        role: "visitor".to_string(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))
    }

    pub fn update_project_git_metadata(
        &self,
        user_id: &str,
        project_id: &str,
        repo_url: Option<&str>,
        branch: Option<&str>,
    ) -> Result<ProjectSummary> {
        let repo_url = clean_optional(repo_url);
        let branch = clean_optional(branch);
        if repo_url.is_none() && branch.is_none() {
            let conn = self.conn()?;
            return find_project_by_id_for_user(&conn, user_id, project_id)?
                .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"));
        }

        let now = now();
        let conn = self.conn()?;
        let changed = conn.execute(
            "UPDATE projects
             SET repo_url = COALESCE(?1, repo_url),
                 branch = COALESCE(?2, branch),
                 updated_at = ?3
             WHERE id = ?4
               AND source_type NOT IN ('agent_balloon', 'chat_memory')
               AND EXISTS (
                 SELECT 1 FROM project_members
                 WHERE project_id = ?4
                   AND user_id = ?5
                   AND role IN ('owner', 'editor')
               )",
            params![repo_url, branch, now, project_id, user_id],
        )?;
        if changed == 0 {
            return Err(anyhow!("项目不存在，或当前用户无权配置 Git"));
        }
        find_project_by_id_for_user(&conn, user_id, project_id)?
            .ok_or_else(|| anyhow!("Git 配置保存后无法读取项目"))
    }

    pub fn update_project_git_config(
        &self,
        user_id: &str,
        project_id: &str,
        repo_url: &str,
        branch: &str,
    ) -> Result<ProjectAccess> {
        let repo_url = repo_url.trim();
        if repo_url.is_empty() {
            return Err(anyhow!("Git 仓库地址不能为空"));
        }
        let branch = branch.trim();
        if branch.is_empty() {
            return Err(anyhow!("Git 分支不能为空"));
        }

        let now = now();
        let conn = self.conn()?;
        let changed = conn.execute(
            "UPDATE projects
             SET source_type = 'github',
                 template = 'github',
                 repo_url = ?1,
                 branch = ?2,
                 updated_at = ?3
             WHERE id = ?4
               AND EXISTS (
                 SELECT 1 FROM project_members
                 WHERE project_id = ?4
                   AND user_id = ?5
                   AND role IN ('owner', 'editor')
               )",
            params![repo_url, branch, now, project_id, user_id],
        )?;
        if changed == 0 {
            return Err(anyhow!("项目不存在，或当前用户无权配置 Git"));
        }
        drop(conn);

        self.get_project_access(user_id, project_id)
    }
}
