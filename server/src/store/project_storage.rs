use anyhow::{anyhow, Result};
use rusqlite::params;

use super::{clean_optional, new_id, now, ProjectSummary, Store};

impl Store {
    pub fn bind_project_storage_repo(
        &self,
        user_id: &str,
        project_id: &str,
        storage_node_id: &str,
        storage_repo_path: &str,
        storage_repo_url: Option<&str>,
        branch: Option<&str>,
    ) -> Result<ProjectSummary> {
        let storage_node_id = storage_node_id.trim();
        if storage_node_id.is_empty() {
            return Err(anyhow!("storage_node_id 不能为空"));
        }
        let storage_repo_path = storage_repo_path.trim();
        if storage_repo_path.is_empty() {
            return Err(anyhow!("storage_repo_path 不能为空"));
        }
        let storage_repo_url = clean_optional(storage_repo_url);
        let branch = clean_optional(branch);
        let now = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let updated = tx.execute(
            "UPDATE projects
             SET storage_node_id = ?1,
                 storage_repo_path = ?2,
                 storage_repo_url = ?3,
                 storage_status = 'ready',
                 repo_url = COALESCE(?3, repo_url),
                 branch = COALESCE(?4, branch),
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
                storage_node_id,
                storage_repo_path,
                storage_repo_url,
                branch,
                now,
                project_id,
                user_id
            ],
        )?;
        if updated == 0 {
            return Err(anyhow!(
                "项目不存在、当前用户不是 owner，或系统归档项目不能绑定硬盘仓库"
            ));
        }
        tx.execute(
            "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
             VALUES (?1, ?2, ?3, 'project_storage_repo_ready', ?4, ?5)",
            params![
                new_id("evt"),
                project_id,
                user_id,
                serde_json::json!({
                    "storage_node_id": storage_node_id,
                    "storage_repo_path": storage_repo_path,
                    "storage_repo_url": storage_repo_url,
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
            .ok_or_else(|| anyhow!("硬盘仓库绑定成功但重新读取项目失败"))
    }
}
