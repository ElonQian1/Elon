//! 项目成员限制状态：禁言与封禁。

use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension};

use super::{
    common::now, is_system_project_source_type, store_types_project::ProjectMemberModerationEntry,
    Store,
};

const DEFAULT_MUTE_MINUTES: i64 = 60;
const MAX_MUTE_MINUTES: i64 = 60 * 24 * 30;

impl Store {
    pub fn update_project_member_moderation(
        &self,
        project_id: &str,
        target_user_id: &str,
        actor_user_id: &str,
        action: &str,
        duration_minutes: Option<i64>,
        note: Option<&str>,
    ) -> Result<ProjectMemberModerationEntry> {
        let action = normalize_moderation_action(action)?;
        let conn = self.conn()?;
        ensure_project_not_system_for_moderation(&conn, project_id)?;
        let target_role: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2",
                params![project_id, target_user_id],
                |row| row.get(0),
            )
            .optional()?;
        match target_role.as_deref() {
            None => anyhow::bail!("目标用户不是该项目成员"),
            Some("owner") => anyhow::bail!("不能限制项目 owner"),
            _ => {}
        }

        let updated_at = now();
        let note = note.map(str::trim).filter(|value| !value.is_empty());
        let mute_until = if action == "mute" {
            let minutes = duration_minutes
                .unwrap_or(DEFAULT_MUTE_MINUTES)
                .clamp(1, MAX_MUTE_MINUTES);
            Some((Utc::now() + Duration::minutes(minutes)).to_rfc3339())
        } else {
            None
        };

        match action {
            "mute" => {
                conn.execute(
                    "INSERT INTO project_member_restrictions
                     (project_id, user_id, muted_until, note, updated_by, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(project_id, user_id) DO UPDATE SET
                       muted_until = excluded.muted_until,
                       note = excluded.note,
                       updated_by = excluded.updated_by,
                       updated_at = excluded.updated_at",
                    params![
                        project_id,
                        target_user_id,
                        mute_until,
                        note,
                        actor_user_id,
                        updated_at
                    ],
                )?;
            }
            "unmute" => {
                conn.execute(
                    "INSERT INTO project_member_restrictions
                     (project_id, user_id, muted_until, note, updated_by, updated_at)
                     VALUES (?1, ?2, NULL, ?3, ?4, ?5)
                     ON CONFLICT(project_id, user_id) DO UPDATE SET
                       muted_until = NULL,
                       note = excluded.note,
                       updated_by = excluded.updated_by,
                       updated_at = excluded.updated_at",
                    params![project_id, target_user_id, note, actor_user_id, updated_at],
                )?;
            }
            "ban" => {
                conn.execute(
                    "INSERT INTO project_member_restrictions
                     (project_id, user_id, muted_until, banned_at, banned_until, note, updated_by, updated_at)
                     VALUES (?1, ?2, NULL, ?3, NULL, ?4, ?5, ?3)
                     ON CONFLICT(project_id, user_id) DO UPDATE SET
                       muted_until = NULL,
                       banned_at = excluded.banned_at,
                       banned_until = NULL,
                       note = excluded.note,
                       updated_by = excluded.updated_by,
                       updated_at = excluded.updated_at",
                    params![project_id, target_user_id, updated_at, note, actor_user_id],
                )?;
            }
            "unban" => {
                conn.execute(
                    "INSERT INTO project_member_restrictions
                     (project_id, user_id, banned_at, banned_until, note, updated_by, updated_at)
                     VALUES (?1, ?2, NULL, NULL, ?3, ?4, ?5)
                     ON CONFLICT(project_id, user_id) DO UPDATE SET
                       banned_at = NULL,
                       banned_until = NULL,
                       note = excluded.note,
                       updated_by = excluded.updated_by,
                       updated_at = excluded.updated_at",
                    params![project_id, target_user_id, note, actor_user_id, updated_at],
                )?;
            }
            _ => unreachable!(),
        }
        drop(conn);
        self.project_member_moderation(project_id, target_user_id)
    }

    pub fn project_member_moderation(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<ProjectMemberModerationEntry> {
        let conn = self.conn()?;
        project_member_moderation_entry(&conn, project_id, user_id)
    }

    pub fn active_project_member_muted_until(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<Option<String>> {
        let conn = self.conn()?;
        let now = now();
        conn.query_row(
            "SELECT muted_until
             FROM project_member_restrictions
             WHERE project_id = ?1
               AND user_id = ?2
               AND muted_until IS NOT NULL
               AND muted_until > ?3",
            params![project_id, user_id, now],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn project_member_is_banned(&self, project_id: &str, user_id: &str) -> Result<bool> {
        let conn = self.conn()?;
        project_member_is_banned_locked(&conn, project_id, user_id)
    }
}

pub(super) fn project_member_is_banned_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    user_id: &str,
) -> Result<bool> {
    let now = now();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
         FROM project_member_restrictions
         WHERE project_id = ?1
           AND user_id = ?2
           AND banned_at IS NOT NULL
           AND (banned_until IS NULL OR banned_until > ?3)",
        params![project_id, user_id, now],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub(super) fn project_member_moderation_entry(
    conn: &rusqlite::Connection,
    project_id: &str,
    user_id: &str,
) -> Result<ProjectMemberModerationEntry> {
    let now = now();
    let entry = conn
        .query_row(
            "SELECT
                pm.project_id,
                pm.user_id,
                COALESCE(target.nickname, target.phone, target.email, pm.user_id) AS account,
                r.muted_until,
                r.banned_at,
                r.banned_until,
                r.note,
                r.updated_by,
                COALESCE(updater.nickname, updater.phone, updater.email, r.updated_by) AS updated_by_account,
                COALESCE(r.updated_at, pm.created_at) AS updated_at,
                CASE WHEN r.muted_until IS NOT NULL AND r.muted_until > ?3 THEN 1 ELSE 0 END AS is_muted,
                CASE WHEN r.banned_at IS NOT NULL AND (r.banned_until IS NULL OR r.banned_until > ?3) THEN 1 ELSE 0 END AS is_banned
             FROM project_members pm
             LEFT JOIN project_member_restrictions r
               ON r.project_id = pm.project_id AND r.user_id = pm.user_id
             LEFT JOIN users target ON target.id = pm.user_id
             LEFT JOIN users updater ON updater.id = r.updated_by
             WHERE pm.project_id = ?1 AND pm.user_id = ?2",
            params![project_id, user_id, now],
            project_member_moderation_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow!("目标用户不是该项目成员"))?;
    Ok(entry)
}

fn project_member_moderation_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProjectMemberModerationEntry> {
    Ok(ProjectMemberModerationEntry {
        project_id: row.get(0)?,
        user_id: row.get(1)?,
        account: row.get(2)?,
        muted_until: row.get(3)?,
        banned_at: row.get(4)?,
        banned_until: row.get(5)?,
        note: row.get(6)?,
        updated_by: row.get(7)?,
        updated_by_account: row.get(8)?,
        updated_at: row.get(9)?,
        is_muted: row.get::<_, i64>(10)? != 0,
        is_banned: row.get::<_, i64>(11)? != 0,
    })
}

fn normalize_moderation_action(action: &str) -> Result<&'static str> {
    match action.trim() {
        "mute" => Ok("mute"),
        "unmute" => Ok("unmute"),
        "ban" => Ok("ban"),
        "unban" => Ok("unban"),
        _ => anyhow::bail!("action 必须为 mute / unmute / ban / unban"),
    }
}

fn ensure_project_not_system_for_moderation(
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
        anyhow::bail!("系统归档项目不能限制成员");
    }
    Ok(())
}
