use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{project_channel_message_from_row, CHANNEL_PERMISSION_VIEW};
use super::super::{new_id, now, ProjectChannelMessage, Store};

impl Store {
    pub fn list_project_channel_messages(
        &self,
        user_id: &str,
        project_id: &str,
        channel_id: &str,
        limit: i64,
    ) -> Result<Vec<ProjectChannelMessage>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT m.id, m.project_id, m.channel_id, m.sender_user_id,
                    COALESCE(u.nickname, u.phone, u.email, m.sender_user_id) AS sender_name,
                    u.avatar_data_url,
                    m.reply_to_message_id,
                    m.kind, m.content, m.task_id,
                    t.status AS task_status,
                    t.error AS task_error,
                    t.apk_url AS task_apk_url, t.codex_thread_id AS task_codex_thread_id,
                    m.suggestion_status,
                    m.suggestion_resolved_by,
                    COALESCE(resolver.nickname, resolver.phone, resolver.email, m.suggestion_resolved_by)
                      AS suggestion_resolved_by_name,
                    m.suggestion_resolved_at,
                    m.created_at
             FROM project_channel_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             LEFT JOIN users resolver ON resolver.id = m.suggestion_resolved_by
             LEFT JOIN tasks t ON t.id = m.task_id
             WHERE m.project_id = ?1 AND m.channel_id = ?2
             ORDER BY m.created_at DESC
             LIMIT ?3",
        )?;
        let mut rows = stmt
            .query_map(
                params![project_id, channel_id, limit.clamp(1, 200)],
                |row| project_channel_message_from_row(row, user_id),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.reverse();
        drop(stmt);
        self.mark_project_channel_read_locked(&conn, user_id, project_id, channel_id)?;
        Ok(rows)
    }

    pub fn insert_project_channel_message(
        &self,
        project_id: &str,
        channel_id: &str,
        sender_user_id: Option<&str>,
        kind: &str,
        content: &str,
        task_id: Option<&str>,
        reply_to_message_id: Option<&str>,
    ) -> Result<ProjectChannelMessage> {
        let content = content.trim();
        if content.is_empty() {
            return Err(anyhow!("消息内容不能为空"));
        }
        let kind = match kind {
            "text" | "suggestion" | "ai_task" | "ai_progress" | "ai_result" | "system" => kind,
            _ => "text",
        };
        let suggestion_status = if kind == "suggestion" {
            Some("open")
        } else {
            None
        };
        let id = new_id("pcm");
        let created_at = now();
        let conn = self.conn()?;
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM project_channels WHERE project_id = ?1 AND id = ?2",
                params![project_id, channel_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(anyhow!("频道不存在"));
        }
        conn.execute(
            "INSERT INTO project_channel_messages (
                id, project_id, channel_id, sender_user_id, kind, content, task_id,
                suggestion_status, reply_to_message_id, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                project_id,
                channel_id,
                sender_user_id,
                kind,
                content,
                task_id,
                suggestion_status,
                reply_to_message_id,
                created_at
            ],
        )?;
        conn.execute(
            "UPDATE project_channels SET updated_at = ?1 WHERE id = ?2",
            params![created_at, channel_id],
        )?;
        conn.query_row(
            "SELECT m.id, m.project_id, m.channel_id, m.sender_user_id,
                    COALESCE(u.nickname, u.phone, u.email, m.sender_user_id) AS sender_name,
                    u.avatar_data_url,
                    m.reply_to_message_id,
                    m.kind, m.content, m.task_id,
                    t.status AS task_status,
                    t.error AS task_error,
                    t.apk_url AS task_apk_url, t.codex_thread_id AS task_codex_thread_id,
                    m.suggestion_status,
                    m.suggestion_resolved_by,
                    COALESCE(resolver.nickname, resolver.phone, resolver.email, m.suggestion_resolved_by)
                      AS suggestion_resolved_by_name,
                    m.suggestion_resolved_at,
                    m.created_at
             FROM project_channel_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             LEFT JOIN users resolver ON resolver.id = m.suggestion_resolved_by
             LEFT JOIN tasks t ON t.id = m.task_id
             WHERE m.project_id = ?1 AND m.channel_id = ?2 AND m.id = ?3",
            params![project_id, channel_id, id],
            |row| project_channel_message_from_row(row, sender_user_id.unwrap_or_default()),
        )
        .map_err(Into::into)
    }

    pub fn insert_project_channel_ai_result_once(
        &self,
        project_id: &str,
        channel_id: &str,
        content: &str,
        task_id: &str,
    ) -> Result<bool> {
        let content = content.trim();
        if content.is_empty() {
            return Err(anyhow!("消息内容不能为空"));
        }
        let created_at = now();
        let conn = self.conn()?;
        let exists: Option<String> = conn
            .query_row(
                "SELECT id
                 FROM project_channel_messages
                 WHERE project_id = ?1
                   AND channel_id = ?2
                   AND task_id = ?3
                   AND kind = 'ai_result'
                 LIMIT 1",
                params![project_id, channel_id, task_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_some() {
            return Ok(false);
        }
        let channel_exists: Option<String> = conn
            .query_row(
                "SELECT id FROM project_channels WHERE project_id = ?1 AND id = ?2",
                params![project_id, channel_id],
                |row| row.get(0),
            )
            .optional()?;
        if channel_exists.is_none() {
            return Err(anyhow!("频道不存在"));
        }

        conn.execute(
            "INSERT INTO project_channel_messages (
                id, project_id, channel_id, sender_user_id, kind, content, task_id, created_at
             )
             VALUES (?1, ?2, ?3, NULL, 'ai_result', ?4, ?5, ?6)",
            params![
                new_id("pcm"),
                project_id,
                channel_id,
                content,
                task_id,
                created_at
            ],
        )?;
        conn.execute(
            "UPDATE project_channels SET updated_at = ?1 WHERE project_id = ?2 AND id = ?3",
            params![created_at, project_id, channel_id],
        )?;
        Ok(true)
    }

    pub fn mark_project_suggestion_updated(
        &self,
        user_id: &str,
        project_id: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<ProjectChannelMessage> {
        self.ensure_project_default_channels(project_id)?;
        let resolved_at = now();
        let conn = self.conn()?;
        let channel_kind: Option<String> = conn
            .query_row(
                "SELECT kind FROM project_channels WHERE project_id = ?1 AND id = ?2",
                params![project_id, channel_id],
                |row| row.get(0),
            )
            .optional()?;
        if channel_kind.as_deref() != Some("suggestions") {
            return Err(anyhow!("只有意见频道的建议可以标记为已更新"));
        }

        let changed = conn.execute(
            "UPDATE project_channel_messages
                SET suggestion_status = 'updated',
                    suggestion_resolved_by = ?1,
                    suggestion_resolved_at = ?2
              WHERE project_id = ?3
                AND channel_id = ?4
                AND id = ?5
                AND kind = 'suggestion'",
            params![user_id, resolved_at, project_id, channel_id, message_id],
        )?;
        if changed == 0 {
            return Err(anyhow!("建议不存在"));
        }
        conn.execute(
            "UPDATE project_channels SET updated_at = ?1 WHERE project_id = ?2 AND id = ?3",
            params![resolved_at, project_id, channel_id],
        )?;

        conn.query_row(
            "SELECT m.id, m.project_id, m.channel_id, m.sender_user_id,
                    COALESCE(u.nickname, u.phone, u.email, m.sender_user_id) AS sender_name,
                    u.avatar_data_url,
                    m.reply_to_message_id,
                    m.kind, m.content, m.task_id,
                    t.status AS task_status,
                    t.error AS task_error,
                    t.apk_url AS task_apk_url, t.codex_thread_id AS task_codex_thread_id,
                    m.suggestion_status,
                    m.suggestion_resolved_by,
                    COALESCE(resolver.nickname, resolver.phone, resolver.email, m.suggestion_resolved_by)
                      AS suggestion_resolved_by_name,
                    m.suggestion_resolved_at,
                    m.created_at
             FROM project_channel_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             LEFT JOIN users resolver ON resolver.id = m.suggestion_resolved_by
             LEFT JOIN tasks t ON t.id = m.task_id
            WHERE m.project_id = ?1 AND m.channel_id = ?2 AND m.id = ?3",
            params![project_id, channel_id, message_id],
            |row| project_channel_message_from_row(row, user_id),
        )
        .map_err(Into::into)
    }

    pub fn ensure_project_default_channels(&self, project_id: &str) -> Result<()> {
        let conn = self.conn()?;
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM projects WHERE id = ?1 AND status != 'deleted'",
                params![project_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(anyhow!("项目不存在"));
        }
        let created_at = now();
        for (name, kind, position) in super::DEFAULT_CHANNEL_CATEGORIES {
            conn.execute(
                "INSERT OR IGNORE INTO project_channel_categories (
                    id, project_id, name, kind, position, created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![new_id("pcc"), project_id, name, kind, position, created_at],
            )?;
        }
        let mut category_stmt = conn.prepare(
            "SELECT kind, id
               FROM project_channel_categories
              WHERE project_id = ?1",
        )?;
        let category_rows = category_stmt
            .query_map(params![project_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(category_stmt);
        let category_ids: std::collections::HashMap<String, String> =
            category_rows.into_iter().collect();
        for (name, kind, position, category_kind) in super::DEFAULT_CHANNELS {
            let category_id = category_ids.get(category_kind).map(String::as_str);
            conn.execute(
                "INSERT OR IGNORE INTO project_channels (
                    id, project_id, category_id, name, kind, position, created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    new_id("pch"),
                    project_id,
                    category_id,
                    name,
                    kind,
                    position,
                    created_at
                ],
            )?;
            if let Some(category_id) = category_id {
                conn.execute(
                    "UPDATE project_channels
                        SET category_id = ?1,
                            permission_sync = COALESCE(permission_sync, 1)
                      WHERE project_id = ?2
                        AND kind = ?3
                        AND (category_id IS NULL OR category_id = '')",
                    params![category_id, project_id, kind],
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn mark_project_channel_read_locked(
        &self,
        conn: &rusqlite::Connection,
        user_id: &str,
        project_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        conn.execute(
            "INSERT INTO project_channel_read_states (
                project_id, channel_id, user_id, last_read_at
             )
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id, channel_id, user_id)
             DO UPDATE SET last_read_at = excluded.last_read_at",
            params![project_id, channel_id, user_id, now()],
        )?;
        Ok(())
    }
}
