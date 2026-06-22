//! 平台默认联合项目的历史兼容工具。
//!
//! v56 曾经把这些平台级项目自动加入所有用户。当前产品规则改为：
//! 左侧项目快捷栏只展示用户创建或明确参与的项目，因此新运行路径不再调用
//! 自动入会逻辑；这里只保留旧迁移函数和清理旧成员关系的反向迁移。

use anyhow::Result;
use rusqlite::{params, Connection};

use super::now;

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

pub(crate) fn remove_legacy_default_joint_project_memberships_conn(
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
        DELETE FROM project_members
         WHERE role = 'member'
           AND project_id IN (SELECT id FROM matched_projects)
           AND NOT EXISTS (
             SELECT 1
               FROM projects owner_project
              WHERE owner_project.id = project_members.project_id
                AND owner_project.created_by = project_members.user_id
           )
           AND NOT EXISTS (
             SELECT 1
               FROM project_join_requests jr
              WHERE jr.project_id = project_members.project_id
                AND jr.user_id = project_members.user_id
                AND jr.status = 'approved'
           )
        "#
    );
    conn.execute(&sql, []).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use rusqlite::params;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_default_joint_projects_{}.db",
            uuid::Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn new_users_do_not_join_existing_default_joint_projects() {
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
        let conn = store.conn().expect("conn should lock");
        let membership_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project.id, member.id],
                |row| row.get(0),
            )
            .expect("membership count should load");
        assert_eq!(membership_count, 0);
    }

    #[test]
    fn registering_default_project_does_not_backfill_existing_users() {
        let store = temp_store();
        let owner = store
            .create_user("owner2@example.com", "secret1", None, None)
            .expect("owner should create");
        let existing_user = store
            .create_user("existing@example.com", "secret1", None, None)
            .expect("existing user should create");

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

        let conn = store.conn().expect("conn should lock");
        let membership_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project.id, existing_user.id],
                |row| row.get(0),
            )
            .expect("membership count should load");
        assert_eq!(membership_count, 0);
    }

    #[test]
    fn cleanup_removes_legacy_default_members_but_preserves_real_roles() {
        let store = temp_store();
        let owner = store
            .create_user("owner3@example.com", "secret1", None, None)
            .expect("owner should create");
        let legacy_member = store
            .create_user("legacy-member@example.com", "secret1", None, None)
            .expect("legacy member should create");
        let admin = store
            .create_user("admin@example.com", "secret1", None, None)
            .expect("admin should create");
        let approved_member = store
            .create_user("approved@example.com", "secret1", None, None)
            .expect("approved member should create");

        let default_project = store
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
        let regular_project = store
            .create_project(&owner.id, "普通项目", Some("regular"), None)
            .expect("regular project should create")
            .project;

        {
            let conn = store.conn().expect("conn should lock");
            conn.execute(
                "INSERT INTO project_members (project_id, user_id, role, created_at)
                 VALUES (?1, ?2, 'member', 'now')",
                params![default_project.id, legacy_member.id],
            )
            .expect("legacy member should insert");
            conn.execute(
                "INSERT INTO project_members (project_id, user_id, role, created_at)
                 VALUES (?1, ?2, 'admin', 'now')",
                params![default_project.id, admin.id],
            )
            .expect("admin member should insert");
            conn.execute(
                "INSERT INTO project_members (project_id, user_id, role, created_at)
                 VALUES (?1, ?2, 'member', 'now')",
                params![default_project.id, approved_member.id],
            )
            .expect("approved member should insert");
            conn.execute(
                "INSERT INTO project_join_requests (
                    id, project_id, user_id, message, status, reviewed_by,
                    reviewed_at, created_at, updated_at
                 )
                 VALUES ('req_approved', ?1, ?2, 'ok', 'approved', ?3, 'now', 'now', 'now')",
                params![default_project.id, approved_member.id, owner.id],
            )
            .expect("approved request should insert");
            conn.execute(
                "INSERT INTO project_members (project_id, user_id, role, created_at)
                 VALUES (?1, ?2, 'member', 'now')",
                params![regular_project.id, legacy_member.id],
            )
            .expect("regular member should insert");
        }

        {
            let conn = store.conn().expect("conn should lock");
            let removed = remove_legacy_default_joint_project_memberships_conn(&conn)
                .expect("cleanup should run");
            assert_eq!(removed, 1);
        }

        let conn = store.conn().expect("conn should lock");
        let legacy_default_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![default_project.id, legacy_member.id],
                |row| row.get(0),
            )
            .expect("legacy default count should load");
        let admin_role: String = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![default_project.id, admin.id],
                |row| row.get(0),
            )
            .expect("admin role should load");
        let approved_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![default_project.id, approved_member.id],
                |row| row.get(0),
            )
            .expect("approved count should load");
        let regular_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![regular_project.id, legacy_member.id],
                |row| row.get(0),
            )
            .expect("regular count should load");

        assert_eq!(legacy_default_count, 0);
        assert_eq!(admin_role, "admin");
        assert_eq!(approved_count, 1);
        assert_eq!(regular_count, 1);
    }
}
