/// store/project_member_conversations.rs — 项目成员个人 AI 会话只读查询
///
/// 职责：
///   - 校验查看者与目标成员都属于同一项目
///   - 列出目标成员在该项目内的个人 AI 会话摘要
///   - 读取某个会话内该成员自己的消息时间线
use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{ProjectMemberConversationEntry, ProjectMemberConversationMessage, Store};

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
               (SELECT COUNT(*) FROM messages m
                WHERE m.project_id = c.project_id
                  AND m.conversation_id = c.id
                  AND m.user_id = c.user_id) AS message_count,
               (SELECT COUNT(*) FROM tasks t
                WHERE t.project_id = c.project_id
                  AND t.conversation_id = c.id
                  AND t.user_id = c.user_id) AS task_count,
               (SELECT m2.content FROM messages m2
                WHERE m2.project_id = c.project_id
                  AND m2.conversation_id = c.id
                  AND m2.user_id = c.user_id
                ORDER BY m2.created_at DESC, m2.id DESC LIMIT 1) AS last_message,
               (SELECT m2.role FROM messages m2
                WHERE m2.project_id = c.project_id
                  AND m2.conversation_id = c.id
                  AND m2.user_id = c.user_id
                ORDER BY m2.created_at DESC, m2.id DESC LIMIT 1) AS last_message_role,
               (SELECT m2.created_at FROM messages m2
                WHERE m2.project_id = c.project_id
                  AND m2.conversation_id = c.id
                  AND m2.user_id = c.user_id
                ORDER BY m2.created_at DESC, m2.id DESC LIMIT 1) AS last_message_at,
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
             ORDER BY c.updated_at DESC
             LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(
                params![project_id, member_user_id, limit.clamp(1, 100)],
                |row| {
                    Ok(ProjectMemberConversationEntry {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        user_id: row.get(2)?,
                        user_account: row.get(3)?,
                        title: row.get(4)?,
                        status: row.get(5)?,
                        message_count: row.get(6)?,
                        task_count: row.get(7)?,
                        last_message: row.get(8)?,
                        last_message_role: row.get(9)?,
                        last_message_at: row.get(10)?,
                        last_task_status: row.get(11)?,
                        created_at: row.get(12)?,
                        updated_at: row.get(13)?,
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
        self.ensure_project_member_conversation_access(requester_id, project_id, member_user_id)?;
        let conn = self.conn()?;
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM conversations
                 WHERE project_id = ?1 AND user_id = ?2 AND id = ?3",
                params![project_id, member_user_id, conversation_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(anyhow!("成员会话不存在"));
        }

        let mut stmt = conn.prepare(
            "SELECT id, project_id, conversation_id, task_id, user_id, role, content, created_at
             FROM (
                SELECT id, project_id, conversation_id, task_id, user_id, role, content, created_at
                FROM messages
                WHERE project_id = ?1
                  AND conversation_id = ?2
                  AND user_id = ?3
                ORDER BY created_at DESC, id DESC
                LIMIT ?4
             )
             ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt
            .query_map(
                params![
                    project_id,
                    conversation_id,
                    member_user_id,
                    limit.clamp(1, 200)
                ],
                |row| {
                    Ok(ProjectMemberConversationMessage {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        conversation_id: row.get(2)?,
                        task_id: row.get(3)?,
                        user_id: row.get(4)?,
                        role: row.get(5)?,
                        content: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_member_conversation_test_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn member_conversation_messages_are_scoped_to_project_member() {
        let store = temp_store();
        let owner = store
            .create_user("owner@example.com", "secret1", Some("Owner"), None)
            .expect("owner should be created");
        let member = store
            .create_user("member@example.com", "secret1", Some("Member"), None)
            .expect("member should be created");
        let outsider = store
            .create_user("outsider@example.com", "secret1", Some("Outsider"), None)
            .expect("outsider should be created");
        let project = store
            .create_project(&owner.id, "Member Conversation Scope", None, None)
            .expect("project should be created")
            .project;
        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should be public");
        store
            .join_project(&member.id, &project.id)
            .expect("member should join project");

        store
            .create_task(&project.id, &owner.id, Some("default"), "owner request")
            .expect("owner task should be created");
        let member_task = store
            .create_task(&project.id, &member.id, Some("default"), "member request")
            .expect("member task should be created");
        store
            .finish_task(&member_task, "done", Some("member reply"), None, None)
            .expect("member task should finish");

        let conversations = store
            .list_project_member_conversations(&owner.id, &project.id, &member.id, 10)
            .expect("owner can inspect member project conversations");
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].message_count, 2);
        assert_eq!(
            conversations[0].last_message.as_deref(),
            Some("member reply")
        );

        let messages = store
            .list_project_member_conversation_messages(
                &owner.id,
                &project.id,
                &member.id,
                "default",
                10,
            )
            .expect("owner can inspect member project conversation messages");
        let contents = messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(contents, vec!["member request", "member reply"]);

        assert!(store
            .list_project_member_conversations(&outsider.id, &project.id, &member.id, 10)
            .is_err());
    }
}
