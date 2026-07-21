use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    is_system_project_source_type, new_id, now, project_branding, project_identities,
    ProjectAccess, ProjectPcWorkspaceBinding, ProjectSummary, Store,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn upsert_project_pc_workspace_binding_tx(
    conn: &Connection,
    project_id: &str,
    owner_user_id: &str,
    node_id: &str,
    workspace_path: &str,
    git_head: Option<&str>,
    repo_url: Option<&str>,
    branch: Option<&str>,
    source: &str,
    now: &str,
) -> Result<()> {
    let node_id = node_id.trim();
    let workspace_path = workspace_path.trim();
    if node_id.is_empty() || workspace_path.is_empty() {
        return Ok(());
    }
    let normalized_workspace_path = project_identities::normalize_workspace_path(workspace_path);
    if normalized_workspace_path.is_empty() {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO project_pc_workspace_bindings (
            id, project_id, owner_user_id, node_id, workspace_path,
            normalized_workspace_path, repo_url, branch, git_head, source, created_at, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
         ON CONFLICT(project_id, owner_user_id, node_id)
         DO UPDATE SET
            workspace_path = excluded.workspace_path,
            normalized_workspace_path = excluded.normalized_workspace_path,
            repo_url = COALESCE(excluded.repo_url, project_pc_workspace_bindings.repo_url),
            branch = COALESCE(excluded.branch, project_pc_workspace_bindings.branch),
            git_head = COALESCE(excluded.git_head, project_pc_workspace_bindings.git_head),
            source = excluded.source,
            updated_at = excluded.updated_at",
        params![
            new_id("ppwb"),
            project_id,
            owner_user_id,
            node_id,
            workspace_path,
            normalized_workspace_path,
            repo_url,
            branch,
            git_head,
            source,
            now,
        ],
    )?;
    Ok(())
}

impl Store {
    pub fn count_active_pc_projects_for_node(&self, node_id: &str) -> Result<i64> {
        let node_id = node_id.trim();
        if node_id.is_empty() {
            return Ok(0);
        }
        self.conn()?
            .query_row(
                "SELECT COUNT(DISTINCT project_id)
                   FROM (
                     SELECT id AS project_id
                       FROM projects
                      WHERE node_id = ?1
                        AND status != 'deleted'
                        AND source_type NOT IN ('agent_balloon', 'chat_memory')
                     UNION
                     SELECT b.project_id
                       FROM project_pc_workspace_bindings b
                       JOIN projects p ON p.id = b.project_id
                      WHERE b.node_id = ?1
                        AND p.status != 'deleted'
                        AND p.source_type NOT IN ('agent_balloon', 'chat_memory')
                   ) active_pc_projects",
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
        upsert_project_pc_workspace_binding_tx(
            &tx,
            project_id,
            user_id,
            node_id,
            workspace_path,
            git_head,
            repo_url,
            branch,
            "pc_workspace_binding",
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
                        'danger_full_access'
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

    /// 为可编辑成员记录其自己的 PC 工作区，不改写项目 owner 的全局主绑定。
    pub fn bind_project_member_to_pc_workspace(
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
        let node_id = node_id.trim();
        if workspace_path.is_empty() {
            return Err(anyhow!("workspace_path 不能为空"));
        }
        if node_id.is_empty() {
            return Err(anyhow!("node_id 不能为空"));
        }

        let now = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let (role, source_type): (String, String) = tx
            .query_row(
                "SELECT pm.role, p.source_type
                   FROM projects p
                   JOIN project_members pm ON pm.project_id = p.id
                  WHERE p.id = ?1 AND pm.user_id = ?2 AND p.status != 'deleted'",
                params![project_id, user_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在或当前用户不是项目成员"))?;
        if !matches!(role.as_str(), "owner" | "admin" | "editor") {
            return Err(anyhow!(
                "只有项目 owner、管理员或协作者可以绑定自己的 PC 工作区"
            ));
        }
        if is_system_project_source_type(&source_type) {
            return Err(anyhow!("系统归档项目不能绑定 PC 工作区"));
        }

        project_identities::upsert_project_workspace_identity(
            &tx,
            project_id,
            user_id,
            node_id,
            workspace_path,
            &now,
        )?;
        upsert_project_pc_workspace_binding_tx(
            &tx,
            project_id,
            user_id,
            node_id,
            workspace_path,
            git_head,
            repo_url,
            branch,
            "member_pc_workspace_binding",
            &now,
        )?;
        tx.execute(
            "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
             VALUES (?1, ?2, ?3, 'member_pc_workspace_bound', ?4, ?5)",
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
        tx.commit()?;
        drop(conn);

        self.list_projects_for_user(user_id)?
            .into_iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| anyhow!("成员工作区绑定成功但重新读取失败"))
    }

    pub fn get_project_pc_workspace_binding(
        &self,
        user_id: &str,
        project_id: &str,
        node_id: &str,
    ) -> Result<Option<ProjectPcWorkspaceBinding>> {
        let node_id = node_id.trim();
        if node_id.is_empty() {
            return Ok(None);
        }
        self.conn()?
            .query_row(
                "SELECT project_id, owner_user_id, node_id, workspace_path,
                        repo_url, branch, git_head, source, updated_at
                   FROM project_pc_workspace_bindings
                  WHERE project_id = ?1
                    AND owner_user_id = ?2
                    AND node_id = ?3
                  LIMIT 1",
                params![project_id, user_id, node_id],
                |row| {
                    Ok(ProjectPcWorkspaceBinding {
                        project_id: row.get(0)?,
                        owner_user_id: row.get(1)?,
                        node_id: row.get(2)?,
                        workspace_path: row.get(3)?,
                        repo_url: row.get(4)?,
                        branch: row.get(5)?,
                        git_head: row.get(6)?,
                        source: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// 返回用户项目，并把 PC 工作区字段解析为指定节点自己的绑定。
    /// 未绑定到该节点的项目会保留在列表中，但 PC 节点与路径字段为空。
    pub fn list_projects_for_user_on_node(
        &self,
        user_id: &str,
        node_id: &str,
    ) -> Result<Vec<ProjectSummary>> {
        let node_id = node_id.trim();
        if node_id.is_empty() {
            return self.list_projects_for_user(user_id);
        }

        let mut projects = self.list_projects_for_user(user_id)?;
        for project in &mut projects {
            project.workspace_path = None;
            project.node_id = None;
            if let Some(binding) =
                self.get_project_pc_workspace_binding(user_id, &project.id, node_id)?
            {
                project.workspace_path = Some(binding.workspace_path);
                project.node_id = Some(binding.node_id);
            }
        }
        Ok(projects)
    }
}

pub(super) fn apply_user_pc_workspace_binding_to_summary(
    conn: &Connection,
    user_id: &str,
    project: &mut ProjectSummary,
) -> Result<()> {
    if let Some(binding) = latest_user_pc_workspace_binding(conn, user_id, &project.id)? {
        project.workspace_path = Some(binding.workspace_path);
        project.node_id = Some(binding.node_id);
    }
    Ok(())
}

pub(super) fn apply_user_pc_workspace_binding_to_access(
    conn: &Connection,
    user_id: &str,
    access: &mut ProjectAccess,
) -> Result<()> {
    if let Some(binding) = latest_user_pc_workspace_binding(conn, user_id, &access.id)? {
        access.workspace_path = Some(binding.workspace_path);
        access.node_id = Some(binding.node_id);
    }
    Ok(())
}

fn latest_user_pc_workspace_binding(
    conn: &Connection,
    user_id: &str,
    project_id: &str,
) -> Result<Option<ProjectPcWorkspaceBinding>> {
    conn.query_row(
        "SELECT project_id, owner_user_id, node_id, workspace_path,
                repo_url, branch, git_head, source, updated_at
           FROM project_pc_workspace_bindings
          WHERE project_id = ?1 AND owner_user_id = ?2
          ORDER BY updated_at DESC
          LIMIT 1",
        params![project_id, user_id],
        |row| {
            Ok(ProjectPcWorkspaceBinding {
                project_id: row.get(0)?,
                owner_user_id: row.get(1)?,
                node_id: row.get(2)?,
                workspace_path: row.get(3)?,
                repo_url: row.get(4)?,
                branch: row.get(5)?,
                git_head: row.get(6)?,
                source: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

#[cfg(test)]
#[path = "pc_project_binding_tests.rs"]
mod pc_project_binding_tests;
