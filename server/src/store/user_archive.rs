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
                    COALESCE(u.role, 'user') AS creator_role
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
        last_task_status: row.get(20)?,
        last_apk_url: row.get(21)?,
        icon_data_url: row.get(22)?,
        updated_at: row.get(23)?,
    };
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
            .create_user("archive@example.com", "secret1", Some("归档用户"), None)
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
            .all(|item| item.owner_account == "系统"));
        assert!(system_projects
            .iter()
            .all(|item| item.project_origin_type == "system"));
        assert!(system_projects
            .iter()
            .all(|item| item.project_origin_label == "系统创建"));
        assert!(system_projects
            .iter()
            .all(|item| item.conversation_count == 0));

        let regular = archive
            .iter()
            .find(|item| item.project.name == "工作台")
            .expect("regular project should be present");
        assert_eq!(regular.owner_account, "归档用户");
        assert_eq!(regular.project_origin_type, "self");
        assert_eq!(regular.project_origin_label, "我创建");
    }

    #[test]
    fn archive_marks_admin_created_member_projects() {
        let store = temp_store();
        let admin = store
            .create_user(
                "admin-created@example.com",
                "secret1",
                Some("Admin"),
                Some("admin"),
            )
            .expect("admin should be created");
        let user = store
            .create_user(
                "member-created@example.com",
                "secret1",
                Some("Member"),
                None,
            )
            .expect("member should be created");
        let project = store
            .create_project(
                &admin.id,
                "管理员项目",
                Some("由管理员创建"),
                Some("android"),
            )
            .expect("admin project should create")
            .project;
        store
            .add_project_member_by_account(&project.id, "member-created@example.com", "member")
            .expect("member should be added");

        let archive = store
            .list_archive_projects_for_user(&user.id)
            .expect("archive should load");
        let item = archive
            .iter()
            .find(|item| item.project.id == project.id)
            .expect("admin-created project should be visible");

        assert_eq!(item.project_origin_type, "admin");
        assert_eq!(item.project_origin_label, "管理员创建");
    }
}
