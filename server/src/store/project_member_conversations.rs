/// store/project_member_conversations.rs — 项目成员个人 AI 会话查看与讨论
///
/// 职责：
///   - 校验查看者与目标成员都属于同一项目
///   - 列出目标成员在该项目内的个人 AI 会话摘要
///   - 读取某个会话内该成员自己的 AI 消息与项目成员讨论
///   - 写入项目成员对该会话的人类讨论消息，不触发 AI 任务
use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{new_id, now, ProjectMemberConversationEntry, ProjectMemberConversationMessage, Store};

impl Store {
    pub fn list_project_member_conversations(
        &self,
        requester_id: &str,
        project_id: &str,
        member_user_id: &str,
        limit: i64,
    ) -> Result<Vec<ProjectMemberConversationEntry>> {
        self.ensure_project_member_conversation_access(requester_id, project_id, member_user_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT
               c.id,
               c.project_id,
               c.user_id,
               COALESCE(u.nickname, u.phone, u.email, c.user_id) AS user_account,
               c.title,
               c.status,
               COALESCE(c.is_public, 1) AS is_public,
               (SELECT COUNT(*) FROM messages m
                WHERE m.project_id = c.project_id
                  AND m.conversation_id = c.id
                  AND m.user_id = c.user_id)
               + (SELECT COUNT(*) FROM project_member_conversation_discussion_messages d
                  WHERE d.project_id = c.project_id
                    AND d.member_user_id = c.user_id
                    AND d.conversation_id = c.id) AS message_count,
               (SELECT COUNT(*) FROM tasks t
                WHERE t.project_id = c.project_id
                  AND t.conversation_id = c.id
                  AND t.user_id = c.user_id) AS task_count,
               (SELECT last.content FROM (
                    SELECT m2.content, m2.created_at, m2.id
                      FROM messages m2
                     WHERE m2.project_id = c.project_id
                       AND m2.conversation_id = c.id
                       AND m2.user_id = c.user_id
                    UNION ALL
                    SELECT d2.content, d2.created_at, d2.id
                      FROM project_member_conversation_discussion_messages d2
                     WHERE d2.project_id = c.project_id
                       AND d2.member_user_id = c.user_id
                       AND d2.conversation_id = c.id
                ) last
                ORDER BY last.created_at DESC, last.id DESC LIMIT 1) AS last_message,
               (SELECT last.role FROM (
                    SELECT m2.role, m2.created_at, m2.id
                      FROM messages m2
                     WHERE m2.project_id = c.project_id
                       AND m2.conversation_id = c.id
                       AND m2.user_id = c.user_id
                    UNION ALL
                    SELECT 'discussion' AS role, d2.created_at, d2.id
                      FROM project_member_conversation_discussion_messages d2
                     WHERE d2.project_id = c.project_id
                       AND d2.member_user_id = c.user_id
                       AND d2.conversation_id = c.id
                ) last
                ORDER BY last.created_at DESC, last.id DESC LIMIT 1) AS last_message_role,
               (SELECT last.created_at FROM (
                    SELECT m2.created_at, m2.id
                      FROM messages m2
                     WHERE m2.project_id = c.project_id
                       AND m2.conversation_id = c.id
                       AND m2.user_id = c.user_id
                    UNION ALL
                    SELECT d2.created_at, d2.id
                      FROM project_member_conversation_discussion_messages d2
                     WHERE d2.project_id = c.project_id
                       AND d2.member_user_id = c.user_id
                       AND d2.conversation_id = c.id
                ) last
                ORDER BY last.created_at DESC, last.id DESC LIMIT 1) AS last_message_at,
               (SELECT t2.status FROM tasks t2
                WHERE t2.project_id = c.project_id
                  AND t2.conversation_id = c.id
                  AND t2.user_id = c.user_id
                ORDER BY t2.created_at DESC LIMIT 1) AS last_task_status,
               c.created_at,
               c.updated_at
             FROM conversations c
             LEFT JOIN users u ON u.id = c.user_id
             WHERE c.project_id = ?1
               AND c.user_id = ?2
               AND (?3 = c.user_id OR COALESCE(c.is_public, 1) = 1)
             ORDER BY COALESCE(last_message_at, c.created_at) DESC, c.created_at DESC, c.id DESC
             LIMIT ?4",
        )?;
        let rows = stmt
            .query_map(
                params![
                    project_id,
                    member_user_id,
                    requester_id,
                    limit.clamp(1, 100)
                ],
                |row| {
                    Ok(ProjectMemberConversationEntry {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        user_id: row.get(2)?,
                        user_account: row.get(3)?,
                        title: row.get(4)?,
                        status: row.get(5)?,
                        is_public: row.get::<_, i64>(6)? != 0,
                        message_count: row.get(7)?,
                        task_count: row.get(8)?,
                        last_message: row.get(9)?,
                        last_message_role: row.get(10)?,
                        last_message_at: row.get(11)?,
                        last_task_status: row.get(12)?,
                        created_at: row.get(13)?,
                        updated_at: row.get(14)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn list_project_member_conversation_messages(
        &self,
        requester_id: &str,
        project_id: &str,
        member_user_id: &str,
        conversation_id: &str,
        limit: i64,
    ) -> Result<Vec<ProjectMemberConversationMessage>> {
        self.ensure_project_member_conversation_visible(
            requester_id,
            project_id,
            member_user_id,
            conversation_id,
        )?;
        let conn = self.conn()?;

        let mut stmt = conn.prepare(
            "SELECT id, project_id, conversation_id, task_id, user_id, sender_name,
                    sender_avatar_data_url, role, content, created_at, outgoing
             FROM (
                SELECT m.id, m.project_id, m.conversation_id, m.task_id, m.user_id,
                       COALESCE(u.nickname, u.phone, u.email, m.user_id) AS sender_name,
                       u.avatar_data_url AS sender_avatar_data_url,
                       m.role, m.content, m.created_at,
                       CASE WHEN LOWER(m.role) IN ('user', 'human') AND m.user_id = ?4 THEN 1 ELSE 0 END AS outgoing
                FROM messages m
                LEFT JOIN users u ON u.id = m.user_id
                WHERE m.project_id = ?1
                  AND m.conversation_id = ?2
                  AND m.user_id = ?3
                UNION ALL
                SELECT d.id, d.project_id, d.conversation_id, NULL AS task_id, d.sender_user_id AS user_id,
                       COALESCE(u.nickname, u.phone, u.email, d.sender_user_id) AS sender_name,
                       u.avatar_data_url AS sender_avatar_data_url,
                       'discussion' AS role, d.content, d.created_at,
                       CASE WHEN d.sender_user_id = ?4 THEN 1 ELSE 0 END AS outgoing
                FROM project_member_conversation_discussion_messages d
                LEFT JOIN users u ON u.id = d.sender_user_id
                WHERE d.project_id = ?1
                  AND d.member_user_id = ?3
                  AND d.conversation_id = ?2
                ORDER BY 10 DESC, 1 DESC
                LIMIT ?5
             )
             ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt
            .query_map(
                params![
                    project_id,
                    conversation_id,
                    member_user_id,
                    requester_id,
                    limit.clamp(1, 200)
                ],
                |row| {
                    Ok(ProjectMemberConversationMessage {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        conversation_id: row.get(2)?,
                        task_id: row.get(3)?,
                        user_id: row.get(4)?,
                        sender_name: row.get(5)?,
                        sender_avatar_data_url: row.get(6)?,
                        role: row.get(7)?,
                        content: row.get(8)?,
                        created_at: row.get(9)?,
                        outgoing: row.get::<_, i64>(10)? != 0,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn insert_project_member_conversation_discussion_message(
        &self,
        requester_id: &str,
        project_id: &str,
        member_user_id: &str,
        conversation_id: &str,
        content: &str,
    ) -> Result<ProjectMemberConversationMessage> {
        self.ensure_project_member_conversation_visible(
            requester_id,
            project_id,
            member_user_id,
            conversation_id,
        )?;
        let content = content.trim();
        if content.is_empty() {
            return Err(anyhow!("消息内容不能为空"));
        }

        let conn = self.conn()?;

        let id = new_id("pmcm");
        let created_at = now();
        conn.execute(
            "INSERT INTO project_member_conversation_discussion_messages (
                id, project_id, member_user_id, conversation_id, sender_user_id, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                project_id,
                member_user_id,
                conversation_id,
                requester_id,
                content,
                created_at
            ],
        )?;
        conn.execute(
            "UPDATE conversations
                SET updated_at = ?1
              WHERE project_id = ?2
                AND user_id = ?3
                AND id = ?4",
            params![created_at, project_id, member_user_id, conversation_id],
        )?;
        let sender: Option<(Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT COALESCE(nickname, phone, email, id), avatar_data_url FROM users WHERE id = ?1",
                params![requester_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        Ok(ProjectMemberConversationMessage {
            id,
            project_id: project_id.to_string(),
            conversation_id: Some(conversation_id.to_string()),
            task_id: None,
            user_id: Some(requester_id.to_string()),
            sender_name: sender.as_ref().and_then(|(name, _)| name.clone()),
            sender_avatar_data_url: sender.and_then(|(_, avatar)| avatar),
            role: "discussion".to_string(),
            content: content.to_string(),
            created_at,
            outgoing: true,
        })
    }

    pub fn update_project_member_conversation_visibility(
        &self,
        requester_id: &str,
        project_id: &str,
        conversation_id: &str,
        is_public: bool,
    ) -> Result<ProjectMemberConversationEntry> {
        self.ensure_project_member_conversation_access(requester_id, project_id, requester_id)?;
        let updated_at = now();
        let changed = {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE conversations
                    SET is_public = ?1,
                        updated_at = ?2
                  WHERE project_id = ?3
                    AND user_id = ?4
                    AND id = ?5",
                params![
                    if is_public { 1 } else { 0 },
                    updated_at,
                    project_id,
                    requester_id,
                    conversation_id
                ],
            )?
        };
        if changed == 0 {
            return Err(anyhow!("成员会话不存在"));
        }
        self.list_project_member_conversations(requester_id, project_id, requester_id, 100)?
            .into_iter()
            .find(|conversation| conversation.id == conversation_id)
            .ok_or_else(|| anyhow!("成员会话不存在"))
    }

    fn ensure_project_member_conversation_access(
        &self,
        requester_id: &str,
        project_id: &str,
        member_user_id: &str,
    ) -> Result<()> {
        let conn = self.conn()?;
        let requester_count: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM project_members pm
             JOIN projects p ON p.id = pm.project_id
             WHERE pm.project_id = ?1
               AND pm.user_id = ?2
               AND p.status != 'deleted'",
            params![project_id, requester_id],
            |row| row.get(0),
        )?;
        if requester_count == 0 {
            return Err(anyhow!("当前用户无权查看该项目成员会话"));
        }
        let member_count: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM project_members
             WHERE project_id = ?1 AND user_id = ?2",
            params![project_id, member_user_id],
            |row| row.get(0),
        )?;
        if member_count == 0 {
            return Err(anyhow!("目标成员不在该项目中"));
        }
        Ok(())
    }

    fn ensure_project_member_conversation_visible(
        &self,
        requester_id: &str,
        project_id: &str,
        member_user_id: &str,
        conversation_id: &str,
    ) -> Result<()> {
        self.ensure_project_member_conversation_access(requester_id, project_id, member_user_id)?;
        let conn = self.conn()?;
        let is_public: Option<i64> = conn
            .query_row(
                "SELECT COALESCE(is_public, 1)
                 FROM conversations
                 WHERE project_id = ?1 AND user_id = ?2 AND id = ?3",
                params![project_id, member_user_id, conversation_id],
                |row| row.get(0),
            )
            .optional()?;
        match is_public {
            Some(value) if value != 0 || requester_id == member_user_id => Ok(()),
            Some(_) => Err(anyhow!("该成员已关闭此会话公开")),
            None => Err(anyhow!("成员会话不存在")),
        }
    }
}


#[cfg(test)]
#[path = "project_member_conversations_tests.rs"]
mod project_member_conversations_tests;
