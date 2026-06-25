use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{
    default_project_runtime_permission, is_system_project_source_type, new_id,
    normalize_project_runtime_permission, now, ProjectRuntimePermission, Store,
    PROJECT_RUNTIME_PERMISSION_PROJECT_WRITE,
};

impl Store {
    pub fn get_project_runtime_permission(
        &self,
        project_id: &str,
    ) -> Result<ProjectRuntimePermission> {
        let project_id = project_id.trim();
        if project_id.is_empty() {
            anyhow::bail!("project_id 不能为空");
        }
        let conn = self.conn()?;
        ensure_project_exists(&conn, project_id)?;
        let record = conn
            .query_row(
                "SELECT mode, updated_by, updated_at
                   FROM project_runtime_permissions
                  WHERE project_id = ?1",
                params![project_id],
                |row| {
                    Ok(ProjectRuntimePermission {
                        project_id: project_id.to_string(),
                        mode: row.get(0)?,
                        updated_by: row.get(1)?,
                        updated_at: row.get(2)?,
                    })
                },
            )
            .optional()?;

        Ok(record.unwrap_or_else(|| ProjectRuntimePermission {
            project_id: project_id.to_string(),
            mode: default_project_runtime_permission(),
            updated_by: None,
            updated_at: None,
        }))
    }

    pub fn set_project_runtime_permission(
        &self,
        project_id: &str,
        user_id: &str,
        mode: &str,
    ) -> Result<ProjectRuntimePermission> {
        let project_id = project_id.trim();
        let user_id = user_id.trim();
        let mode = normalize_project_runtime_permission(mode).ok_or_else(|| {
            anyhow!("mode 必须为 project_write、full_access 或 danger_full_access")
        })?;
        if project_id.is_empty() || user_id.is_empty() {
            anyhow::bail!("project_id 和 user_id 不能为空");
        }

        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let source_type: String = tx
            .query_row(
                "SELECT source_type FROM projects WHERE id = ?1 AND status != 'deleted'",
                params![project_id],
                |row| row.get(0),
            )
            .map_err(|_| anyhow!("项目不存在"))?;
        if is_system_project_source_type(&source_type) {
            anyhow::bail!("系统归档项目不能开启项目运行权限");
        }
        let old_mode: String = tx
            .query_row(
                "SELECT mode FROM project_runtime_permissions WHERE project_id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or_else(|| PROJECT_RUNTIME_PERMISSION_PROJECT_WRITE.to_string());
        let updated_at = now();
        tx.execute(
            "INSERT INTO project_runtime_permissions (project_id, mode, updated_by, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id) DO UPDATE SET
               mode = excluded.mode,
               updated_by = excluded.updated_by,
               updated_at = excluded.updated_at,
               expires_at = NULL",
            params![project_id, mode, user_id, updated_at],
        )?;
        if old_mode != mode {
            tx.execute(
                "INSERT INTO project_runtime_permission_audit
                   (id, project_id, user_id, old_mode, new_mode, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    new_id("prpa"),
                    project_id,
                    user_id,
                    old_mode,
                    mode,
                    updated_at
                ],
            )?;
        }
        tx.commit()?;

        Ok(ProjectRuntimePermission {
            project_id: project_id.to_string(),
            mode: mode.to_string(),
            updated_by: Some(user_id.to_string()),
            updated_at: Some(updated_at),
        })
    }
}

fn ensure_project_exists(conn: &rusqlite::Connection, project_id: &str) -> Result<()> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM projects WHERE id = ?1 AND status != 'deleted'",
            params![project_id],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        anyhow::bail!("项目不存在");
    }
    Ok(())
}
