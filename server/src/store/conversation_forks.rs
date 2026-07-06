use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{clean_optional, new_id, now, safe_external_id, Store};

#[derive(Debug, Clone)]
pub(crate) struct ConversationForkResult {
    pub conversation_id: String,
    pub source_conversation_id: String,
    pub source_message_id: String,
    pub title: Option<String>,
    pub copied_message_count: usize,
}

struct SourceConversation {
    title: Option<String>,
    locked_agent_name: Option<String>,
    is_public: bool,
}

struct MessageCopy {
    task_id: Option<String>,
    user_id: Option<String>,
    role: String,
    content: String,
    created_at: String,
}

impl Store {
    pub(crate) fn fork_conversation_at_message(
        &self,
        project_id: &str,
        user_id: &str,
        source_conversation_id: &str,
        source_message_id: &str,
        new_conversation_id: Option<&str>,
        title: Option<&str>,
    ) -> Result<ConversationForkResult> {
        let source_conversation_id = safe_external_id(source_conversation_id, "default");
        let source_message_id = clean_optional(Some(source_message_id))
            .ok_or_else(|| anyhow!("message_id 不能为空"))?;
        let new_conversation_id = clean_optional(new_conversation_id)
            .map(|value| safe_external_id(value, "fork"))
            .unwrap_or_else(|| new_id("conv"));
        if new_conversation_id == source_conversation_id {
            return Err(anyhow!("新会话 ID 不能与源会话相同"));
        }

        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let source = tx
            .query_row(
                "SELECT title, locked_agent_name, COALESCE(is_public, 1)
                 FROM conversations
                 WHERE project_id = ?1 AND user_id = ?2 AND id = ?3",
                params![project_id, user_id, source_conversation_id],
                |row| {
                    Ok(SourceConversation {
                        title: row.get(0)?,
                        locked_agent_name: row.get(1)?,
                        is_public: row.get::<_, i64>(2)? != 0,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("源会话不存在"))?;
        let selected: (String, String) = tx
            .query_row(
                "SELECT created_at, id
                 FROM messages
                 WHERE project_id = ?1 AND conversation_id = ?2 AND id = ?3",
                params![project_id, source_conversation_id, source_message_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("源消息不存在"))?;
        let duplicate: Option<i64> = tx
            .query_row(
                "SELECT 1 FROM conversations
                 WHERE project_id = ?1 AND user_id = ?2 AND id = ?3",
                params![project_id, user_id, new_conversation_id],
                |row| row.get(0),
            )
            .optional()?;
        if duplicate.is_some() {
            return Err(anyhow!("新会话已存在"));
        }

        let messages = {
            let mut stmt = tx.prepare(
                "SELECT task_id, user_id, role, content, created_at
                 FROM messages
                 WHERE project_id = ?1
                   AND conversation_id = ?2
                   AND (created_at < ?3 OR (created_at = ?3 AND id <= ?4))
                 ORDER BY created_at ASC, id ASC",
            )?;
            let rows = stmt.query_map(
                params![
                    project_id,
                    source_conversation_id,
                    selected.0.as_str(),
                    selected.1.as_str()
                ],
                |row| {
                    Ok(MessageCopy {
                        task_id: row.get(0)?,
                        user_id: row.get(1)?,
                        role: row.get(2)?,
                        content: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        };
        if messages.is_empty() {
            return Err(anyhow!("没有可复制的消息"));
        }

        let forked_at = now();
        let title = fork_title(title, source.title.as_deref());
        tx.execute(
            "INSERT INTO conversations (
                project_id, user_id, id, title, status, created_at, updated_at, locked_agent_name, is_public
             )
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?5, ?6, ?7)",
            params![
                project_id,
                user_id,
                new_conversation_id,
                title.as_deref(),
                forked_at,
                source.locked_agent_name.as_deref(),
                if source.is_public { 1 } else { 0 }
            ],
        )?;
        for message in &messages {
            tx.execute(
                "INSERT INTO messages (
                    id, project_id, conversation_id, task_id, user_id, role, content, created_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    new_id("msg"),
                    project_id,
                    new_conversation_id,
                    message.task_id.as_deref(),
                    message.user_id.as_deref(),
                    message.role.as_str(),
                    message.content.as_str(),
                    message.created_at.as_str()
                ],
            )?;
        }
        tx.commit()?;

        Ok(ConversationForkResult {
            conversation_id: new_conversation_id,
            source_conversation_id,
            source_message_id: source_message_id.to_string(),
            title,
            copied_message_count: messages.len(),
        })
    }
}

fn fork_title(requested: Option<&str>, source: Option<&str>) -> Option<String> {
    if let Some(value) = clean_optional(requested) {
        return Some(value.chars().take(80).collect());
    }
    let base = clean_optional(source).unwrap_or("分叉会话");
    let base = base.chars().take(64).collect::<String>();
    Some(format!("{base} · 分叉"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_conversation_fork_test_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn fork_conversation_copies_messages_through_selected_message() {
        let store = temp_store();
        let user = store
            .create_user("fork-owner@example.com", "secret1", Some("Fork Owner"), None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Fork Project", None, None)
            .expect("project should be created")
            .project;
        store
            .ensure_conversation(&project.id, &user.id, Some("source"), Some("源会话"))
            .expect("conversation should be created");
        store
            .add_message(&project.id, Some("source"), None, Some(&user.id), "user", "one")
            .expect("first message should be inserted");
        store
            .add_message(
                &project.id,
                Some("source"),
                None,
                Some(&user.id),
                "assistant",
                "two",
            )
            .expect("second message should be inserted");
        store
            .add_message(&project.id, Some("source"), None, Some(&user.id), "user", "three")
            .expect("third message should be inserted");
        let source_messages = store
            .list_user_conversation_messages(&project.id, &user.id, "source", 10)
            .expect("source messages should list");

        let result = store
            .fork_conversation_at_message(
                &project.id,
                &user.id,
                "source",
                &source_messages[1].id,
                Some("forked"),
                None,
            )
            .expect("fork should succeed");

        assert_eq!(result.conversation_id, "forked");
        assert_eq!(result.copied_message_count, 2);
        assert_eq!(result.title.as_deref(), Some("源会话 · 分叉"));
        let fork_messages = store
            .list_user_conversation_messages(&project.id, &user.id, "forked", 10)
            .expect("fork messages should list");
        let contents = fork_messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(contents, vec!["one", "two"]);
    }

    #[test]
    fn fork_conversation_preserves_locked_agent() {
        let store = temp_store();
        let user = store
            .create_user("fork-agent@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Fork Agent Project", None, None)
            .expect("project should be created")
            .project;
        store
            .ensure_conversation(&project.id, &user.id, Some("source"), Some("Agent Source"))
            .expect("conversation should be created");
        store
            .set_conversation_locked_agent_if_unset(&project.id, &user.id, "source", "claude-cli")
            .expect("agent should lock");
        store
            .add_message(&project.id, Some("source"), None, Some(&user.id), "user", "hello")
            .expect("message should be inserted");
        let source_messages = store
            .list_user_conversation_messages(&project.id, &user.id, "source", 10)
            .expect("source messages should list");

        store
            .fork_conversation_at_message(
                &project.id,
                &user.id,
                "source",
                &source_messages[0].id,
                Some("forked"),
                Some("Custom Fork"),
            )
            .expect("fork should succeed");

        let locked = store
            .get_conversation_locked_agent(&project.id, &user.id, "forked")
            .expect("locked agent should load");
        assert_eq!(locked.as_deref(), Some("claude-cli"));
    }
}
