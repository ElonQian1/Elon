/// store/project_space.rs — 项目空间频道与共享消息
///
/// 职责：
///   - 为项目补齐默认频道
///   - 返回项目空间首页需要的摘要、频道、成员
///   - 保存项目频道消息与 AI 任务状态
use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{new_id, now, ProjectChannel, ProjectChannelMessage, ProjectSpaceSummary, Store};

const DEFAULT_CHANNELS: [(&str, &str, i64); 7] = [
    ("公告", "announcements", 10),
    ("讨论", "discussion", 20),
    ("需求", "requirements", 30),
    ("意见", "suggestions", 35),
    ("问题反馈", "issues", 40),
    ("AI开发", "ai_development", 50),
    ("构建发布", "builds", 60),
];

impl Store {
    pub fn project_space_summary(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<ProjectSpaceSummary> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        conn.query_row(
            "SELECT p.id, p.name, p.description, pm.role,
                    (SELECT COUNT(*) FROM project_members count_pm WHERE count_pm.project_id = p.id),
                    p.updated_at
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id
             WHERE p.id = ?1 AND pm.user_id = ?2 AND p.status != 'deleted'",
            params![project_id, user_id],
            |row| {
                Ok(ProjectSpaceSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    role: row.get(3)?,
                    member_count: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))
    }

    pub fn list_project_space_channels(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<Vec<ProjectChannel>> {
        self.ensure_project_default_channels(project_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT c.id, c.project_id, c.name, c.kind, c.position,
                    (
                      SELECT m.content
                      FROM project_channel_messages m
                      WHERE m.project_id = c.project_id AND m.channel_id = c.id
                      ORDER BY m.created_at DESC LIMIT 1
                    ) AS last_message,
                    (
                      SELECT m.created_at
                      FROM project_channel_messages m
                      WHERE m.project_id = c.project_id AND m.channel_id = c.id
                      ORDER BY m.created_at DESC LIMIT 1
                    ) AS last_message_at,
                    (
                      SELECT COUNT(*)
                      FROM project_channel_messages unread
                      LEFT JOIN project_channel_read_states rs
                        ON rs.project_id = unread.project_id
                       AND rs.channel_id = unread.channel_id
                       AND rs.user_id = ?1
                      WHERE unread.project_id = c.project_id
                        AND unread.channel_id = c.id
                        AND COALESCE(rs.last_read_at, '') < unread.created_at
                        AND COALESCE(unread.sender_user_id, '') != ?1
                    ) AS unread_count
             FROM project_channels c
             WHERE c.project_id = ?2
             ORDER BY c.position, c.created_at",
        )?;
        let rows = stmt
            .query_map(params![user_id, project_id], |row| {
                Ok(ProjectChannel {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    kind: row.get(3)?,
                    position: row.get(4)?,
                    last_message: row.get(5)?,
                    last_message_at: row.get(6)?,
                    unread_count: row.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_project_channel_kind(&self, project_id: &str, channel_id: &str) -> Result<String> {
        self.ensure_project_default_channels(project_id)?;
        self.conn()?
            .query_row(
                "SELECT kind FROM project_channels WHERE project_id = ?1 AND id = ?2",
                params![project_id, channel_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("频道不存在"))
    }

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
                    m.kind, m.content, m.task_id,
                    m.suggestion_status,
                    m.suggestion_resolved_by,
                    COALESCE(resolver.nickname, resolver.phone, resolver.email, m.suggestion_resolved_by)
                      AS suggestion_resolved_by_name,
                    m.suggestion_resolved_at,
                    m.created_at
             FROM project_channel_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             LEFT JOIN users resolver ON resolver.id = m.suggestion_resolved_by
             WHERE m.project_id = ?1 AND m.channel_id = ?2
             ORDER BY m.created_at DESC
             LIMIT ?3",
        )?;
        let mut rows = stmt
            .query_map(
                params![project_id, channel_id, limit.clamp(1, 200)],
                |row| {
                    let sender_user_id: Option<String> = row.get(3)?;
                    Ok(ProjectChannelMessage {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        channel_id: row.get(2)?,
                        outgoing: sender_user_id.as_deref() == Some(user_id),
                        sender_user_id,
                        sender_name: row.get(4)?,
                        kind: row.get(5)?,
                        content: row.get(6)?,
                        task_id: row.get(7)?,
                        suggestion_status: row.get(8)?,
                        suggestion_resolved_by: row.get(9)?,
                        suggestion_resolved_by_name: row.get(10)?,
                        suggestion_resolved_at: row.get(11)?,
                        created_at: row.get(12)?,
                    })
                },
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
                suggestion_status, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                project_id,
                channel_id,
                sender_user_id,
                kind,
                content,
                task_id,
                suggestion_status,
                created_at
            ],
        )?;
        conn.execute(
            "UPDATE project_channels SET updated_at = ?1 WHERE id = ?2",
            params![created_at, channel_id],
        )?;
        drop(conn);

        Ok(ProjectChannelMessage {
            id,
            project_id: project_id.to_string(),
            channel_id: channel_id.to_string(),
            sender_user_id: sender_user_id.map(ToOwned::to_owned),
            sender_name: None,
            kind: kind.to_string(),
            content: content.to_string(),
            task_id: task_id.map(ToOwned::to_owned),
            suggestion_status: suggestion_status.map(ToOwned::to_owned),
            suggestion_resolved_by: None,
            suggestion_resolved_by_name: None,
            suggestion_resolved_at: None,
            created_at,
            outgoing: sender_user_id.is_some(),
        })
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
                    m.kind, m.content, m.task_id,
                    m.suggestion_status,
                    m.suggestion_resolved_by,
                    COALESCE(resolver.nickname, resolver.phone, resolver.email, m.suggestion_resolved_by)
                      AS suggestion_resolved_by_name,
                    m.suggestion_resolved_at,
                    m.created_at
             FROM project_channel_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             LEFT JOIN users resolver ON resolver.id = m.suggestion_resolved_by
             WHERE m.project_id = ?1 AND m.channel_id = ?2 AND m.id = ?3",
            params![project_id, channel_id, message_id],
            |row| {
                let sender_user_id: Option<String> = row.get(3)?;
                Ok(ProjectChannelMessage {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    channel_id: row.get(2)?,
                    outgoing: sender_user_id.as_deref() == Some(user_id),
                    sender_user_id,
                    sender_name: row.get(4)?,
                    kind: row.get(5)?,
                    content: row.get(6)?,
                    task_id: row.get(7)?,
                    suggestion_status: row.get(8)?,
                    suggestion_resolved_by: row.get(9)?,
                    suggestion_resolved_by_name: row.get(10)?,
                    suggestion_resolved_at: row.get(11)?,
                    created_at: row.get(12)?,
                })
            },
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
        for (name, kind, position) in DEFAULT_CHANNELS {
            conn.execute(
                "INSERT OR IGNORE INTO project_channels (
                    id, project_id, name, kind, position, created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![new_id("pch"), project_id, name, kind, position, created_at],
            )?;
        }
        Ok(())
    }

    fn mark_project_channel_read_locked(
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_project_suggestions_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn default_channels_include_suggestions() {
        let store = temp_store();
        let owner = store
            .create_user("suggestions-owner@example.com", "secret1", None, None)
            .expect("owner should be created");
        let project = store
            .create_project(&owner.id, "Suggestions Project", None, None)
            .expect("project should be created")
            .project;

        let channels = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("channels should list");

        assert!(channels.iter().any(|channel| channel.kind == "suggestions"));
    }

    #[test]
    fn suggestion_message_can_be_marked_updated() {
        let store = temp_store();
        let owner = store
            .create_user("suggestions-resolver@example.com", "secret1", None, None)
            .expect("owner should be created");
        let project = store
            .create_project(&owner.id, "Suggestion Resolve", None, None)
            .expect("project should be created")
            .project;
        let channel = store
            .list_project_space_channels(&owner.id, &project.id)
            .expect("channels should list")
            .into_iter()
            .find(|channel| channel.kind == "suggestions")
            .expect("suggestions channel should exist");

        let message = store
            .insert_project_channel_message(
                &project.id,
                &channel.id,
                Some(&owner.id),
                "suggestion",
                "希望增加深色模式",
                None,
            )
            .expect("suggestion should insert");
        assert_eq!(message.suggestion_status.as_deref(), Some("open"));

        let updated = store
            .mark_project_suggestion_updated(&owner.id, &project.id, &channel.id, &message.id)
            .expect("suggestion should update");

        assert_eq!(updated.suggestion_status.as_deref(), Some("updated"));
        assert_eq!(
            updated.suggestion_resolved_by.as_deref(),
            Some(owner.id.as_str())
        );
    }
}
