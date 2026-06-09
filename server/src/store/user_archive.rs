use anyhow::Result;
use rusqlite::params;

use super::{
    is_system_project_source_type,
    store_types::{ProjectSummary, UserArchiveProject},
    system_project_key_for_source_type, Store,
};

impl Store {
    pub fn list_archive_projects_for_user(&self, user_id: &str) -> Result<Vec<UserArchiveProject>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                    p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id, p.status,
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
                    p.updated_at,
                    (
                        SELECT COUNT(*) FROM conversations c
                        WHERE c.project_id = p.id
                    ) AS conversation_count,
                    COALESCE(u.phone, u.email, p.created_by) AS owner_account,
                    p.created_by AS owner_id
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
            .query_map(params![user_id], archive_project_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(projects)
    }
}

fn archive_project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserArchiveProject> {
    let project = ProjectSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        workspace_key: row.get(3)?,
        template: row.get(4)?,
        source_type: row.get(5)?,
        repo_url: row.get(6)?,
        branch: row.get(7)?,
        workspace_path: row.get(8)?,
        node_id: row.get(9)?,
        status: row.get(10)?,
        role: row.get(11)?,
        member_count: row.get(12)?,
        is_public: row.get::<_, i64>(13)? != 0,
        join_mode: row.get(14)?,
        last_task_status: row.get(15)?,
        last_apk_url: row.get(16)?,
        updated_at: row.get(17)?,
    };
    let conversation_count = row.get(18)?;
    let owner_account = row.get(19)?;
    let owner_id = row.get(20)?;
    let system_key = system_project_key_for_source_type(&project.source_type).map(str::to_string);
    let workspace_kind = workspace_kind_for_project(&project).to_string();

    Ok(UserArchiveProject {
        project,
        owner_account,
        owner_id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{
        is_system_project_source_type, CHAT_MEMORY_PROJECT_NAME, PHONE_CONTROL_PROJECT_NAME,
    };
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path =
            std::env::temp_dir().join(format!("elon_user_archive_{}.db", Uuid::new_v4().simple()));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn archive_lists_system_and_regular_projects() {
        let store = temp_store();
        let user = store
            .create_user("archive@example.com", "secret1", None, None)
            .expect("user should be created");

        store
            .ensure_balloon_project_for_user(&user.id)
            .expect("phone project should exist");
        store
            .ensure_chat_memory_project_for_user(&user.id)
            .expect("chat project should exist");
        store
            .create_project(&user.id, "工作台", Some("PC 项目"), Some("android"))
            .expect("regular project should create");

        let archive = store
            .list_archive_projects_for_user(&user.id)
            .expect("archive should load");

        assert_eq!(archive.len(), 3);
        let names = archive
            .iter()
            .map(|item| item.project.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&PHONE_CONTROL_PROJECT_NAME));
        assert!(names.contains(&CHAT_MEMORY_PROJECT_NAME));
        assert!(names.contains(&"工作台"));

        let system_projects = archive
            .iter()
            .filter(|item| is_system_project_source_type(&item.project.source_type))
            .collect::<Vec<_>>();
        assert_eq!(system_projects.len(), 2);
        assert!(system_projects
            .iter()
            .all(|item| item.workspace_kind == "system_archive"));
        assert!(system_projects
            .iter()
            .all(|item| item.conversation_count == 0));
    }
}
