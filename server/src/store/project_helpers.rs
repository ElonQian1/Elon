use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::common::{clean_optional, new_id, now};
use super::{
    pc_project_binding, project_branding, project_identities, project_roles, CreateProjectResult,
    ProjectSummary,
};

// ── 私有帮助函数 ──────────────────────────────────────────────────────────────

pub(super) fn project_summary_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProjectSummary> {
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
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_external_project_binding(
    conn: &Connection,
    user_id: &str,
    project_id: &str,
    name_override: Option<&str>,
    description_override: Option<&str>,
    workspace_path: &str,
    node_id: Option<&str>,
    repo_url: Option<&str>,
    branch: Option<&str>,
    now: &str,
    event_type: &str,
) -> Result<ProjectSummary> {
    let tx = conn.unchecked_transaction()?;
    let changed = tx.execute(
        "UPDATE projects
         SET name = COALESCE(?2, name),
             description = COALESCE(?3, description),
             template = 'local',
             source_type = 'local_path',
             workspace_path = ?4,
             node_id = ?5,
             repo_url = COALESCE(?6, repo_url),
             branch = COALESCE(?7, branch),
             is_public = CASE WHEN ?1 = 'elon-self' THEN 1 ELSE is_public END,
             join_mode = CASE WHEN ?1 = 'elon-self' THEN 'approval' ELSE join_mode END,
             updated_at = ?8
         WHERE id = ?1 AND status != 'deleted'",
        params![
            project_id,
            name_override,
            description_override,
            workspace_path,
            node_id,
            repo_url,
            branch,
            now
        ],
    )?;
    if changed == 0 {
        anyhow::bail!("项目不存在，或当前用户无权访问");
    }

    tx.execute(
        "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            new_id("evt"),
            project_id,
            user_id,
            event_type,
            serde_json::json!({
                "workspace_path": workspace_path,
                "node_id": node_id,
                "repo_url": repo_url,
                "branch": branch,
            })
            .to_string(),
            now
        ],
    )?;

    let project = find_project_by_id_for_user(&tx, user_id, project_id)?
        .ok_or_else(|| anyhow!("项目绑定后无法读取"))?;
    project_identities::replace_project_identities(
        &tx,
        project_id,
        user_id,
        project.node_id.as_deref(),
        project.workspace_path.as_deref().unwrap_or(workspace_path),
        project.repo_url.as_deref(),
        project.branch.as_deref(),
        now,
    )?;
    if let Some(node_id) = project.node_id.as_deref() {
        pc_project_binding::upsert_project_pc_workspace_binding_tx(
            &tx,
            project_id,
            user_id,
            node_id,
            project.workspace_path.as_deref().unwrap_or(workspace_path),
            None,
            project.repo_url.as_deref(),
            project.branch.as_deref(),
            event_type,
            now,
        )?;
    }
    tx.commit()?;

    find_project_by_id_for_user(conn, user_id, project_id)?
        .ok_or_else(|| anyhow!("项目绑定后无法读取"))
}

pub(super) fn find_owner_project_by_name(
    conn: &Connection,
    user_id: &str,
    name: &str,
) -> Result<Option<ProjectSummary>> {
    let mut project = conn
        .query_row(
            "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                    p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id,
                    p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                    p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
                    COALESCE(pm.role, 'owner') AS role,
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
                        'project_write'
                    ) AS runtime_permission,
                    p.display_name
             FROM projects p
             LEFT JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = ?1
             WHERE p.created_by = ?1 AND p.name = ?2 AND p.status != 'deleted'
             ORDER BY p.updated_at DESC
             LIMIT 1",
            params![user_id, name],
            project_summary_from_row,
        )
        .optional()?;
    if let Some(project) = &mut project {
        apply_effective_project_summary_role(conn, user_id, project)?;
    }
    Ok(project)
}

pub(super) fn find_owner_project_by_workspace_path(
    conn: &Connection,
    user_id: &str,
    workspace_path: &str,
) -> Result<Option<ProjectSummary>> {
    let expected = normalize_workspace_path_for_match(workspace_path);
    if expected.is_empty() {
        return Ok(None);
    }
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id,
                p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
                COALESCE(pm.role, 'owner') AS role,
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
                    'project_write'
                ) AS runtime_permission,
                p.display_name
         FROM projects p
         LEFT JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = ?1
         WHERE p.created_by = ?1
           AND p.status != 'deleted'
           AND p.source_type IN ('local_path', 'pc_managed')
           AND p.workspace_path IS NOT NULL
           AND TRIM(p.workspace_path) != ''
         ORDER BY p.updated_at DESC",
    )?;
    let mut rows = stmt.query_map(params![user_id], project_summary_from_row)?;
    while let Some(project) = rows.next() {
        let mut project = project?;
        apply_effective_project_summary_role(conn, user_id, &mut project)?;
        if project
            .workspace_path
            .as_deref()
            .map(normalize_workspace_path_for_match)
            .as_deref()
            == Some(expected.as_str())
        {
            return Ok(Some(project));
        }
    }
    Ok(None)
}

pub(super) fn normalize_workspace_path_for_match(path: &str) -> String {
    project_identities::normalize_workspace_path(path)
}

pub(super) fn find_project_by_id_for_user(
    conn: &Connection,
    user_id: &str,
    project_id: &str,
) -> Result<Option<ProjectSummary>> {
    let mut project = conn
        .query_row(
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
                        'project_write'
                    ) AS runtime_permission,
                    p.display_name
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = ?2
             WHERE p.id = ?1 AND p.status != 'deleted'
             LIMIT 1",
            params![project_id, user_id],
            project_summary_from_row,
        )
        .optional()?;
    if let Some(project) = &mut project {
        apply_effective_project_summary_role(conn, user_id, project)?;
    }
    Ok(project)
}

pub(super) fn apply_effective_project_summary_role(
    conn: &Connection,
    user_id: &str,
    project: &mut ProjectSummary,
) -> Result<()> {
    if let Some(role) =
        project_roles::project_member_effective_role_locked(conn, &project.id, user_id)?
    {
        project.role = role;
    }
    Ok(())
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
