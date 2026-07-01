use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{new_id, now, project_branding, project_identities, ProjectSummary, Store};

impl Store {
    pub fn count_active_pc_projects_for_node(&self, node_id: &str) -> Result<i64> {
        let node_id = node_id.trim();
        if node_id.is_empty() {
            return Ok(0);
        }
        self.conn()?
            .query_row(
                "SELECT COUNT(*)
                 FROM projects
                 WHERE node_id = ?1
                   AND status != 'deleted'
                   AND source_type NOT IN ('agent_balloon', 'chat_memory')",
                params![node_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn bind_project_to_pc_workspace(
        &self,
        user_id: &str,
        project_id: &str,
        workspace_path: &str,
        node_id: &str,
        git_head: Option<&str>,
        repo_url: Option<&str>,
        branch: Option<&str>,
    ) -> Result<ProjectSummary> {
        let workspace_path = workspace_path.trim();
        if workspace_path.is_empty() {
            return Err(anyhow!("workspace_path 不能为空"));
        }
        let node_id = node_id.trim();
        if node_id.is_empty() {
            return Err(anyhow!("node_id 不能为空"));
        }
        let repo_url = repo_url.map(str::trim).filter(|value| !value.is_empty());
        let branch = branch.map(str::trim).filter(|value| !value.is_empty());

        let now = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let updated = tx.execute(
            "UPDATE projects
             SET source_type = 'pc_managed',
                 workspace_path = ?1,
                 node_id = ?2,
                 repo_url = COALESCE(?3, repo_url),
                 branch = COALESCE(?4, branch),
                 status = 'active',
                 updated_at = ?5
             WHERE id = ?6
               AND source_type NOT IN ('agent_balloon', 'chat_memory')
               AND EXISTS (
                 SELECT 1 FROM project_members pm
                 WHERE pm.project_id = projects.id
                   AND pm.user_id = ?7
                   AND pm.role = 'owner'
               )",
            params![
                workspace_path,
                node_id,
                repo_url,
                branch,
                now,
                project_id,
                user_id
            ],
        )?;
        if updated == 0 {
            return Err(anyhow!(
                "项目不存在、当前用户不是 owner，或系统归档项目不能绑定 PC 工作区"
            ));
        }
        tx.execute(
            "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
             VALUES (?1, ?2, ?3, 'pc_workspace_provisioned', ?4, ?5)",
            params![
                new_id("evt"),
                project_id,
                user_id,
                serde_json::json!({
                    "workspace_path": workspace_path,
                    "node_id": node_id,
                    "git_head": git_head,
                    "repo_url": repo_url,
                    "branch": branch,
                })
                .to_string(),
                now
            ],
        )?;
        project_identities::upsert_project_workspace_identity(
            &tx,
            project_id,
            user_id,
            node_id,
            workspace_path,
            &now,
        )?;
        tx.commit()?;
        drop(conn);

        let conn = self.conn()?;
        conn.query_row(
            "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                    p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id,
                    p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                    p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
                    pm.role,
                    (SELECT COUNT(*) FROM project_members pm2 WHERE pm2.project_id = p.id) AS member_count,
                    p.is_public,
                    p.join_mode,
                    (SELECT t.status FROM tasks t WHERE t.project_id = p.id
                     ORDER BY t.created_at DESC LIMIT 1) AS last_task_status,
                    (SELECT t.apk_url FROM tasks t
                     WHERE t.project_id = p.id AND t.apk_url IS NOT NULL AND t.apk_url != ''
                     ORDER BY t.created_at DESC LIMIT 1) AS last_apk_url,
                    p.icon_data_url,
                    p.updated_at,
                    COALESCE(
                        (SELECT prp.mode
                           FROM project_runtime_permissions prp
                          WHERE prp.project_id = p.id),
                        'project_write'
                    ) AS runtime_permission,
                    p.display_name
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id
             WHERE p.id = ?1 AND pm.user_id = ?2",
            params![project_id, user_id],
            |row| {
                let mut project = ProjectSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    display_name: row.get(25)?,
                    description: row.get(2)?,
                    workspace_key: row.get(3)?,
                    template: row.get(4)?,
                    source_type: row.get(5)?,
                    repo_url: row.get(6)?,
                    branch: row.get(7)?,
                    workspace_path: row.get(8)?,
                    node_id: row.get(9)?,
                    storage_node_id: row.get(10)?,
                    storage_repo_path: row.get(11)?,
                    storage_repo_url: row.get(12)?,
                    storage_worktree_path: row.get(13)?,
                    storage_status: row.get(14)?,
                    status: row.get(15)?,
                    role: row.get(16)?,
                    member_count: row.get(17)?,
                    is_public: row.get::<_, i64>(18)? != 0,
                    join_mode: row.get(19)?,
                    runtime_permission: row.get(24)?,
                    last_task_status: row.get(20)?,
                    last_apk_url: row.get(21)?,
                    icon_data_url: row.get(22)?,
                    updated_at: row.get(23)?,
                };
                project_branding::apply_project_summary_branding(&mut project);
                Ok(project)
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("项目绑定成功但重新读取失败"))
    }
}
