use anyhow::Result;
use rusqlite::params;

use super::{
    default_project_runtime_permission, is_system_project_source_type, project_branding,
    store_types_project::{ProjectSummary, UserArchiveProject},
    system_project_key_for_source_type, Store,
};

impl Store {
    pub fn list_archive_projects_for_user(&self, user_id: &str) -> Result<Vec<UserArchiveProject>> {
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
                    (
                        SELECT COUNT(*) FROM conversations c
                        WHERE c.project_id = p.id
                    ) AS conversation_count,
                    COALESCE(u.nickname, u.phone, u.email, p.created_by) AS owner_account,
                    p.created_by AS owner_id,
                    COALESCE(u.role, 'user') AS creator_role,
                    COALESCE(
                        (SELECT prp.mode
                           FROM project_runtime_permissions prp
                          WHERE prp.project_id = p.id),
                        'project_write'
                    ) AS runtime_permission,
                    p.display_name
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id
             LEFT JOIN users u ON u.id = p.created_by
             WHERE pm.user_id = ?1 AND p.status != 'deleted'
             ORDER BY
                CASE
                    WHEN p.source_type IN ('agent_balloon', 'chat_memory') THEN 0
                    WHEN pm.role = 'owner' THEN 1
                    ELSE 2
                END,
                p.updated_at DESC",
        )?;

        let projects = stmt
            .query_map(params![user_id], |row| {
                archive_project_from_row(row, user_id)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(projects)
    }
}

fn archive_project_from_row(
    row: &rusqlite::Row<'_>,
    current_user_id: &str,
) -> rusqlite::Result<UserArchiveProject> {
    let mut project = ProjectSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        display_name: row.get(29)?,
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
        runtime_permission: row
            .get(28)
            .unwrap_or_else(|_| default_project_runtime_permission()),
        last_task_status: row.get(20)?,
        last_apk_url: row.get(21)?,
        icon_data_url: row.get(22)?,
        updated_at: row.get(23)?,
    };
    project_branding::apply_project_summary_branding(&mut project);
    let conversation_count = row.get(24)?;
    let system_key = system_project_key_for_source_type(&project.source_type).map(str::to_string);
    let owner_account = if system_key.is_some() {
        "系统".to_string()
    } else {
        row.get(25)?
    };
    let owner_id: String = row.get(26)?;
    let workspace_kind = workspace_kind_for_project(&project).to_string();
    let creator_role: String = row.get(27)?;
    let (project_origin_type, project_origin_label) = project_origin_for(
        system_key.as_deref(),
        &owner_id,
        &creator_role,
        current_user_id,
    );

    Ok(UserArchiveProject {
        project,
        owner_account,
        owner_id,
        project_origin_type: project_origin_type.to_string(),
        project_origin_label: project_origin_label.to_string(),
        conversation_count,
        workspace_kind,
        system_key,
        conversation_route: None,
        workspace_status: None,
    })
}

fn workspace_kind_for_project(project: &ProjectSummary) -> &'static str {
    if is_system_project_source_type(&project.source_type) {
        "system_archive"
    } else if project
        .node_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        "pc_node_workspace"
    } else if project
        .workspace_path
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        "external_workspace"
    } else {
        "server_workspace"
    }
}

fn project_origin_for(
    system_key: Option<&str>,
    owner_id: &str,
    creator_role: &str,
    current_user_id: &str,
) -> (&'static str, &'static str) {
    if system_key.is_some() {
        return ("system", "系统创建");
    }
    if owner_id == current_user_id {
        return ("self", "我创建");
    }
    if creator_role.trim().eq_ignore_ascii_case("admin") {
        return ("admin", "管理员创建");
    }
    ("member", "他人创建")
}

#[cfg(test)]
#[path = "user_archive_tests.rs"]
mod tests;
