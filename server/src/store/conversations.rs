//! 会话（Conversation）与消息（Message）数据库操作。
//!
//! 负责会话的创建/幂等更新、消息写入、消息历史查询以及管理后台会话总览。

use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{
    clean_optional, new_id, now, safe_external_id, AdminConversationEntry, ConversationMessage,
    Store, UserConversationEntry, UserConversationMessage,
};

impl Store {
    /// 确保指定会话存在，不存在则创建；存在则幂等更新 title/updated_at。
    pub fn ensure_conversation(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        title: Option<&str>,
    ) -> Result<String> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let now = now();
        self.conn()?.execute(
            "INSERT INTO conversations (
                project_id, user_id, id, title, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?5)
             ON CONFLICT(project_id, user_id, id) DO UPDATE SET
                title = COALESCE(excluded.title, conversations.title),
                updated_at = excluded.updated_at",
            params![
                project_id,
                user_id,
                conversation_id,
                clean_optional(title),
                now
            ],
        )?;
        Ok(conversation_id)
    }

    /// 向消息表写入一条消息（用户消息、AI 回复或系统消息）。
    pub fn add_message(
        &self,
        project_id: &str,
        conversation_id: Option<&str>,
        task_id: Option<&str>,
        user_id: Option<&str>,
        role: &str,
        content: &str,
    ) -> Result<()> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        self.conn()?.execute(
            "INSERT INTO messages (
                id, project_id, conversation_id, task_id, user_id, role, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                new_id("msg"),
                project_id,
                conversation_id,
                clean_optional(task_id),
                clean_optional(user_id),
                role,
                content,
                now()
            ],
        )?;
        Ok(())
    }

    /// 查询最近的会话消息（限 1-30 条，按时间正序返回）。
    pub fn list_recent_conversation_messages(
        &self,
        project_id: &str,
        conversation_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ConversationMessage>> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let limit = limit.clamp(1, 30) as i64;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT role, content
             FROM (
                SELECT role, content, created_at, id
                FROM messages
                WHERE project_id = ?1
                  AND conversation_id = ?2
                ORDER BY created_at DESC, id DESC
                LIMIT ?3
             )
             ORDER BY created_at ASC, id ASC",
        )?;
        let messages = stmt
            .query_map(params![project_id, conversation_id, limit], |row| {
                Ok(ConversationMessage {
                    role: row.get(0)?,
                    content: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(messages)
    }

    /// 列出当前用户在指定归档项目下的会话摘要。
    pub fn list_user_conversations(
        &self,
        project_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<UserConversationEntry>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT
               c.id,
               c.project_id,
               c.user_id,
               c.title,
               c.status,
               (SELECT COUNT(*) FROM messages m
                WHERE m.project_id = c.project_id
                  AND m.conversation_id = c.id) AS message_count,
               (SELECT m2.content FROM messages m2
                WHERE m2.project_id = c.project_id
                  AND m2.conversation_id = c.id
                ORDER BY m2.created_at DESC, m2.id DESC LIMIT 1) AS last_message,
               (SELECT m2.role FROM messages m2
                WHERE m2.project_id = c.project_id
                  AND m2.conversation_id = c.id
                ORDER BY m2.created_at DESC, m2.id DESC LIMIT 1) AS last_message_role,
               (SELECT m2.created_at FROM messages m2
                WHERE m2.project_id = c.project_id
                  AND m2.conversation_id = c.id
                ORDER BY m2.created_at DESC, m2.id DESC LIMIT 1) AS last_message_at,
               c.created_at,
               c.updated_at
             FROM conversations c
             WHERE c.project_id = ?1
               AND c.user_id = ?2
             ORDER BY c.updated_at DESC
             LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(params![project_id, user_id, limit.clamp(1, 100)], |row| {
                Ok(UserConversationEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    user_id: row.get(2)?,
                    title: row.get(3)?,
                    status: row.get(4)?,
                    message_count: row.get(5)?,
                    last_message: row.get(6)?,
                    last_message_role: row.get(7)?,
                    last_message_at: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// 读取当前用户自己的某个普通会话消息。
    pub fn list_user_conversation_messages(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
        limit: i64,
    ) -> Result<Vec<UserConversationMessage>> {
        let conversation_id = safe_external_id(conversation_id, "default");
        let conn = self.conn()?;
        let exists: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM conversations
                 WHERE project_id = ?1 AND user_id = ?2 AND id = ?3",
                params![project_id, user_id, conversation_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(anyhow!("会话不存在"));
        }

        let mut stmt = conn.prepare(
            "SELECT id, project_id, conversation_id, task_id, user_id, sender_name,
                    role, content, created_at, outgoing
             FROM (
                SELECT m.id,
                       m.project_id,
                       m.conversation_id,
                       m.task_id,
                       m.user_id,
                       CASE
                         WHEN m.user_id = ?2 THEN COALESCE(u.nickname, u.phone, u.email, '我')
                         WHEN LOWER(m.role) IN ('assistant', 'ai') THEN '一龙AI'
                         ELSE COALESCE(u.nickname, u.phone, u.email, m.user_id, m.role)
                       END AS sender_name,
                       m.role,
                       m.content,
                       m.created_at,
                       CASE WHEN m.user_id = ?2 OR LOWER(m.role) = 'user' THEN 1 ELSE 0 END AS outgoing
                FROM messages m
                LEFT JOIN users u ON u.id = m.user_id
                WHERE m.project_id = ?1
                  AND m.conversation_id = ?3
                ORDER BY m.created_at DESC, m.id DESC
                LIMIT ?4
             )
             ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt
            .query_map(
                params![project_id, user_id, conversation_id, limit.clamp(1, 200)],
                |row| {
                    Ok(UserConversationMessage {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        conversation_id: row.get(2)?,
                        task_id: row.get(3)?,
                        user_id: row.get(4)?,
                        sender_name: row.get(5)?,
                        role: row.get(6)?,
                        content: row.get(7)?,
                        created_at: row.get(8)?,
                        outgoing: row.get::<_, i64>(9)? != 0,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// 管理员总览：列出某项目下所有会话，附带消息数、任务数和最后任务状态。
    pub fn list_conversations_for_project_admin(
        &self,
        project_id: &str,
    ) -> Result<Vec<AdminConversationEntry>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT
               c.id,
               c.project_id,
               c.user_id,
               COALESCE(u.phone, u.email, c.user_id) AS user_account,
               c.title,
               c.status,
               (SELECT COUNT(*) FROM messages m
                WHERE m.project_id = c.project_id
                  AND m.conversation_id = c.id) AS message_count,
               (SELECT COUNT(*) FROM tasks t
                WHERE t.project_id = c.project_id
                  AND t.conversation_id = c.id) AS task_count,
               (SELECT t2.status FROM tasks t2
                WHERE t2.project_id = c.project_id
                  AND t2.conversation_id = c.id
                ORDER BY t2.created_at DESC LIMIT 1) AS last_task_status,
               c.created_at,
               c.updated_at
             FROM conversations c
             LEFT JOIN users u ON u.id = c.user_id
             WHERE c.project_id = ?1
             ORDER BY c.updated_at DESC",
        )?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(AdminConversationEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    user_id: row.get(2)?,
                    user_account: row.get(3)?,
                    title: row.get(4)?,
                    status: row.get(5)?,
                    message_count: row.get(6)?,
                    task_count: row.get(7)?,
                    last_task_status: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// 获取会话的软锁定 agent（首次使用的 CLI 选项 ID）。
    pub fn get_conversation_locked_agent(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<Option<String>> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT locked_agent_name FROM conversations
             WHERE project_id = ?1 AND user_id = ?2 AND id = ?3",
            params![project_id, user_id, conversation_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map(|opt| opt.flatten())
        .map_err(Into::into)
    }

    /// 设置会话的软锁定 agent（仅在首次时设置，已有值则忽略）。
    pub fn set_conversation_locked_agent_if_unset(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
        agent_name: &str,
    ) -> Result<()> {
        self.conn()?.execute(
            "UPDATE conversations
             SET locked_agent_name = ?1
             WHERE project_id = ?2 AND user_id = ?3 AND id = ?4
               AND (locked_agent_name IS NULL OR locked_agent_name = '')",
            params![agent_name, project_id, user_id, conversation_id],
        )?;
        Ok(())
    }
}
