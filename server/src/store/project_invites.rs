use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use super::project_roles::normalize_project_member_role_for_project;
use super::{
    is_system_project_source_type, new_id, now, ProjectInviteLink, ProjectInvitePreview, Store,
};

impl Store {
    pub fn list_project_invite_links(&self, project_id: &str) -> Result<Vec<ProjectInviteLink>> {
        let conn = self.conn()?;
        ensure_project_can_use_invites_locked(&conn, project_id)?;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, code, role, max_uses, use_count, expires_at,
                    temporary, revoked_at, created_by, created_at, updated_at
               FROM project_invite_links
              WHERE project_id = ?1
              ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![project_id], invite_link_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn create_project_invite_link(
        &self,
        project_id: &str,
        created_by: &str,
        role: &str,
        expires_in_hours: Option<i64>,
        max_uses: Option<i64>,
        temporary: bool,
    ) -> Result<ProjectInviteLink> {
        let conn = self.conn()?;
        ensure_project_can_use_invites_locked(&conn, project_id)?;
        let role = normalize_project_member_role_for_project(&conn, project_id, role)?;
        if role == "owner" {
            return Err(anyhow!("邀请链接不能授予 owner"));
        }
        let max_uses = max_uses
            .filter(|value| *value > 0)
            .map(|value| value.min(10_000));
        let expires_at = expires_in_hours
            .filter(|value| *value > 0)
            .map(|hours| (Utc::now() + Duration::hours(hours.min(24 * 365))).to_rfc3339());
        let created_at = now();
        let id = new_id("pil");
        let code = unique_invite_code_locked(&conn)?;
        conn.execute(
            "INSERT INTO project_invite_links
             (id, project_id, code, role, max_uses, use_count, expires_at, temporary,
              revoked_at, created_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, NULL, ?8, ?9, ?9)",
            params![
                id,
                project_id,
                code,
                role,
                max_uses,
                expires_at,
                temporary as i64,
                created_by,
                created_at
            ],
        )?;
        drop(conn);
        self.get_project_invite_link_by_code(&code)
    }

    pub fn revoke_project_invite_link(
        &self,
        project_id: &str,
        code: &str,
    ) -> Result<ProjectInviteLink> {
        let conn = self.conn()?;
        ensure_project_can_use_invites_locked(&conn, project_id)?;
        let updated_at = now();
        let changed = conn.execute(
            "UPDATE project_invite_links
                SET revoked_at = COALESCE(revoked_at, ?1),
                    updated_at = ?1
              WHERE project_id = ?2 AND code = ?3",
            params![updated_at, project_id, code],
        )?;
        if changed == 0 {
            return Err(anyhow!("邀请链接不存在"));
        }
        drop(conn);
        self.get_project_invite_link_by_code(code)
    }

    pub fn get_project_invite_preview(&self, code: &str) -> Result<ProjectInvitePreview> {
        let conn = self.conn()?;
        let invite = invite_link_by_code_locked(&conn, code)?;
        validate_invite_link(&invite)?;
        conn.query_row(
            "SELECT p.id, p.name, p.display_name
               FROM projects p
              WHERE p.id = ?1 AND p.status != 'deleted'",
            params![invite.project_id],
            |row| {
                Ok(ProjectInvitePreview {
                    project_id: row.get(0)?,
                    project_name: row.get(1)?,
                    display_name: row.get(2)?,
                    role: invite.role.clone(),
                    max_uses: invite.max_uses,
                    use_count: invite.use_count,
                    expires_at: invite.expires_at.clone(),
                    temporary: invite.temporary,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("邀请项目不存在"))
    }

    pub fn join_project_by_invite_link(
        &self,
        user_id: &str,
        code: &str,
    ) -> Result<(bool, ProjectInvitePreview)> {
        let conn = self.conn()?;
        let invite = invite_link_by_code_locked(&conn, code)?;
        validate_invite_link(&invite)?;
        let preview = invite_preview_locked(&conn, &invite)?;
        let banned_count: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM project_member_restrictions
             WHERE project_id = ?1
               AND user_id = ?2
               AND banned_at IS NOT NULL
               AND (banned_until IS NULL OR banned_until > ?3)",
            params![invite.project_id, user_id, now()],
            |row| row.get(0),
        )?;
        if banned_count > 0 {
            return Err(anyhow!("你已被该项目封禁，无法加入"));
        }
        let existing: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![invite.project_id, user_id],
                |row| row.get(0),
            )
            .optional()?;
        if existing.is_some() {
            return Ok((true, preview));
        }
        conn.execute(
            "INSERT INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![invite.project_id, user_id, invite.role, now()],
        )?;
        conn.execute(
            "UPDATE project_invite_links
                SET use_count = use_count + 1,
                    updated_at = ?1
              WHERE code = ?2",
            params![now(), code],
        )?;
        Ok((false, preview))
    }

    pub fn get_project_invite_link_by_code(&self, code: &str) -> Result<ProjectInviteLink> {
        let conn = self.conn()?;
        invite_link_by_code_locked(&conn, code)
    }
}

fn ensure_project_can_use_invites_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> Result<()> {
    let source_type: String = conn
        .query_row(
            "SELECT source_type FROM projects WHERE id = ?1 AND status != 'deleted'",
            params![project_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("项目不存在"))?;
    if is_system_project_source_type(&source_type) {
        return Err(anyhow!("系统归档项目不能创建邀请链接"));
    }
    Ok(())
}

fn unique_invite_code_locked(conn: &rusqlite::Connection) -> Result<String> {
    for _ in 0..8 {
        let code = Uuid::new_v4().simple().to_string()[..12].to_string();
        let exists: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM project_invite_links WHERE code = ?1",
                params![code],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Ok(code);
        }
    }
    Err(anyhow!("生成邀请链接失败，请重试"))
}

fn invite_link_by_code_locked(
    conn: &rusqlite::Connection,
    code: &str,
) -> Result<ProjectInviteLink> {
    let code = code.trim();
    if code.is_empty() {
        return Err(anyhow!("邀请码不能为空"));
    }
    conn.query_row(
        "SELECT id, project_id, code, role, max_uses, use_count, expires_at,
                temporary, revoked_at, created_by, created_at, updated_at
           FROM project_invite_links
          WHERE code = ?1",
        params![code],
        invite_link_from_row,
    )
    .optional()?
    .ok_or_else(|| anyhow!("邀请链接不存在"))
}

fn validate_invite_link(invite: &ProjectInviteLink) -> Result<()> {
    if invite.revoked_at.is_some() {
        return Err(anyhow!("邀请链接已撤销"));
    }
    if let Some(expires_at) = invite.expires_at.as_deref() {
        if expires_at <= now().as_str() {
            return Err(anyhow!("邀请链接已过期"));
        }
    }
    if let Some(max_uses) = invite.max_uses {
        if invite.use_count >= max_uses {
            return Err(anyhow!("邀请链接使用次数已达上限"));
        }
    }
    Ok(())
}

fn invite_preview_locked(
    conn: &rusqlite::Connection,
    invite: &ProjectInviteLink,
) -> Result<ProjectInvitePreview> {
    conn.query_row(
        "SELECT p.id, p.name, p.display_name
           FROM projects p
          WHERE p.id = ?1 AND p.status != 'deleted'",
        params![invite.project_id],
        |row| {
            Ok(ProjectInvitePreview {
                project_id: row.get(0)?,
                project_name: row.get(1)?,
                display_name: row.get(2)?,
                role: invite.role.clone(),
                max_uses: invite.max_uses,
                use_count: invite.use_count,
                expires_at: invite.expires_at.clone(),
                temporary: invite.temporary,
            })
        },
    )
    .optional()?
    .ok_or_else(|| anyhow!("邀请项目不存在"))
}

fn invite_link_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectInviteLink> {
    Ok(ProjectInviteLink {
        id: row.get(0)?,
        project_id: row.get(1)?,
        code: row.get(2)?,
        role: row.get(3)?,
        max_uses: row.get(4)?,
        use_count: row.get(5)?,
        expires_at: row.get(6)?,
        temporary: row.get::<_, i64>(7)? != 0,
        revoked_at: row.get(8)?,
        created_by: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}


#[cfg(test)]
#[path = "project_invites_tests.rs"]
mod tests;
