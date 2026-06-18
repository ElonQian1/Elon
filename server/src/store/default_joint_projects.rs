//! 默认加入的联合开发项目。
//!
//! 这些是平台级代码项目。用户创建账号、登录或项目绑定到 PC 工作区后，
//! 都应自动获得 member 权限，出现在“联合项目”列表中。

use anyhow::Result;
use rusqlite::{params, Connection};

use super::{now, Store};

const DEFAULT_JOINT_PROJECT_VALUES_SQL: &str =
    "VALUES ('bb64a', '一龙网游加速器'), ('fb2', '多冠体育')";
const DEFAULT_JOINT_PROJECT_MATCH_SQL: &str = r#"
(
  LOWER(TRIM(p.name)) = d.identifier
  OR TRIM(p.name) = d.display_name
  OR TRIM(COALESCE(p.display_name, '')) = d.display_name
  OR LOWER(RTRIM(REPLACE(TRIM(COALESCE(p.workspace_path, '')), char(92), '/'), '/')) = d.identifier
  OR LOWER(RTRIM(REPLACE(TRIM(COALESCE(p.workspace_path, '')), char(92), '/'), '/')) LIKE '%/' || d.identifier
  OR LOWER(RTRIM(REPLACE(TRIM(COALESCE(p.storage_repo_path, '')), char(92), '/'), '/')) = d.identifier
  OR LOWER(RTRIM(REPLACE(TRIM(COALESCE(p.storage_repo_path, '')), char(92), '/'), '/')) LIKE '%/' || d.identifier
  OR LOWER(RTRIM(REPLACE(TRIM(COALESCE(p.storage_worktree_path, '')), char(92), '/'), '/')) = d.identifier
  OR LOWER(RTRIM(REPLACE(TRIM(COALESCE(p.storage_worktree_path, '')), char(92), '/'), '/')) LIKE '%/' || d.identifier
)
"#;

impl Store {
    pub fn ensure_default_joint_project_memberships_for_user(
        &self,
        user_id: &str,
    ) -> Result<usize> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Ok(0);
        }
        let conn = self.conn()?;
        ensure_default_joint_project_memberships_for_user_conn(&conn, user_id, &now())
    }

    pub fn ensure_default_joint_project_memberships_for_project(
        &self,
        project_id: &str,
    ) -> Result<usize> {
        let project_id = project_id.trim();
        if project_id.is_empty() {
            return Ok(0);
        }
        let conn = self.conn()?;
        ensure_default_joint_project_memberships_for_project_conn(&conn, project_id, &now())
    }
}

pub(crate) fn ensure_default_joint_project_memberships_for_all_users_conn(
    conn: &Connection,
) -> Result<usize> {
    let sql = format!(
        r#"
        WITH
          default_projects(identifier, display_name) AS ({DEFAULT_JOINT_PROJECT_VALUES_SQL}),
          matched_projects AS (
            SELECT DISTINCT p.id
              FROM projects p
              JOIN default_projects d
                ON {DEFAULT_JOINT_PROJECT_MATCH_SQL}
             WHERE p.status != 'deleted'
               AND p.source_type IN ('local_path', 'pc_managed')
          )
        INSERT OR IGNORE INTO project_members (project_id, user_id, role, created_at)
        SELECT mp.id, u.id, 'member', ?1
          FROM matched_projects mp
          JOIN users u ON u.status = 'active'
        "#
    );
    conn.execute(&sql, params![now()]).map_err(Into::into)
}

pub(crate) fn ensure_default_joint_project_memberships_for_user_conn(
    conn: &Connection,
    user_id: &str,
    created_at: &str,
) -> Result<usize> {
    let sql = format!(
        r#"
        WITH
          default_projects(identifier, display_name) AS ({DEFAULT_JOINT_PROJECT_VALUES_SQL}),
          matched_projects AS (
            SELECT DISTINCT p.id
              FROM projects p
              JOIN default_projects d
                ON {DEFAULT_JOINT_PROJECT_MATCH_SQL}
             WHERE p.status != 'deleted'
               AND p.source_type IN ('local_path', 'pc_managed')
          )
        INSERT OR IGNORE INTO project_members (project_id, user_id, role, created_at)
        SELECT mp.id, u.id, 'member', ?2
          FROM matched_projects mp
          JOIN users u ON u.id = ?1 AND u.status = 'active'
        "#
    );
    conn.execute(&sql, params![user_id, created_at])
        .map_err(Into::into)
}

