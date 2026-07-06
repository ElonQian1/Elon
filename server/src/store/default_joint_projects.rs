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
#[path = "default_joint_projects_tests.rs"]
mod tests;
