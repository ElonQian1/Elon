use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::{new_id, now, FriendChatMessage, FriendGroupMessage, Store};

pub(crate) const SOCIAL_AI_USER_ID: &str = "usr_elon_ai";
pub(crate) const SOCIAL_AI_DISPLAY_NAME: &str = "EL";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SocialAiHistoryMessage {
    pub speaker: String,
    pub content: String,
    pub from_request_user: bool,
}

impl Store {
    pub(crate) fn list_recent_friend_messages_for_social_ai(
        &self,
        user_id: &str,
        friend_id: &str,
        limit: i64,
    ) -> Result<Vec<SocialAiHistoryMessage>> {
        let conn = self.conn()?;
        ensure_friend_pair_for_social_ai(&conn, user_id, friend_id)?;
        let mut stmt = conn.prepare(
            "SELECT m.sender_user_id,
                    COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                    m.content
             FROM friend_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             WHERE (
                 (m.sender_user_id = ?1 AND m.receiver_user_id = ?2)
                 OR (m.sender_user_id = ?2 AND m.receiver_user_id = ?1)
                 OR (
                     m.sender_user_id = ?3
                     AND m.receiver_user_id = ?1
                     AND m.context_user_id = ?2
                 )
             )
             ORDER BY m.created_at DESC
             LIMIT ?4",
        )?;
        let mut messages = stmt
            .query_map(
                params![user_id, friend_id, SOCIAL_AI_USER_ID, limit.clamp(1, 30)],
                |row| {
                    let sender_user_id: String = row.get(0)?;
                    let sender_name: String = row.get(1)?;
                    Ok(history_message_from_row(
                        user_id,
                        &sender_user_id,
                        &sender_name,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        messages.reverse();
        Ok(messages)
    }

    pub(crate) fn list_recent_group_messages_for_social_ai(
        &self,
        user_id: &str,
        group_id: &str,
        limit: i64,
    ) -> Result<Vec<SocialAiHistoryMessage>> {
        let conn = self.conn()?;
        ensure_group_member_for_social_ai(&conn, user_id, group_id)?;
        let mut stmt = conn.prepare(
            "SELECT m.sender_user_id,
                    COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                    m.content
             FROM friend_group_messages m
             JOIN users u ON u.id = m.sender_user_id
             WHERE m.group_id = ?1
             ORDER BY m.created_at DESC
             LIMIT ?2",
        )?;
        let mut messages = stmt
            .query_map(params![group_id, limit.clamp(1, 30)], |row| {
                let sender_user_id: String = row.get(0)?;
                let sender_name: String = row.get(1)?;
                Ok(history_message_from_row(
                    user_id,
                    &sender_user_id,
                    &sender_name,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        messages.reverse();
        Ok(messages)
    }

    pub fn insert_friend_social_ai_reply(
        &self,
        user_id: &str,
        friend_id: &str,
        content: &str,
    ) -> Result<Vec<FriendChatMessage>> {
        let content = normalize_reply_content(content)?;
        let conn = self.conn()?;
        ensure_friend_pair_for_social_ai(&conn, user_id, friend_id)?;
        ensure_social_ai_user(&conn)?;

        let created_at = now();
        let user_message_id = new_id("fai");
        let friend_message_id = new_id("fai");
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO friend_messages (
                id, sender_user_id, receiver_user_id, context_user_id, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                user_message_id,
                SOCIAL_AI_USER_ID,
                user_id,
                friend_id,
                content,
                created_at
            ],
        )?;
        tx.execute(
            "INSERT INTO friend_messages (
                id, sender_user_id, receiver_user_id, context_user_id, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                friend_message_id,
                SOCIAL_AI_USER_ID,
                friend_id,
                user_id,
                content,
                created_at
            ],
        )?;
        tx.commit()?;

        Ok(vec![
            social_ai_friend_message(user_message_id, user_id, friend_id, &content, &created_at),
            social_ai_friend_message(friend_message_id, friend_id, user_id, &content, &created_at),
        ])
    }

    pub fn insert_group_social_ai_reply(
        &self,
        group_id: &str,
        content: &str,
    ) -> Result<FriendGroupMessage> {
        let content = normalize_reply_content(content)?;
        let conn = self.conn()?;
        ensure_group_exists(&conn, group_id)?;
        ensure_social_ai_user(&conn)?;

        let id = new_id("gai");
        let created_at = now();
        conn.execute(
            "INSERT INTO friend_group_messages (
                id, group_id, sender_user_id, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, group_id, SOCIAL_AI_USER_ID, content, created_at],
        )?;
        conn.execute(
            "UPDATE friend_groups SET updated_at = ?1 WHERE id = ?2",
            params![created_at, group_id],
        )?;

        Ok(FriendGroupMessage {
            id,
            group_id: group_id.to_string(),
            sender_user_id: SOCIAL_AI_USER_ID.to_string(),
            sender_name: SOCIAL_AI_DISPLAY_NAME.to_string(),
            content,
            attachments: Vec::new(),
            created_at,
            outgoing: false,
        })
    }
}

fn history_message_from_row(
    request_user_id: &str,
    sender_user_id: &str,
    sender_name: &str,
    content: String,
) -> SocialAiHistoryMessage {
    let from_request_user = sender_user_id == request_user_id;
    let speaker = if sender_user_id == SOCIAL_AI_USER_ID {
        SOCIAL_AI_DISPLAY_NAME.to_string()
    } else if from_request_user {
        "我".to_string()
    } else {
        sender_name.to_string()
    };
    SocialAiHistoryMessage {
        speaker,
        content,
        from_request_user,
    }
}

fn social_ai_friend_message(
    id: String,
    receiver_user_id: &str,
    context_user_id: &str,
    content: &str,
    created_at: &str,
) -> FriendChatMessage {
    FriendChatMessage {
        id,
        sender_user_id: SOCIAL_AI_USER_ID.to_string(),
        receiver_user_id: receiver_user_id.to_string(),
        sender_name: Some(SOCIAL_AI_DISPLAY_NAME.to_string()),
        content: content.to_string(),
        attachments: Vec::new(),
        created_at: created_at.to_string(),
        context_user_id: Some(context_user_id.to_string()),
        outgoing: false,
    }
}

fn ensure_social_ai_user(conn: &Connection) -> Result<()> {
    let now = now();
    conn.execute(
        "INSERT OR IGNORE INTO users (
            id, email, password_hash, nickname, role, status, created_at, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            SOCIAL_AI_USER_ID,
            "el@system.local",
            "system-ai-user",
            SOCIAL_AI_DISPLAY_NAME,
            "assistant",
            "active",
            now
        ],
    )?;
    Ok(())
}

fn ensure_friend_pair_for_social_ai(
    conn: &Connection,
    user_id: &str,
    friend_id: &str,
) -> Result<()> {
    if user_id == friend_id {
        return Err(anyhow!("不能和自己聊天"));
    }
    let exists = conn
        .query_row(
            "SELECT 1
             FROM user_friends
             WHERE user_id = ?1 AND friend_user_id = ?2
             LIMIT 1",
            params![user_id, friend_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(anyhow!("只能和已添加的好友聊天"))
    }
}

fn ensure_group_member_for_social_ai(
    conn: &Connection,
    user_id: &str,
    group_id: &str,
) -> Result<()> {
    let exists = conn
        .query_row(
            "SELECT 1
             FROM friend_group_members
             WHERE group_id = ?1 AND user_id = ?2
             LIMIT 1",
            params![group_id, user_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(anyhow!("你不在这个群聊中"))
    }
}

fn ensure_group_exists(conn: &Connection, group_id: &str) -> Result<()> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM friend_groups WHERE id = ?1 LIMIT 1",
            params![group_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(anyhow!("群聊不存在"))
    }
}

fn normalize_reply_content(content: &str) -> Result<String> {
    let content = content.trim();
    if content.is_empty() {
        return Err(anyhow!("AI 回复不能为空"));
    }
    Ok(content.chars().take(4000).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_social_ai_store_test_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn friend_ai_reply_is_visible_to_both_sides_in_context() {
        let store = temp_store();
        let alice = store
            .create_user("alice@example.com", "secret1", Some("Alice"), None)
            .expect("alice should be created");
        let bob = store
            .create_user("bob@example.com", "secret1", Some("Bob"), None)
            .expect("bob should be created");
        store
            .add_friend(&alice.id, Some("email"), "bob@example.com")
            .expect("alice can add bob");

        store
            .send_friend_message(&alice.id, &bob.id, "@EL 这句话是什么意思？", None)
            .expect("trigger message should be stored");
        let inserted = store
            .insert_friend_social_ai_reply(&alice.id, &bob.id, "这是一个解释。")
            .expect("ai reply should be inserted");
        assert_eq!(inserted.len(), 2);

        let alice_messages = store
            .list_friend_messages(&alice.id, &bob.id, None, 20)
            .expect("alice can list messages");
        let alice_ai = alice_messages.last().expect("alice sees ai reply");
        assert_eq!(alice_ai.sender_user_id, SOCIAL_AI_USER_ID);
        assert_eq!(alice_ai.context_user_id.as_deref(), Some(bob.id.as_str()));
        assert_eq!(
            alice_ai.sender_name.as_deref(),
            Some(SOCIAL_AI_DISPLAY_NAME)
        );

        let bob_messages = store
            .list_friend_messages(&bob.id, &alice.id, None, 20)
            .expect("bob can list messages");
        let bob_ai = bob_messages.last().expect("bob sees ai reply");
        assert_eq!(bob_ai.sender_user_id, SOCIAL_AI_USER_ID);
        assert_eq!(bob_ai.context_user_id.as_deref(), Some(alice.id.as_str()));
    }

    #[test]
    fn group_ai_reply_uses_el_sender() {
        let store = temp_store();
        let alice = store
            .create_user("group-alice@example.com", "secret1", Some("Alice"), None)
            .expect("alice should be created");
        let bob = store
            .create_user("group-bob@example.com", "secret1", Some("Bob"), None)
            .expect("bob should be created");
        store
            .add_friend(&alice.id, Some("email"), "group-bob@example.com")
            .expect("alice can add bob");
        let group = store
            .create_friend_group(&alice.id, Some("测试群"), &[bob.id.clone()])
            .expect("group should be created");

        store
            .insert_group_social_ai_reply(&group.id, "群聊里的 EL 回复。")
            .expect("ai group reply should be inserted");
        let messages = store
            .list_friend_group_messages(&alice.id, &group.id, None, 20)
            .expect("group messages should load");
        let ai = messages.last().expect("ai reply should be visible");
        assert_eq!(ai.sender_user_id, SOCIAL_AI_USER_ID);
        assert_eq!(ai.sender_name, SOCIAL_AI_DISPLAY_NAME);
        assert!(!ai.outgoing);
    }
}