pub(crate) fn ensure_default_joint_project_memberships_for_project_conn(
    conn: &Connection,
    project_id: &str,
    created_at: &str,
) -> Result<usize> {
    let sql = format!(
        r#"
        WITH
          default_projects(identifier, display_name) AS ({DEFAULT_JOINT_PROJECT_VALUES_SQL}),
          matched_projects AS (
            SELECT DISTINCT p.id
              FROM projects p
              JOIN default_projects d
                ON {DEFAULT_JOINT_PROJECT_MATCH_SQL}
             WHERE p.id = ?1
               AND p.status != 'deleted'
               AND p.source_type IN ('local_path', 'pc_managed')
          )
        INSERT OR IGNORE INTO project_members (project_id, user_id, role, created_at)
        SELECT mp.id, u.id, 'member', ?2
          FROM matched_projects mp
          JOIN users u ON u.status = 'active'
        "#
    );
    conn.execute(&sql, params![project_id, created_at])
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_default_joint_projects_{}.db",
            uuid::Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn new_users_join_existing_default_joint_projects() {
        let store = temp_store();
        let owner = store
            .create_user("owner@example.com", "secret1", None, None)
            .expect("owner should create");
        let project = store
            .register_external_project(
                &owner.id,
                None,
                "bb64a",
                Some("bb64a 项目"),
                r"D:\rust\active-projects\bb64a",
                Some("node-a"),
                None,
                None,
            )
            .expect("default project should register")
            .project;

        let member = store
            .create_user("member@example.com", "secret1", None, None)
            .expect("member should create");
        let joined = store
            .list_joined_projects(&member.id)
            .expect("joined projects should load");

        let default_project = joined
            .iter()
            .find(|item| item.id == project.id)
            .expect("member should default join bb64a");
        assert_eq!(default_project.viewer_role.as_deref(), Some("member"));
        assert_eq!(
            default_project.display_name.as_deref(),
            Some("一龙网游加速器")
        );
    }

    #[test]
    fn project_backfill_joins_existing_users_but_ignores_template_namesakes() {
        let store = temp_store();
        let owner = store
            .create_user("owner2@example.com", "secret1", None, None)
            .expect("owner should create");
        let existing_user = store
            .create_user("existing@example.com", "secret1", None, None)
            .expect("existing user should create");

        let template = store
            .create_project(&owner.id, "fb2", Some("个人模板项目"), None)
            .expect("template namesake should create")
            .project;
        store
            .ensure_default_joint_project_memberships_for_project(&template.id)
            .expect("namesake check should run");

        let conn = store.conn().expect("conn should lock");
        let template_members: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![template.id, existing_user.id],
                |row| row.get(0),
            )
            .expect("template membership should count");
        drop(conn);
        assert_eq!(template_members, 0);

        let project = store
            .register_external_project(
                &owner.id,
                None,
                "fb2",
                Some("多冠体育赛事应用"),
                r"D:\rust\active-projects\fb2",
                Some("node-a"),
                None,
                None,
            )
            .expect("fb2 project should register")
            .project;

        let joined = store
            .list_joined_projects(&existing_user.id)
            .expect("joined projects should load");
        assert!(joined.iter().any(|item| {
            item.id == project.id && item.display_name.as_deref() == Some("多冠体育")
        }));
    }

    #[test]
    fn existing_owner_role_is_preserved() {
        let store = temp_store();
        let owner = store
            .create_user("owner3@example.com", "secret1", None, None)
            .expect("owner should create");
        let project = store
            .register_external_project(
                &owner.id,
                None,
                "bb64a",
                Some("bb64a 项目"),
                r"D:\rust\active-projects\bb64a",
                Some("node-a"),
                None,
                None,
            )
            .expect("project should register")
            .project;

        store
            .ensure_default_joint_project_memberships_for_user(&owner.id)
            .expect("owner ensure should run");
        let access = store
            .get_project_access(&owner.id, &project.id)
            .expect("owner access should load");

        assert_eq!(access.role, "owner");
    }
}
