//! 项目成员管理审计日志。

use anyhow::Result;
use rusqlite::params;

use super::{
    common::{new_id, now},
    store_types_project::ProjectMemberAuditEntry,
    Store,
};

impl Store {
    pub fn record_project_member_audit(
        &self,
        project_id: &str,
        actor_user_id: Option<&str>,
        target_user_id: Option<&str>,
        action: &str,
        old_role: Option<&str>,
        new_role: Option<&str>,
        note: Option<&str>,
    ) -> Result<ProjectMemberAuditEntry> {
        let conn = self.conn()?;
        let id = new_id("pma");
        let created_at = now();
        conn.execute(
            "INSERT INTO project_member_audit
             (id, project_id, actor_user_id, target_user_id, action, old_role, new_role, note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                project_id,
                actor_user_id,
                target_user_id,
                action.trim(),
                old_role.map(str::trim).filter(|value| !value.is_empty()),
                new_role.map(str::trim).filter(|value| !value.is_empty()),
                note.map(str::trim).filter(|value| !value.is_empty()),
                created_at
            ],
        )?;
        drop(conn);
        self.get_project_member_audit_entry(&id)
    }

    pub fn list_project_member_audit(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<ProjectMemberAuditEntry>> {
        let conn = self.conn()?;
        let limit = limit.clamp(1, 100);
        let mut stmt = conn.prepare(
            "SELECT
                a.id,
                a.project_id,
                a.actor_user_id,
                COALESCE(actor.nickname, actor.phone, actor.email, a.actor_user_id) AS actor_account,
                a.target_user_id,
                COALESCE(target.nickname, target.phone, target.email, a.target_user_id) AS target_account,
                a.action,
                a.old_role,
                a.new_role,
                a.note,
                a.created_at
             FROM project_member_audit a
             LEFT JOIN users actor ON actor.id = a.actor_user_id
             LEFT JOIN users target ON target.id = a.target_user_id
             WHERE a.project_id = ?1
             ORDER BY a.created_at DESC, a.id DESC
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![project_id, limit], project_member_audit_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    fn get_project_member_audit_entry(&self, audit_id: &str) -> Result<ProjectMemberAuditEntry> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT
                a.id,
                a.project_id,
                a.actor_user_id,
                COALESCE(actor.nickname, actor.phone, actor.email, a.actor_user_id) AS actor_account,
                a.target_user_id,
                COALESCE(target.nickname, target.phone, target.email, a.target_user_id) AS target_account,
                a.action,
                a.old_role,
                a.new_role,
                a.note,
                a.created_at
             FROM project_member_audit a
             LEFT JOIN users actor ON actor.id = a.actor_user_id
             LEFT JOIN users target ON target.id = a.target_user_id
             WHERE a.id = ?1",
            params![audit_id],
            project_member_audit_from_row,
        )
        .map_err(Into::into)
    }
}

fn project_member_audit_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProjectMemberAuditEntry> {
    Ok(ProjectMemberAuditEntry {
        id: row.get(0)?,
        project_id: row.get(1)?,
        actor_user_id: row.get(2)?,
        actor_account: row.get(3)?,
        target_user_id: row.get(4)?,
        target_account: row.get(5)?,
        action: row.get(6)?,
        old_role: row.get(7)?,
        new_role: row.get(8)?,
        note: row.get(9)?,
        created_at: row.get(10)?,
    })
}
